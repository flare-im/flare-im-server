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

    // 创建数据库连接池
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/flare_im".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // 初始化存储库
    let repository = Arc::new(PostgresRepository::new(pool.clone()));
    repository.create_tables().await?;

    // 创建通知服务
    let notification_service = NotificationServiceImpl::new(
        repository.clone(),
        repository.clone(),
        repository.clone(),
    );

    // 注册推送提供商
    // 1. 极光推送
    let jpush_app_key = std::env::var("JPUSH_APP_KEY")
        .unwrap_or_else(|_| "your_app_key".to_string());
    let jpush_master_secret = std::env::var("JPUSH_MASTER_SECRET")
        .unwrap_or_else(|_| "your_master_secret".to_string());
    let jpush_provider = JPushProvider::new(jpush_app_key, jpush_master_secret);
    notification_service.register_provider(Box::new(jpush_provider)).await?;

    // 2. 个推
    let getui_app_id = std::env::var("GETUI_APP_ID")
        .unwrap_or_else(|_| "your_app_id".to_string());
    let getui_app_key = std::env::var("GETUI_APP_KEY")
        .unwrap_or_else(|_| "your_app_key".to_string());
    let getui_master_secret = std::env::var("GETUI_MASTER_SECRET")
        .unwrap_or_else(|_| "your_master_secret".to_string());
    let getui_provider = GetuiProvider::new(getui_app_id, getui_app_key, getui_master_secret);
    notification_service.register_provider(Box::new(getui_provider)).await?;

    // 3. 华为推送
    let huawei_app_id = std::env::var("HUAWEI_APP_ID")
        .unwrap_or_else(|_| "your_app_id".to_string());
    let huawei_app_secret = std::env::var("HUAWEI_APP_SECRET")
        .unwrap_or_else(|_| "your_app_secret".to_string());
    let huawei_provider = HuaweiProvider::new(huawei_app_id, huawei_app_secret);
    notification_service.register_provider(Box::new(huawei_provider)).await?;

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