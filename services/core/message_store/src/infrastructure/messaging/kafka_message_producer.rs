use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde_json::json;
use crate::domain::entities::message::Message;
use std::time::Duration;

pub struct KafkaMessageProducer {
    producer: FutureProducer,
    topic: String,
}

impl KafkaMessageProducer {
    pub fn new(brokers: &str, topic: &str) -> Result<Self, rdkafka::error::KafkaError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()?;

        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    pub async fn send_message(&self, message: &Message) -> Result<(), Error> {
        let payload = serde_json::to_string(&json!({
            "id": message.id.to_string(),
            "session_id": message.session_id,
            "sender_id": message.sender_id,
            "content_type": message.content_type,
            "content": message.content,
            "status": format!("{:?}", message.status),
            "metadata": message.metadata,
            "created_at": message.created_at,
            "updated_at": message.updated_at,
        })).map_err(|e| Error::Serialization(e.to_string()))?;

        let record = FutureRecord::to(&self.topic)
            .key(&message.id.to_string())
            .payload(&payload);

        self.producer.send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| Error::Producer(e.to_string()))?;

        Ok(())
    }

    pub async fn send_messages(&self, messages: &[Message]) -> Result<(), Error> {
        for message in messages {
            self.send_message(message).await?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kafka producer error: {0}")]
    Producer(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
} 