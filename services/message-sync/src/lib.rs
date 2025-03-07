use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::kafka::{KafkaConfig, KafkaConsumer, KafkaProducer};
use flare_rpc_core::redis::{RedisClient, RedisConfig};
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.sync");

// 消息同步服务
pub struct MessageSyncService {
    // Redis 客户端，用于存储同步状态
    redis: Arc<RedisClient>,
    // Kafka 消费者，用于消费消息
    consumer: Arc<KafkaConsumer>,
    // Kafka 生产者，用于消息转发
    producer: Arc<KafkaProducer>,
}

impl MessageSyncService {
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

        // 初始化 Kafka
        let kafka_config = KafkaConfig {
            brokers: config.kafka.brokers.clone(),
            group_id: config.kafka.group_id.clone(),
            topics: config.kafka.topics.clone(),
        };
        let consumer = Arc::new(KafkaConsumer::new(kafka_config.clone()).await?);
        let producer = Arc::new(KafkaProducer::new(kafka_config).await?);

        Ok(Self {
            redis,
            consumer,
            producer,
        })
    }

    // 获取用户同步状态
    async fn get_sync_state(&self, user_id: &str, device_id: &str) -> Result<Option<SyncState>> {
        let key = format!("sync:{}:{}", user_id, device_id);
        let state: Option<String> = self.redis.get(&key).await?;
        
        if let Some(state_str) = state {
            let state: SyncState = serde_json::from_str(&state_str)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    // 更新同步状态
    async fn update_sync_state(&self, state: &SyncState) -> Result<()> {
        let key = format!("sync:{}:{}", state.user_id, state.device_id);
        let state_str = serde_json::to_string(state)?;
        self.redis.set(&key, &state_str, Some(3600 * 24)).await?;
        Ok(())
    }

    // 获取未同步的消息
    async fn get_unsync_messages(
        &self,
        user_id: &str,
        device_id: &str,
        last_sync_time: i64,
        limit: i32,
    ) -> Result<Vec<MessageData>> {
        // 从消息存储服务获取未同步的消息
        // 这里需要调用 message-store 服务
        // TODO: 实现具体的消息获取逻辑
        Ok(vec![])
    }
}

#[tonic::async_trait]
impl message_sync_server::MessageSync for MessageSyncService {
    // 同步消息
    async fn sync_messages(
        &self,
        request: Request<SyncMessagesRequest>,
    ) -> Result<Response<SyncMessagesResponse>, Status> {
        let req = request.into_inner();
        info!("收到同步消息请求: {:?}", req);

        // 获取同步状态
        let state = match self.get_sync_state(&req.user_id, &req.device_id).await {
            Ok(Some(state)) => state,
            Ok(None) => SyncState {
                user_id: req.user_id.clone(),
                device_id: req.device_id.clone(),
                last_sync_time: 0,
                last_ack_time: 0,
            },
            Err(e) => {
                error!("获取同步状态失败: {}", e);
                return Ok(Response::new(SyncMessagesResponse {
                    messages: vec![],
                    has_more: false,
                    next_sync_key: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                }));
            }
        };

        // 获取未同步的消息
        match self.get_unsync_messages(
            &req.user_id,
            &req.device_id,
            state.last_sync_time,
            req.limit,
        ).await {
            Ok(messages) => {
                let has_more = messages.len() as i32 >= req.limit;
                let last_message = messages.last();
                
                // 更新同步状态
                if let Some(msg) = last_message {
                    let new_state = SyncState {
                        user_id: req.user_id,
                        device_id: req.device_id,
                        last_sync_time: msg.send_time,
                        last_ack_time: state.last_ack_time,
                    };
                    if let Err(e) = self.update_sync_state(&new_state).await {
                        error!("更新同步状态失败: {}", e);
                    }
                }

                let response = SyncMessagesResponse {
                    messages,
                    has_more,
                    next_sync_key: "".to_string(), // TODO: 实现分页
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("获取未同步消息失败: {}", e);
                let response = SyncMessagesResponse {
                    messages: vec![],
                    has_more: false,
                    next_sync_key: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 确认消息同步
    async fn ack_sync(
        &self,
        request: Request<AckSyncRequest>,
    ) -> Result<Response<AckSyncResponse>, Status> {
        let req = request.into_inner();
        info!("收到确认同步请求: {:?}", req);

        // 更新同步状态
        let state = SyncState {
            user_id: req.user_id,
            device_id: req.device_id,
            last_sync_time: req.sync_time,
            last_ack_time: req.ack_time,
        };

        match self.update_sync_state(&state).await {
            Ok(_) => {
                let response = AckSyncResponse {
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("更新同步状态失败: {}", e);
                let response = AckSyncResponse {
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 获取同步状态
    async fn get_sync_state(
        &self,
        request: Request<GetSyncStateRequest>,
    ) -> Result<Response<GetSyncStateResponse>, Status> {
        let req = request.into_inner();
        info!("收到获取同步状态请求: {:?}", req);

        match self.get_sync_state(&req.user_id, &req.device_id).await {
            Ok(Some(state)) => {
                let response = GetSyncStateResponse {
                    state: Some(state),
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Ok(None) => {
                let response = GetSyncStateResponse {
                    state: None,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("获取同步状态失败: {}", e);
                let response = GetSyncStateResponse {
                    state: None,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动消息同步服务
pub async fn start_message_sync(config: common::config::Config) -> Result<()> {
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

    // 创建消息同步服务
    let sync_service = MessageSyncService::new(&config).await?;
    let sync_server = message_sync_server::MessageSyncServer::new(sync_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(sync_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 