use anyhow::Result;
use flare_rpc_core as core;
use log::{info, error};
use tonic::transport::Server;

mod service;
mod config;
mod storage;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    core::init_logger();
    info!("Starting Media Service...");

    // 加载配置
    let config = config::load_config()?;
    info!("Configuration loaded successfully");

    // 初始化 S3 客户端
    let s3_client = storage::init_s3_client(&config.s3).await?;
    info!("S3 client initialized successfully");

    // 创建服务实例
    let addr = config.server.addr.parse()?;
    let media_service = service::MediaService::new(config.clone(), s3_client);

    info!("Media Service listening on {}", addr);

    // 启动 gRPC 服务器
    Server::builder()
        .add_service(proto_crate::media::media_server::MediaServer::new(media_service))
        .serve(addr)
        .await?;

    Ok(())
} 