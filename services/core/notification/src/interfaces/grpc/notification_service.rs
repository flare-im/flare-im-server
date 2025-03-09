use tonic::{Request, Response, Status};
use std::sync::Arc;
use proto_crate::api::im::service::notification::{
    notification_server::Notification,
    SendNotificationRequest,
    SendNotificationResponse,
    RegisterDeviceRequest,
    RegisterDeviceResponse,
    UnregisterDeviceRequest,
    UnregisterDeviceResponse,
    UpdateDeviceTokenRequest,
    UpdateDeviceTokenResponse,
    CreateTemplateRequest,
    CreateTemplateResponse,
    UpdateTemplateRequest,
    UpdateTemplateResponse,
    DeleteTemplateRequest,
    DeleteTemplateResponse,
    GetTemplateRequest,
    GetTemplateResponse,
    CancelNotificationRequest,
    CancelNotificationResponse,
    GetNotificationStatusRequest,
    GetNotificationStatusResponse,
    GetUserNotificationsRequest,
    GetUserNotificationsResponse,
    GetPlatformStatisticsRequest,
    GetPlatformStatisticsResponse,
};
use crate::domain::{
    services::notification_service::NotificationService,
    entities::notification::{
        Notification as DomainNotification,
        NotificationTemplate as DomainTemplate,
        DeviceInfo,
        Platform,
        NotificationType,
        Priority,
        TargetType,
        NotificationMetadata,
    },
};
use uuid::Uuid;
use chrono::Utc;

pub struct NotificationGrpcService {
    notification_service: Arc<dyn NotificationService + Send + Sync>,
}

impl NotificationGrpcService {
    pub fn new(notification_service: impl NotificationService + Send + Sync + 'static) -> Self {
        Self {
            notification_service: Arc::new(notification_service),
        }
    }

    // 转换请求到领域模型
    fn convert_to_domain_notification(&self, request: &SendNotificationRequest) -> Result<DomainNotification, Status> {
        Ok(DomainNotification {
            id: Uuid::new_v4(),
            title: request.title.clone(),
            content: request.content.clone(),
            notification_type: match request.notification_type() {
                0 => NotificationType::Message,
                1 => NotificationType::System,
                2 => NotificationType::Activity,
                _ => NotificationType::Custom(request.notification_type.to_string()),
            },
            priority: match request.priority() {
                0 => Priority::Low,
                1 => Priority::Normal,
                2 => Priority::High,
                _ => Priority::Normal,
            },
            target_type: match request.target_type() {
                0 => TargetType::Single,
                1 => TargetType::Multiple,
                2 => TargetType::Topic,
                3 => TargetType::Broadcast,
                _ => return Err(Status::invalid_argument("Invalid target type")),
            },
            target_users: request.target_users.clone(),
            platform: request.platform.iter().map(|p| match p {
                0 => Platform::IOS,
                1 => Platform::Android,
                2 => Platform::Web,
                3 => Platform::All,
                _ => Platform::All,
            }).collect(),
            status: crate::domain::entities::notification::NotificationStatus::Pending,
            metadata: NotificationMetadata {
                category: request.category.clone(),
                badge: request.badge.map(|b| b as i32),
                sound: request.sound.clone(),
                image_url: request.image_url.clone(),
                deep_link: request.deep_link.clone(),
                custom_data: request.custom_data.clone(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            scheduled_at: None,
            expired_at: None,
        })
    }

    // 转换领域模型到响应
    fn convert_to_response(&self, result: &crate::domain::entities::notification::NotificationResult) -> SendNotificationResponse {
        SendNotificationResponse {
            notification_id: result.notification_id.to_string(),
            success: result.success,
            sent_count: result.sent_count,
            failed_count: result.failed_count,
            error: result.error.clone(),
            platform_results: result.platform_results.iter().map(|pr| {
                proto_crate::api::im::service::notification::PlatformResult {
                    platform: match pr.platform {
                        Platform::IOS => 0,
                        Platform::Android => 1,
                        Platform::Web => 2,
                        Platform::All => 3,
                    },
                    provider: pr.provider.clone(),
                    success: pr.success,
                    message_id: pr.message_id.clone(),
                    error: pr.error.clone(),
                }
            }).collect(),
        }
    }
}

#[tonic::async_trait]
impl Notification for NotificationGrpcService {
    async fn send_notification(
        &self,
        request: Request<SendNotificationRequest>,
    ) -> Result<Response<SendNotificationResponse>, Status> {
        let notification = self.convert_to_domain_notification(request.get_ref())?;
        
        match self.notification_service.send_notification(notification).await {
            Ok(result) => Ok(Response::new(self.convert_to_response(&result))),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn register_device(
        &self,
        request: Request<RegisterDeviceRequest>,
    ) -> Result<Response<RegisterDeviceResponse>, Status> {
        let req = request.get_ref();
        let device_info = DeviceInfo {
            user_id: req.user_id.clone(),
            device_id: req.device_id.clone(),
            platform: match req.platform() {
                0 => Platform::IOS,
                1 => Platform::Android,
                2 => Platform::Web,
                _ => Platform::All,
            },
            push_token: req.push_token.clone(),
            app_version: req.app_version.clone(),
            provider: req.provider.clone(),
            is_active: true,
            last_active_at: Utc::now(),
        };

        match self.notification_service.register_device(device_info).await {
            Ok(_) => Ok(Response::new(RegisterDeviceResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn unregister_device(
        &self,
        request: Request<UnregisterDeviceRequest>,
    ) -> Result<Response<UnregisterDeviceResponse>, Status> {
        let req = request.get_ref();
        match self.notification_service.unregister_device(&req.user_id, &req.device_id).await {
            Ok(_) => Ok(Response::new(UnregisterDeviceResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn update_device_token(
        &self,
        request: Request<UpdateDeviceTokenRequest>,
    ) -> Result<Response<UpdateDeviceTokenResponse>, Status> {
        let req = request.get_ref();
        match self.notification_service.update_device_token(&req.user_id, &req.device_id, &req.new_token).await {
            Ok(_) => Ok(Response::new(UpdateDeviceTokenResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn create_template(
        &self,
        request: Request<CreateTemplateRequest>,
    ) -> Result<Response<CreateTemplateResponse>, Status> {
        let req = request.get_ref();
        let template = DomainTemplate {
            id: Uuid::new_v4(),
            name: req.name.clone(),
            title_template: req.title_template.clone(),
            content_template: req.content_template.clone(),
            category: req.category.clone(),
            platform: req.platform.iter().map(|p| match p {
                0 => Platform::IOS,
                1 => Platform::Android,
                2 => Platform::Web,
                3 => Platform::All,
                _ => Platform::All,
            }).collect(),
            metadata: NotificationMetadata {
                category: req.category.clone(),
                badge: None,
                sound: None,
                image_url: None,
                deep_link: None,
                custom_data: Default::default(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        match self.notification_service.create_template(template).await {
            Ok(created) => Ok(Response::new(CreateTemplateResponse {
                template_id: created.id.to_string(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn update_template(
        &self,
        request: Request<UpdateTemplateRequest>,
    ) -> Result<Response<UpdateTemplateResponse>, Status> {
        let req = request.get_ref();
        let template_id = Uuid::parse_str(&req.template_id)
            .map_err(|_| Status::invalid_argument("Invalid template ID"))?;

        let template = DomainTemplate {
            id: template_id,
            name: req.name.clone(),
            title_template: req.title_template.clone(),
            content_template: req.content_template.clone(),
            category: req.category.clone(),
            platform: req.platform.iter().map(|p| match p {
                0 => Platform::IOS,
                1 => Platform::Android,
                2 => Platform::Web,
                3 => Platform::All,
                _ => Platform::All,
            }).collect(),
            metadata: NotificationMetadata {
                category: req.category.clone(),
                badge: None,
                sound: None,
                image_url: None,
                deep_link: None,
                custom_data: Default::default(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        match self.notification_service.update_template(template).await {
            Ok(_) => Ok(Response::new(UpdateTemplateResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn delete_template(
        &self,
        request: Request<DeleteTemplateRequest>,
    ) -> Result<Response<DeleteTemplateResponse>, Status> {
        let req = request.get_ref();
        match self.notification_service.delete_template(&req.template_id).await {
            Ok(_) => Ok(Response::new(DeleteTemplateResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_template(
        &self,
        request: Request<GetTemplateRequest>,
    ) -> Result<Response<GetTemplateResponse>, Status> {
        let req = request.get_ref();
        match self.notification_service.get_template(&req.template_id).await {
            Ok(Some(template)) => Ok(Response::new(GetTemplateResponse {
                template: Some(proto_crate::api::im::service::notification::NotificationTemplate {
                    template_id: template.id.to_string(),
                    name: template.name,
                    title_template: template.title_template,
                    content_template: template.content_template,
                    category: template.category,
                    platform: template.platform.iter().map(|p| match p {
                        Platform::IOS => 0,
                        Platform::Android => 1,
                        Platform::Web => 2,
                        Platform::All => 3,
                    }).collect(),
                }),
            })),
            Ok(None) => Ok(Response::new(GetTemplateResponse { template: None })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn cancel_notification(
        &self,
        request: Request<CancelNotificationRequest>,
    ) -> Result<Response<CancelNotificationResponse>, Status> {
        let req = request.get_ref();
        match self.notification_service.cancel_notification(&req.notification_id).await {
            Ok(_) => Ok(Response::new(CancelNotificationResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_notification_status(
        &self,
        request: Request<GetNotificationStatusRequest>,
    ) -> Result<Response<GetNotificationStatusResponse>, Status> {
        let req = request.get_ref();
        match self.notification_service.get_notification_status(&req.notification_id).await {
            Ok(Some(result)) => Ok(Response::new(GetNotificationStatusResponse {
                status: Some(proto_crate::api::im::service::notification::NotificationStatus {
                    notification_id: result.notification_id.to_string(),
                    success: result.success,
                    sent_count: result.sent_count,
                    failed_count: result.failed_count,
                    error: result.error,
                    platform_results: result.platform_results.iter().map(|pr| {
                        proto_crate::api::im::service::notification::PlatformResult {
                            platform: match pr.platform {
                                Platform::IOS => 0,
                                Platform::Android => 1,
                                Platform::Web => 2,
                                Platform::All => 3,
                            },
                            provider: pr.provider.clone(),
                            success: pr.success,
                            message_id: pr.message_id.clone(),
                            error: pr.error.clone(),
                        }
                    }).collect(),
                }),
            })),
            Ok(None) => Ok(Response::new(GetNotificationStatusResponse { status: None })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_user_notifications(
        &self,
        request: Request<GetUserNotificationsRequest>,
    ) -> Result<Response<GetUserNotificationsResponse>, Status> {
        let req = request.get_ref();
        let notification_type = match req.notification_type() {
            0 => Some(NotificationType::Message),
            1 => Some(NotificationType::System),
            2 => Some(NotificationType::Activity),
            _ => None,
        };

        match self.notification_service.get_user_notifications(
            &req.user_id,
            notification_type,
            req.limit,
            req.offset,
        ).await {
            Ok(notifications) => Ok(Response::new(GetUserNotificationsResponse {
                notifications: notifications.iter().map(|n| {
                    proto_crate::api::im::service::notification::Notification {
                        notification_id: n.id.to_string(),
                        title: n.title.clone(),
                        content: n.content.clone(),
                        notification_type: match n.notification_type {
                            NotificationType::Message => 0,
                            NotificationType::System => 1,
                            NotificationType::Activity => 2,
                            NotificationType::Custom(_) => 3,
                        },
                        priority: match n.priority {
                            Priority::Low => 0,
                            Priority::Normal => 1,
                            Priority::High => 2,
                        },
                        target_type: match n.target_type {
                            TargetType::Single => 0,
                            TargetType::Multiple => 1,
                            TargetType::Topic => 2,
                            TargetType::Broadcast => 3,
                        },
                        target_users: n.target_users.clone(),
                        platform: n.platform.iter().map(|p| match p {
                            Platform::IOS => 0,
                            Platform::Android => 1,
                            Platform::Web => 2,
                            Platform::All => 3,
                        }).collect(),
                        category: n.metadata.category.clone(),
                        badge: n.metadata.badge.map(|b| b as u32),
                        sound: n.metadata.sound.clone(),
                        image_url: n.metadata.image_url.clone(),
                        deep_link: n.metadata.deep_link.clone(),
                        custom_data: n.metadata.custom_data.clone(),
                        created_at: n.created_at.timestamp() as u64,
                    }
                }).collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_platform_statistics(
        &self,
        request: Request<GetPlatformStatisticsRequest>,
    ) -> Result<Response<GetPlatformStatisticsResponse>, Status> {
        let req = request.get_ref();
        let platform = match req.platform() {
            0 => Platform::IOS,
            1 => Platform::Android,
            2 => Platform::Web,
            _ => Platform::All,
        };

        let start_time = chrono::DateTime::from_timestamp(req.start_time as i64, 0)
            .ok_or_else(|| Status::invalid_argument("Invalid start time"))?;
        let end_time = chrono::DateTime::from_timestamp(req.end_time as i64, 0)
            .ok_or_else(|| Status::invalid_argument("Invalid end time"))?;

        match self.notification_service.get_platform_statistics(platform, start_time, end_time).await {
            Ok(stats) => Ok(Response::new(GetPlatformStatisticsResponse {
                total_sent: stats.total_sent,
                total_failed: stats.total_failed,
                success_rate: stats.success_rate,
                average_latency: stats.average_latency,
                error_counts: stats.error_counts,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
} 