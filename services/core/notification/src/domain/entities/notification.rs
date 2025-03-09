use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub notification_type: NotificationType,
    pub priority: Priority,
    pub target_type: TargetType,
    pub target_users: Vec<String>,
    pub platform: Vec<Platform>,
    pub status: NotificationStatus,
    pub metadata: NotificationMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMetadata {
    pub category: String,
    pub badge: Option<i32>,
    pub sound: Option<String>,
    pub image_url: Option<String>,
    pub deep_link: Option<String>,
    pub custom_data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Message,      // 消息通知
    System,       // 系统通知
    Activity,     // 活动通知
    Custom(String), // 自定义通知
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetType {
    Single,     // 单个用户
    Multiple,   // 多个用户
    Topic,      // 主题
    Broadcast,  // 广播
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    IOS,
    Android,
    Web,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,    // 待发送
    Scheduled,  // 定时发送
    Sending,    // 发送中
    Sent,       // 已发送
    Failed,     // 发送失败
    Cancelled,  // 已取消
    Expired,    // 已过期
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub notification_id: Uuid,
    pub success: bool,
    pub platform_results: Vec<PlatformResult>,
    pub sent_count: u32,
    pub failed_count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformResult {
    pub platform: Platform,
    pub provider: String,
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub user_id: String,
    pub device_id: String,
    pub platform: Platform,
    pub push_token: String,
    pub app_version: String,
    pub provider: String,
    pub is_active: bool,
    pub last_active_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub name: String,
    pub title_template: String,
    pub content_template: String,
    pub category: String,
    pub platform: Vec<Platform>,
    pub metadata: NotificationMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
} 