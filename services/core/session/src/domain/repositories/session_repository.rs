use async_trait::async_trait;
use crate::domain::entities::session::{Session, SessionMember, OnlineStatus};

#[async_trait]
pub trait SessionRepository {
    // 会话基本操作
    async fn save(&self, session: Session) -> Result<(), Error>;
    async fn get_by_id(&self, session_id: &str) -> Result<Option<Session>, Error>;
    async fn delete(&self, session_id: &str) -> Result<(), Error>;
    
    // 会话列表查询
    async fn get_user_sessions(&self, user_id: &str, limit: u32, offset: u32) -> Result<Vec<Session>, Error>;
    async fn get_sessions_by_type(&self, session_type: &str, limit: u32, offset: u32) -> Result<Vec<Session>, Error>;
    
    // 会话成员操作
    async fn add_member(&self, session_id: &str, member: SessionMember) -> Result<(), Error>;
    async fn remove_member(&self, session_id: &str, user_id: &str) -> Result<(), Error>;
    async fn update_member_status(&self, session_id: &str, user_id: &str, status: OnlineStatus) -> Result<(), Error>;
    
    // 会话状态操作
    async fn update_latest_message(&self, session_id: &str, message_id: &str, preview: &str) -> Result<(), Error>;
    async fn increment_unread_count(&self, session_id: &str, user_id: &str) -> Result<i32, Error>;
    async fn reset_unread_count(&self, session_id: &str, user_id: &str) -> Result<(), Error>;
    
    // 会话设置操作
    async fn update_settings(&self, session_id: &str, settings: std::collections::HashMap<String, String>) -> Result<(), Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),
    
    #[error("Session not found: {0}")]
    NotFound(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    #[error("Duplicate session: {0}")]
    Duplicate(String),
    
    #[error("Member not found: {0}")]
    MemberNotFound(String),
} 