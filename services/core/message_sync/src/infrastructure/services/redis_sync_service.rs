use crate::domain::{
    entities::*,
    services::{SyncService, SyncType, StatusType, Error},
};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use log::{info, warn, error};

pub struct RedisSyncService {
    redis: Arc<ConnectionManager>,
    message_store: Arc<dyn MessageStore>,
    session_service: Arc<dyn SessionService>,
}

impl RedisSyncService {
    pub fn new(
        redis: Arc<ConnectionManager>,
        message_store: Arc<dyn MessageStore>,
        session_service: Arc<dyn SessionService>,
    ) -> Self {
        Self {
            redis,
            message_store,
            session_service,
        }
    }

    // 生成序列号
    async fn generate_sequence(&self, conversation_id: &str, count: i32) -> Result<(i64, i64), Error> {
        let mut conn = self.redis.clone();
        let key = format!("sequence:conversation:{}", conversation_id);
        
        let start: i64 = conn.incr(&key, count as i64).await.map_err(|e| {
            Error::StorageError(format!("Failed to generate sequence: {}", e))
        })?;
        
        let end = start + count as i64 - 1;
        Ok((start, end))
    }

    // 获取同步点键
    fn get_sync_point_key(&self, user_id: &str, device_id: &str) -> String {
        format!("sync:point:{}:{}", user_id, device_id)
    }
}

#[async_trait]
impl SyncService for RedisSyncService {
    async fn sync(&self, user_id: &str, device_id: &str, last_sequence: i64, sync_type: SyncType) -> Result<SyncResult, Error> {
        match sync_type {
            SyncType::Incremental => self.incremental_sync(user_id, device_id, last_sequence, 100).await,
            SyncType::Full => self.full_sync(user_id, device_id, 100, 0).await,
            SyncType::Quick => {
                let mut result = self.incremental_sync(user_id, device_id, last_sequence, 20).await?;
                result.conversations = self.message_store.get_recent_conversations(user_id, 10).await?;
                Ok(result)
            }
        }
    }

    async fn incremental_sync(&self, user_id: &str, device_id: &str, last_sequence: i64, limit: i32) -> Result<SyncResult, Error> {
        // 获取增量消息
        let messages = self.message_store
            .get_messages_after_sequence(user_id, last_sequence, limit)
            .await?;

        // 获取消息操作
        let operations = self.message_store
            .get_operations_after_sequence(user_id, last_sequence, limit)
            .await?;

        // 获取当前序列号
        let current_sequence = if let Some(last_msg) = messages.last() {
            last_msg.sequence
        } else if let Some(last_op) = operations.last() {
            last_op.sequence
        } else {
            last_sequence
        };

        Ok(SyncResult {
            messages,
            conversations: vec![],
            operations,
            user_statuses: vec![],
            current_sequence,
            sync_time: Utc::now(),
            has_more: messages.len() >= limit as usize,
        })
    }

    async fn full_sync(&self, user_id: &str, device_id: &str, limit: i32, offset: i32) -> Result<SyncResult, Error> {
        // 获取所有会话
        let conversations = self.message_store.get_user_conversations(user_id).await?;
        
        // 获取会话消息
        let messages = self.message_store
            .get_conversation_messages(conversations.iter().map(|c| c.id).collect(), limit, offset)
            .await?;

        // 获取用户状态
        let user_statuses = self.session_service
            .get_users_status(conversations.iter().flat_map(|c| c.members.clone()).collect())
            .await?;

        // 获取当前序列号
        let current_sequence = if let Some(last_msg) = messages.last() {
            last_msg.sequence
        } else {
            0
        };

        Ok(SyncResult {
            messages,
            conversations,
            operations: vec![],
            user_statuses,
            current_sequence,
            sync_time: Utc::now(),
            has_more: messages.len() >= limit as usize,
        })
    }

    async fn message_operation(&self, operation: MessageOperation) -> Result<i64, Error> {
        // 生成序列号
        let (sequence, _) = self.generate_sequence(&operation.message_id.to_string(), 1).await?;
        
        // 保存操作
        let mut operation = operation;
        operation.sequence = sequence;
        self.message_store.save_operation(operation.clone()).await?;
        
        // 广播操作
        // TODO: 实现操作广播

        Ok(sequence)
    }

    async fn sync_status(&self, user_id: &str, device_id: &str, message_ids: Vec<Uuid>, status_type: StatusType) -> Result<Vec<MessageStatus>, Error> {
        // 获取消息状态
        let statuses = self.message_store.get_messages_status(message_ids).await?;
        
        // 更新状态
        match status_type {
            StatusType::Delivery => {
                for status in &statuses {
                    self.message_store.update_delivery_status(
                        status.message_id,
                        DeliveryStatus::Delivered,
                        user_id,
                    ).await?;
                }
            }
            StatusType::Read => {
                for status in &statuses {
                    self.message_store.update_read_status(
                        status.message_id,
                        ReadStatus::Read,
                        user_id,
                    ).await?;
                }
            }
            _ => {}
        }
        
        Ok(statuses)
    }

    async fn get_sequence(&self, conversation_id: Uuid, count: i32) -> Result<(i64, i64), Error> {
        self.generate_sequence(&conversation_id.to_string(), count).await
    }

    async fn update_sync_point(&self, sync_point: SyncPoint) -> Result<(), Error> {
        let mut conn = self.redis.clone();
        let key = self.get_sync_point_key(&sync_point.user_id, &sync_point.device_id);
        
        let _: () = conn.set(&key, serde_json::to_string(&sync_point).map_err(|e| {
            Error::StorageError(format!("Failed to serialize sync point: {}", e))
        })?).await.map_err(|e| {
            Error::StorageError(format!("Failed to save sync point: {}", e))
        })?;
        
        Ok(())
    }

    async fn get_sync_point(&self, user_id: &str, device_id: &str) -> Result<Option<SyncPoint>, Error> {
        let mut conn = self.redis.clone();
        let key = self.get_sync_point_key(user_id, device_id);
        
        let data: Option<String> = conn.get(&key).await.map_err(|e| {
            Error::StorageError(format!("Failed to get sync point: {}", e))
        })?;
        
        match data {
            Some(json) => Ok(Some(serde_json::from_str(&json).map_err(|e| {
                Error::StorageError(format!("Failed to deserialize sync point: {}", e))
            })?)),
            None => Ok(None),
        }
    }
}

#[async_trait]
pub trait MessageStore: Send + Sync {
    async fn get_messages_after_sequence(&self, user_id: &str, sequence: i64, limit: i32) -> Result<Vec<Message>, Error>;
    async fn get_operations_after_sequence(&self, user_id: &str, sequence: i64, limit: i32) -> Result<Vec<MessageOperation>, Error>;
    async fn get_user_conversations(&self, user_id: &str) -> Result<Vec<Conversation>, Error>;
    async fn get_conversation_messages(&self, conversation_ids: Vec<Uuid>, limit: i32, offset: i32) -> Result<Vec<Message>, Error>;
    async fn get_recent_conversations(&self, user_id: &str, limit: i32) -> Result<Vec<Conversation>, Error>;
    async fn save_operation(&self, operation: MessageOperation) -> Result<(), Error>;
    async fn get_messages_status(&self, message_ids: Vec<Uuid>) -> Result<Vec<MessageStatus>, Error>;
    async fn update_delivery_status(&self, message_id: Uuid, status: DeliveryStatus, user_id: &str) -> Result<(), Error>;
    async fn update_read_status(&self, message_id: Uuid, status: ReadStatus, user_id: &str) -> Result<(), Error>;
}

#[async_trait]
pub trait SessionService: Send + Sync {
    async fn get_users_status(&self, user_ids: Vec<String>) -> Result<Vec<UserStatus>, Error>;
} 