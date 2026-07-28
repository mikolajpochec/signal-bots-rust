use crate::commands::CommandRouter;
use crate::context::MessageContext;
use crate::error::BotError;
use crate::rate_limit::RateLimiter;
use futures::StreamExt;
use signal_bot_rpc::SignalCliClient;
use signal_bot_plugins::PluginManager;
use tokio::signal;
use tracing::{error, info, warn};

pub struct Engine {
    client: SignalCliClient,
    router: CommandRouter,
    rate_limiter: Option<RateLimiter>,
    allowed_groups: Vec<String>,
    plugin_manager: Option<PluginManager>,
    start_time: std::time::Instant,
}

impl Engine {
    pub fn new(
        client: SignalCliClient,
        router: CommandRouter,
        rate_limiter: Option<RateLimiter>,
        allowed_groups: Vec<String>,
        plugin_manager: Option<PluginManager>,
    ) -> Self {
        Self {
            client,
            router,
            rate_limiter,
            allowed_groups,
            plugin_manager,
            start_time: std::time::Instant::now(),
        }
    }

    /// Run the bot event loop. This blocks forever (until shutdown signal).
    pub async fn run(&self) -> Result<(), BotError> {
        info!("Starting bot engine...");

        if let Some(pm) = &self.plugin_manager {
            pm.load_persisted_reminders(self.client.clone());
            
            // Broadcast on_spawn event
            let sys_ctx = signal_bot_plugins::lua_api::PluginContext {
                client: self.client.clone(),
                trigger: String::new(),
                sender_uuid: String::new(),
                sender_number: None,
                sender_name: None,
                group_id: None,
                text: String::new(),
                timestamp: 0,
                is_group: false,
                args: vec![],
                bot_uptime: 0,
                allowed_groups: self.allowed_groups.clone(),
                reaction_emoji: None,
                reaction_target_author: None,
                reaction_target_timestamp: None,
                reaction_is_remove: None,
            };
            pm.broadcast_lifecycle("on_spawn", sys_ctx).await;
        }

        // Subscribe to receive messages from the daemon over JSON-RPC
        if let Err(e) = self.client.call("subscribeReceive", serde_json::json!({})).await {
            warn!("Failed to subscribe to receive (might already be subscribed by on-start): {}", e);
        }

        let mut messages = self.client.messages();

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("Received Ctrl-C, shutting down.");
                    if let Some(pm) = &self.plugin_manager {
                        let sys_ctx = signal_bot_plugins::lua_api::PluginContext {
                            client: self.client.clone(),
                            trigger: String::new(),
                            sender_uuid: String::new(),
                            sender_number: None,
                            sender_name: None,
                            group_id: None,
                            text: String::new(),
                            timestamp: 0,
                            is_group: false,
                            args: vec![],
                            bot_uptime: self.start_time.elapsed().as_secs(),
                            allowed_groups: self.allowed_groups.clone(),
                            reaction_emoji: None,
                            reaction_target_author: None,
                            reaction_target_timestamp: None,
                            reaction_is_remove: None,
                        };
                        pm.broadcast_lifecycle("on_death", sys_ctx).await;
                    }
                    break;
                }
                msg_opt = messages.next() => {
                    match msg_opt {
                        Some(Ok(envelope)) => {
                            if let Some(data) = &envelope.data_message {
                                let text = data.message.clone().unwrap_or_default();
                                let has_reaction = data.reaction.is_some();
                                if text.is_empty() && !has_reaction {
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
                                    sender_uuid: sender_uuid.clone(),
                                    sender_number: envelope.source.clone(),
                                    sender_name: envelope.source_name.clone(),
                                    group_id: group_id.clone(),
                                    text: text.clone(),
                                    timestamp: envelope.timestamp.unwrap_or(0),
                                    is_group,
                                };

                                let (cmd_result, fallback) = self.router.route(ctx).await;
                                match cmd_result {
                                    Some(Ok(())) => {}
                                    Some(Err(e)) => {
                                        error!("Error executing command: {}", e);
                                    }
                                    None => {
                                        // No built-in command matched — try plugins
                                        if let Some((ctx, args_with_trigger)) = fallback {
                                            if let Some(pm) = &self.plugin_manager {
                                                // args_with_trigger[0] is the trigger, rest are args
                                                if !args_with_trigger.is_empty() {
                                                    let trigger = &args_with_trigger[0];
                                                    let args = args_with_trigger[1..].to_vec();
                                                    if pm.has_plugin(trigger) {
                                                        let plugin_ctx = signal_bot_plugins::lua_api::PluginContext {
                                                            client: self.client.clone(),
                                                            trigger: trigger.to_string(),
                                                            sender_uuid: ctx.sender_uuid,
                                                            sender_number: ctx.sender_number,
                                                            sender_name: ctx.sender_name,
                                                            group_id: ctx.group_id,
                                                            text: ctx.text,
                                                            timestamp: ctx.timestamp,
                                                            is_group: ctx.is_group,
                                                            args,
                                                            bot_uptime: self.start_time.elapsed().as_secs(),
                                                            allowed_groups: self.allowed_groups.clone(),
                                                            reaction_emoji: None,
                                                            reaction_target_author: None,
                                                            reaction_target_timestamp: None,
                                                            reaction_is_remove: None,
                                                        };
                                                        if let Some(Err(e)) = pm.execute(trigger, plugin_ctx).await {
                                                            error!("Plugin error: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(reaction) = &data.reaction {
                                    if let Some(pm) = &self.plugin_manager {
                                        let plugin_ctx = signal_bot_plugins::lua_api::PluginContext {
                                            client: self.client.clone(),
                                            trigger: String::new(),
                                            sender_uuid: sender_uuid.clone(),
                                            sender_number: envelope.source.clone(),
                                            sender_name: envelope.source_name.clone(),
                                            group_id: group_id.clone(),
                                            text: text.clone(),
                                            timestamp: envelope.timestamp.unwrap_or(0),
                                            is_group,
                                            args: vec![],
                                            bot_uptime: self.start_time.elapsed().as_secs(),
                                            allowed_groups: self.allowed_groups.clone(),
                                            reaction_emoji: Some(reaction.emoji.clone()),
                                            reaction_target_author: reaction.target_author.clone(),
                                            reaction_target_timestamp: reaction.target_sent_timestamp,
                                            reaction_is_remove: reaction.is_remove,
                                        };
                                        pm.broadcast_reaction(plugin_ctx).await;
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
