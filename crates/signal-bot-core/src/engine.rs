use crate::commands::CommandRouter;
use crate::context::MessageContext;
use crate::error::BotError;
use crate::rate_limit::RateLimiter;
use futures::StreamExt;
use signal_bot_rpc::SignalCliClient;
use tokio::signal;
use tracing::{error, info, warn};

pub struct Engine {
    client: SignalCliClient,
    router: CommandRouter,
    rate_limiter: Option<RateLimiter>,
    allowed_groups: Vec<String>,
}

impl Engine {
    pub fn new(
        client: SignalCliClient,
        router: CommandRouter,
        rate_limiter: Option<RateLimiter>,
        allowed_groups: Vec<String>,
    ) -> Self {
        Self {
            client,
            router,
            rate_limiter,
            allowed_groups,
        }
    }

    /// Run the bot event loop. This blocks forever (until shutdown signal).
    pub async fn run(&self) -> Result<(), BotError> {
        info!("Starting bot engine...");

        let mut messages = self.client.messages();

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("Received Ctrl-C, shutting down.");
                    break;
                }
                msg_opt = messages.next() => {
                    match msg_opt {
                        Some(Ok(envelope)) => {
                            if let Some(data) = &envelope.data_message {
                                let text = data.message.clone().unwrap_or_default();
                                if text.is_empty() {
                                    continue;
                                }

                                let group_id = data.group_info.as_ref().map(|g| g.group_id.clone());
                                let is_group = group_id.is_some();

                                if !self.allowed_groups.is_empty() {
                                    if let Some(gid) = &group_id {
                                        if !self.allowed_groups.contains(gid) {
                                            continue;
                                        }
                                    } else {
                                        continue;
                                    }
                                }

                                let sender_uuid = envelope.source_uuid.clone().unwrap_or_default();
                                let key = group_id.clone().unwrap_or_else(|| sender_uuid.clone());

                                if let Some(rl) = &self.rate_limiter {
                                    if !rl.check(&key) {
                                        warn!("Rate limit exceeded for key: {}", key);
                                        continue;
                                    }
                                }

                                let ctx = MessageContext {
                                    client: self.client.clone(),
                                    sender_uuid,
                                    sender_number: envelope.source.clone(),
                                    sender_name: envelope.source_name.clone(),
                                    group_id,
                                    text,
                                    timestamp: envelope.timestamp.unwrap_or(0),
                                    is_group,
                                };

                                if let Some(res) = self.router.route(ctx).await {
                                    if let Err(e) = res {
                                        error!("Error executing command: {}", e);
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("Error receiving message: {}", e);
                        }
                        None => {
                            error!("Message stream ended unexpectedly.");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
