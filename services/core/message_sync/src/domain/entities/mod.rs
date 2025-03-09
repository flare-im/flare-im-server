use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPoint {
    pub user_id: String,
    pub device_id: String,
    pub sequence: i64,
    pub sync_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageOperation {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: String,
    pub device_id: String,
    pub operation_type: OperationType,
    pub operation_data: Vec<u8>,
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Recall,
    Delete,
    Edit,
    Pin,
    Unpin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStatus {
    pub message_id: Uuid,
    pub delivery_status: DeliveryStatus,
    pub read_status: ReadStatus,
    pub received_by: Vec<String>,
    pub read_by: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadStatus {
    Unread,
    Read,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: String,
    pub content: String,
    pub content_type: String,
    pub metadata: MessageMetadata,
    pub status: MessageStatus,
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub custom_data: HashMap<String, String>,
    pub mentions: Vec<String>,
    pub reply_to: Option<String>,
    pub forward_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub conversation_type: ConversationType,
    pub name: String,
    pub avatar: String,
    pub members: Vec<String>,
    pub owner_id: String,
    pub settings: ConversationSettings,
    pub last_message: Option<Message>,
    pub unread_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationType {
    Private,
    Group,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSettings {
    pub mute: bool,
    pub stick_on_top: bool,
    pub join_approval_required: bool,
    pub only_owner_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatus {
    pub user_id: String,
    pub online_status: OnlineStatus,
    pub device_id: String,
    pub last_active_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnlineStatus {
    Online,
    Offline,
    Away,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub messages: Vec<Message>,
    pub conversations: Vec<Conversation>,
    pub operations: Vec<MessageOperation>,
    pub user_statuses: Vec<UserStatus>,
    pub current_sequence: i64,
    pub sync_time: DateTime<Utc>,
    pub has_more: bool,
} 