use crate::application::message::MessageService;
use crate::infrastructure::config::get_config;
use crate::interfaces::grpc::service::GrpcMessageService;
use anyhow::Result;
use flare_rpc_core::discover::consul::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::AppBuilder;
use log::info;
use proto_crate::api::im::gateway::message_gateway_server::MessageGatewayServer;
use std::net::SocketAddr;
use std::time::Duration;
use tonic::transport::Server;

pub async fn start_grpc_server() -> Result<()> {
    info!("Starting gRPC server...");

    // 获取全局配置
    let config = get_config();
    // 创建 Consul 配置
    let con_addr = format!("{}:{}", config.consul.host, config.consul.port);
    let consul_config = ConsulConfig {
        addr: con_addr,
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };
    // 创建 Consul 注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(config.consul.register_interval)).await?;

    // 创建服务地址
    let addr: SocketAddr = format!("{}:{}", config.service.host, config.service.port).parse()?;
    info!("gRPC server listening on {}", addr);
    // 创建并配置应用
    let mut app_builder = AppBuilder::new(config.service.name.clone())
        .version("1.0.0")
        .weight(config.service.weight)
        .register(registry);
    // 处理tag
    for t in config.service.tags.clone() {
        app_builder = app_builder.tag(t)
    }
    // 处理mate
    for (k, v) in config.service.metadata.clone() {
        app_builder = app_builder.meta(k, v)
    }
    let app = app_builder.build();

    // 创建服务实例
    let message_service = MessageService::new();
    let grpc_handler = GrpcMessageService::new(message_service);

    // 运行服务器
    app.run(config.service.host.clone().as_str(), config.service.port, |mut server, addr| async move {
        server.add_service(MessageGatewayServer::new(grpc_handler))
            .serve(addr)
            .await
            .map_err(|e| e.into())
    }).await.expect("server start filed");
    Ok(())
} 