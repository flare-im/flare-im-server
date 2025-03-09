use crate::domain::{
    entities::*,
    services::{SyncService, SyncType, StatusType, Error},
};
use std::sync::Arc;
use uuid::Uuid;
use log::{info, warn, error};

pub struct SyncManager {
    sync_service: Arc<dyn SyncService>,
}

impl SyncManager {
    pub fn new(sync_service: Arc<dyn SyncService>) -> Self {
        Self { sync_service }
    }

    // 处理同步请求
    pub async fn handle_sync(&self, user_id: &str, device_id: &str, last_sequence: i64, sync_type: SyncType) -> Result<SyncResult, Error> {
        info!("Handling sync request for user {} device {}", user_id, device_id);
        
        // 获取同步点
        let sync_point = self.sync_service.get_sync_point(user_id, device_id).await?;
        
        // 验证序列号
        if let Some(point) = &sync_point {
            if last_sequence < point.sequence {
                warn!("Invalid sequence number: client {} < server {}", last_sequence, point.sequence);
                return Err(Error::SequenceError("Client sequence is behind server".to_string()));
            }
        }
        
        // 执行同步
        let result = self.sync_service.sync(user_id, device_id, last_sequence, sync_type).await?;
        
        // 更新同步点
        self.sync_service.update_sync_point(SyncPoint {
            user_id: user_id.to_string(),
            device_id: device_id.to_string(),
            sequence: result.current_sequence,
            sync_time: result.sync_time,
        }).await?;
        
        Ok(result)
    }

    // 处理增量同步
    pub async fn handle_incremental_sync(&self, user_id: &str, device_id: &str, last_sequence: i64, limit: i32) -> Result<SyncResult, Error> {
        info!("Handling incremental sync for user {} device {}", user_id, device_id);
        
        let result = self.sync_service.incremental_sync(user_id, device_id, last_sequence, limit).await?;
        
        Ok(result)
    }

    // 处理全量同步
    pub async fn handle_full_sync(&self, user_id: &str, device_id: &str, limit: i32, offset: i32) -> Result<SyncResult, Error> {
        info!("Handling full sync for user {} device {}", user_id, device_id);
        
        let result = self.sync_service.full_sync(user_id, device_id, limit, offset).await?;
        
        Ok(result)
    }

    // 处理消息操作
    pub async fn handle_message_operation(&self, operation: MessageOperation) -> Result<i64, Error> {
        info!("Handling message operation {:?} for message {}", operation.operation_type, operation.message_id);
        
        // 验证操作权限
        self.validate_operation_permission(&operation).await?;
        
        // 执行操作
        let sequence = self.sync_service.message_operation(operation).await?;
        
        Ok(sequence)
    }

    // 处理状态同步
    pub async fn handle_status_sync(&self, user_id: &str, device_id: &str, message_ids: Vec<Uuid>, status_type: StatusType) -> Result<Vec<MessageStatus>, Error> {
        info!("Handling status sync for user {} device {}", user_id, device_id);
        
        let statuses = self.sync_service.sync_status(user_id, device_id, message_ids, status_type).await?;
        
        Ok(statuses)
    }

    // 获取序列号
    pub async fn handle_get_sequence(&self, conversation_id: Uuid, count: i32) -> Result<(i64, i64), Error> {
        info!("Getting sequence numbers for conversation {}", conversation_id);
        
        let (start, end) = self.sync_service.get_sequence(conversation_id, count).await?;
        
        Ok((start, end))
    }

    // 验证操作权限
    async fn validate_operation_permission(&self, operation: &MessageOperation) -> Result<(), Error> {
        // TODO: 实现具体的权限验证逻辑
        Ok(())
    }
} 