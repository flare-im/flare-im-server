use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::entities::message::Message;

#[async_trait]
pub trait MessageRepository {
    async fn save(&self, message: Message) -> Result<(), Error>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Message>, Error>;
    async fn get_by_session(&self, session_id: &str, limit: u32, offset: u32) -> Result<Vec<Message>, Error>;
    async fn delete(&self, id: Uuid) -> Result<(), Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),
    #[error("Message not found: {0}")]
    NotFound(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
} 