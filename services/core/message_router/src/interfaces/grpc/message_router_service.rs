use crate::application::message_router::MessageRouterService;
use proto_crate::api::im::service::router::{
    message_router_server::MessageRouter,
    DistributeMessagesRequest, DistributeMessagesResponse,
    DistributeResult, FilterMessagesRequest,
    FilterMessagesResponse, FilterResult,
    HandleMessagesPriorityRequest, HandleMessagesPriorityResponse,
    PriorityResult, RouteUpstreamMessagesRequest, RouteUpstreamMessagesResponse, RouteUpstreamResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use proto_crate::api::im::common::Error;
use crate::entities::Message;

pub struct MessageRouterGrpcService {
    message_router: Arc<MessageRouterService>,
}

impl MessageRouterGrpcService {
    pub fn new(message_router: Arc<MessageRouterService>) -> Self {
        Self { message_router }
    }
}

#[tonic::async_trait]
impl MessageRouter for MessageRouterGrpcService {
    async fn filter_messages(
        &self,
        request: Request<FilterMessagesRequest>,
    ) -> Result<Response<FilterMessagesResponse>, Status> {
        let req = request.into_inner();
        let mut results = Vec::new();

        for filter_message in req.messages {
            let proto_msg = filter_message.message.as_ref()
                .ok_or_else(|| Status::invalid_argument("message is required"))?;
            let message = Message::from_proto(proto_msg);
            
            let pre_process_code = self.message_router.pre_process(&message).await
                .map_err(|e| Status::internal(e.to_string()))?;

            results.push(FilterResult {
                message_id: message.server_msg_id.clone(),
                passed: pre_process_code == 0,
                filter_results: HashMap::new(),
                error: if pre_process_code != 0 {
                    Some(Error {
                        code: pre_process_code,
                        message: format!("Message filter failed with code: {}", pre_process_code),
                        details: "".to_string(),
                    })
                } else {
                    None
                },
            });
        }

        Ok(Response::new(FilterMessagesResponse {
            results,
            error: None,
        }))
    }

    async fn route_upstream_messages(
        &self,
        request: Request<RouteUpstreamMessagesRequest>,
    ) -> Result<Response<RouteUpstreamMessagesResponse>, Status> {
        let req = request.into_inner();
        let mut results = Vec::new();

        for upstream_message in req.messages {
            let proto_msg = upstream_message.message.as_ref()
                .ok_or_else(|| Status::invalid_argument("message is required"))?;
            let message = Message::from_proto(proto_msg);
            
            let (success, error, routes) = self.message_router.route_message(&message).await
                .map_err(|e| Status::internal(e.to_string()))?;

            results.push(RouteUpstreamResult {
                message_id: message.server_msg_id.clone(),
                success,
                error: error.map(|e|Error {
                    code: 0,
                    message: e,
                    details: "".to_string(),
                }),
            });
        }

        Ok(Response::new(RouteUpstreamMessagesResponse {
            results,
            error: None,
        }))
    }

    async fn distribute_messages(
        &self,
        request: Request<DistributeMessagesRequest>,
    ) -> Result<Response<DistributeMessagesResponse>, Status> {
        let req = request.into_inner();
        let mut messages = Vec::new();

        for distribute_message in req.messages {
            let proto_msg = distribute_message.message.as_ref()
                .ok_or_else(|| Status::invalid_argument("message is required"))?;
            let message = Message::from_proto(proto_msg);
            messages.push(message);
        }

        let results_map = self.message_router.process_messages(messages).await
            .map_err(|e| Status::internal(e.to_string()))?;

        let results = results_map.into_iter()
            .map(|(message_id, (success, error, routes))| {
                let mut distribution_results = HashMap::new();
                for route in routes {
                    distribution_results.insert(route, success);
                }

                DistributeResult {
                    message_id,
                    distribution_results,
                    error: error.map(|e| Error {
                        code: 0,
                        message: e,
                        details: "".to_string(),
                    }),
                }
            })
            .collect();

        Ok(Response::new(DistributeMessagesResponse {
            results,
            error: None,
        }))
    }

    async fn handle_messages_priority(
        &self,
        request: Request<HandleMessagesPriorityRequest>,
    ) -> Result<Response<HandleMessagesPriorityResponse>, Status> {
        let req = request.into_inner();
        let mut results = Vec::new();

        for priority_message in req.messages {
            let proto_msg = priority_message.message.as_ref()
                .ok_or_else(|| Status::invalid_argument("message is required"))?;
            let message = Message::from_proto(proto_msg);
            
            // 默认优先级处理
            let priority = message.options.get("priority")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0);

            results.push(PriorityResult {
                message_id: message.server_msg_id.clone(),
                priority,
                error: None,
            });
        }

        Ok(Response::new(HandleMessagesPriorityResponse {
            results,
            error: None,
        }))
    }
} 