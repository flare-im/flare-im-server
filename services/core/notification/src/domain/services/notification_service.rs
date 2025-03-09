use async_trait::async_trait;
use crate::domain::entities::notification::{
    Notification, NotificationResult, DeviceInfo, NotificationTemplate,
    Platform, NotificationType,
};

#[async_trait]
pub trait NotificationService {
    // 发送通知
    async fn send_notification(&self, notification: Notification) -> Result<NotificationResult, Error>;
    async fn batch_send_notifications(&self, notifications: Vec<Notification>) -> Result<Vec<NotificationResult>, Error>;
    
    // 设备管理
    async fn register_device(&self, device_info: DeviceInfo) -> Result<(), Error>;
    async fn unregister_device(&self, user_id: &str, device_id: &str) -> Result<(), Error>;
    async fn update_device_token(&self, user_id: &str, device_id: &str, new_token: &str) -> Result<(), Error>;
    
    // 模板管理
    async fn create_template(&self, template: NotificationTemplate) -> Result<NotificationTemplate, Error>;
    async fn update_template(&self, template: NotificationTemplate) -> Result<NotificationTemplate, Error>;
    async fn delete_template(&self, template_id: &str) -> Result<(), Error>;
    async fn get_template(&self, template_id: &str) -> Result<Option<NotificationTemplate>, Error>;
    
    // 通知状态管理
    async fn cancel_notification(&self, notification_id: &str) -> Result<(), Error>;
    async fn get_notification_status(&self, notification_id: &str) -> Result<Option<NotificationResult>, Error>;
    
    // 统计查询
    async fn get_user_notifications(
        &self,
        user_id: &str,
        notification_type: Option<NotificationType>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Notification>, Error>;
    
    async fn get_platform_statistics(
        &self,
        platform: Platform,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<PlatformStatistics, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Service error: {0}")]
    Service(String),
    
    #[error("Provider error: {0}")]
    Provider(String),
    
    #[error("Template error: {0}")]
    Template(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
}

#[derive(Debug)]
pub struct PlatformStatistics {
    pub total_sent: u64,
    pub total_failed: u64,
    pub success_rate: f64,
    pub average_latency: f64,
    pub error_counts: std::collections::HashMap<String, u64>,
} 