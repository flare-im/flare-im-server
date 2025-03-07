use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub kafka: KafkaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub addr: String,
    pub max_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub brokers: Vec<String>,
    pub group_id: String,
    pub topic: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                addr: "127.0.0.1:50051".to_string(),
                max_connections: 10000,
            },
            redis: RedisConfig {
                url: "redis://127.0.0.1:6379".to_string(),
                pool_size: 10,
            },
            kafka: KafkaConfig {
                brokers: vec!["localhost:9092".to_string()],
                group_id: "message_gateway".to_string(),
                topic: "messages".to_string(),
            },
        }
    }
}

pub fn load_config() -> Result<Config> {
    // 首先尝试从环境变量获取配置文件路径
    let config_path = std::env::var("CONFIG_PATH")
        .unwrap_or_else(|_| "config/message_gateway.yaml".to_string());

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