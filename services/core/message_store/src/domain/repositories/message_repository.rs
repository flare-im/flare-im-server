use async_trait::async_trait;
use crate::domain::entities::message::{Message, MessageQuery, MessageBatch};

#[async_trait]
pub trait MessageRepository {
    // 基本操作
    async fn save(&self, message: Message) -> Result<(), Error>;
    async fn get_by_id(&self, message_id: &str) -> Result<Option<Message>, Error>;
    async fn delete(&self, message_id: &str) -> Result<(), Error>;
    
    // 批量操作
    async fn batch_save(&self, messages: Vec<Message>) -> Result<(), Error>;
    async fn batch_delete(&self, message_ids: Vec<String>) -> Result<(), Error>;
    
    // 查询操作
    async fn query_messages(&self, query: MessageQuery) -> Result<MessageBatch, Error>;
    async fn get_session_messages(&self, session_id: &str, limit: u32, offset: u32) -> Result<MessageBatch, Error>;
    async fn get_user_messages(&self, user_id: &str, limit: u32, offset: u32) -> Result<MessageBatch, Error>;
    
    // 状态操作
    async fn update_status(&self, message_id: &str, status: crate::domain::entities::message::MessageStatus) -> Result<(), Error>;
    async fn batch_update_status(&self, message_ids: Vec<String>, status: crate::domain::entities::message::MessageStatus) -> Result<(), Error>;
    
    // 统计操作
    async fn count_session_messages(&self, session_id: &str) -> Result<u64, Error>;
    async fn count_user_messages(&self, user_id: &str) -> Result<u64, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),
    
    #[error("Message not found: {0}")]
    NotFound(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    #[error("Duplicate message: {0}")]
    Duplicate(String),
    
    #[error("Query error: {0}")]
    QueryError(String),
} 