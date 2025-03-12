use crate::domain::{
    entities::MessageStatus,
    services::MessageService,
};
use anyhow::Result;
use log::{error, info, warn};
use proto_crate::api::im::common::MessageData;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MessageRouterService {
    message_service: Arc<dyn MessageService>,
}

impl MessageRouterService {
    pub fn new(
        message_service: Arc<dyn MessageService>,
    ) -> Self {
        Self {
            message_service,
        }
    }

    /// 消息预处理和校验
    pub async fn pre_process(&self, message: &MessageData) -> Result<i32> {
        // 1. 基础格式校验
        let pre_process_code = self.message_service.pre_process(message).await?;
        Ok(pre_process_code as i32)
    }

    /// 消息路由处理
    pub async fn route_message(&self, message: &MessageData) -> Result<(bool, Option<String>, Vec<String>)> {
        // 1. 先存储消息
        if let Err(e) = self.message_service.handle_message_storage(message).await {
            error!("Failed to store message {}: {}", message.server_msg_id, e);
            return Ok((false, Some(e.to_string()), vec![]));
        }
        
        // 2. 下发消息
        if let Err(e) = self.message_service.handle_message_distribution(message).await {
            error!("Failed to route message {}: {}", message.server_msg_id, e);
            return Ok((false, Some(e.to_string()), vec![]));
        }

        // 3. 下发成功，处理消息同步
        if let Err(e) = self.message_service.handle_message_sync(message).await {
            warn!("Failed to sync message {}: {}", message.server_msg_id, e);
        }

        Ok((true, None, vec![]))
    }

    /// 消息重试处理
    pub async fn retry_message(&self, message: &MessageData) -> Result<()> {
        // 1. 检查是否需要重试
        if message.status == MessageStatus::Failed as i32 {
            self.message_service.handle_message_retry(message).await?;
        }

        Ok(())
    }

    /// 批量消息处理
    pub async fn process_messages(&self, messages: Vec<MessageData>) -> Result<HashMap<String, (bool, Option<String>, Vec<String>)>> {
        let mut results = HashMap::new();

        for message in messages {
            // 1. 预处理
            let pre_process_code = self.pre_process(&message).await?;
            if pre_process_code != 0 { // 0 表示 Ok
                results.insert(message.server_msg_id.clone(), (
                    false,
                    Some(format!("Pre-process failed with code: {}", pre_process_code)),
                    vec![],
                ));
                continue;
            }

            // 2. 路由处理
            match self.route_message(&message).await {
                Ok(result) => {
                    results.insert(message.server_msg_id.clone(), result);
                }
                Err(e) => {
                    error!("Failed to route message {}: {}", message.server_msg_id, e);
                    results.insert(message.server_msg_id.clone(), (
                        false,
                        Some(e.to_string()),
                        vec![],
                    ));
                }
            }
        }

        Ok(results)
    }
}

