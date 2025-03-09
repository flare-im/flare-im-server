use crate::domain::{
    entities::message::{Message, MessageQuery, MessageBatch, MessageStatus},
    repositories::message_repository::MessageRepository,
    services::message_service::MessageService,
};
use chrono::Utc;
use uuid::Uuid;

pub struct MessageManager<R: MessageRepository, S: MessageService> {
    message_repository: R,
    message_service: S,
}

impl<R: MessageRepository, S: MessageService> MessageManager<R, S> {
    pub fn new(message_repository: R, message_service: S) -> Self {
        Self {
            message_repository,
            message_service,
        }
    }

    // 存储消息
    pub async fn store_message(&self, message: Message) -> Result<Message, Error> {
        // 验证消息
        self.validate_message(&message)?;

        // 处理消息(压缩、加密等)
        let processed_message = self.message_service.process_message(message).await?;
        
        // 保存到仓储
        self.message_repository.save(processed_message.clone()).await?;

        // 分发消息
        self.message_service.dispatch_message(processed_message.clone()).await?;

        Ok(processed_message)
    }

    // 批量存储消息
    pub async fn store_messages(&self, messages: Vec<Message>) -> Result<Vec<Message>, Error> {
        // 验证消息
        for message in &messages {
            self.validate_message(message)?;
        }

        // 批量处理消息
        let processed_messages = self.message_service.batch_process_messages(messages).await?;
        
        // 批量保存
        self.message_repository.batch_save(processed_messages.clone()).await?;

        // 批量分发
        self.message_service.batch_dispatch_messages(processed_messages.clone()).await?;

        Ok(processed_messages)
    }

    // 查询消息历史
    pub async fn query_history(&self, query: MessageQuery) -> Result<MessageBatch, Error> {
        self.message_service.query_messages(query).await
            .map_err(Error::from)
    }

    // 获取会话消息
    pub async fn get_session_messages(&self, session_id: &str, limit: u32, offset: u32) -> Result<MessageBatch, Error> {
        self.message_repository.get_session_messages(session_id, limit, offset).await
            .map_err(Error::from)
    }

    // 标记消息已读
    pub async fn mark_messages_read(&self, message_ids: Vec<String>) -> Result<(), Error> {
        self.message_service.mark_as_read(message_ids).await
            .map_err(Error::from)
    }

    // 标记消息已送达
    pub async fn mark_messages_delivered(&self, message_ids: Vec<String>) -> Result<(), Error> {
        self.message_service.mark_as_delivered(message_ids).await
            .map_err(Error::from)
    }

    // 删除消息
    pub async fn delete_messages(&self, message_ids: Vec<String>) -> Result<(), Error> {
        self.message_service.delete_messages(message_ids).await
            .map_err(Error::from)
    }

    // 清空会话消息
    pub async fn clear_session_messages(&self, session_id: &str) -> Result<(), Error> {
        self.message_service.clear_session_messages(session_id).await
            .map_err(Error::from)
    }

    // 验证消息
    fn validate_message(&self, message: &Message) -> Result<(), Error> {
        if message.session_id.is_empty() {
            return Err(Error::ValidationError("Session ID is required".to_string()));
        }
        if message.sender_id.is_empty() {
            return Err(Error::ValidationError("Sender ID is required".to_string()));
        }
        if message.content.is_empty() {
            return Err(Error::ValidationError("Content is required".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(#[from] crate::domain::repositories::message_repository::Error),
    
    #[error("Service error: {0}")]
    Service(#[from] crate::domain::services::message_service::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
} 