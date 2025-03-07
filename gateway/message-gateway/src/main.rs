use anyhow::Result;
use flare_rpc_core as core;
use log::{info, error};
use tonic::transport::Server;

mod service;
mod handler;
mod config;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    core::init_logger();
    info!("Starting Message Gateway Service...");

    // 加载配置
    let config = config::load_config()?;
    info!("Configuration loaded successfully");

    // 创建服务实例
    let addr = config.server.addr.parse()?;
    let message_gateway = service::MessageGatewayService::new(config.clone());

    info!("Message Gateway Service listening on {}", addr);

    // 启动 gRPC 服务器
    Server::builder()
        .add_service(proto_crate::message_gateway::message_gateway_server::MessageGatewayServer::new(message_gateway))
        .serve(addr)
        .await?;

    Ok(())
} 