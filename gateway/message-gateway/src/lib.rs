use anyhow::Result;
use async_trait::async_trait;
use flare_core::context::AppContext;
use flare_core::error::Result as FlareResult;
use flare_im_core::connections::quic_conf::create_server_config;
use flare_im_core::server::auth_handler::{AuthCommandHandler, AuthHandler, DefAuthHandler};
use flare_im_core::server::handlers::ServerMessageHandler;
use flare_im_core::server::server_handler::{DefServerHandler, ServerCommandHandler, ServerHandler};
use flare_im_core::server::sys_handler::{DefSystemHandler, SystemCommandHandler, SystemHandler};
use flare_im_core::telecom::FlareServer;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use log::{error, info};
use std::time::Duration;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.gateway");

// 消息网关处理器
struct MessageGatewayHandler {
    // 可以添加需要的字段，如连接管理器等
}

impl MessageGatewayHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ServerHandler for MessageGatewayHandler {
    async fn handle_send_message(&self, ctx: &AppContext) -> FlareResult<Response> {
        let mut response = Response::default();
        let msg = ctx.data();
        
        // 处理客户端发送的消息
        if let Ok(content) = String::from_utf8(msg.to_vec()) {
            info!("Received message from client: {}", content);
            
            // 这里可以添加消息处理逻辑
            // 例如：消息转发、存储等
            
            response.code = 0; // Success
            response.message = "Message received".to_string();
        }
        
        Ok(response)
    }

    async fn handle_pull_message(&self, _ctx: &AppContext) -> FlareResult<Response> {
        Ok(Response::default())
    }

    async fn handle_request(&self, _ctx: &AppContext) -> FlareResult<Response> {
        Ok(Response::default())
    }

    async fn handle_ack(&self, _ctx: &AppContext) -> FlareResult<Response> {
        Ok(Response::default())
    }
}

// gRPC 服务实现
struct MessageGatewayService;

#[tonic::async_trait]
impl message_gateway_server::MessageGateway for MessageGatewayService {
    async fn push_message(
        &self,
        request: Request<PushMessageRequest>,
    ) -> Result<Response<PushMessageResponse>, Status> {
        let req = request.into_inner();
        info!("Received push message request: {:?}", req);
        
        // 这里实现消息推送逻辑
        // 1. 检查接收者在线状态
        // 2. 根据设备类型选择推送方式
        // 3. 处理离线消息存储
        // 4. 返回推送结果
        
        let response = PushMessageResponse {
            server_msg_id: 0, // 需要生成
            push_results: Default::default(),
            status: 0, // Success
            error: "".to_string(),
        };
        
        Ok(Response::new(response))
    }

    async fn batch_push_message(
        &self,
        request: Request<BatchPushMessageRequest>,
    ) -> Result<Response<BatchPushMessageResponse>, Status> {
        let req = request.into_inner();
        info!("Received batch push message request: {:?}", req);
        
        // 实现批量推送逻辑
        
        let response = BatchPushMessageResponse {
            results: vec![],
            status: 0, // Success
            error: "".to_string(),
        };
        
        Ok(Response::new(response))
    }

    async fn broadcast_message(
        &self,
        request: Request<BroadcastMessageRequest>,
    ) -> Result<Response<BroadcastMessageResponse>, Status> {
        let req = request.into_inner();
        info!("Received broadcast message request: {:?}", req);
        
        // 实现广播消息逻辑
        
        let response = BroadcastMessageResponse {
            server_msg_id: 0, // 需要生成
            success_count: 0,
            failed_count: 0,
            status: 0, // Success
            error: "".to_string(),
        };
        
        Ok(Response::new(response))
    }

    async fn get_user_status(
        &self,
        request: Request<GetUserStatusRequest>,
    ) -> Result<Response<GetUserStatusResponse>, Status> {
        let req = request.into_inner();
        info!("Received get user status request: {:?}", req);
        
        // 实现获取用户状态逻辑
        
        let response = GetUserStatusResponse {
            user_status: Default::default(),
            status: 0, // Success
            error: "".to_string(),
        };
        
        Ok(Response::new(response))
    }
}

/// 启动消息网关服务
pub async fn start_message_gateway(config: common::config::Config) -> Result<()> {
    // 初始化日志
    common::log::init_logger(&config.log)?;

    // 创建 Consul 配置
    let consul_config = ConsulConfig {
        addr: format!("{}:{}", config.consul.host, config.consul.port),
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };

    // 创建 Consul 注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(15)).await?;

    // 创建并配置应用
    let app = AppBuilder::new(&config.service.name)
        .version(&config.service.metadata.get("version").unwrap_or(&"1.0.0".to_string()))
        .tags(&config.service.tags)
        .meta("protocol", "grpc")
        .weight(10)
        .register(registry)
        .build();

    // 创建消息网关处理器
    let handler = ServerMessageHandler::<MessageGatewayHandler, DefAuthHandler, DefSystemHandler>::new(
        AuthCommandHandler::new(DefAuthHandler::new()),
        ServerCommandHandler::new(MessageGatewayHandler::new()),
        SystemCommandHandler::new(DefSystemHandler::new()),
    );

    // 创建并配置 WebSocket/QUIC 服务器
    let ws_server = FlareServer::builder()
        .ws_addr(format!("{}:{}", config.service.host, config.service.port))
        .quic_addr(format!("{}:{}", config.service.host, config.service.port + 1))
        .quic_server_name("message.gateway.quic.cn")
        .quic_cert_path("certs/cert.pem")
        .quic_key_path("certs/key.pem")
        .handler(handler)
        .build()?;

    // 创建 gRPC 服务
    let message_gateway_service = MessageGatewayService;
    let message_gateway_server = message_gateway_server::MessageGatewayServer::new(message_gateway_service);

    // 启动服务
    let grpc_addr = format!("{}:{}", config.service.host, config.service.port + 2);
    app.run(&config.service.host, config.service.port + 2, |mut server| async move {
        server
            .add_service(message_gateway_server)
            .serve(grpc_addr.parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    // 启动 WebSocket/QUIC 服务器
    if let Err(e) = ws_server.run().await {
        error!("WebSocket/QUIC server error: {}", e);
    }

    Ok(())
}
