use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::domain::{
    entities::notification::{
        Notification, NotificationResult, DeviceInfo, NotificationTemplate,
        Platform, NotificationType, PlatformResult,
    },
    services::notification_service::{NotificationService, Error as ServiceError, PlatformStatistics},
    providers::push_provider::PushProvider,
};

pub struct NotificationServiceImpl {
    providers: Arc<RwLock<Vec<Box<dyn PushProvider + Send + Sync>>>>,
    device_repository: Arc<dyn DeviceRepository + Send + Sync>,
    template_repository: Arc<dyn TemplateRepository + Send + Sync>,
    notification_repository: Arc<dyn NotificationRepository + Send + Sync>,
}

impl NotificationServiceImpl {
    pub fn new(
        device_repository: Arc<dyn DeviceRepository + Send + Sync>,
        template_repository: Arc<dyn TemplateRepository + Send + Sync>,
        notification_repository: Arc<dyn NotificationRepository + Send + Sync>,
    ) -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
            device_repository,
            template_repository,
            notification_repository,
        }
    }

    // 注册推送提供商
    pub async fn register_provider(&self, provider: Box<dyn PushProvider + Send + Sync>) -> Result<(), ServiceError> {
        provider.initialize().await
            .map_err(|e| ServiceError::Provider(e.to_string()))?;

        let mut providers = self.providers.write().await;
        providers.push(provider);
        Ok(())
    }

    // 获取适合的推送提供商
    async fn get_provider_for_platform(&self, platform: Platform) -> Option<Box<dyn PushProvider + Send + Sync>> {
        let providers = self.providers.read().await;
        for provider in providers.iter() {
            if provider.get_supported_platforms().contains(&platform) {
                return Some(provider.clone());
            }
        }
        None
    }

    // 处理单个平台的推送
    async fn handle_platform_push(
        &self,
        notification: &Notification,
        platform: Platform,
        provider: &Box<dyn PushProvider + Send + Sync>,
    ) -> PlatformResult {
        match provider.send_push(notification).await {
            Ok(result) => result,
            Err(e) => PlatformResult {
                platform,
                provider: provider.get_provider_name().to_string(),
                success: false,
                message_id: None,
                error: Some(e.to_string()),
            },
        }
    }
}

#[async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn send_notification(&self, notification: Notification) -> Result<NotificationResult, ServiceError> {
        // 验证通知
        if notification.title.is_empty() || notification.content.is_empty() {
            return Err(ServiceError::InvalidRequest("Title and content are required".to_string()));
        }

        // 保存通知
        self.notification_repository.save_notification(&notification).await
            .map_err(|e| ServiceError::Service(e.to_string()))?;

        let mut platform_results = Vec::new();
        let mut sent_count = 0;
        let mut failed_count = 0;

        // 处理每个平台的推送
        for platform in &notification.platform {
            if let Some(provider) = self.get_provider_for_platform(platform.clone()).await {
                let result = self.handle_platform_push(&notification, platform.clone(), &provider).await;
                if result.success {
                    sent_count += 1;
                } else {
                    failed_count += 1;
                }
                platform_results.push(result);
            } else {
                failed_count += 1;
                platform_results.push(PlatformResult {
                    platform: platform.clone(),
                    provider: "unknown".to_string(),
                    success: false,
                    message_id: None,
                    error: Some("No provider available".to_string()),
                });
            }
        }

        Ok(NotificationResult {
            notification_id: notification.id,
            success: failed_count == 0,
            platform_results,
            sent_count,
            failed_count,
            error: None,
        })
    }

    async fn batch_send_notifications(&self, notifications: Vec<Notification>) -> Result<Vec<NotificationResult>, ServiceError> {
        let mut results = Vec::new();
        for notification in notifications {
            match self.send_notification(notification).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // 记录错误但继续处理其他通知
                    log::error!("Failed to send notification: {}", e);
                }
            }
        }
        Ok(results)
    }

    async fn register_device(&self, device_info: DeviceInfo) -> Result<(), ServiceError> {
        // 验证设备令牌
        if let Some(provider) = self.get_provider_for_platform(device_info.platform.clone()).await {
            provider.validate_push_token(&device_info.push_token, device_info.platform.clone())
                .await
                .map_err(|e| ServiceError::Provider(e.to_string()))?;
        }

        // 保存设备信息
        self.device_repository.save_device(device_info)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn unregister_device(&self, user_id: &str, device_id: &str) -> Result<(), ServiceError> {
        self.device_repository.delete_device(user_id, device_id)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn update_device_token(&self, user_id: &str, device_id: &str, new_token: &str) -> Result<(), ServiceError> {
        self.device_repository.update_device_token(user_id, device_id, new_token)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn create_template(&self, template: NotificationTemplate) -> Result<NotificationTemplate, ServiceError> {
        if template.name.is_empty() || template.title_template.is_empty() || template.content_template.is_empty() {
            return Err(ServiceError::InvalidRequest("Template name, title and content are required".to_string()));
        }

        self.template_repository.save_template(template)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn update_template(&self, template: NotificationTemplate) -> Result<NotificationTemplate, ServiceError> {
        self.template_repository.save_template(template)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn delete_template(&self, template_id: &str) -> Result<(), ServiceError> {
        self.template_repository.delete_template(template_id)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn get_template(&self, template_id: &str) -> Result<Option<NotificationTemplate>, ServiceError> {
        self.template_repository.get_template(template_id)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn cancel_notification(&self, notification_id: &str) -> Result<(), ServiceError> {
        let notification = self.notification_repository.get_notification(notification_id)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))?
            .ok_or_else(|| ServiceError::NotFound("Notification not found".to_string()))?;

        for result in notification.platform_results {
            if let Some(message_id) = result.message_id {
                if let Some(provider) = self.get_provider_for_platform(result.platform).await {
                    if let Err(e) = provider.cancel_push(&message_id).await {
                        log::error!("Failed to cancel push for provider {}: {}", provider.get_provider_name(), e);
                    }
                }
            }
        }

        self.notification_repository.update_notification_status(
            notification_id,
            crate::domain::entities::notification::NotificationStatus::Cancelled,
        )
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn get_notification_status(&self, notification_id: &str) -> Result<Option<NotificationResult>, ServiceError> {
        self.notification_repository.get_notification(notification_id)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn get_user_notifications(
        &self,
        user_id: &str,
        notification_type: Option<NotificationType>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Notification>, ServiceError> {
        self.notification_repository.get_user_notifications(user_id, notification_type, limit, offset)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn get_platform_statistics(
        &self,
        platform: Platform,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<PlatformStatistics, ServiceError> {
        self.notification_repository.get_platform_statistics(platform, start_time, end_time)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))
    }
}

#[async_trait]
pub trait DeviceRepository {
    async fn save_device(&self, device: DeviceInfo) -> Result<(), ServiceError>;
    async fn delete_device(&self, user_id: &str, device_id: &str) -> Result<(), ServiceError>;
    async fn update_device_token(&self, user_id: &str, device_id: &str, new_token: &str) -> Result<(), ServiceError>;
    async fn get_user_devices(&self, user_id: &str) -> Result<Vec<DeviceInfo>, ServiceError>;
}

#[async_trait]
pub trait TemplateRepository {
    async fn save_template(&self, template: NotificationTemplate) -> Result<NotificationTemplate, ServiceError>;
    async fn delete_template(&self, template_id: &str) -> Result<(), ServiceError>;
    async fn get_template(&self, template_id: &str) -> Result<Option<NotificationTemplate>, ServiceError>;
    async fn get_templates_by_category(&self, category: &str) -> Result<Vec<NotificationTemplate>, ServiceError>;
}

#[async_trait]
pub trait NotificationRepository {
    async fn save_notification(&self, notification: &Notification) -> Result<(), ServiceError>;
    async fn get_notification(&self, notification_id: &str) -> Result<Option<NotificationResult>, ServiceError>;
    async fn update_notification_status(&self, notification_id: &str, status: crate::domain::entities::notification::NotificationStatus) -> Result<(), ServiceError>;
    async fn get_user_notifications(&self, user_id: &str, notification_type: Option<NotificationType>, limit: u32, offset: u32) -> Result<Vec<Notification>, ServiceError>;
    async fn get_platform_statistics(&self, platform: Platform, start_time: chrono::DateTime<chrono::Utc>, end_time: chrono::DateTime<chrono::Utc>) -> Result<PlatformStatistics, ServiceError>;
} 