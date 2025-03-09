use anyhow::Result;
use flare_im_core::server::auth_handler::AuthCommandHandler;
use flare_im_core::server::handlers::ServerMessageHandler;
use flare_im_core::server::server_handler::ServerCommandHandler;
use flare_im_core::server::sys_handler::SystemCommandHandler;
use flare_im_core::telecom::FlareServer;
use log::{info, error};

use crate::application::auth::AuthService;
use crate::application::message::MessageService;
use crate::application::system::SystemService;
use crate::infrastructure::config::get_config;
use super::auth::CustomAuthHandler;
use super::message::CustomMessageHandler;
use super::system::CustomSystemHandler;

pub async fn start_im_server() -> Result<()> {
    info!("Starting IM server...");

    // 获取全局配置
    let config = get_config();
    let ws_port = config.extensions.get("websocket")
        .and_then(|v| v.get("port"))
        .and_then(|v| v.as_u64())
        .unwrap_or(8080);
    let quic_port = config.extensions.get("quic")
        .and_then(|v| v.get("port"))
        .and_then(|v| v.as_u64())
        .unwrap_or(8081);
    let quic_server_name = config.extensions.get("quic")
        .and_then(|v| v.get("server_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("hugo.im.quic.cn");
    let cert_path = config.extensions.get("quic")
        .and_then(|v| v.get("cert_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("certs/cert.pem");
    let key_path = config.extensions.get("quic")
        .and_then(|v| v.get("key_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("certs/key.pem");

    // 创建服务实例
    let auth_service = AuthService::new();
    let message_service = MessageService::new();
    let system_service = SystemService::new();

    // 创建自定义处理器
    let auth_handler = CustomAuthHandler::new(auth_service);
    let message_handler = CustomMessageHandler::new(message_service);
    let system_handler = CustomSystemHandler::new(system_service);
    
    // 创建服务器处理器
    let handler = ServerMessageHandler::<CustomMessageHandler, CustomAuthHandler, CustomSystemHandler>::new(
        AuthCommandHandler::new(auth_handler),
        ServerCommandHandler::new(message_handler),
        SystemCommandHandler::new(system_handler)
    );

    // 创建并配置服务器
    let server = FlareServer::builder()
        .ws_addr(format!("{}:{}", config.service.host, ws_port))
        .quic_addr(format!("{}:{}", config.service.host, quic_port))
        .quic_server_name(quic_server_name)
        .quic_cert_path(cert_path)
        .quic_key_path(key_path)
        .handler(handler)
        .build()?;

    info!("IM server starting on ws://{}:{} and quic://{}:{}", 
        config.service.host, ws_port,
        config.service.host, quic_port
    );
    
    // 运行服务器
    if let Err(e) = server.run().await {
        error!("IM Server error: {}", e);
    }

    Ok(())
} 