use anyhow::Result;
use flare_core::logs::{LogConfig, Logger};
use log::{info, error};
use std::sync::Arc;
use flare_rpc_core::{
    discover::consul::{ConsulConfig, ConsulRegistry},
    AppBuilder,
};
use std::time::Duration;
use tonic::transport::Server;
use message_router::{
    domain::services::{MessageService, MessageServiceImpl},
    application::message_router::MessageRouterService,
    infrastructure::repositories::{
        MessageRepositoryImpl,
        RouteRepositoryImpl,
        FriendRepositoryImpl,
        GroupRepositoryImpl,
        ContentFilterRepositoryImpl,
    },
    interfaces::{
        grpc::message_router_service::MessageRouterGrpcService,
        consumers::MessageDistributionConsumer,
    },
};
use proto_crate::api::im::service::router::message_router_server::MessageRouterServer;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    Logger::init(Some(LogConfig::default()))?;

    info!("Starting Message Router Service...");

    // 初始化消息服务
    let message_service = init_message_service()?;
    let message_router_service = Arc::new(MessageRouterService::new(message_service.clone()));
    let grpc_service = MessageRouterGrpcService::new(message_router_service);

    // 初始化并启动 Kafka 消费者
    let consumer = MessageDistributionConsumer::new(message_service)?;
    tokio::spawn(async move {
        if let Err(e) = consumer.start().await {
            error!("Kafka consumer error: {}", e);
        }
    });

    // 启动 gRPC 服务
    let addr = get_service_addr()?;
    let consul_config = ConsulConfig {
        addr: "127.0.0.1:8500".parse()?,
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };
    // 创建 Consul 注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(5)).await?;


    let app = AppBuilder::new("message_router")
        .weight(1)
        .register(registry)
        .build();

    info!("Message Router Service listening on {}", addr);

    app.run("127.0.0.1", 50052, |mut server, addr| async move {
        server
            .add_service(MessageRouterServer::new(grpc_service))
            .serve(addr)
            .await
            .map_err(|e| e.into())
    }).await.expect("server start filed");

    Ok(())
}

fn init_message_service() -> Result<Arc<MessageServiceImpl>> {
    let message_repo = Arc::new(MessageRepositoryImpl::new()?);
    let route_repo = Arc::new(RouteRepositoryImpl::new());
    let friend_repo = Arc::new(FriendRepositoryImpl::new());
    let group_repo = Arc::new(GroupRepositoryImpl::new());
    let content_filter_repo = Arc::new(ContentFilterRepositoryImpl::new());

    Ok(Arc::new(MessageServiceImpl::new(
        message_repo,
        route_repo,
        friend_repo,
        group_repo,
        content_filter_repo,
    )))
}

fn get_service_addr() -> Result<String> {
    Ok("127.0.0.1:50052".to_string())
}