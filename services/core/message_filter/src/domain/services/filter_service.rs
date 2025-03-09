use async_trait::async_trait;
use crate::domain::entities::filter::{
    FilterRule, FilterRequest, FilterResult, RuleType, FilterAction,
};

#[async_trait]
pub trait FilterService {
    // 内容过滤
    async fn filter_content(&self, request: FilterRequest) -> Result<FilterResult, Error>;
    async fn batch_filter_content(&self, requests: Vec<FilterRequest>) -> Result<Vec<FilterResult>, Error>;
    
    // 规则管理
    async fn add_rule(&self, rule: FilterRule) -> Result<FilterRule, Error>;
    async fn update_rule(&self, rule: FilterRule) -> Result<FilterRule, Error>;
    async fn delete_rule(&self, rule_id: &str) -> Result<(), Error>;
    
    // 规则查询
    async fn get_rule(&self, rule_id: &str) -> Result<Option<FilterRule>, Error>;
    async fn get_rules_by_type(&self, rule_type: RuleType) -> Result<Vec<FilterRule>, Error>;
    
    // 规则状态管理
    async fn enable_rule(&self, rule_id: &str) -> Result<(), Error>;
    async fn disable_rule(&self, rule_id: &str) -> Result<(), Error>;
    
    // 规则导入导出
    async fn import_rules(&self, rules: Vec<FilterRule>) -> Result<(), Error>;
    async fn export_rules(&self, rule_type: Option<RuleType>) -> Result<Vec<FilterRule>, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Service error: {0}")]
    Service(String),
    
    #[error("Filter error: {0}")]
    Filter(String),
    
    #[error("Rule error: {0}")]
    Rule(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
} 