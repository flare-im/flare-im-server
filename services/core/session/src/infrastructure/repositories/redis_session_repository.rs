use async_trait::async_trait;
use redis::{AsyncCommands, RedisError};
use crate::domain::{
    entities::session::{Session, SessionMember, OnlineStatus},
    repositories::session_repository::{SessionRepository, Error as RepoError},
};

pub struct RedisSessionRepository {
    client: redis::Client,
}

impl RedisSessionRepository {
    pub fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    fn session_key(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    fn user_sessions_key(user_id: &str) -> String {
        format!("user:{}:sessions", user_id)
    }

    fn session_members_key(session_id: &str) -> String {
        format!("session:{}:members", session_id)
    }

    fn member_key(session_id: &str, user_id: &str) -> String {
        format!("session:{}:member:{}", session_id, user_id)
    }

    fn unread_count_key(session_id: &str, user_id: &str) -> String {
        format!("session:{}:unread:{}", session_id, user_id)
    }
}

#[async_trait]
impl SessionRepository for RedisSessionRepository {
    async fn save(&self, session: Session) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 序列化会话数据
        let session_data = serde_json::to_string(&session)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;

        // 保存会话数据
        conn.set(Self::session_key(&session.id), session_data).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 为每个成员添加会话引用
        for member in &session.members {
            conn.sadd(
                Self::user_sessions_key(&member.user_id),
                &session.id,
            ).await
                .map_err(|e| RepoError::Repository(e.to_string()))?;

            // 保存成员数据
            let member_data = serde_json::to_string(member)
                .map_err(|e| RepoError::InvalidData(e.to_string()))?;
            
            conn.set(
                Self::member_key(&session.id, &member.user_id),
                member_data,
            ).await
                .map_err(|e| RepoError::Repository(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_by_id(&self, session_id: &str) -> Result<Option<Session>, RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let data: Option<String> = conn.get(Self::session_key(session_id)).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        match data {
            Some(json) => {
                let session = serde_json::from_str(&json)
                    .map_err(|e| RepoError::InvalidData(e.to_string()))?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, session_id: &str) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 获取会话数据以获取成员列表
        if let Some(session) = self.get_by_id(session_id).await? {
            // 删除每个成员的会话引用
            for member in &session.members {
                conn.srem(
                    Self::user_sessions_key(&member.user_id),
                    session_id,
                ).await
                    .map_err(|e| RepoError::Repository(e.to_string()))?;

                // 删除成员数据
                conn.del(Self::member_key(session_id, &member.user_id)).await
                    .map_err(|e| RepoError::Repository(e.to_string()))?;
            }
        }

        // 删除会话数据
        conn.del(Self::session_key(session_id)).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn get_user_sessions(&self, user_id: &str, limit: u32, offset: u32) -> Result<Vec<Session>, RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 获取用户的会话ID列表
        let session_ids: Vec<String> = conn.smembers(Self::user_sessions_key(user_id)).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let mut sessions = Vec::new();
        for id in session_ids.iter().skip(offset as usize).take(limit as usize) {
            if let Some(session) = self.get_by_id(id).await? {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }

    async fn get_sessions_by_type(&self, session_type: &str, limit: u32, offset: u32) -> Result<Vec<Session>, RepoError> {
        // 在实际实现中，可能需要维护一个按类型索引的会话列表
        // 这里简单返回空列表
        Ok(Vec::new())
    }

    async fn add_member(&self, session_id: &str, member: SessionMember) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 序列化成员数据
        let member_data = serde_json::to_string(&member)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;

        // 保存成员数据
        conn.set(
            Self::member_key(session_id, &member.user_id),
            member_data,
        ).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 添加到用户的会话列表
        conn.sadd(
            Self::user_sessions_key(&member.user_id),
            session_id,
        ).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn remove_member(&self, session_id: &str, user_id: &str) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 删除成员数据
        conn.del(Self::member_key(session_id, user_id)).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        // 从用户的会话列表中移除
        conn.srem(
            Self::user_sessions_key(user_id),
            session_id,
        ).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn update_member_status(&self, session_id: &str, user_id: &str, status: OnlineStatus) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let member_key = Self::member_key(session_id, user_id);
        
        // 获取现有成员数据
        let data: Option<String> = conn.get(&member_key).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        if let Some(json) = data {
            let mut member: SessionMember = serde_json::from_str(&json)
                .map_err(|e| RepoError::InvalidData(e.to_string()))?;

            // 更新状态
            member.online_status = status;

            // 保存更新后的数据
            let updated_data = serde_json::to_string(&member)
                .map_err(|e| RepoError::InvalidData(e.to_string()))?;

            conn.set(member_key, updated_data).await
                .map_err(|e| RepoError::Repository(e.to_string()))?;

            Ok(())
        } else {
            Err(RepoError::MemberNotFound(format!("Member {} not found in session {}", user_id, session_id)))
        }
    }

    async fn update_latest_message(&self, session_id: &str, message_id: &str, preview: &str) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        if let Some(mut session) = self.get_by_id(session_id).await? {
            // 更新最新消息
            session.latest_message = Some(crate::domain::entities::session::LatestMessage {
                message_id: uuid::Uuid::parse_str(message_id)
                    .map_err(|e| RepoError::InvalidData(e.to_string()))?,
                sender_id: String::new(), // 需要从消息中获取
                content_type: String::new(), // 需要从消息中获取
                content_preview: preview.to_string(),
                sent_at: chrono::Utc::now(),
            });

            // 保存更新后的会话
            self.save(session).await?;
        }

        Ok(())
    }

    async fn increment_unread_count(&self, session_id: &str, user_id: &str) -> Result<i32, RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let key = Self::unread_count_key(session_id, user_id);
        let count: i32 = conn.incr(key, 1).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(count)
    }

    async fn reset_unread_count(&self, session_id: &str, user_id: &str) -> Result<(), RepoError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        conn.set(Self::unread_count_key(session_id, user_id), 0).await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn update_settings(&self, session_id: &str, settings: std::collections::HashMap<String, String>) -> Result<(), RepoError> {
        if let Some(mut session) = self.get_by_id(session_id).await? {
            // 更新设置
            session.settings.custom_settings = settings;

            // 保存更新后的会话
            self.save(session).await?;
        }

        Ok(())
    }
} 