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

pub struct GetuiProvider {
    app_id: String,
    app_key: String,
    master_secret: String,
    client: Client,
    base_url: String,
    auth_token: Option<String>,
    token_expire_time: Option<chrono::DateTime<Utc>>,
}

impl GetuiProvider {
    pub fn new(app_id: String, app_key: String, master_secret: String) -> Self {
        Self {
            app_id,
            app_key,
            master_secret,
            client: Client::new(),
            base_url: "https://restapi.getui.com/v2".to_string(),
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
        let url = format!("{}/{}/auth", self.base_url, self.app_id);
        let timestamp = Utc::now().timestamp_millis().to_string();
        let sign = format!("{}{}{}", self.app_key, timestamp, self.master_secret);
        let sign = format!("{:x}", md5::compute(sign));

        let response = self.client
            .post(&url)
            .json(&json!({
                "sign": sign,
                "timestamp": timestamp,
                "appkey": self.app_key
            }))
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

        let token = result["data"]["token"].as_str()
            .ok_or_else(|| ProviderError::Authentication("Invalid token response".to_string()))?
            .to_string();

        let expire_time = Utc::now() + chrono::Duration::hours(23);
        self.auth_token = Some(token.clone());
        self.token_expire_time = Some(expire_time);

        Ok(token)
    }

    // 构建推送负载
    fn build_push_payload(&self, notification: &Notification) -> serde_json::Value {
        let audience = match &notification.target_type {
            crate::domain::entities::notification::TargetType::Single => {
                json!({
                    "alias": [notification.target_users[0]]
                })
            }
            crate::domain::entities::notification::TargetType::Multiple => {
                json!({
                    "alias": notification.target_users
                })
            }
            crate::domain::entities::notification::TargetType::Broadcast => {
                json!({
                    "all": true
                })
            }
            crate::domain::entities::notification::TargetType::Topic => {
                json!({
                    "tag": notification.target_users
                })
            }
        };

        let push_message = json!({
            "notification": {
                "title": notification.title,
                "body": notification.content,
                "click_type": "startapp",
                "badge_add_num": notification.metadata.badge.unwrap_or(1)
            },
            "transmission": serde_json::to_string(&notification.metadata.custom_data).unwrap_or_default()
        });

        json!({
            "request_id": notification.id.to_string(),
            "audience": audience,
            "push_message": push_message,
            "settings": {
                "ttl": 3600000,
                "strategy": {
                    "default": 1
                }
            }
        })
    }
}

#[async_trait]
impl PushProvider for GetuiProvider {
    fn get_provider_name(&self) -> &str {
        "getui"
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
        let url = format!("{}/{}/push/single/cid", self.base_url, self.app_id);
        let payload = self.build_push_payload(notification);

        let response = self.client
            .post(&url)
            .header("token", token)
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
            message_id: Some(result["data"]["task_id"].as_str().unwrap_or_default().to_string()),
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
        let mut this = self.clone();
        let token = this.get_auth_token().await?;
        let url = format!("{}/{}/task/{}", self.base_url, self.app_id, message_id);

        let response = self.client
            .delete(&url)
            .header("token", token)
            .send()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Provider(error));
        }

        Ok(())
    }

    async fn get_push_status(&self, message_id: &str) -> Result<Option<PlatformResult>, ProviderError> {
        let mut this = self.clone();
        let token = this.get_auth_token().await?;
        let url = format!("{}/{}/task/detail/{}", self.base_url, self.app_id, message_id);

        let response = self.client
            .get(&url)
            .header("token", token)
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

        let mut this = self.clone();
        let auth_token = this.get_auth_token().await?;
        let url = format!("{}/{}/user/cid/{}", self.base_url, self.app_id, token);

        let response = self.client
            .get(&url)
            .header("token", auth_token)
            .send()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        Ok(response.status().is_success())
    }
} 