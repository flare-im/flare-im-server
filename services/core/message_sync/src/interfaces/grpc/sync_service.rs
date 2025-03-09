use crate::application::SyncManager;
use crate::domain::{
    entities::*,
    services::{SyncType, StatusType, Error},
};
use proto_crate::api::im::service::sync::*;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct MessageSyncGrpcService {
    sync_manager: Arc<SyncManager>,
}

impl MessageSyncGrpcService {
    pub fn new(sync_manager: Arc<SyncManager>) -> Self {
        Self { sync_manager }
    }

    // 转换同步类型
    fn convert_sync_type(&self, sync_type: i32) -> SyncType {
        match sync_type {
            1 => SyncType::Incremental,
            2 => SyncType::Full,
            3 => SyncType::Quick,
            _ => SyncType::Incremental,
        }
    }

    // 转换状态类型
    fn convert_status_type(&self, status_type: i32) -> StatusType {
        match status_type {
            1 => StatusType::Delivery,
            2 => StatusType::Read,
            3 => StatusType::Online,
            4 => StatusType::Typing,
            _ => StatusType::Delivery,
        }
    }

    // 转换错误为 Status
    fn convert_error(&self, error: Error) -> Status {
        match error {
            Error::Service(msg) => Status::internal(msg),
            Error::InvalidRequest(msg) => Status::invalid_argument(msg),
            Error::NotFound(msg) => Status::not_found(msg),
            Error::PermissionDenied(msg) => Status::permission_denied(msg),
            Error::SequenceError(msg) => Status::failed_precondition(msg),
            Error::ConflictError(msg) => Status::already_exists(msg),
            Error::StorageError(msg) => Status::internal(msg),
        }
    }
}

#[tonic::async_trait]
impl message_sync_server::MessageSync for MessageSyncGrpcService {
    async fn sync(&self, request: Request<SyncRequest>) -> Result<Response<SyncResponse>, Status> {
        let req = request.into_inner();
        
        let result = self.sync_manager
            .handle_sync(
                &req.user_id,
                &req.device_id,
                req.last_sequence,
                self.convert_sync_type(req.sync_type),
            )
            .await
            .map_err(|e| self.convert_error(e))?;
            
        Ok(Response::new(SyncResponse {
            messages: result.messages.into_iter().map(|m| m.into()).collect(),
            conversations: result.conversations.into_iter().map(|c| c.into()).collect(),
            current_sequence: result.current_sequence,
            sync_time: result.sync_time.timestamp_millis(),
            has_more: result.has_more,
        }))
    }

    async fn incremental_sync(&self, request: Request<IncrementalSyncRequest>) -> Result<Response<IncrementalSyncResponse>, Status> {
        let req = request.into_inner();
        
        let result = self.sync_manager
            .handle_incremental_sync(
                &req.user_id,
                &req.device_id,
                req.last_sequence,
                req.limit,
            )
            .await
            .map_err(|e| self.convert_error(e))?;
            
        Ok(Response::new(IncrementalSyncResponse {
            messages: result.messages.into_iter().map(|m| m.into()).collect(),
            operations: result.operations.into_iter().map(|o| o.into()).collect(),
            current_sequence: result.current_sequence,
            has_more: result.has_more,
        }))
    }

    async fn full_sync(&self, request: Request<FullSyncRequest>) -> Result<Response<FullSyncResponse>, Status> {
        let req = request.into_inner();
        
        let result = self.sync_manager
            .handle_full_sync(
                &req.user_id,
                &req.device_id,
                req.limit,
                req.offset,
            )
            .await
            .map_err(|e| self.convert_error(e))?;
            
        Ok(Response::new(FullSyncResponse {
            messages: result.messages.into_iter().map(|m| m.into()).collect(),
            conversations: result.conversations.into_iter().map(|c| c.into()).collect(),
            user_statuses: result.user_statuses.into_iter().map(|s| s.into()).collect(),
            current_sequence: result.current_sequence,
            has_more: result.has_more,
        }))
    }

    async fn message_operation(&self, request: Request<MessageOperationRequest>) -> Result<Response<MessageOperationResponse>, Status> {
        let req = request.into_inner();
        
        let operation = MessageOperation {
            id: Uuid::new_v4(),
            message_id: Uuid::parse_str(&req.message_id).map_err(|_| Status::invalid_argument("Invalid message ID"))?,
            user_id: req.user_id,
            device_id: req.device_id,
            operation_type: match req.operation_type {
                1 => OperationType::Recall,
                2 => OperationType::Delete,
                3 => OperationType::Edit,
                4 => OperationType::Pin,
                5 => OperationType::Unpin,
                _ => return Err(Status::invalid_argument("Invalid operation type")),
            },
            operation_data: req.operation_data,
            sequence: 0,
            created_at: Utc::now(),
        };
        
        let sequence = self.sync_manager
            .handle_message_operation(operation)
            .await
            .map_err(|e| self.convert_error(e))?;
            
        Ok(Response::new(MessageOperationResponse {
            success: true,
            sequence,
            timestamp: Utc::now().timestamp_millis(),
        }))
    }

    async fn sync_status(&self, request: Request<SyncStatusRequest>) -> Result<Response<SyncStatusResponse>, Status> {
        let req = request.into_inner();
        
        let message_ids = req.message_ids.into_iter()
            .map(|id| Uuid::parse_str(&id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Status::invalid_argument("Invalid message ID"))?;
            
        let statuses = self.sync_manager
            .handle_status_sync(
                &req.user_id,
                &req.device_id,
                message_ids,
                self.convert_status_type(req.status_type),
            )
            .await
            .map_err(|e| self.convert_error(e))?;
            
        Ok(Response::new(SyncStatusResponse {
            message_statuses: statuses.into_iter().map(|s| s.into()).collect(),
            sync_time: Utc::now().timestamp_millis(),
        }))
    }

    async fn get_sequence(&self, request: Request<GetSequenceRequest>) -> Result<Response<GetSequenceResponse>, Status> {
        let req = request.into_inner();
        
        let conversation_id = Uuid::parse_str(&req.conversation_id)
            .map_err(|_| Status::invalid_argument("Invalid conversation ID"))?;
            
        let (start, end) = self.sync_manager
            .handle_get_sequence(conversation_id, req.count)
            .await
            .map_err(|e| self.convert_error(e))?;
            
        Ok(Response::new(GetSequenceResponse {
            start_sequence: start,
            end_sequence: end,
        }))
    }
} 