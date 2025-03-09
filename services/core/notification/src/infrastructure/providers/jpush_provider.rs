use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crate::domain::{
    entities::notification::{
        Notification, Platform, PlatformResult,
    },
    providers::push_provider::{PushProvider, Error as ProviderError},
};

pub struct JPushProvider {
    app_key: String,
    master_secret: String,
    client: Client,
    base_url: String,
}

impl JPushProvider {
    pub fn new(app_key: String, master_secret: String) -> Self {
        Self {
            app_key,
            master_secret,
            client: Client::new(),
            base_url: "https://api.jpush.cn/v3".to_string(),
        }
    }

    // 获取认证头
    fn get_auth_header(&self) -> String {
        let auth = format!("{}:{}", self.app_key, self.master_secret);
        format!("Basic {}", BASE64.encode(auth.as_bytes()))
    }

    // 构建推送负载
    fn build_push_payload(&self, notification: &Notification) -> serde_json::Value {
        let mut platform = vec![];
        for p in &notification.platform {
            match p {
                Platform::IOS => platform.push("ios"),
                Platform::Android => platform.push("android"),
                Platform::All => {
                    platform = vec!["ios", "android"];
                    break;
                }
                _ => continue,
            }
        }

        let audience = match &notification.target_type {
            crate::domain::entities::notification::TargetType::Single => {
                json!({ "alias": [notification.target_users[0]] })
            }
            crate::domain::entities::notification::TargetType::Multiple => {
                json!({ "alias": notification.target_users })
            }
            crate::domain::entities::notification::TargetType::Broadcast => {
                json!("all")
            }
            crate::domain::entities::notification::TargetType::Topic => {
                json!({ "tag": notification.target_users })
            }
        };

        let notification_payload = json!({
            "title": notification.title,
            "body": notification.content,
            "extras": notification.metadata.custom_data
        });

        json!({
            "platform": platform,
            "audience": audience,
            "notification": {
                "android": {
                    "alert": notification.content,
                    "title": notification.title,
                    "extras": notification.metadata.custom_data
                },
                "ios": {
                    "alert": {
                        "title": notification.title,
                        "body": notification.content
                    },
                    "sound": notification.metadata.sound.clone().unwrap_or_else(|| "default".to_string()),
                    "badge": notification.metadata.badge.unwrap_or(1),
                    "extras": notification.metadata.custom_data
                }
            },
            "options": {
                "time_to_live": 86400,
                "apns_production": true
            }
        })
    }
}

#[async_trait]
impl PushProvider for JPushProvider {
    fn get_provider_name(&self) -> &str {
        "jpush"
    }

    fn get_supported_platforms(&self) -> Vec<Platform> {
        vec![Platform::IOS, Platform::Android]
    }

    async fn initialize(&self) -> Result<(), ProviderError> {
        // 验证认证信息
        let url = format!("{}/devices", self.base_url);
        let response = self.client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .send()
            .await
            .map_err(|e| ProviderError::Initialization(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::Authentication("Invalid credentials".to_string()));
        }

        Ok(())
    }

    async fn send_push(&self, notification: &Notification) -> Result<PlatformResult, ProviderError> {
        let url = format!("{}/push", self.base_url);
        let payload = self.build_push_payload(notification);

        let response = self.client
            .post(&url)
            .header("Authorization", self.get_auth_header())
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
            platform: notification.platform[0].clone(),
            provider: self.get_provider_name().to_string(),
            success: true,
            message_id: result["msg_id"].as_str().map(|s| s.to_string()),
            error: None,
        })
    }

    async fn batch_send_push(&self, notifications: &[Notification]) -> Result<Vec<PlatformResult>, ProviderError> {
        let mut results = Vec::new();
        for notification in notifications {
            match self.send_push(notification).await {
                Ok(result) => results.push(result),
                Err(e) => results.push(PlatformResult {
                    platform: notification.platform[0].clone(),
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
        // JPush 不支持取消推送
        Err(ProviderError::Provider("Cancel push not supported".to_string()))
    }

    async fn get_push_status(&self, message_id: &str) -> Result<Option<PlatformResult>, ProviderError> {
        let url = format!("{}/status/message", self.base_url);
        let response = self.client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .query(&[("msg_id", message_id)])
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
            platform: Platform::All,
            provider: self.get_provider_name().to_string(),
            success: true,
            message_id: Some(message_id.to_string()),
            error: None,
        }))
    }

    async fn validate_push_token(&self, token: &str, platform: Platform) -> Result<bool, ProviderError> {
        let url = format!("{}/devices/{}", self.base_url, token);
        let response = self.client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .send()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        Ok(response.status().is_success())
    }
} 