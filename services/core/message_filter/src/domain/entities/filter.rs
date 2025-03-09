use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub id: Uuid,
    pub name: String,
    pub rule_type: RuleType,
    pub pattern: String,
    pub action: FilterAction,
    pub priority: i32,
    pub is_enabled: bool,
    pub metadata: RuleMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    Keyword,      // 关键词
    Regex,        // 正则表达式
    Dictionary,   // 词典
    ImageHash,    // 图片哈希
    MediaType,    // 媒体类型
    Custom,       // 自定义规则
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
    Block,        // 阻止
    Replace,      // 替换
    Warn,         // 警告
    Log,          // 记录
    Review,       // 人工审核
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMetadata {
    pub description: String,
    pub category: String,
    pub replacement: Option<String>,
    pub custom_config: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    pub is_blocked: bool,
    pub matched_rules: Vec<MatchedRule>,
    pub modified_content: Option<String>,
    pub review_required: bool,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedRule {
    pub rule_id: Uuid,
    pub rule_name: String,
    pub rule_type: RuleType,
    pub action: FilterAction,
    pub matched_content: String,
    pub position: Option<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRequest {
    pub content: String,
    pub content_type: String,
    pub metadata: FilterMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterMetadata {
    pub user_id: String,
    pub session_id: String,
    pub device_info: Option<String>,
    pub custom_properties: std::collections::HashMap<String, String>,
} 