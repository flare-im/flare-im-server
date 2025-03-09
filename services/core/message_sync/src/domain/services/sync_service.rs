use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::entities::*;

#[async_trait]
pub trait SyncService: Send + Sync {
    // 基本同步
    async fn sync(&self, user_id: &str, device_id: &str, last_sequence: i64, sync_type: SyncType) -> Result<SyncResult, Error>;
    
    // 增量同步
    async fn incremental_sync(&self, user_id: &str, device_id: &str, last_sequence: i64, limit: i32) -> Result<SyncResult, Error>;
    
    // 全量同步
    async fn full_sync(&self, user_id: &str, device_id: &str, limit: i32, offset: i32) -> Result<SyncResult, Error>;
    
    // 消息操作
    async fn message_operation(&self, operation: MessageOperation) -> Result<i64, Error>;
    
    // 状态同步
    async fn sync_status(&self, user_id: &str, device_id: &str, message_ids: Vec<Uuid>, status_type: StatusType) -> Result<Vec<MessageStatus>, Error>;
    
    // 获取序列号
    async fn get_sequence(&self, conversation_id: Uuid, count: i32) -> Result<(i64, i64), Error>;
    
    // 更新同步点
    async fn update_sync_point(&self, sync_point: SyncPoint) -> Result<(), Error>;
    
    // 获取同步点
    async fn get_sync_point(&self, user_id: &str, device_id: &str) -> Result<Option<SyncPoint>, Error>;
}

#[derive(Debug, Clone)]
pub enum SyncType {
    Incremental,
    Full,
    Quick,
}

#[derive(Debug, Clone)]
pub enum StatusType {
    Delivery,
    Read,
    Online,
    Typing,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Service error: {0}")]
    Service(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Sequence error: {0}")]
    SequenceError(String),

    #[error("Conflict error: {0}")]
    ConflictError(String),

    #[error("Storage error: {0}")]
    StorageError(String),
} 