use async_trait::async_trait;
use crate::domain::entities::notification::{
    Notification, NotificationResult, Platform, PlatformResult,
};

#[async_trait]
pub trait PushProvider {
    // 获取提供商名称
    fn get_provider_name(&self) -> &str;
    
    // 获取支持的平台
    fn get_supported_platforms(&self) -> Vec<Platform>;
    
    // 初始化提供商
    async fn initialize(&self) -> Result<(), Error>;
    
    // 发送推送
    async fn send_push(&self, notification: &Notification) -> Result<PlatformResult, Error>;
    
    // 批量发送推送
    async fn batch_send_push(&self, notifications: &[Notification]) -> Result<Vec<PlatformResult>, Error>;
    
    // 取消推送
    async fn cancel_push(&self, message_id: &str) -> Result<(), Error>;
    
    // 查询推送状态
    async fn get_push_status(&self, message_id: &str) -> Result<Option<PlatformResult>, Error>;
    
    // 验证推送令牌
    async fn validate_push_token(&self, token: &str, platform: Platform) -> Result<bool, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Provider initialization error: {0}")]
    Initialization(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    
    #[error("Send error: {0}")]
    SendError(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
    
    #[error("Provider error: {0}")]
    Provider(String),
} 