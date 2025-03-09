use tonic::{Request, Response, Status};
use uuid::Uuid;
use crate::{
    application::message_router::MessageRouterService,
    domain::{
        entities::message::{Message, MessageContentType, MessageMetadata, MessagePriority, QosLevel},
        repositories::message_repository::MessageRepository,
        services::routing_service::RoutingService,
    },
};
use api::im::service::router::{
    message_router_server::MessageRouter,
    RouteMessageRequest,
    RouteMessageResponse,
    DistributeMessageRequest,
    DistributeMessageResponse,
    UpdateRouteTableRequest,
    UpdateRouteTableResponse,
};

pub struct MessageRouterGrpcService<R: MessageRepository, S: RoutingService> {
    service: MessageRouterService<R, S>,
}

impl<R: MessageRepository, S: RoutingService> MessageRouterGrpcService<R, S> {
    pub fn new(service: MessageRouterService<R, S>) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl<R: MessageRepository + Send + Sync + 'static, S: RoutingService + Send + Sync + 'static> 
    MessageRouter for MessageRouterGrpcService<R, S> 
{
    async fn route_message(
        &self,
        request: Request<RouteMessageRequest>,
    ) -> Result<Response<RouteMessageResponse>, Status> {
        let req = request.into_inner();
        
        let message = Message {
            id: Uuid::new_v4(),
            session_id: req.session_id,
            sender_id: req.sender_id,
            content_type: convert_content_type(req.content_type),
            content: req.content,
            metadata: MessageMetadata {
                priority: convert_priority(req.priority),
                qos_level: convert_qos_level(req.qos_level),
                need_receipt: req.need_receipt,
                need_offline_storage: req.need_offline_storage,
                need_offline_push: req.need_offline_push,
                extra: req.metadata,
            },
            created_at: chrono::Utc::now(),
        };

        self.service.handle_upstream_message(message)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RouteMessageResponse {
            success: true,
            error: None,
        }))
    }

    async fn distribute_message(
        &self,
        request: Request<DistributeMessageRequest>,
    ) -> Result<Response<DistributeMessageResponse>, Status> {
        let req = request.into_inner();
        
        let message = Message {
            id: Uuid::new_v4(),
            session_id: req.session_id,
            sender_id: req.sender_id,
            content_type: convert_content_type(req.content_type),
            content: req.content,
            metadata: MessageMetadata {
                priority: convert_priority(req.priority),
                qos_level: convert_qos_level(req.qos_level),
                need_receipt: req.need_receipt,
                need_offline_storage: req.need_offline_storage,
                need_offline_push: req.need_offline_push,
                extra: req.metadata,
            },
            created_at: chrono::Utc::now(),
        };

        let results = self.service.handle_downstream_message(message)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DistributeMessageResponse {
            results: results.into_iter().map(convert_distribution_result).collect(),
            error: None,
        }))
    }

    async fn update_route_table(
        &self,
        request: Request<UpdateRouteTableRequest>,
    ) -> Result<Response<UpdateRouteTableResponse>, Status> {
        let req = request.into_inner();
        
        let route_info = crate::domain::services::routing_service::RouteInfo {
            session_id: req.session_id,
            gateway_id: req.gateway_id,
            user_ids: req.user_ids,
            metadata: req.metadata,
        };

        self.service.update_route_table(route_info)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(UpdateRouteTableResponse {
            success: true,
            error: None,
        }))
    }
}

// 辅助函数：转换消息内容类型
fn convert_content_type(content_type: i32) -> MessageContentType {
    match content_type {
        0 => MessageContentType::Text,
        1 => MessageContentType::Image,
        2 => MessageContentType::Video,
        3 => MessageContentType::Audio,
        4 => MessageContentType::File,
        5 => MessageContentType::Location,
        _ => MessageContentType::Custom(content_type.to_string()),
    }
}

// 辅助函数：转换消息优先级
fn convert_priority(priority: i32) -> MessagePriority {
    match priority {
        0 => MessagePriority::High,
        1 => MessagePriority::Normal,
        _ => MessagePriority::Low,
    }
}

// 辅助函数：转换 QoS 级别
fn convert_qos_level(qos_level: i32) -> QosLevel {
    match qos_level {
        0 => QosLevel::AtMostOnce,
        1 => QosLevel::AtLeastOnce,
        _ => QosLevel::ExactlyOnce,
    }
}

// 辅助函数：转换分发结果
fn convert_distribution_result(
    result: crate::domain::services::routing_service::DistributionResult,
) -> api::im::service::router::DistributionResult {
    api::im::service::router::DistributionResult {
        user_id: result.user_id,
        gateway_id: result.gateway_id,
        status: match result.status {
            crate::domain::services::routing_service::DistributionStatus::Success => 0,
            crate::domain::services::routing_service::DistributionStatus::Offline => 1,
            crate::domain::services::routing_service::DistributionStatus::Failed => 2,
        },
        error: result.error,
    }
} 