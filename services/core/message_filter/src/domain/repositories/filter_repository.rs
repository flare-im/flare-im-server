use async_trait::async_trait;
use crate::domain::entities::filter::{FilterRule, RuleType};

#[async_trait]
pub trait FilterRepository {
    // 规则基本操作
    async fn save_rule(&self, rule: FilterRule) -> Result<(), Error>;
    async fn get_rule(&self, rule_id: &str) -> Result<Option<FilterRule>, Error>;
    async fn delete_rule(&self, rule_id: &str) -> Result<(), Error>;
    
    // 规则查询
    async fn get_rules_by_type(&self, rule_type: RuleType) -> Result<Vec<FilterRule>, Error>;
    async fn get_enabled_rules(&self) -> Result<Vec<FilterRule>, Error>;
    async fn get_rules_by_category(&self, category: &str) -> Result<Vec<FilterRule>, Error>;
    
    // 规则批量操作
    async fn batch_save_rules(&self, rules: Vec<FilterRule>) -> Result<(), Error>;
    async fn batch_delete_rules(&self, rule_ids: Vec<String>) -> Result<(), Error>;
    
    // 规则状态操作
    async fn enable_rule(&self, rule_id: &str) -> Result<(), Error>;
    async fn disable_rule(&self, rule_id: &str) -> Result<(), Error>;
    
    // 规则统计
    async fn count_rules(&self) -> Result<u64, Error>;
    async fn count_rules_by_type(&self, rule_type: RuleType) -> Result<u64, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),
    
    #[error("Rule not found: {0}")]
    NotFound(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    #[error("Duplicate rule: {0}")]
    Duplicate(String),
} 