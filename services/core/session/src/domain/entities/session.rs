use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub session_type: SessionType,
    pub name: String,
    pub avatar_url: Option<String>,
    pub members: Vec<SessionMember>,
    pub latest_message: Option<LatestMessage>,
    pub unread_count: i32,
    pub settings: SessionSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Private,
    Group,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMember {
    pub user_id: String,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub device_info: DeviceInfo,
    pub online_status: OnlineStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub platform: Platform,
    pub gateway_id: String,
    pub connection_id: String,
    pub connected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    IOS,
    Android,
    Web,
    Desktop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnlineStatus {
    Online,
    Offline,
    Away,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestMessage {
    pub message_id: Uuid,
    pub sender_id: String,
    pub content_type: String,
    pub content_preview: String,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    pub mute_notification: bool,
    pub stick_on_top: bool,
    pub encryption_enabled: bool,
    pub auto_delete_after: Option<i32>,
    pub custom_settings: std::collections::HashMap<String, String>,
} 