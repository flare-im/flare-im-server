use anyhow::Result;
use crate::domain::auth::AuthManager;

pub struct AuthService {
    auth_manager: AuthManager,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            auth_manager: AuthManager::new(),
        }
    }

    pub async fn authenticate(&self, auth_data: &[u8]) -> Result<String> {
        self.auth_manager.authenticate(auth_data).await
    }

    pub async fn refresh_token(&self, refresh_token: &[u8]) -> Result<String> {
        self.auth_manager.refresh_token(refresh_token).await
    }

    pub async fn logout(&self, user_id: &str) -> Result<()> {
        self.auth_manager.logout(user_id).await
    }
} 