use anyhow::Result;
use flare_core::logs::Logger;
use log::{info, error};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use media::{
    infrastructure::{
        services::s3_storage_service::S3StorageService,
        repositories::postgres_repository::PostgresMediaRepository,
    },
    interfaces::grpc::media_service::MediaGrpcService,
};
use proto_crate::api::im::service::media::media_server::MediaServer;
use flare_rpc_core::{
    discover::consul::{ConsulConfig, ConsulRegistry},
    AppBuilder,
};
use std::time::Duration;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    Logger::init("media", "debug")?;
    info!("Starting Media Service...");

    // 创建数据库连接池
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/flare_im".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // 初始化存储库
    let repository = Arc::new(PostgresMediaRepository::new(pool.clone()));
    repository.create_tables().await?;

    // 创建存储服务
    let endpoint = std::env::var("S3_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:9000".to_string());
    let region = std::env::var("S3_REGION")
        .unwrap_or_else(|_| "us-east-1".to_string());
    let access_key = std::env::var("S3_ACCESS_KEY")
        .unwrap_or_else(|_| "minioadmin".to_string());
    let secret_key = std::env::var("S3_SECRET_KEY")
        .unwrap_or_else(|_| "minioadmin".to_string());

    let storage_service = S3StorageService::new(
        endpoint,
        region,
        access_key,
        secret_key,
        repository.clone(),
    ).await?;

    // 创建 gRPC 服务
    let grpc_service = MediaGrpcService::new(storage_service);

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
        .unwrap_or_else(|_| "50056".to_string())
        .parse::<u16>()?;
    let addr = format!("{}:{}", service_host, service_port).parse()?;

    // 创建应用构建器
    let app = AppBuilder::new("media")
        .version("1.0.0")
        .weight(1)
        .tag("prod")
        .meta("type", "media")
        .register(registry)
        .build();

    info!("Media Service listening on {}", addr);

    // 运行服务
    app.run(service_host.as_str(), service_port, |mut server, addr| async move {
        server
            .add_service(MediaServer::new(grpc_service))
            .serve(addr)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 