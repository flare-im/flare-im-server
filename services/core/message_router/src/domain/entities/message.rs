use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: String,
    pub sender_id: String,
    pub content_type: MessageContentType,
    pub content: Vec<u8>,
    pub metadata: MessageMetadata,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContentType {
    Text,
    Image,
    Video,
    Audio,
    File,
    Location,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub priority: MessagePriority,
    pub qos_level: QosLevel,
    pub need_receipt: bool,
    pub need_offline_storage: bool,
    pub need_offline_push: bool,
    pub extra: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessagePriority {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QosLevel {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
} 