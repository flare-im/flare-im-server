use anyhow::Result;
use crate::domain::system::SystemManager;

pub struct SystemService {
    system_manager: SystemManager,
}

impl SystemService {
    pub fn new() -> Self {
        Self {
            system_manager: SystemManager::new(),
        }
    }

    pub async fn update_heartbeat(&self, user_id: &str) -> Result<()> {
        self.system_manager.update_heartbeat(user_id).await
    }

    pub async fn process_system_notice(&self, notice_data: &[u8]) -> Result<()> {
        self.system_manager.process_system_notice(notice_data).await
    }

    pub async fn update_config(&self, config_data: &[u8]) -> Result<()> {
        self.system_manager.update_config(config_data).await
    }
} 