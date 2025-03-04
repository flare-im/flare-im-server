mod env;
pub use env::Environment;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// 服务名称
    pub name: String,
    /// 服务地址
    pub host: String,
    /// 服务端口
    pub port: u16,
    /// 服务权重
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// 服务标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 服务元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// 输出目录
    pub output_dir: PathBuf,
    /// 文件前缀
    pub file_prefix: String,
    /// 日志级别 (0: ERROR, 1: WARN, 2: INFO, 3: DEBUG, 4: TRACE)
    #[serde(default = "default_log_level_num")]
    pub level: u8,
    /// 单个文件最大大小(MB)
    #[serde(default = "default_max_size")]
    pub max_size: u64,
    /// 最大备份数量
    #[serde(default = "default_max_backups")]
    pub max_backups: u32,
    /// 文件保留天数
    #[serde(default = "default_max_age")]
    pub max_age: u32,
    /// 是否压缩
    #[serde(default)]
    pub compress: bool,
    /// 当前日期
    #[serde(skip)]
    pub current_date: String,
    /// 时间格式
    #[serde(skip)]
    pub time_format: String,
    /// 是否输出到控制台
    #[serde(skip)]
    pub console_output: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("logs"),
            file_prefix: String::from("app"),
            level: default_log_level_num(),
            max_size: default_max_size(),
            max_backups: default_max_backups(),
            max_age: default_max_age(),
            compress: false,
            current_date: String::new(),
            time_format: String::from("%Y-%m-%d %H:%M:%S%.3f"),
            console_output: true,
        }
    }
}

/// Consul配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulConfig {
    /// Consul地址
    pub host: String,
    /// Consul端口
    pub port: u16,
    /// 服务注册间隔(秒)
    #[serde(default = "default_register_interval")]
    pub register_interval: u64,
    /// 服务心跳间隔(秒)
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,
}

/// Redis配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis地址
    pub host: String,
    /// Redis端口
    pub port: u16,
    /// 密码
    #[serde(default)]
    pub password: Option<String>,
    /// 数据库
    #[serde(default)]
    pub database: u8,
    /// 连接池大小
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

/// Kafka配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Kafka地址列表
    pub brokers: Vec<String>,
    /// 消费者组ID
    #[serde(default)]
    pub group_id: Option<String>,
    /// 主题列表
    #[serde(default)]
    pub topics: Vec<String>,
}

/// PostgreSQL配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// 数据库地址
    pub host: String,
    /// 数据库端口
    pub port: u16,
    /// 数据库名称
    pub database: String,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 连接池大小
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

/// ClickHouse配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickHouseConfig {
    /// 数据库地址
    pub host: String,
    /// 数据库端口
    pub port: u16,
    /// 数据库名称
    pub database: String,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 连接池大小
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

/// MinIO配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinioConfig {
    /// 服务地址
    pub endpoint: String,
    /// Access Key
    pub access_key: String,
    /// Secret Key
    pub secret_key: String,
    /// 是否使用SSL
    #[serde(default)]
    pub use_ssl: bool,
    /// 存储桶名称
    pub bucket: String,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 运行环境
    #[serde(default)]
    pub env: Environment,
    /// 服务配置
    pub service: ServiceConfig,
    /// 日志配置
    #[serde(default)]
    pub log: LogConfig,
    /// Consul配置
    pub consul: ConsulConfig,
    /// Redis配置
    #[serde(default)]
    pub redis: Option<RedisConfig>,
    /// Kafka配置
    #[serde(default)]
    pub kafka: Option<KafkaConfig>,
    /// PostgreSQL配置
    #[serde(default)]
    pub postgres: Option<PostgresConfig>,
    /// ClickHouse配置
    #[serde(default)]
    pub clickhouse: Option<ClickHouseConfig>,
    /// MinIO配置
    #[serde(default)]
    pub minio: Option<MinioConfig>,
    /// 扩展配置
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl Config {
    /// 从指定环境的配置文件加载配置
    pub fn from_env_file<P: AsRef<Path>>(env: Environment) -> anyhow::Result<Self> {
        let env_name = env.as_str();
        let config_path = format!("config/{}.yaml", env_name);
        Self::from_file(config_path)
    }

    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// 从环境变量加载配置
    pub fn from_env() -> anyhow::Result<Self> {
        envy::from_env().map_err(|e| anyhow::anyhow!("Failed to load config from env: {}", e))
    }

    /// 获取运行环境
    pub fn environment(&self) -> Environment {
        self.env
    }

    /// 是否是开发环境
    pub fn is_development(&self) -> bool {
        self.env.is_development()
    }

    /// 是否是测试环境
    pub fn is_testing(&self) -> bool {
        self.env.is_testing()
    }

    /// 是否是生产环境
    pub fn is_production(&self) -> bool {
        self.env.is_production()
    }
}

// 默认值函数
fn default_weight() -> u32 { 100 }
fn default_log_level_num() -> u8 { 2 } // INFO level
fn default_max_size() -> u64 { 100 } // 100MB
fn default_max_backups() -> u32 { 31 } // 31 backups
fn default_max_age() -> u32 { 31 } // 31 days
fn default_true() -> bool { true }
fn default_register_interval() -> u64 { 10 }
fn default_heartbeat_interval() -> u64 { 5 }
fn default_pool_size() -> u32 { 10 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialize() {
        let yaml = r#"
env: development
service:
  name: test-service
  host: 127.0.0.1
  port: 8080
consul:
  host: localhost
  port: 8500
redis:
  host: localhost
  port: 6379
  password: redis123
kafka:
  brokers:
    - localhost:9092
  group_id: test-group
  topics:
    - test-topic
extensions:
  custom_setting: 
    key: value
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.service.name, "test-service");
        assert_eq!(config.service.port, 8080);
        assert!(config.redis.is_some());
        assert!(config.kafka.is_some());
        assert!(config.extensions.contains_key("custom_setting"));
        assert!(config.is_development());
    }
} 