use crate::error::BotError;

/// Context for an incoming message, providing methods to reply, react, etc.
pub struct MessageContext {
    /// The signal-cli RPC client
    pub client: signal_bot_rpc::SignalCliClient,
    /// Sender's UUID
    pub sender_uuid: String,
    /// Sender's phone number (if available)
    pub sender_number: Option<String>,
    /// Sender's profile name (if available)
    pub sender_name: Option<String>,
    /// Group ID (None for direct messages)
    pub group_id: Option<String>,
    /// The message text
    pub text: String,
    /// Message timestamp
    pub timestamp: u64,
    /// Whether this is a group message
    pub is_group: bool,
}

impl MessageContext {
    /// Reply to the message (sends to the same conversation)
    pub async fn reply(&self, text: &str) -> Result<(), BotError> {
        if self.is_group {
            if let Some(group_id) = &self.group_id {
                self.client.send_group_message(group_id, text, &[]).await.map_err(BotError::Rpc)?;
            }
        } else {
            self.client.send_message(&self.sender_uuid, text, &[]).await.map_err(BotError::Rpc)?;
        }
        Ok(())
    }

    /// React to the message with an emoji
    pub async fn react(&self, emoji: &str) -> Result<(), BotError> {
        let recipient = if self.is_group {
            self.group_id.clone().unwrap_or_default()
        } else {
            self.sender_uuid.clone()
        };
        self.client
            .send_reaction(&recipient, emoji, &self.sender_uuid, self.timestamp)
            .await
            .map_err(BotError::Rpc)?;
        Ok(())
    }
}
