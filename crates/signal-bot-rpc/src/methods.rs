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

    /// Edit a previously sent message
    pub async fn edit_message(&self, recipient: Option<&str>, group_id: Option<&str>, message: &str, edit_timestamp: u64) -> Result<SendResult, RpcError> {
        let mut params = json!({
            "message": message,
            "editTimestamp": edit_timestamp,
        });
        if let Some(r) = recipient {
            params.as_object_mut().unwrap().insert("recipient".to_string(), json!([r]));
        } else if let Some(g) = group_id {
            params.as_object_mut().unwrap().insert("groupId".to_string(), json!(g));
        } else {
            return Err(RpcError::InvalidResponse("Missing recipient or group_id for edit_message".to_string()));
        }
        
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
    
    /// Register a new Signal account
    pub async fn register(&self, account: &str, captcha: Option<&str>, voice: bool) -> Result<(), RpcError> {
        let mut params = json!({
            "account": account,
            "voice": voice,
        });
        if let Some(c) = captcha {
            params.as_object_mut().unwrap().insert("captcha".to_string(), json!(c));
        }
        
        self.call("register", params).await?;
        Ok(())
    }

    /// Verify a newly registered Signal account
    pub async fn verify(&self, account: &str, code: &str, pin: Option<&str>) -> Result<(), RpcError> {
        let mut params = json!({
            "account": account,
            "verificationCode": code,
        });
        if let Some(p) = pin {
            params.as_object_mut().unwrap().insert("pin".to_string(), json!(p));
        }
        
        self.call("verify", params).await?;
        Ok(())
    }
}
