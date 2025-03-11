use anyhow::Result;
use flare_core::logs::Logger;
use log::{info, error};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use notification::{
    infrastructure::{
        services::notification_service_impl::NotificationServiceImpl,
        repositories::postgres_repository::PostgresRepository,
        providers::{
            jpush_provider::JPushProvider,
            getui_provider::GetuiProvider,
            huawei_provider::HuaweiProvider,
        },
    },
    interfaces::grpc::notification_service::NotificationGrpcService,
};
use proto_crate::api::im::service::notification::notification_server::NotificationServer;
use flare_rpc_core::{
    discover::consul::{ConsulConfig, ConsulRegistry},
    AppBuilder,
};
use std::time::Duration;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    Logger::init("notification", "debug")?;
    info!("Starting Notification Service...");


    // 创建 gRPC 服务
    let grpc_service = NotificationGrpcService::new(notification_service);

    // 配置 Consul
    let consul_host = std::env::var("CONSUL_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let consul_port = std::env::var("CONSUL_PORT")
        .unwrap_or_else(|_| "8500".to_string())
        .parse::<u16>()?;
    let consul_addr = format!("{}:{}", consul_host, consul_port).parse()?;
    
    let consul_config = ConsulConfig {
        addr: consul_addr,
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };

    // 创建服务注册器
    let registry = ConsulRegistry::new(
        consul_config,
        Duration::from_secs(30), // 注册间隔
    ).await?;

    // 配置服务
    let service_host = std::env::var("SERVICE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let service_port = std::env::var("SERVICE_PORT")
        .unwrap_or_else(|_| "50055".to_string())
        .parse::<u16>()?;
    let addr = format!("{}:{}", service_host, service_port).parse()?;

    // 创建应用构建器
    let app = AppBuilder::new("notification")
        .version("1.0.0")
        .weight(1)
        .tag("prod")
        .meta("type", "notification")
        .register(registry)
        .build();

    info!("Notification Service listening on {}", addr);

    // 运行服务
    app.run(service_host.as_str(), service_port, |mut server, addr| async move {
        server
            .add_service(NotificationServer::new(grpc_service))
            .serve(addr)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 