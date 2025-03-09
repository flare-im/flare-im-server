use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: String,
    pub sender_id: String,
    pub content_type: String,
    pub content: String,
    pub status: MessageStatus,
    pub metadata: MessageMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub device_id: String,
    pub client_msg_id: String,
    pub reply_to: Option<Uuid>,
    pub mentions: Vec<String>,
    pub is_encrypted: bool,
    pub compression: Option<String>,
    pub custom_properties: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQuery {
    pub session_id: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: u32,
    pub offset: u32,
    pub content_types: Option<Vec<String>>,
    pub sender_id: Option<String>,
    pub status: Option<MessageStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatch {
    pub messages: Vec<Message>,
    pub total_count: u64,
    pub has_more: bool,
} 