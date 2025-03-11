use async_trait::async_trait;
use anyhow::Result;
use std::collections::HashMap;

/// 路由信息
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// 网关地址
    pub address: String,
    /// 权重
    pub weight: i32,
    /// 当前负载
    pub load: i32,
}

/// 路由仓储接口
#[async_trait]
pub trait RouteRepository: Send + Sync {

    /// 获取带权重的路由信息
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// 
    /// # 返回
    /// * `Result<Vec<Route>, Error>` - 带权重的路由列表
    async fn get_routes_with_weight(&self, user_id: &str) -> Result<Vec<RouteInfo>>;

    /// 批量获取带权重的路由信息
    /// 
    /// # 参数
    /// * `user_ids` - 用户ID列表
    /// 
    /// # 返回
    /// * `Result<HashMap<String, Vec<RouteInfo>>, Error>` - 用户ID到路由列表的映射
    async fn get_routes_with_weight_batch(&self, user_ids: &[String]) -> Result<HashMap<String, Vec<RouteInfo>>>;
}