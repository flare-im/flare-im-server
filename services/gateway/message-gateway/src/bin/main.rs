use anyhow::Result;
use flare_core::{
    logs::Logger,
};
use log::{info, error};
use tokio::try_join;
use common::config::{Config, Environment};
use message_gateway::infrastructure::config::init_config;
use message_gateway::infrastructure::log::init_log;
use message_gateway::interfaces::grpc::server::start_grpc_server;
use message_gateway::interfaces::im::start_im_server;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载配置
    init_config(Environment::Development)?;

    // 初始化日志
    init_log()?;

    // 启动 gRPC 服务和 IM 服务
    try_join!(
        start_grpc_server(),
        start_im_server()
    )?;

    Ok(())
} 