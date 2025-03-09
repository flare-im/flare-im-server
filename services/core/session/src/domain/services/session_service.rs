use async_trait::async_trait;
use crate::domain::entities::session::{Session, SessionMember, OnlineStatus, DeviceInfo};

#[async_trait]
pub trait SessionService {
    // 会话生命周期管理
    async fn create_session(&self, session: Session) -> Result<Session, Error>;
    async fn close_session(&self, session_id: &str) -> Result<(), Error>;
    
    // 会话连接管理
    async fn connect(&self, session_id: &str, user_id: &str, device_info: DeviceInfo) -> Result<(), Error>;
    async fn disconnect(&self, session_id: &str, user_id: &str, device_id: &str) -> Result<(), Error>;
    async fn heartbeat(&self, session_id: &str, user_id: &str, device_id: &str) -> Result<(), Error>;
    
    // 会话状态同步
    async fn sync_session_state(&self, session_id: &str, user_id: &str) -> Result<SessionState, Error>;
    async fn sync_unread_messages(&self, session_id: &str, user_id: &str) -> Result<Vec<UnreadMessage>, Error>;
    
    // 会话恢复
    async fn recover_session(&self, user_id: &str, device_info: DeviceInfo) -> Result<Vec<Session>, Error>;
}

#[derive(Debug)]
pub struct SessionState {
    pub session: Session,
    pub online_members: Vec<SessionMember>,
    pub last_sync_time: i64,
}

#[derive(Debug)]
pub struct UnreadMessage {
    pub message_id: String,
    pub sender_id: String,
    pub content_type: String,
    pub content_preview: String,
    pub sent_at: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Session error: {0}")]
    Session(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Sync error: {0}")]
    Sync(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
} 