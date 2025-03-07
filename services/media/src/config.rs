use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub s3: S3Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub addr: String,
    pub max_file_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                addr: "127.0.0.1:50052".to_string(),
                max_file_size: 10 * 1024 * 1024, // 10MB
            },
            s3: S3Config {
                endpoint: "http://localhost:9000".to_string(),
                region: "us-east-1".to_string(),
                bucket: "media".to_string(),
                access_key_id: "minioadmin".to_string(),
                secret_access_key: "minioadmin".to_string(),
            },
        }
    }
}

pub fn load_config() -> Result<Config> {
    // 首先尝试从环境变量获取配置文件路径
    let config_path = std::env::var("CONFIG_PATH")
        .unwrap_or_else(|_| "config/media.yaml".to_string());

    // 尝试读取配置文件
    match std::fs::read_to_string(&config_path) {
        Ok(contents) => {
            // 解析 YAML 配置
            let config: Config = serde_yaml::from_str(&contents)?;
            Ok(config)
        }
        Err(e) => {
            log::warn!("Failed to read config file: {}, using default config", e);
            Ok(Config::default())
        }
    }
} 