use async_trait::async_trait;
use crate::domain::entities::message::Message;

#[async_trait]
pub trait RoutingService {
    // 上行消息路由
    async fn route_upstream(&self, message: Message) -> Result<(), Error>;
    
    // 下行消息分发
    async fn distribute_downstream(&self, message: Message) -> Result<Vec<DistributionResult>, Error>;
    
    // 获取路由信息
    async fn get_route_info(&self, session_id: &str) -> Result<RouteInfo, Error>;
    
    // 更新路由表
    async fn update_route_table(&self, route_info: RouteInfo) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct RouteInfo {
    pub session_id: String,
    pub gateway_id: String,
    pub user_ids: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug)]
pub struct DistributionResult {
    pub user_id: String,
    pub gateway_id: String,
    pub status: DistributionStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum DistributionStatus {
    Success,
    Offline,
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Routing error: {0}")]
    Routing(String),
    #[error("Distribution error: {0}")]
    Distribution(String),
    #[error("Invalid route: {0}")]
    InvalidRoute(String),
} 