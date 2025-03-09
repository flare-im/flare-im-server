use async_trait::async_trait;
use crate::domain::entities::message::{Message, MessageQuery, MessageBatch, MessageStatus};

#[async_trait]
pub trait MessageService {
    // 消息处理
    async fn process_message(&self, message: Message) -> Result<Message, Error>;
    async fn batch_process_messages(&self, messages: Vec<Message>) -> Result<Vec<Message>, Error>;
    
    // 消息分发
    async fn dispatch_message(&self, message: Message) -> Result<(), Error>;
    async fn batch_dispatch_messages(&self, messages: Vec<Message>) -> Result<(), Error>;
    
    // 消息查询
    async fn query_messages(&self, query: MessageQuery) -> Result<MessageBatch, Error>;
    async fn get_message_history(&self, session_id: &str, before_id: Option<String>, limit: u32) -> Result<MessageBatch, Error>;
    
    // 消息状态更新
    async fn mark_as_delivered(&self, message_ids: Vec<String>) -> Result<(), Error>;
    async fn mark_as_read(&self, message_ids: Vec<String>) -> Result<(), Error>;
    
    // 消息删除
    async fn delete_messages(&self, message_ids: Vec<String>) -> Result<(), Error>;
    async fn clear_session_messages(&self, session_id: &str) -> Result<(), Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Service error: {0}")]
    Service(String),
    
    #[error("Processing error: {0}")]
    Processing(String),
    
    #[error("Dispatch error: {0}")]
    Dispatch(String),
    
    #[error("Query error: {0}")]
    Query(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
} 