use thiserror::Error;

/// RPC errors.
#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection lost")]
    ConnectionLost,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("RPC error {code}: {message}")]
    RpcError { code: i64, message: String },

    #[error("Request timed out")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
