use crate::domain::{
    entities::session::{Session, SessionMember, DeviceInfo, OnlineStatus},
    repositories::session_repository::SessionRepository,
    services::session_service::{SessionService, SessionState, UnreadMessage},
};
use chrono::Utc;
use uuid::Uuid;

pub struct SessionManager<R: SessionRepository, S: SessionService> {
    session_repository: R,
    session_service: S,
}

impl<R: SessionRepository, S: SessionService> SessionManager<R, S> {
    pub fn new(session_repository: R, session_service: S) -> Self {
        Self {
            session_repository,
            session_service,
        }
    }

    // 创建新会话
    pub async fn create_session(&self, session: Session) -> Result<Session, Error> {
        // 验证会话数据
        self.validate_session(&session)?;

        // 创建会话
        let session = self.session_service.create_session(session).await?;
        
        // 保存到仓储
        self.session_repository.save(session.clone()).await?;

        Ok(session)
    }

    // 处理用户连接
    pub async fn handle_connection(&self, session_id: &str, user_id: &str, device_info: DeviceInfo) -> Result<(), Error> {
        // 验证会话存在
        let session = self.session_repository.get_by_id(session_id).await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        // 建立连接
        self.session_service.connect(session_id, user_id, device_info.clone()).await?;

        // 更新成员状态
        self.session_repository
            .update_member_status(session_id, user_id, OnlineStatus::Online)
            .await?;

        Ok(())
    }

    // 处理用户断开连接
    pub async fn handle_disconnection(&self, session_id: &str, user_id: &str, device_id: &str) -> Result<(), Error> {
        // 处理断开连接
        self.session_service.disconnect(session_id, user_id, device_id).await?;

        // 更新成员状态
        self.session_repository
            .update_member_status(session_id, user_id, OnlineStatus::Offline)
            .await?;

        Ok(())
    }

    // 同步会话状态
    pub async fn sync_session(&self, session_id: &str, user_id: &str) -> Result<SessionState, Error> {
        // 同步会话状态
        let state = self.session_service.sync_session_state(session_id, user_id).await?;

        // 重置未读消息计数
        self.session_repository.reset_unread_count(session_id, user_id).await?;

        Ok(state)
    }

    // 恢复用户会话
    pub async fn recover_user_sessions(&self, user_id: &str, device_info: DeviceInfo) -> Result<Vec<Session>, Error> {
        // 恢复会话列表
        let sessions = self.session_service.recover_session(user_id, device_info).await?;

        // 更新会话状态
        for session in sessions.iter() {
            self.session_repository
                .update_member_status(&session.id, user_id, OnlineStatus::Online)
                .await?;
        }

        Ok(sessions)
    }

    // 处理心跳
    pub async fn handle_heartbeat(&self, session_id: &str, user_id: &str, device_id: &str) -> Result<(), Error> {
        self.session_service.heartbeat(session_id, user_id, device_id).await?;
        Ok(())
    }

    // 更新会话最新消息
    pub async fn update_latest_message(&self, session_id: &str, message_id: &str, preview: &str) -> Result<(), Error> {
        self.session_repository.update_latest_message(session_id, message_id, preview).await?;
        Ok(())
    }

    // 验证会话数据
    fn validate_session(&self, session: &Session) -> Result<(), Error> {
        if session.id.is_empty() {
            return Err(Error::ValidationError("Session ID is required".to_string()));
        }
        if session.members.is_empty() {
            return Err(Error::ValidationError("Session must have at least one member".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(#[from] crate::domain::repositories::session_repository::Error),
    
    #[error("Service error: {0}")]
    Service(#[from] crate::domain::services::session_service::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
} 