use anyhow::Result;
use flare_core::logs::Logger;
use log::{info, error};
use sqlx::postgres::PgPoolOptions;
use message_filter::{
    application::filter_manager::FilterManager,
    infrastructure::{
        repositories::postgres_filter_repository::PostgresFilterRepository,
        services::filter_service_impl::FilterServiceImpl,
    },
    interfaces::grpc::filter_service::FilterGrpcService,
};
use proto_crate::api::im::service::filter::filter_server::FilterServer;
use flare_rpc_core::{
    discover::consul::{ConsulConfig, ConsulRegistry},
    AppBuilder,
};
use std::time::Duration;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    Logger::init("message_filter", "debug")?;
    info!("Starting Message Filter Service...");

    // 创建数据库连接池
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/flare_im".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // 创建过滤服务组件
    let filter_repository = PostgresFilterRepository::new(pool);
    let filter_service = FilterServiceImpl::new();
    let filter_manager = FilterManager::new(filter_repository, filter_service);

    // 创建 gRPC 服务
    let grpc_service = FilterGrpcService::new(filter_manager);

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
        .unwrap_or_else(|_| "50054".to_string())
        .parse::<u16>()?;
    let addr = format!("{}:{}", service_host, service_port).parse()?;

    // 创建应用构建器
    let app = AppBuilder::new("message-filter")
        .version("1.0.0")
        .weight(1)
        .tag("prod")
        .meta("type", "filter")
        .register(registry)
        .build();

    info!("Message Filter Service listening on {}", addr);

    // 运行服务
    app.run(service_host.as_str(), service_port, |mut server, addr| async move {
        server
            .add_service(FilterServer::new(grpc_service))
            .serve(addr)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 