use async_trait::async_trait;
use crate::{
    domain::{
        entities::message::{Message, MessageQuery, MessageBatch, MessageStatus},
        services::message_service::{MessageService, Error as ServiceError},
    },
    infrastructure::messaging::kafka_message_producer::{KafkaMessageProducer, Error as KafkaError},
};
use lz4::block::CompressionMode;

pub struct MessageServiceImpl {
    kafka_producer: KafkaMessageProducer,
}

impl MessageServiceImpl {
    pub fn new(kafka_producer: KafkaMessageProducer) -> Self {
        Self { kafka_producer }
    }

    // 压缩消息内容
    fn compress_content(&self, content: &str) -> Result<Vec<u8>, ServiceError> {
        let mut compressed = Vec::new();
        lz4::block::compress_to_vec(
            content.as_bytes(),
            &mut compressed,
            CompressionMode::HIGHCOMPRESSION,
        ).map_err(|e| ServiceError::Processing(e.to_string()))?;
        Ok(compressed)
    }

    // 解压消息内容
    fn decompress_content(&self, compressed: &[u8]) -> Result<String, ServiceError> {
        let mut decompressed = Vec::new();
        lz4::block::decompress_to_vec(
            compressed,
            &mut decompressed,
        ).map_err(|e| ServiceError::Processing(e.to_string()))?;

        String::from_utf8(decompressed)
            .map_err(|e| ServiceError::Processing(e.to_string()))
    }
}

#[async_trait]
impl MessageService for MessageServiceImpl {
    async fn process_message(&self, mut message: Message) -> Result<Message, ServiceError> {
        // 检查是否需要压缩
        if message.content.len() > 1024 && message.metadata.compression.is_none() {
            let compressed = self.compress_content(&message.content)?;
            message.content = base64::encode(&compressed);
            message.metadata.compression = Some("lz4".to_string());
        }

        Ok(message)
    }

    async fn batch_process_messages(&self, messages: Vec<Message>) -> Result<Vec<Message>, ServiceError> {
        let mut processed_messages = Vec::new();
        for message in messages {
            processed_messages.push(self.process_message(message).await?);
        }
        Ok(processed_messages)
    }

    async fn dispatch_message(&self, message: Message) -> Result<(), ServiceError> {
        self.kafka_producer.send_message(&message).await
            .map_err(|e| ServiceError::Dispatch(e.to_string()))?;
        Ok(())
    }

    async fn batch_dispatch_messages(&self, messages: Vec<Message>) -> Result<(), ServiceError> {
        self.kafka_producer.send_messages(&messages).await
            .map_err(|e| ServiceError::Dispatch(e.to_string()))?;
        Ok(())
    }

    async fn query_messages(&self, query: MessageQuery) -> Result<MessageBatch, ServiceError> {
        // 这里应该调用仓储层的查询方法
        // 由于我们已经在仓储层实现了查询逻辑，这里直接返回错误
        Err(ServiceError::Query("Not implemented in service layer".to_string()))
    }

    async fn get_message_history(&self, session_id: &str, before_id: Option<String>, limit: u32) -> Result<MessageBatch, ServiceError> {
        // 这里应该调用仓储层的查询方法
        // 由于我们已经在仓储层实现了查询逻辑，这里直接返回错误
        Err(ServiceError::Query("Not implemented in service layer".to_string()))
    }

    async fn mark_as_delivered(&self, message_ids: Vec<String>) -> Result<(), ServiceError> {
        // 这里应该调用仓储层的更新方法
        // 由于我们已经在仓储层实现了更新逻辑，这里直接返回错误
        Err(ServiceError::Service("Not implemented in service layer".to_string()))
    }

    async fn mark_as_read(&self, message_ids: Vec<String>) -> Result<(), ServiceError> {
        // 这里应该调用仓储层的更新方法
        // 由于我们已经在仓储层实现了更新逻辑，这里直接返回错误
        Err(ServiceError::Service("Not implemented in service layer".to_string()))
    }

    async fn delete_messages(&self, message_ids: Vec<String>) -> Result<(), ServiceError> {
        // 这里应该调用仓储层的删除方法
        // 由于我们已经在仓储层实现了删除逻辑，这里直接返回错误
        Err(ServiceError::Service("Not implemented in service layer".to_string()))
    }

    async fn clear_session_messages(&self, session_id: &str) -> Result<(), ServiceError> {
        // 这里应该调用仓储层的删除方法
        // 由于我们已经在仓储层实现了删除逻辑，这里直接返回错误
        Err(ServiceError::Service("Not implemented in service layer".to_string()))
    }
} 