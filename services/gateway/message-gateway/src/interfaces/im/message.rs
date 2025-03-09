use flare_core::error::Result;
use async_trait::async_trait;
use flare_core::context::AppContext;
use flare_core::flare_net::net::{Response, ResCode};
use flare_im_core::server::server_handler::ServerHandler;
use log::{info, error};
use prost::Message;

use crate::application::message::MessageService;

pub struct CustomMessageHandler {
    message_service: MessageService,
}

impl CustomMessageHandler {
    pub fn new(message_service: MessageService) -> Self {
        Self { message_service }
    }
}

#[async_trait]
impl ServerHandler for CustomMessageHandler {
    async fn handle_send_message(&self, ctx: &AppContext) -> Result<Response> {
        let mut response = Response::default();
        let msg = ctx.data();
        
        match self.message_service.handle_message(msg).await {
            Ok(result) => {
                response.code = ResCode::Success as i32;
                response.message = "Message sent".to_string();
                response.data = result;
            }
            Err(e) => {
                error!("Failed to handle message: {}", e);
                response.code = ResCode::BusinessError as i32;
                response.message = e.to_string();
            }
        }
        
        Ok(response)
    }

    async fn handle_pull_message(&self, ctx: &AppContext) -> Result<Response> {
        let mut response = Response::default();
        let user_id: Option<String> = ctx.user_id();

        if let Some(user_id) = user_id {
            match self.message_service.pull_messages(user_id.as_str()).await {
                Ok(messages) => {
                    response.code = ResCode::Success as i32;
                    response.message = "Messages pulled".to_string();
                    response.data = messages;
                }
                Err(e) => {
                    error!("Failed to pull messages: {}", e);
                    response.code = ResCode::BusinessError as i32;
                    response.message = e.to_string();
                }
            }
        }
        
        Ok(response)
    }

    async fn handle_request(&self, ctx: &AppContext) -> Result<Response> {
        let mut response = Response::default();
        let request_data = ctx.data();
        
        match self.message_service.handle_request(request_data).await {
            Ok(result) => {
                response.code = ResCode::Success as i32;
                response.message = "Request processed".to_string();
                response.data = result;
            }
            Err(e) => {
                error!("Failed to handle request: {}", e);
                response.code = ResCode::BusinessError as i32;
                response.message = e.to_string();
            }
        }
        
        Ok(response)
    }

    async fn handle_ack(&self, ctx: &AppContext) -> Result<Response> {
        let mut response = Response::default();
        let ack_data = ctx.data();
        
        match self.message_service.handle_ack(ack_data).await {
            Ok(_) => {
                response.code = ResCode::Success as i32;
                response.message = "Ack processed".to_string();
            }
            Err(e) => {
                error!("Failed to handle ack: {}", e);
                response.code = ResCode::BusinessError as i32;
                response.message = e.to_string();
            }
        }
        
        Ok(response)
    }
} 