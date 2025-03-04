use anyhow::Result;
use common::config::{Config, Environment};
use message_gateway::start_message_gateway;
use tracing::error;

#[tokio::main]
async fn main() -> Result<()> {
    // 从环境变量获取运行环境
    let env = std::env::var("FLARE_ENV")
        .unwrap_or_else(|_| "development".to_string())
        .parse::<Environment>()
        .unwrap_or(Environment::Development);

    // 加载配置
    let config = Config::from_env_file(env)?;

    // 初始化日志
    common::log::init_logger(&config.log)?;

    // 启动服务
    if let Err(e) = start_message_gateway(config).await {
        error!("消息网关服务启动失败: {}", e);
        std::process::exit(1);
    }

    Ok(())
} 