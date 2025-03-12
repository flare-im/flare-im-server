use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error, instrument, debug};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
    util::Timeout,
    error::KafkaError,
    types::RDKafkaErrorCode,
};
use std::sync::Arc;
use prost::Message;
use tokio::sync::Semaphore;

use proto_crate::api::im::common::{MessageData, MessagePayload};
use common::topic::KafkaTopics;
use crate::domain::{
    repositories::{MessageRepository, RouteInfo},
    entities::{MessageStatus, DeviceStatus, UserStatus},
};

const MAX_RETRY_COUNT: i32 = 3;
const BASE_RETRY_DELAY_MS: u64 = 100;
const KAFKA_TIMEOUT_MS: u64 = 1500;
const MAX_INFLIGHT_MESSAGES: usize = 10000;



pub struct MessageRepositoryImpl {
    producer: FutureProducer,
    inflight_semaphore: Arc<Semaphore>,
}

impl MessageRepositoryImpl {
    pub fn new() -> Result<Self> {
        let mut config = ClientConfig::new();
        
        // Kafka 基础配置
        config.set("bootstrap.servers", "localhost:9092")  // Kafka 服务器地址
             .set("client.id", "message-router")          // 客户端标识
             .set("acks", "all")                         // 需要所有副本确认
             .set("enable.idempotence", "true")          // 启用幂等性
             .set("message.timeout.ms", "30000")         // 消息超时时间
             .set("request.timeout.ms", "15000")         // 请求超时时间
             .set("retries", "3")                        // 重试次数
             .set("retry.backoff.ms", "100")             // 重试间隔
             .set("compression.type", "snappy")          // 压缩类型
             .set("queue.buffering.max.messages", "10000") // 最大缓冲消息数
             .set("queue.buffering.max.ms", "5");        // 最大缓冲时间
        
        let producer = config.create()
            .map_err(|e| anyhow!("Failed to create Kafka producer: {}", e))?;

        Ok(Self {
            producer,
            inflight_semaphore: Arc::new(Semaphore::new(MAX_INFLIGHT_MESSAGES)),
        })
    }

    #[instrument(skip(self, message))]
    async fn send_to_kafka(&self, topic: &str, message: &MessageData) -> Result<()> {
        let _permit = self.inflight_semaphore.acquire().await?;
        
        debug!("Sending message {} to topic {}", message.server_msg_id, topic);
        let key = message.server_msg_id.clone();
        // 构建消息负载
        let payload = MessagePayload {
            msg_id: message.server_msg_id.clone(),
            msg: Some(message.clone()),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            metadata: HashMap::new(),
        };

        let mut payload_bytes = Vec::new();
        payload.encode(&mut payload_bytes)?;
        let record = FutureRecord::to(topic)
            .key(&key)
            .payload(&payload_bytes)
            .timestamp(payload.timestamp);

        match self.producer.send(record, Timeout::After(Duration::from_millis(KAFKA_TIMEOUT_MS))).await {
            Ok((partition, offset)) => {
                info!(
                    "Message sent successfully: topic={}, partition={}, offset={}", 
                    topic, partition, offset
                );
                Ok(())
            }
            Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) => {
                warn!("Kafka producer queue full, retrying...");
                tokio::time::sleep(Duration::from_millis(100)).await;
                Err(anyhow!("Kafka producer queue full"))
            }
            Err((err, _)) => {
                error!("Failed to send message: {}", err);
                Err(anyhow!("Kafka send error: {}", err))
            }
        }
    }

    async fn retry_with_backoff<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut retry_count = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRY_COUNT {
                        error!("Operation failed after {} retries: {}", MAX_RETRY_COUNT, e);
                        return Err(e);
                    }
                    let delay = BASE_RETRY_DELAY_MS * (1 << retry_count);
                    warn!("Retrying operation after {}ms. Error: {}", delay, e);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
}

#[async_trait]
impl MessageRepository for MessageRepositoryImpl {
    #[instrument(skip(self, message))]
    async fn save_message(&self, message: &MessageData) -> Result<()> {
        self.retry_with_backoff(|| async { 
            self.send_to_kafka(KafkaTopics::MESSAGE_STORE, message).await 
        }).await
    }

    #[instrument(skip(self, message))]
    async fn handle_message_distribution(&self, message: &MessageData) -> Result<()> {
        self.retry_with_backoff(|| async {
            self.send_to_kafka(KafkaTopics::MESSAGE_DISTRIBUTION, message).await
        }).await
    }

    async fn push_message(&self, message: &MessageData, routers: Vec<RouteInfo>) -> Result<()> {
        if routers.is_empty() {
            return Err(anyhow!("No available routes for message {}", message.server_msg_id));
        }

        info!("Starting to push message {} to {} routes", message.server_msg_id, routers.len());

        let semaphore = Arc::new(Semaphore::new(10));
        let mut tasks = Vec::new();

        for route in routers {
            let message = message.clone();
            let sem = semaphore.clone();
            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await?;
                info!("Pushing message {} to route {}", message.server_msg_id, route.address);
                Ok::<(), anyhow::Error>(())
            });
            tasks.push(task);
        }

        for task in tasks {
            task.await??;
        }

        info!("Message {} pushed to all routes successfully", message.server_msg_id);
        Ok(())
    }

    async fn send_offline_notification(&self, userid: &str, message: &MessageData) -> Result<()> {
        info!("Sending offline notification for user {}", userid);
        self.retry_with_backoff(|| async {
            self.send_to_kafka(KafkaTopics::OFFLINE_NOTIFICATIONS, message).await
        }).await
    }

    async fn update_message_status(&self, message_id: &str, status: MessageStatus) -> Result<()> {
        let message = MessageData {
            server_msg_id: message_id.to_string(),
            status: status as i32,
            ..Default::default()
        };

        self.retry_with_backoff(|| async {
            self.send_to_kafka(KafkaTopics::MESSAGE_STATUS, &message).await
        }).await
    }

    async fn get_recent_message_count(&self, user_id: &str, seconds: i32) -> Result<i32> {
        let current_time = self.current_timestamp();
        let start_time = current_time - seconds as i64;
        
        info!("Getting message count for user {} from {} to {}", user_id, start_time, current_time);
        Ok(5)
    }

    async fn get_group_daily_message_count(&self, group_id: &str) -> Result<i32> {
        let current_time = self.current_timestamp();
        let day_start = current_time - (current_time % 86400);
        
        info!("Getting daily message count for group {} on {}", group_id, day_start);
        Ok(100)
    }

    async fn get_private_daily_message_count(&self, sender_id: &str, receiver_id: &str) -> Result<i32> {
        let current_time = self.current_timestamp();
        let day_start = current_time - (current_time % 86400);
        
        info!("Getting daily message count between {} and {} on {}", sender_id, receiver_id, day_start);
        Ok(50)
    }

    async fn get_user_status(&self, user_id: &str) -> Result<UserStatus> {
        info!("Getting status for user {}", user_id);
        Ok(UserStatus {
            user_id: user_id.to_string(),
            status: 1,
            is_banned: false,
            ban_time: None,
            ban_reason: None,
            ban_expire_time: None,
            last_active_time: self.current_timestamp(),
        })
    }

    async fn get_device_status(&self, device_id: &str) -> Result<DeviceStatus> {
        info!("Getting status for device {}", device_id);
        Ok(DeviceStatus {
            device_id: device_id.to_string(),
            device_type: 1,
            device_name: "iPhone".to_string(),
            is_banned: false,
            ban_time: None,
            ban_reason: None,
            ban_expire_time: None,
            last_active_time: self.current_timestamp(),
        })
    }

    fn current_timestamp(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    #[instrument(skip(self, message))]
    async fn save_to_dead_letter(&self, message: &MessageData, error: String, retry_count: i32) -> Result<()> {
        let _permit = self.inflight_semaphore.acquire().await?;
        
        // 构建原始消息负载
        let original_payload = MessagePayload {
            msg_id: message.server_msg_id.clone(),
            msg: Some(message.clone()),
            timestamp: message.send_time as i64,
            metadata: message.options.clone(),
        };

        // 构建死信消息
        let dead_letter = proto_crate::api::im::common::DeadLetterMessage {
            original_message: Some(original_payload),
            error_reason: error.clone(),
            retry_count,
            max_retry_count: message.options.get("max_retries")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(3),
            last_retry_time: message.options.get("last_retry_time")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or_else(|| self.current_timestamp()),
            dead_time: self.current_timestamp(),
            error_metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("error_time".to_string(), self.current_timestamp().to_string());
                metadata.insert("original_status".to_string(), message.status.to_string());
                if let Some(device_id) = message.options.get("device_id") {
                    metadata.insert("device_id".to_string(), device_id.clone());
                }
                metadata
            },
        };

        // 序列化死信消息
        let mut dead_letter_bytes = Vec::new();
        dead_letter.encode(&mut dead_letter_bytes)?;

        // 发送到死信队列
        let record = FutureRecord::to(KafkaTopics::DEAD_LETTER)
            .key(&message.server_msg_id)
            .payload(&dead_letter_bytes)
            .timestamp(self.current_timestamp());

        debug!(
            "Saving message {} to dead letter queue, error: {}, retry count: {}", 
            message.server_msg_id, error, retry_count
        );

        match self.producer.send(record, Timeout::After(Duration::from_millis(KAFKA_TIMEOUT_MS))).await {
            Ok((partition, offset)) => {
                info!(
                    "Message saved to dead letter queue: msg_id={}, partition={}, offset={}", 
                    message.server_msg_id, partition, offset
                );
                Ok(())
            }
            Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) => {
                warn!("Dead letter queue full, retrying...");
                tokio::time::sleep(Duration::from_millis(100)).await;
                Err(anyhow!("Dead letter queue full"))
            }
            Err((err, _)) => {
                error!("Failed to save message to dead letter queue: {}", err);
                Err(anyhow!("Dead letter queue error: {}", err))
            }
        }
    }
} 