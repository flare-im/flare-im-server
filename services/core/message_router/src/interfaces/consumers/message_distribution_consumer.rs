use anyhow::{Result, anyhow};
use rdkafka::{
    consumer::{Consumer, StreamConsumer, CommitMode},
    ClientConfig,
    Message,
    message::OwnedMessage,
};
use tracing::{error, instrument, warn};
use std::sync::Arc;
use log::debug;
use tokio::sync::Semaphore;
use serde_json::from_slice;
use common::utils::msg_utils::is_group_message;
use common::topic::KafkaTopics;
use proto_crate::api::im::common::{MessageData, MessagePayload};
use crate::domain::services::MessageService;

pub struct MessageDistributionConsumer {
    consumer: StreamConsumer,
    message_service: Arc<dyn MessageService>,
    concurrent_limit: Arc<Semaphore>,
}

impl MessageDistributionConsumer {
    const MAX_CONCURRENT_MESSAGES: usize = 100;

    pub fn new(message_service: Arc<dyn MessageService>) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "message_distribution_group")
            .set("bootstrap.servers", "localhost:9092")
            .set("enable.auto.commit", "false")  // 禁用自动提交
            .set("auto.offset.reset", "earliest")
            .set("max.poll.interval.ms", "300000")
            .set("session.timeout.ms", "30000")
            .create()?;

        consumer.subscribe(&[KafkaTopics::MESSAGE_DISTRIBUTION])?;

        Ok(Self {
            consumer,
            message_service,
            concurrent_limit: Arc::new(Semaphore::new(Self::MAX_CONCURRENT_MESSAGES)),
        })
    }

    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        debug!("Starting message distribution consumer");

        loop {
            match self.consumer.recv().await {
                Ok(borrowed_message) => {
                    let _permit = self.concurrent_limit.acquire().await?;
                    let service = self.message_service.clone();
                    
                    // 转换为 OwnedMessage
                    let owned_message = borrowed_message.detach();
                    
                    tokio::spawn(async move {
                        match Self::process_message(owned_message.clone(), service.clone()).await {
                            Ok(_) => {
                                // 消息处理成功，提交 offset
                                debug!("Message processed successfully");
                            }
                            Err(e) => {
                                error!("Failed to process message: {}", e);
                                // 消息处理失败，根据重试策略处理
                                if let Err(retry_err) = Self::handle_message_failure(&owned_message, e, &service).await {
                                    error!("Failed to handle message failure: {}", retry_err);
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                }
            }
        }
    }

    #[instrument(skip(message, service))]
    async fn process_message(
        message: OwnedMessage,
        service: Arc<dyn MessageService>,
    ) -> Result<()> {
        let payload = match message.payload() {
            Some(data) => {
                let payload: MessagePayload = from_slice(data)?;
                payload
            }
            None => return Err(anyhow!("Empty message payload")),
        };

        debug!(
            "Processing message: id={}, timestamp={}", 
            payload.msg_id,
            payload.timestamp
        );

        let msg = payload.msg.ok_or_else(|| anyhow!("Message content is required"))?;

        if is_group_message(&msg) {
            Self::handle_group_message(&msg, &service).await?;
        } else {
            Self::handle_single_message(&msg, &service).await?;
        }
        Ok(())
    }

    #[instrument(skip(message, service))]
    async fn handle_group_message(
        message: &MessageData,
        service: &Arc<dyn MessageService>,
    ) -> Result<()> {
        match service.handle_group_message(message).await {
            Ok(result) => {
                if !result.success {
                    // 如果消息处理不成功但没有抛出错误，记录原因并进入重试
                    let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    warn!(
                        "Group message {} processing failed: {}",
                        message.server_msg_id,
                        error_msg
                    );
                    service.handle_message_retry(message).await?;
                } else {
                    debug!(
                        "Group message {} processed successfully, delivered to {} routes",
                        message.server_msg_id,
                        result.routes.len()
                    );
                }
            }
            Err(e) => {
                error!(
                    "Failed to process group message {}: {}",
                    message.server_msg_id,
                    e
                );
                service.handle_message_retry(message).await?;
            }
        }
        Ok(())
    }

    #[instrument(skip(message, service))]
    async fn handle_single_message(
        message: &MessageData,
        service: &Arc<dyn MessageService>,
    ) -> Result<()> {
        match service.handle_message(message).await {
            Ok(result) => {
                if !result.success {
                    // 如果消息处理不成功但没有抛出错误，记录原因并进入重试
                    let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    warn!(
                        "Single message {} processing failed: {}",
                        message.server_msg_id,
                        error_msg
                    );
                    service.handle_message_retry(message).await?;
                } else {
                    debug!(
                        "Single message {} processed successfully, delivered to {} routes",
                        message.server_msg_id,
                        result.routes.len()
                    );
                }
            }
            Err(e) => {
                error!(
                    "Failed to process single message {}: {}",
                    message.server_msg_id,
                    e
                );
                service.handle_message_retry(message).await?;
            }
        }
        Ok(())
    }

    #[instrument(skip(message, error, service))]
    async fn handle_message_failure(
        message: &OwnedMessage, 
        error: anyhow::Error,
        service: &Arc<dyn MessageService>,
    ) -> Result<()> {
        let payload = match message.payload() {
            Some(data) => {
                let payload: MessagePayload = from_slice(data)?;
                payload
            }
            None => return Err(anyhow!("Empty message payload")),
        };

        if let Some(msg) = payload.msg {
            error!(
                "Message {} processing failed with error: {}, entering retry flow",
                msg.server_msg_id,
                error
            );
            service.handle_message_retry(&msg).await?;
        }

        Ok(())
    }
} 