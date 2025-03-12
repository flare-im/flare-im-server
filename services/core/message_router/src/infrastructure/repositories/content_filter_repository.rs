use crate::domain::repositories::ContentFilterRepository;
use crate::domain::repositories::FilterResult;
use anyhow::Result;
use async_trait::async_trait;
use proto_crate::api::im::common::MessageData;

pub struct ContentFilterRepositoryImpl;

impl ContentFilterRepositoryImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ContentFilterRepository for ContentFilterRepositoryImpl {
    async fn check(&self, _message: &MessageData) -> Result<FilterResult> {
        // 模拟内容检查，这里简单实现返回通过
        Ok(FilterResult {
            passed: true,
            reason: None,
            sensitive_words: vec![],
            risk_level: 0,
        })
    }
} 