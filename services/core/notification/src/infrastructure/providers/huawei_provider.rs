use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use crate::domain::{
    entities::notification::{
        Notification, Platform, PlatformResult,
    },
    providers::push_provider::{PushProvider, Error as ProviderError},
};

pub struct HuaweiProvider {
    app_id: String,
    app_secret: String,
    client: Client,
    base_url: String,
    auth_token: Option<String>,
    token_expire_time: Option<chrono::DateTime<Utc>>,
}

impl HuaweiProvider {
    pub fn new(app_id: String, app_secret: String) -> Self {
        Self {
            app_id,
            app_secret,
            client: Client::new(),
            base_url: "https://push-api.cloud.huawei.com/v1".to_string(),
            auth_token: None,
            token_expire_time: None,
        }
    }

    // 获取认证令牌
    async fn get_auth_token(&mut self) -> Result<String, ProviderError> {
        // 检查现有令牌是否有效
        if let (Some(token), Some(expire_time)) = (&self.auth_token, self.token_expire_time) {
            if expire_time > Utc::now() {
                return Ok(token.clone());
            }
        }

        // 获取新令牌
        let url = "https://oauth-login.cloud.huawei.com/oauth2/v3/token";
        let response = self.client
            .post(url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.app_id),
                ("client_secret", &self.app_secret),
            ])
            .send()
            .await
            .map_err(|e| ProviderError::Authentication(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::Authentication(format!(
                "Failed to get auth token: {}",
                response.text().await.unwrap_or_default()
            )));
        }

        let result: serde_json::Value = response.json()
            .await
            .map_err(|e| ProviderError::Authentication(e.to_string()))?;

        let token = result["access_token"].as_str()
            .ok_or_else(|| ProviderError::Authentication("Invalid token response".to_string()))?
            .to_string();

        let expires_in = result["expires_in"].as_i64()
            .ok_or_else(|| ProviderError::Authentication("Invalid expires_in".to_string()))?;

        let expire_time = Utc::now() + chrono::Duration::seconds(expires_in);
        self.auth_token = Some(token.clone());
        self.token_expire_time = Some(expire_time);

        Ok(token)
    }

    // 构建推送负载
    fn build_push_payload(&self, notification: &Notification) -> serde_json::Value {
        let message = json!({
            "notification": {
                "title": notification.title,
                "body": notification.content,
                "click_action": {
                    "type": 1,  // 打开应用
                },
            },
            "android": {
                "notification": {
                    "title": notification.title,
                    "body": notification.content,
                    "click_action": {
                        "type": 1,
                    },
                    "badge": {
                        "add_num": notification.metadata.badge.unwrap_or(1),
                        "class": "com.huawei.android.launcher.badge.BadgeProvider"
                    },
                    "importance": match notification.priority {
                        crate::domain::entities::notification::Priority::High => "HIGH",
                        crate::domain::entities::notification::Priority::Normal => "NORMAL",
                        crate::domain::entities::notification::Priority::Low => "LOW",
                    },
                    "style": 0,
                    "auto_clear": 86400,
                },
                "ttl": "86400",
                "fast_app_target": 2,
            },
            "data": serde_json::to_string(&notification.metadata.custom_data).unwrap_or_default(),
        });

        let target = match &notification.target_type {
            crate::domain::entities::notification::TargetType::Single => {
                json!({
                    "token": [notification.target_users[0]]
                })
            }
            crate::domain::entities::notification::TargetType::Multiple => {
                json!({
                    "token": notification.target_users
                })
            }
            crate::domain::entities::notification::TargetType::Broadcast => {
                json!({
                    "topic": "ALL"
                })
            }
            crate::domain::entities::notification::TargetType::Topic => {
                json!({
                    "topic": notification.target_users[0]
                })
            }
        };

        json!({
            "validate_only": false,
            "message": message,
            "target": target
        })
    }
}

#[async_trait]
impl PushProvider for HuaweiProvider {
    fn get_provider_name(&self) -> &str {
        "huawei"
    }

    fn get_supported_platforms(&self) -> Vec<Platform> {
        vec![Platform::Android]
    }

    async fn initialize(&self) -> Result<(), ProviderError> {
        // 验证认证信息
        let mut this = self.clone();
        this.get_auth_token().await?;
        Ok(())
    }

    async fn send_push(&self, notification: &Notification) -> Result<PlatformResult, ProviderError> {
        let mut this = self.clone();
        let token = this.get_auth_token().await?;
        let url = format!("{}/{}/messages:send", self.base_url, self.app_id);
        let payload = self.build_push_payload(notification);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::SendError(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::SendError(error));
        }

        let result: serde_json::Value = response.json()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        Ok(PlatformResult {
            platform: Platform::Android,
            provider: self.get_provider_name().to_string(),
            success: true,
            message_id: result["code"].as_str().map(|s| s.to_string()),
            error: None,
        })
    }

    async fn batch_send_push(&self, notifications: &[Notification]) -> Result<Vec<PlatformResult>, ProviderError> {
        let mut results = Vec::new();
        for notification in notifications {
            match self.send_push(notification).await {
                Ok(result) => results.push(result),
                Err(e) => results.push(PlatformResult {
                    platform: Platform::Android,
                    provider: self.get_provider_name().to_string(),
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        Ok(results)
    }

    async fn cancel_push(&self, message_id: &str) -> Result<(), ProviderError> {
        // 华为推送不支持取消推送
        Err(ProviderError::Provider("Cancel push not supported".to_string()))
    }

    async fn get_push_status(&self, message_id: &str) -> Result<Option<PlatformResult>, ProviderError> {
        let mut this = self.clone();
        let token = this.get_auth_token().await?;
        let url = format!("{}/{}/messages/status/{}", self.base_url, self.app_id, message_id);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let result: serde_json::Value = response.json()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        Ok(Some(PlatformResult {
            platform: Platform::Android,
            provider: self.get_provider_name().to_string(),
            success: true,
            message_id: Some(message_id.to_string()),
            error: None,
        }))
    }

    async fn validate_push_token(&self, token: &str, platform: Platform) -> Result<bool, ProviderError> {
        if platform != Platform::Android {
            return Err(ProviderError::PlatformNotSupported(format!("{:?}", platform)));
        }

        // 华为推送不提供令牌验证 API，我们假设令牌有效
        Ok(true)
    }
} 