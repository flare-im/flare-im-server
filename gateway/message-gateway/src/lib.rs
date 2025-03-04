use anyhow::Result;
use flare_rpc_core::server::GrpcServer;
use flare_im_core::message::MessageService;
use proto::api::im::message::{
    SendMessageRequest, SendMessageResponse,
    GetMessageRequest, GetMessageResponse,
    DeleteMessageRequest, DeleteMessageResponse,
};
use tonic::{Request, Response, Status};
use tracing::{info, error};

/// 消息网关服务
pub struct MessageGatewayService {
    message_service: MessageService,
}

impl MessageGatewayService {
    /// 创建新的消息网关服务实例
    pub fn new(message_service: MessageService) -> Self {
        Self { message_service }
    }
}

#[tonic::async_trait]
impl proto::api::im::message::message_gateway_server::MessageGateway for MessageGatewayService {
    /// 发送消息
    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到发送消息请求: {:?}", req);
        
        match self.message_service.send_message(req.clone()).await {
            Ok(message_id) => {
                let response = SendMessageResponse {
                    message_id,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("发送消息失败: {}", e);
                let response = SendMessageResponse {
                    message_id: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    /// 获取消息
    async fn get_message(
        &self,
        request: Request<GetMessageRequest>,
    ) -> Result<Response<GetMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到获取消息请求: {:?}", req);

        match self.message_service.get_message(req.message_id).await {
            Ok(message) => {
                let response = GetMessageResponse {
                    message: Some(message),
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("获取消息失败: {}", e);
                let response = GetMessageResponse {
                    message: None,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    /// 删除消息
    async fn delete_message(
        &self,
        request: Request<DeleteMessageRequest>,
    ) -> Result<Response<DeleteMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到删除消息请求: {:?}", req);

        match self.message_service.delete_message(req.message_id).await {
            Ok(_) => {
                let response = DeleteMessageResponse {
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("删除消息失败: {}", e);
                let response = DeleteMessageResponse {
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动消息网关服务
pub async fn start_message_gateway(config: common::config::Config) -> Result<()> {
    let message_service = MessageService::new(config.clone())?;
    let service = MessageGatewayService::new(message_service);
    
    let server = GrpcServer::new(config)
        .register_service(proto::api::im::message::message_gateway_server::MessageGatewayServer::new(service))
        .build()?;

    info!("消息网关服务启动成功");
    server.serve().await?;
    Ok(())
}
