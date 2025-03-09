use crate::domain::{
    entities::message::{Message, MessagePriority, QosLevel},
    repositories::message_repository::MessageRepository,
    services::routing_service::{RoutingService, RouteInfo, DistributionResult},
};

pub struct MessageRouterService<R: MessageRepository, S: RoutingService> {
    message_repository: R,
    routing_service: S,
}

impl<R: MessageRepository, S: RoutingService> MessageRouterService<R, S> {
    pub fn new(message_repository: R, routing_service: S) -> Self {
        Self {
            message_repository,
            routing_service,
        }
    }

    // 处理上行消息
    pub async fn handle_upstream_message(&self, message: Message) -> Result<(), Error> {
        // 1. 验证消息
        self.validate_message(&message)?;

        // 2. 根据 QoS 级别处理
        match message.metadata.qos_level {
            QosLevel::AtMostOnce => {
                self.routing_service.route_upstream(message).await?;
            }
            QosLevel::AtLeastOnce | QosLevel::ExactlyOnce => {
                // 先存储后路由
                self.message_repository.save(message.clone()).await?;
                self.routing_service.route_upstream(message).await?;
            }
        }

        Ok(())
    }

    // 处理下行消息
    pub async fn handle_downstream_message(&self, message: Message) -> Result<Vec<DistributionResult>, Error> {
        // 1. 获取路由信息
        let route_info = self.routing_service.get_route_info(&message.session_id).await?;

        // 2. 根据优先级处理
        match message.metadata.priority {
            MessagePriority::High => {
                // 高优先级消息直接分发
                self.routing_service.distribute_downstream(message).await?
            }
            MessagePriority::Normal | MessagePriority::Low => {
                // 普通和低优先级消息先存储
                self.message_repository.save(message.clone()).await?;
                self.routing_service.distribute_downstream(message).await?
            }
        };

        Ok(vec![])
    }

    // 更新路由表
    pub async fn update_route_table(&self, route_info: RouteInfo) -> Result<(), Error> {
        self.routing_service.update_route_table(route_info).await?;
        Ok(())
    }

    // 验证消息
    fn validate_message(&self, message: &Message) -> Result<(), Error> {
        if message.session_id.is_empty() {
            return Err(Error::ValidationError("Session ID is required".to_string()));
        }
        if message.sender_id.is_empty() {
            return Err(Error::ValidationError("Sender ID is required".to_string()));
        }
        if message.content.is_empty() {
            return Err(Error::ValidationError("Message content is required".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(#[from] crate::domain::repositories::message_repository::Error),
    
    #[error("Routing error: {0}")]
    Routing(#[from] crate::domain::services::routing_service::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
} 