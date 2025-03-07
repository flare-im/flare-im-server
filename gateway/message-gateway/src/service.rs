use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use log::{info, error, debug};

use proto_crate::message_gateway::{
    message_gateway_server::MessageGateway,
    ConnectRequest, ConnectResponse,
    DisconnectRequest, DisconnectResponse,
    SendMessageRequest, SendMessageResponse,
};

use crate::config::Config;
use crate::handler::ConnectionHandler;

#[derive(Debug)]
pub struct MessageGatewayService {
    config: Arc<Config>,
    handler: Arc<RwLock<ConnectionHandler>>,
}

impl MessageGatewayService {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            handler: Arc::new(RwLock::new(ConnectionHandler::new())),
        }
    }
}

#[tonic::async_trait]
impl MessageGateway for MessageGatewayService {
    async fn connect(
        &self,
        request: Request<ConnectRequest>
    ) -> Result<Response<ConnectResponse>, Status> {
        let req = request.into_inner();
        debug!("Received connect request from user_id: {}", req.user_id);

        let handler = self.handler.write().await;
        match handler.handle_connect(req.user_id, req.device_id).await {
            Ok(_) => {
                info!("User {} connected successfully", req.user_id);
                Ok(Response::new(ConnectResponse {
                    session_id: format!("{}:{}", req.user_id, req.device_id),
                }))
            }
            Err(e) => {
                error!("Failed to handle connect request: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }

    async fn disconnect(
        &self,
        request: Request<DisconnectRequest>
    ) -> Result<Response<DisconnectResponse>, Status> {
        let req = request.into_inner();
        debug!("Received disconnect request for session: {}", req.session_id);

        let handler = self.handler.write().await;
        match handler.handle_disconnect(&req.session_id).await {
            Ok(_) => {
                info!("Session {} disconnected successfully", req.session_id);
                Ok(Response::new(DisconnectResponse {}))
            }
            Err(e) => {
                error!("Failed to handle disconnect request: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }

    async fn send_message(
        &self,
        request: Request<SendMessageRequest>
    ) -> Result<Response<SendMessageResponse>, Status> {
        let req = request.into_inner();
        debug!("Received message from {}: {:?}", req.from_user_id, req.content);

        let handler = self.handler.read().await;
        match handler.handle_message(req).await {
            Ok(_) => {
                Ok(Response::new(SendMessageResponse {
                    message_id: uuid::Uuid::new_v4().to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to handle message: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
} 