use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// 运行环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// 开发环境
    Development,
    /// 测试环境
    Testing,
    /// 生产环境
    Production,
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

impl FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "testing" | "test" => Ok(Environment::Testing),
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(format!("Unknown environment: {}", s)),
        }
    }
}

impl Environment {
    /// 获取环境名称
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Testing => "testing",
            Environment::Production => "production",
        }
    }

    /// 是否是开发环境
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }

    /// 是否是测试环境
    pub fn is_testing(&self) -> bool {
        matches!(self, Environment::Testing)
    }

    /// 是否是生产环境
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
} 