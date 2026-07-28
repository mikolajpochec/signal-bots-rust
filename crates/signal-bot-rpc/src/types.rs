use serde::{Deserialize, Serialize};

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Incoming envelope from signal-cli
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    pub source: Option<String>,
    pub source_uuid: Option<String>,
    pub source_name: Option<String>,
    pub timestamp: Option<u64>,
    pub data_message: Option<DataMessage>,
    pub sync_message: Option<SyncMessage>,
    pub receipt_message: Option<serde_json::Value>,
    pub typing_message: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataMessage {
    pub message: Option<String>,
    pub timestamp: Option<u64>,
    pub group_info: Option<GroupInfo>,
    pub attachments: Option<Vec<Attachment>>,
    pub reaction: Option<Reaction>,
    pub quote: Option<Quote>,
    pub mentions: Option<Vec<Mention>>,
    pub expires_in_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMessage {
    pub sent_message: Option<SentMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentMessage {
    pub destination: Option<String>,
    pub destination_uuid: Option<String>,
    pub timestamp: Option<u64>,
    pub message: Option<String>,
    pub group_info: Option<GroupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfo {
    pub group_id: String,
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub id: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    pub emoji: String,
    pub target_author: Option<String>,
    pub target_sent_timestamp: Option<u64>,
    pub is_remove: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub id: Option<u64>,
    pub author: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mention {
    pub start: Option<u32>,
    pub length: Option<u32>,
    pub uuid: Option<String>,
}

/// Group info returned by listGroups
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_member: Option<bool>,
    pub is_admin: Option<bool>,
    pub members: Option<Vec<GroupMember>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
    pub uuid: String,
    pub number: Option<String>,
}

/// Contact info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub uuid: Option<String>,
    pub number: Option<String>,
    pub name: Option<String>,
    pub profile_name: Option<String>,
}

/// Result of sending a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResult {
    pub timestamp: Option<u64>,
}
