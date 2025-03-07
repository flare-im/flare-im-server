use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::kafka::{KafkaConfig, KafkaConsumer, KafkaProducer};
use flare_rpc_core::timescaledb::{TimescaleClient, TimescaleConfig};
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.store");

// 消息存储服务
pub struct MessageStoreService {
    // TimescaleDB 客户端，用于消息持久化
    db: Arc<TimescaleClient>,
    // Kafka 消费者，用于消费消息
    consumer: Arc<KafkaConsumer>,
    // Kafka 生产者，用于消息转发
    producer: Arc<KafkaProducer>,
}

impl MessageStoreService {
    pub async fn new(config: &common::config::Config) -> Result<Self> {
        // 初始化 TimescaleDB 客户端
        let db_config = TimescaleConfig {
            host: config.timescaledb.host.clone(),
            port: config.timescaledb.port,
            username: config.timescaledb.username.clone(),
            password: config.timescaledb.password.clone(),
            database: config.timescaledb.database.clone(),
            max_connections: config.timescaledb.max_connections,
        };
        let db = Arc::new(TimescaleClient::new(db_config).await?);

        // 初始化 Kafka 消费者
        let kafka_config = KafkaConfig {
            brokers: config.kafka.brokers.clone(),
            group_id: config.kafka.group_id.clone(),
            topics: config.kafka.topics.clone(),
        };
        let consumer = Arc::new(KafkaConsumer::new(kafka_config.clone()).await?);
        let producer = Arc::new(KafkaProducer::new(kafka_config).await?);

        Ok(Self {
            db,
            consumer,
            producer,
        })
    }

    // 存储消息
    async fn store_message(&self, message: &MessageData) -> Result<i64> {
        let sql = "INSERT INTO messages (
            message_id, conversation_id, sender_id, message_type,
            content, send_time, sequence, extra
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id";

        let id = self.db
            .query_one(sql, &[
                &message.message_id,
                &message.conversation_id,
                &message.sender_id,
                &message.message_type,
                &message.content,
                &message.send_time,
                &message.sequence,
                &serde_json::to_value(&message.extra)?,
            ])
            .await?
            .get(0);

        Ok(id)
    }

    // 查询消息
    async fn query_messages(
        &self,
        conversation_id: &str,
        start_time: i64,
        end_time: i64,
        limit: i32,
    ) -> Result<Vec<MessageData>> {
        let sql = "SELECT * FROM messages 
            WHERE conversation_id = $1 
            AND send_time BETWEEN $2 AND $3
            ORDER BY send_time DESC 
            LIMIT $4";

        let rows = self.db
            .query(sql, &[
                &conversation_id,
                &start_time,
                &end_time,
                &limit,
            ])
            .await?;

        let mut messages = Vec::new();
        for row in rows {
            let message = MessageData {
                message_id: row.get("message_id"),
                conversation_id: row.get("conversation_id"),
                sender_id: row.get("sender_id"),
                message_type: row.get("message_type"),
                content: row.get("content"),
                send_time: row.get("send_time"),
                sequence: row.get("sequence"),
                extra: serde_json::from_value(row.get("extra"))?,
            };
            messages.push(message);
        }

        Ok(messages)
    }
}

#[tonic::async_trait]
impl message_store_server::MessageStore for MessageStoreService {
    // 存储消息
    async fn store_message(
        &self,
        request: Request<StoreMessageRequest>,
    ) -> Result<Response<StoreMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到存储消息请求: {:?}", req);

        let message = req.message.ok_or_else(|| Status::invalid_argument("消息不能为空"))?;

        match self.store_message(&message).await {
            Ok(id) => {
                // 发送消息到同步队列
                if let Err(e) = self.producer
                    .send("message.sync", &message)
                    .await
                {
                    error!("发送消息到同步队列失败: {}", e);
                }

                let response = StoreMessageResponse {
                    message_id: message.message_id,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("存储消息失败: {}", e);
                let response = StoreMessageResponse {
                    message_id: message.message_id,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 批量存储消息
    async fn batch_store_messages(
        &self,
        request: Request<BatchStoreMessageRequest>,
    ) -> Result<Response<BatchStoreMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到批量存储消息请求: {:?}", req);

        let mut results = Vec::new();
        for message in req.messages {
            match self.store_message(&message).await {
                Ok(_) => {
                    results.push(StoreMessageResponse {
                        message_id: message.message_id,
                        status: 0,
                        error: "".to_string(),
                    });
                }
                Err(e) => {
                    error!("存储消息失败: {}", e);
                    results.push(StoreMessageResponse {
                        message_id: message.message_id,
                        status: 1,
                        error: e.to_string(),
                    });
                }
            }
        }

        let response = BatchStoreMessageResponse {
            results,
            status: 0,
            error: "".to_string(),
        };
        Ok(Response::new(response))
    }

    // 查询消息历史
    async fn query_message_history(
        &self,
        request: Request<QueryMessageHistoryRequest>,
    ) -> Result<Response<QueryMessageHistoryResponse>, Status> {
        let req = request.into_inner();
        info!("收到查询消息历史请求: {:?}", req);

        match self.query_messages(
            &req.conversation_id,
            req.start_time,
            req.end_time,
            req.limit,
        ).await {
            Ok(messages) => {
                let response = QueryMessageHistoryResponse {
                    messages,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("查询消息历史失败: {}", e);
                let response = QueryMessageHistoryResponse {
                    messages: vec![],
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动消息存储服务
pub async fn start_message_store(config: common::config::Config) -> Result<()> {
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

    // 创建消息存储服务
    let store_service = MessageStoreService::new(&config).await?;
    let store_server = message_store_server::MessageStoreServer::new(store_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(store_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 