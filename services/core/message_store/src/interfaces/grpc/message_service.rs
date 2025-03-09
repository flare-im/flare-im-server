use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;
use crate::{
    application::message_manager::MessageManager,
    domain::{
        entities::message::{Message, MessageMetadata, MessageQuery, MessageStatus},
        repositories::message_repository::MessageRepository,
        services::message_service::MessageService,
    },
};
use api::im::service::store::{
    store_server::Store as GrpcStore,
    StoreMessageRequest,
    StoreMessageResponse,
    BatchStoreMessageRequest,
    BatchStoreMessageResponse,
    GetMessageRequest,
    GetMessageResponse,
    QueryMessagesRequest,
    QueryMessagesResponse,
    GetSessionMessagesRequest,
    GetSessionMessagesResponse,
    UpdateMessageStatusRequest,
    UpdateMessageStatusResponse,
    DeleteMessagesRequest,
    DeleteMessagesResponse,
    ClearSessionMessagesRequest,
    ClearSessionMessagesResponse,
};

pub struct MessageGrpcService<R: MessageRepository, S: MessageService> {
    message_manager: MessageManager<R, S>,
}

impl<R: MessageRepository, S: MessageService> MessageGrpcService<R, S> {
    pub fn new(message_manager: MessageManager<R, S>) -> Self {
        Self { message_manager }
    }

    // 转换消息状态
    fn convert_status(status: i32) -> MessageStatus {
        match status {
            0 => MessageStatus::Pending,
            1 => MessageStatus::Sent,
            2 => MessageStatus::Delivered,
            3 => MessageStatus::Read,
            4 => MessageStatus::Failed,
            5 => MessageStatus::Deleted,
            _ => MessageStatus::Pending,
        }
    }

    // 转换为 gRPC 消息
    fn to_grpc_message(message: &Message) -> api::im::service::store::Message {
        api::im::service::store::Message {
            id: message.id.to_string(),
            session_id: message.session_id.clone(),
            sender_id: message.sender_id.clone(),
            content_type: message.content_type.clone(),
            content: message.content.clone(),
            status: match message.status {
                MessageStatus::Pending => 0,
                MessageStatus::Sent => 1,
                MessageStatus::Delivered => 2,
                MessageStatus::Read => 3,
                MessageStatus::Failed => 4,
                MessageStatus::Deleted => 5,
            },
            metadata: Some(api::im::service::store::MessageMetadata {
                device_id: message.metadata.device_id.clone(),
                client_msg_id: message.metadata.client_msg_id.clone(),
                reply_to: message.metadata.reply_to.map(|id| id.to_string()),
                mentions: message.metadata.mentions.clone(),
                is_encrypted: message.metadata.is_encrypted,
                compression: message.metadata.compression.clone().unwrap_or_default(),
                custom_properties: message.metadata.custom_properties.clone(),
            }),
            created_at: message.created_at.timestamp_millis(),
            updated_at: message.updated_at.timestamp_millis(),
        }
    }

    // 从 gRPC 请求转换为领域消息
    fn from_grpc_request(req: &StoreMessageRequest) -> Result<Message, Status> {
        Ok(Message {
            id: Uuid::new_v4(),
            session_id: req.session_id.clone(),
            sender_id: req.sender_id.clone(),
            content_type: req.content_type.clone(),
            content: req.content.clone(),
            status: Self::convert_status(req.status),
            metadata: MessageMetadata {
                device_id: req.metadata.as_ref().map(|m| m.device_id.clone()).unwrap_or_default(),
                client_msg_id: req.metadata.as_ref().map(|m| m.client_msg_id.clone()).unwrap_or_default(),
                reply_to: req.metadata.as_ref().and_then(|m| m.reply_to.as_ref()).and_then(|id| Uuid::parse_str(id).ok()),
                mentions: req.metadata.as_ref().map(|m| m.mentions.clone()).unwrap_or_default(),
                is_encrypted: req.metadata.as_ref().map(|m| m.is_encrypted).unwrap_or_default(),
                compression: if req.metadata.as_ref().map(|m| m.compression.is_empty()).unwrap_or(true) {
                    None
                } else {
                    req.metadata.as_ref().map(|m| m.compression.clone())
                },
                custom_properties: req.metadata.as_ref().map(|m| m.custom_properties.clone()).unwrap_or_default(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

#[tonic::async_trait]
impl<R: MessageRepository + Send + Sync + 'static, S: MessageService + Send + Sync + 'static> 
    GrpcStore for MessageGrpcService<R, S> 
{
    async fn store_message(
        &self,
        request: Request<StoreMessageRequest>,
    ) -> Result<Response<StoreMessageResponse>, Status> {
        let req = request.into_inner();
        let message = Self::from_grpc_request(&req)?;

        let stored_message = self.message_manager.store_message(message)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(StoreMessageResponse {
            message_id: stored_message.id.to_string(),
            error: None,
        }))
    }

    async fn batch_store_message(
        &self,
        request: Request<BatchStoreMessageRequest>,
    ) -> Result<Response<BatchStoreMessageResponse>, Status> {
        let req = request.into_inner();
        let messages = req.messages.into_iter()
            .map(|m| Self::from_grpc_request(&m))
            .collect::<Result<Vec<_>, _>>()?;

        let stored_messages = self.message_manager.store_messages(messages)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(BatchStoreMessageResponse {
            message_ids: stored_messages.into_iter().map(|m| m.id.to_string()).collect(),
            error: None,
        }))
    }

    async fn get_message(
        &self,
        request: Request<GetMessageRequest>,
    ) -> Result<Response<GetMessageResponse>, Status> {
        let req = request.into_inner();
        
        let message = self.message_manager.message_repository.get_by_id(&req.message_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetMessageResponse {
            message: message.map(|m| Self::to_grpc_message(&m)),
            error: None,
        }))
    }

    async fn query_messages(
        &self,
        request: Request<QueryMessagesRequest>,
    ) -> Result<Response<QueryMessagesResponse>, Status> {
        let req = request.into_inner();
        
        let query = MessageQuery {
            session_id: req.session_id,
            start_time: req.start_time.map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or_default()),
            end_time: req.end_time.map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or_default()),
            limit: req.limit,
            offset: req.offset,
            content_types: if req.content_types.is_empty() { None } else { Some(req.content_types) },
            sender_id: if req.sender_id.is_empty() { None } else { Some(req.sender_id) },
            status: req.status.map(Self::convert_status),
        };

        let result = self.message_manager.query_history(query)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(QueryMessagesResponse {
            messages: result.messages.iter().map(Self::to_grpc_message).collect(),
            total_count: result.total_count,
            has_more: result.has_more,
            error: None,
        }))
    }

    async fn get_session_messages(
        &self,
        request: Request<GetSessionMessagesRequest>,
    ) -> Result<Response<GetSessionMessagesResponse>, Status> {
        let req = request.into_inner();
        
        let result = self.message_manager.get_session_messages(&req.session_id, req.limit, req.offset)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetSessionMessagesResponse {
            messages: result.messages.iter().map(Self::to_grpc_message).collect(),
            total_count: result.total_count,
            has_more: result.has_more,
            error: None,
        }))
    }

    async fn update_message_status(
        &self,
        request: Request<UpdateMessageStatusRequest>,
    ) -> Result<Response<UpdateMessageStatusResponse>, Status> {
        let req = request.into_inner();
        
        match req.status {
            2 => { // Delivered
                self.message_manager.mark_messages_delivered(req.message_ids)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
            }
            3 => { // Read
                self.message_manager.mark_messages_read(req.message_ids)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
            }
            _ => return Err(Status::invalid_argument("Invalid status")),
        }

        Ok(Response::new(UpdateMessageStatusResponse {
            success: true,
            error: None,
        }))
    }

    async fn delete_messages(
        &self,
        request: Request<DeleteMessagesRequest>,
    ) -> Result<Response<DeleteMessagesResponse>, Status> {
        let req = request.into_inner();
        
        self.message_manager.delete_messages(req.message_ids)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DeleteMessagesResponse {
            success: true,
            error: None,
        }))
    }

    async fn clear_session_messages(
        &self,
        request: Request<ClearSessionMessagesRequest>,
    ) -> Result<Response<ClearSessionMessagesResponse>, Status> {
        let req = request.into_inner();
        
        self.message_manager.clear_session_messages(&req.session_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ClearSessionMessagesResponse {
            success: true,
            error: None,
        }))
    }
} 