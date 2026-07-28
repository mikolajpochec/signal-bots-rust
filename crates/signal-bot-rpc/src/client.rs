use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use futures::{Stream, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UnixStream};
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, warn};

use crate::error::RpcError;
use crate::types::{Envelope, JsonRpcRequest, JsonRpcResponse};

/// Transport method for connecting to signal-cli daemon.
#[derive(Debug, Clone)]
pub enum Transport {
    /// Connect via Unix domain socket.
    Unix(PathBuf),
    /// Connect via TCP (e.g. "localhost:7583").
    Tcp(String),
}

/// Client for communicating with signal-cli's JSON-RPC daemon.
///
/// This client is cheaply cloneable (uses `Arc` internally).
#[derive(Clone)]
pub struct SignalCliClient {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    request_id: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<JsonRpcResponse, RpcError>>>>,
    tx_write: mpsc::Sender<String>,
    /// Broadcast channel sends Envelope directly (Envelope impls Clone).
    tx_events: broadcast::Sender<Envelope>,
    is_connected: AtomicBool,
}

impl SignalCliClient {
    /// Connect to a signal-cli daemon via the specified transport.
    pub async fn connect(transport: Transport) -> Result<Self, RpcError> {
        let (tx_events, _) = broadcast::channel::<Envelope>(1024);
        let (tx_write, mut rx_write) = mpsc::channel::<String>(100);

        let inner = Arc::new(ClientInner {
            request_id: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            tx_write,
            tx_events: tx_events.clone(),
            is_connected: AtomicBool::new(true),
        });

        let client = Self { inner };
        let client_clone = client.clone();
        let client_clone2 = client.clone();

        match transport {
            Transport::Unix(path) => {
                let stream = UnixStream::connect(&path).await.map_err(|e| {
                    RpcError::ConnectionFailed(format!("Unix socket {}: {}", path.display(), e))
                })?;
                let (r, mut w) = tokio::io::split(stream);

                // Writer task
                tokio::spawn(async move {
                    while let Some(msg) = rx_write.recv().await {
                        let mut data = msg;
                        data.push('\n');
                        if w.write_all(data.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                });

                // Reader task
                tokio::spawn(async move {
                    let mut reader = BufReader::new(r).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        client_clone.handle_line(&line).await;
                    }
                    client_clone.mark_disconnected().await;
                });
            }
            Transport::Tcp(addr) => {
                let stream = TcpStream::connect(&addr).await.map_err(|e| {
                    RpcError::ConnectionFailed(format!("TCP {}: {}", addr, e))
                })?;
                let (r, mut w) = tokio::io::split(stream);

                // Writer task
                tokio::spawn(async move {
                    while let Some(msg) = rx_write.recv().await {
                        let mut data = msg;
                        data.push('\n');
                        if w.write_all(data.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                });

                // Reader task
                tokio::spawn(async move {
                    let mut reader = BufReader::new(r).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        client_clone2.handle_line(&line).await;
                    }
                    client_clone2.mark_disconnected().await;
                });
            }
        }

        Ok(client)
    }

    /// Convenience constructor for Unix socket transport.
    pub async fn connect_unix(path: impl Into<PathBuf>) -> Result<Self, RpcError> {
        Self::connect(Transport::Unix(path.into())).await
    }

    /// Convenience constructor for TCP transport.
    pub async fn connect_tcp(addr: &str) -> Result<Self, RpcError> {
        Self::connect(Transport::Tcp(addr.to_string())).await
    }

    async fn mark_disconnected(&self) {
        self.inner.is_connected.store(false, Ordering::SeqCst);
        warn!("signal-cli connection lost");

        // Resolve all pending requests with ConnectionLost
        let mut pending = self.inner.pending.lock().await;
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err(RpcError::ConnectionLost));
        }
    }

    async fn handle_line(&self, line: &str) {
        if line.trim().is_empty() {
            return;
        }

        tracing::info!("RAW JSON-RPC: {}", line);

        // Try to parse as JSON-RPC response (has an id field)
        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(line) {
            if let Some(id) = resp.id {
                debug!(id, "received RPC response");
                let mut pending = self.inner.pending.lock().await;
                if let Some(tx) = pending.remove(&id) {
                    let _ = tx.send(Ok(resp));
                }
                return;
            }
        }

        // Try to parse as notification (no id — signal-cli sends "receive" notifications)
        if let Ok(val) = serde_json::from_str::<Value>(line) {
            if val.get("method").and_then(|m| m.as_str()) == Some("receive") {
                if let Some(params) = val.get("params") {
                    if let Some(envelope_val) = params.get("envelope") {
                        match serde_json::from_value::<Envelope>(envelope_val.clone()) {
                            Ok(envelope) => {
                                debug!("received envelope from {:?}", envelope.source_uuid);
                                let _ = self.inner.tx_events.send(envelope);
                            }
                            Err(e) => {
                                warn!("failed to parse envelope: {}", e);
                            }
                        }
                        return;
                    }
                }
            }
            debug!("unhandled JSON-RPC message");
        }
    }

    /// Send a raw JSON-RPC request and await the response.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        if !self.is_connected() {
            return Err(RpcError::ConnectionLost);
        }

        let id = self.inner.request_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: method.to_string(),
            params: Some(params),
        };

        let json = serde_json::to_string(&req)?;
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.inner.pending.lock().await;
            pending.insert(id, tx);
        }

        self.inner
            .tx_write
            .send(json)
            .await
            .map_err(|_| RpcError::ConnectionLost)?;

        let resp = match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(_)) => return Err(RpcError::ConnectionLost),
            Err(_) => {
                let mut pending = self.inner.pending.lock().await;
                pending.remove(&id);
                return Err(RpcError::Timeout);
            }
        }?;

        if let Some(err) = resp.error {
            return Err(RpcError::RpcError {
                code: err.code,
                message: err.message,
            });
        }

        resp.result
            .ok_or_else(|| RpcError::InvalidResponse("Missing result in response".to_string()))
    }

    /// Get a stream of incoming message envelopes (notifications from signal-cli daemon).
    pub fn messages(&self) -> impl Stream<Item = Result<Envelope, RpcError>> + '_ {
        let rx = self.inner.tx_events.subscribe();
        BroadcastStream::new(rx).map(|res| match res {
            Ok(envelope) => Ok(envelope),
            Err(_) => Err(RpcError::ConnectionLost), // Lag or closed
        })
    }

    /// Check if the connection is alive.
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected.load(Ordering::SeqCst)
    }
}
