use serde_json::json;

use crate::client::SignalCliClient;
use crate::error::RpcError;
use crate::types::{Contact, Group, SendResult};

impl SignalCliClient {
    /// Send a message to a single recipient
    pub async fn send_message(&self, recipient: &str, message: &str, attachments: &[&str]) -> Result<SendResult, RpcError> {
        let params = json!({
            "recipient": [recipient],
            "message": message,
            "attachment": attachments,
        });
        
        let result = self.call("send", params).await?;
        serde_json::from_value(result).map_err(Into::into)
    }
    
    /// Send a message to a group
    pub async fn send_group_message(&self, group_id: &str, message: &str, attachments: &[&str]) -> Result<SendResult, RpcError> {
        let params = json!({
            "groupId": group_id,
            "message": message,
            "attachment": attachments,
        });
        
        let result = self.call("send", params).await?;
        serde_json::from_value(result).map_err(Into::into)
    }
    
    /// Send a reaction to a message
    pub async fn send_reaction(&self, recipient: &str, emoji: &str, target_author: &str, target_timestamp: u64) -> Result<(), RpcError> {
        let params = json!({
            "recipient": recipient,
            "reaction": emoji,
            "targetAuthor": target_author,
            "targetTimestamp": target_timestamp,
        });
        
        self.call("sendReaction", params).await?;
        Ok(())
    }
    
    /// List all groups
    pub async fn list_groups(&self) -> Result<Vec<Group>, RpcError> {
        let result = self.call("listGroups", json!({})).await?;
        serde_json::from_value(result).map_err(Into::into)
    }
    
    /// List contacts
    pub async fn list_contacts(&self) -> Result<Vec<Contact>, RpcError> {
        let result = self.call("listContacts", json!({})).await?;
        serde_json::from_value(result).map_err(Into::into)
    }
    
    /// Get the registered account number/uuid
    pub async fn whoami(&self) -> Result<String, RpcError> {
        let result = self.call("getUser", json!({})).await?;
        
        if let Some(obj) = result.as_object() {
            if let Some(number) = obj.get("number").and_then(|n| n.as_str()) {
                return Ok(number.to_string());
            }
            if let Some(uuid) = obj.get("uuid").and_then(|u| u.as_str()) {
                return Ok(uuid.to_string());
            }
        }
        
        Err(RpcError::InvalidResponse("getUser didn't return a valid string".to_string()))
    }
}
