use async_trait::async_trait;
use redis::{AsyncCommands, RedisError};
use uuid::Uuid;
use crate::domain::{
    entities::message::Message,
    repositories::message_repository::{MessageRepository, Error as RepoError},
};

pub struct RedisMessageRepository {
    client: redis::Client,
}

impl RedisMessageRepository {
    pub fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    fn message_key(id: Uuid) -> String {
        format!("message:{}", id)
    }

    fn session_key(session_id: &str) -> String {
        format!("session:{}:messages", session_id)
    }
}

#[async_trait]
impl MessageRepository for RedisMessageRepository {
    async fn save(&self, message: Message) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let message_data = serde_json::to_string(&message)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;

        // 存储消息数据
        conn.set(Self::message_key(message.id), message_data).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 添加到会话消息列表
        conn.zadd(
            Self::session_key(&message.session_id),
            message.id.to_string(),
            message.created_at.timestamp_millis(),
        ).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Message>, RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let data: Option<String> = conn.get(Self::message_key(id)).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        match data {
            Some(json) => {
                let message = serde_json::from_str(&json)
                    .map_err(|e| RepoError::InvalidData(e.to_string()))?;
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    async fn get_by_session(&self, session_id: &str, limit: u32, offset: u32) -> Result<Vec<Message>, RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 获取消息ID列表
        let message_ids: Vec<String> = conn.zrange(
            Self::session_key(session_id),
            offset as isize,
            (offset + limit) as isize,
        ).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let mut messages = Vec::new();
        for id_str in message_ids {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                if let Some(message) = self.get_by_id(id).await? {
                    messages.push(message);
                }
            }
        }

        Ok(messages)
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 获取消息数据以获取会话ID
        if let Some(message) = self.get_by_id(id).await? {
            // 从会话消息列表中删除
            conn.zrem(
                Self::session_key(&message.session_id),
                id.to_string(),
            ).await
                .map_err(|e| RepoError::Repository(e.to_string()))?;
        }

        // 删除消息数据
        conn.del(Self::message_key(id)).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }
} 