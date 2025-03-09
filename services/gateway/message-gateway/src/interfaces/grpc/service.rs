use tonic::{Request, Response, Status};
use proto_crate::api::im::gateway::{message_gateway_server::MessageGateway, PushMessageRequest, PushMessageResponse, BatchPushMessageRequest, BatchPushMessageResponse, BroadcastMessageRequest, BroadcastMessageResponse, GetUserStatusRequest, GetUserStatusResponse, RegisterConnectionRequest, RegisterConnectionResponse, UnregisterConnectionRequest, UnregisterConnectionResponse, HeartBeatRequest, HeartBeatResponse};

use crate::application::message::MessageService;

pub struct GrpcMessageService {
    message_service: MessageService,
}

impl GrpcMessageService {
    pub fn new(message_service: MessageService) -> Self {
        Self { message_service }
    }
}

#[tonic::async_trait]
impl MessageGateway for GrpcMessageService {
    async fn push_message(
        &self,
        request: Request<PushMessageRequest>
    ) -> Result<Response<PushMessageResponse>, Status> {
        let req = request.into_inner();
        match self.message_service.push_message(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(e.to_string()))
        }
    }

    async fn batch_push_message(
        &self,
        request: Request<BatchPushMessageRequest>
    ) -> Result<Response<BatchPushMessageResponse>, Status> {
        let req = request.into_inner();
        match self.message_service.batch_push_message(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(e.to_string()))
        }
    }

    async fn broadcast_message(
        &self,
        request: Request<BroadcastMessageRequest>
    ) -> Result<Response<BroadcastMessageResponse>, Status> {
        let req = request.into_inner();
        match self.message_service.broadcast_message(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(e.to_string()))
        }
    }

    async fn get_user_status(
        &self,
        request: Request<GetUserStatusRequest>
    ) -> Result<Response<GetUserStatusResponse>, Status> {
        let req = request.into_inner();
        match self.message_service.get_user_status(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(e.to_string()))
        }
    }

    async fn register_connection(&self, request: Request<RegisterConnectionRequest>) -> Result<Response<RegisterConnectionResponse>, Status> {
        todo!()
    }

    async fn unregister_connection(&self, request: Request<UnregisterConnectionRequest>) -> Result<Response<UnregisterConnectionResponse>, Status> {
        todo!()
    }

    async fn heart_beat(&self, request: Request<HeartBeatRequest>) -> Result<Response<HeartBeatResponse>, Status> {
        todo!()
    }
} 