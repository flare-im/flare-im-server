use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::kafka::{KafkaConsumer, KafkaProducer, KafkaConfig};
use flare_rpc_core::redis::{RedisClient, RedisConfig};
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.notification");

// 通知服务
pub struct NotificationService {
    // Redis 客户端，用于存储通知配置和状态
    redis: Arc<RedisClient>,
    // Kafka 生产者，用于发送通知
    producer: Arc<KafkaProducer>,
    // Kafka 消费者，用于接收通知请求
    consumer: Arc<KafkaConsumer>,
}

impl NotificationService {
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
            client_id: config.kafka.client_id.clone(),
        };
        let producer = Arc::new(KafkaProducer::new(kafka_config.clone()).await?);

        // 初始化 Kafka 消费者
        let consumer = Arc::new(KafkaConsumer::new(kafka_config).await?);

        Ok(Self {
            redis,
            producer,
            consumer,
        })
    }

    // 保存通知配置
    async fn save_notification_config(&self, user_id: &str, config: &NotificationConfig) -> Result<()> {
        let key = format!("notification:config:{}", user_id);
        self.redis.set(&key, serde_json::to_string(config)?).await?;
        Ok(())
    }

    // 获取通知配置
    async fn get_notification_config(&self, user_id: &str) -> Result<Option<NotificationConfig>> {
        let key = format!("notification:config:{}", user_id);
        if let Some(value) = self.redis.get(&key).await? {
            Ok(Some(serde_json::from_str(&value)?))
        } else {
            Ok(None)
        }
    }

    // 发送通知
    async fn send_notification(&self, notification: &Notification) -> Result<String> {
        // 生成通知 ID
        let notification_id = uuid::Uuid::new_v4().to_string();

        // 序列化通知
        let payload = serde_json::to_string(&notification)?;

        // 发送到 Kafka
        self.producer
            .send("notifications", &notification_id, payload.as_bytes())
            .await?;

        Ok(notification_id)
    }
}

#[tonic::async_trait]
impl notification_server::Notification for NotificationService {
    // 更新通知配置
    async fn update_notification_config(
        &self,
        request: Request<UpdateNotificationConfigRequest>,
    ) -> Result<Response<UpdateNotificationConfigResponse>, Status> {
        let req = request.into_inner();
        info!("收到更新通知配置请求: {:?}", req);

        let config = req.config.ok_or_else(|| Status::invalid_argument("配置不能为空"))?;

        match self.save_notification_config(&req.user_id, &config).await {
            Ok(()) => {
                let response = UpdateNotificationConfigResponse {
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("保存通知配置失败: {}", e);
                let response = UpdateNotificationConfigResponse {
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 获取通知配置
    async fn get_notification_config(
        &self,
        request: Request<GetNotificationConfigRequest>,
    ) -> Result<Response<GetNotificationConfigResponse>, Status> {
        let req = request.into_inner();
        info!("收到获取通知配置请求: {:?}", req);

        match self.get_notification_config(&req.user_id).await {
            Ok(config) => {
                let response = GetNotificationConfigResponse {
                    config,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("获取通知配置失败: {}", e);
                let response = GetNotificationConfigResponse {
                    config: None,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 发送通知
    async fn send_notification(
        &self,
        request: Request<SendNotificationRequest>,
    ) -> Result<Response<SendNotificationResponse>, Status> {
        let req = request.into_inner();
        info!("收到发送通知请求: {:?}", req);

        let notification = req.notification.ok_or_else(|| Status::invalid_argument("通知不能为空"))?;

        match self.send_notification(&notification).await {
            Ok(notification_id) => {
                let response = SendNotificationResponse {
                    notification_id,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("发送通知失败: {}", e);
                let response = SendNotificationResponse {
                    notification_id: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动通知服务
pub async fn start_notification_service(config: common::config::Config) -> Result<()> {
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

    // 创建通知服务
    let notification_service = NotificationService::new(&config).await?;
    let notification_server = notification_server::NotificationServer::new(notification_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(notification_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
}
