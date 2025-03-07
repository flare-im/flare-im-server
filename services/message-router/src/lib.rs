use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::kafka::{KafkaConfig, KafkaProducer};
use flare_rpc_core::redis::{RedisClient, RedisConfig};
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.router");

// 消息路由服务
pub struct MessageRouterService {
    // Redis 客户端，用于存储路由表
    redis: Arc<RedisClient>,
    // Kafka 生产者，用于消息持久化
    kafka: Arc<KafkaProducer>,
}

impl MessageRouterService {
    pub async fn new(config: &common::config::Config) -> Result<Self> {
        // 初始化 Redis 客户端
        let redis_config = RedisConfig {
            host: config.redis.host.clone(),
            port: config.redis.port,
            password: config.redis.password.clone(),
            database: config.redis.database,
            pool_size: config.redis.pool_size,
        };
        let redis = Arc::new(RedisClient::new(redis_config).await?);

        // 初始化 Kafka 生产者
        let kafka_config = KafkaConfig {
            brokers: config.kafka.brokers.clone(),
            group_id: config.kafka.group_id.clone(),
            topics: config.kafka.topics.clone(),
        };
        let kafka = Arc::new(KafkaProducer::new(kafka_config).await?);

        Ok(Self { redis, kafka })
    }

    // 查询用户路由信息
    async fn get_user_route(&self, user_id: &str) -> Result<Option<UserRoute>> {
        let key = format!("route:user:{}", user_id);
        let route: Option<String> = self.redis.get(&key).await?;
        
        if let Some(route_str) = route {
            let route: UserRoute = serde_json::from_str(&route_str)?;
            Ok(Some(route))
        } else {
            Ok(None)
        }
    }

    // 更新用户路由信息
    async fn update_user_route(&self, user_id: &str, route: &UserRoute) -> Result<()> {
        let key = format!("route:user:{}", user_id);
        let route_str = serde_json::to_string(route)?;
        self.redis.set(&key, &route_str, Some(3600)).await?;
        Ok(())
    }
}

#[tonic::async_trait]
impl message_router_server::MessageRouter for MessageRouterService {
    // 路由消息
    async fn route_message(
        &self,
        request: Request<RouteMessageRequest>,
    ) -> Result<Response<RouteMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到消息路由请求: {:?}", req);

        // 查询接收者路由信息
        let mut success_routes = Vec::new();
        let mut failed_routes = Vec::new();

        for receiver_id in req.receiver_ids {
            match self.get_user_route(&receiver_id).await {
                Ok(Some(route)) => {
                    success_routes.push(RouteResult {
                        user_id: receiver_id,
                        gateway_addr: route.gateway_addr,
                        status: 0,
                        error: "".to_string(),
                    });
                }
                Ok(None) => {
                    failed_routes.push(RouteResult {
                        user_id: receiver_id,
                        gateway_addr: "".to_string(),
                        status: 1,
                        error: "User route not found".to_string(),
                    });
                }
                Err(e) => {
                    error!("查询用户路由失败: {}", e);
                    failed_routes.push(RouteResult {
                        user_id: receiver_id,
                        gateway_addr: "".to_string(),
                        status: 2,
                        error: e.to_string(),
                    });
                }
            }
        }

        // 发送消息到 Kafka
        if let Err(e) = self.kafka
            .send("message.route", &req.message.unwrap_or_default())
            .await
        {
            error!("发送消息到 Kafka 失败: {}", e);
        }

        let response = RouteMessageResponse {
            message_id: req.message.map(|m| m.message_id).unwrap_or_default(),
            routes: [success_routes, failed_routes].concat(),
            status: 0,
            error: "".to_string(),
        };

        Ok(Response::new(response))
    }

    // 更新路由表
    async fn update_route(
        &self,
        request: Request<UpdateRouteRequest>,
    ) -> Result<Response<UpdateRouteResponse>, Status> {
        let req = request.into_inner();
        info!("收到更新路由请求: {:?}", req);

        let route = UserRoute {
            user_id: req.user_id.clone(),
            gateway_addr: req.gateway_addr.clone(),
            device_type: req.device_type,
            last_active: req.last_active,
        };

        match self.update_user_route(&req.user_id, &route).await {
            Ok(_) => {
                let response = UpdateRouteResponse {
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("更新路由表失败: {}", e);
                let response = UpdateRouteResponse {
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 删除路由
    async fn delete_route(
        &self,
        request: Request<DeleteRouteRequest>,
    ) -> Result<Response<DeleteRouteResponse>, Status> {
        let req = request.into_inner();
        info!("收到删除路由请求: {:?}", req);

        let key = format!("route:user:{}", req.user_id);
        match self.redis.del(&key).await {
            Ok(_) => {
                let response = DeleteRouteResponse {
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("删除路由失败: {}", e);
                let response = DeleteRouteResponse {
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

// 用户路由信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UserRoute {
    user_id: String,
    gateway_addr: String,
    device_type: i32,
    last_active: i64,
}

/// 启动消息路由服务
pub async fn start_message_router(config: common::config::Config) -> Result<()> {
    // 初始化日志
    common::log::init_logger(&config.log)?;

    // 创建 Consul 配置
    let consul_config = ConsulConfig {
        addr: format!("{}:{}", config.consul.host, config.consul.port),
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };

    // 创建 Consul 注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(15)).await?;

    // 创建并配置应用
    let app = AppBuilder::new(&config.service.name)
        .version(&config.service.metadata.get("version").unwrap_or(&"1.0.0".to_string()))
        .tags(&config.service.tags)
        .meta("protocol", "grpc")
        .weight(10)
        .register(registry)
        .build();

    // 创建消息路由服务
    let router_service = MessageRouterService::new(&config).await?;
    let router_server = message_router_server::MessageRouterServer::new(router_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(router_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 