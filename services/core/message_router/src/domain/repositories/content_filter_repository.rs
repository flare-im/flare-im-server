use async_trait::async_trait;
use anyhow::Result;
use proto_crate::api::im::common::MessageData;
/// 内容过滤结果
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// 是否通过过滤
    pub passed: bool,
    /// 过滤原因
    pub reason: Option<String>,
    /// 敏感词列表
    pub sensitive_words: Vec<String>,
    /// 风险等级
    pub risk_level: i32,
}

/// 内容过滤仓储接口
#[async_trait]
pub trait ContentFilterRepository: Send + Sync {
    /// 检查文本内容安全性
    /// 
    /// # 参数
    /// * `content` - 要检查的文本内容
    /// 
    /// # 返回
    /// * `Result<FilterResult, Error>` - 过滤结果
    async fn check(&self, message: &MessageData) -> Result<FilterResult>;
}