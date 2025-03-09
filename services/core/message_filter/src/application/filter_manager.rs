use crate::domain::{
    entities::filter::{
        FilterRule, FilterRequest, FilterResult, RuleType, FilterAction, RiskLevel,
    },
    repositories::filter_repository::FilterRepository,
    services::filter_service::FilterService,
};
use chrono::Utc;
use uuid::Uuid;

pub struct FilterManager<R: FilterRepository, S: FilterService> {
    filter_repository: R,
    filter_service: S,
}

impl<R: FilterRepository, S: FilterService> FilterManager<R, S> {
    pub fn new(filter_repository: R, filter_service: S) -> Self {
        Self {
            filter_repository,
            filter_service,
        }
    }

    // 过滤内容
    pub async fn filter_content(&self, request: FilterRequest) -> Result<FilterResult, Error> {
        // 验证请求
        self.validate_request(&request)?;

        // 执行过滤
        let result = self.filter_service.filter_content(request).await?;

        // 记录高风险结果
        if result.risk_level >= RiskLevel::High {
            // TODO: 记录日志或发送告警
        }

        Ok(result)
    }

    // 批量过滤内容
    pub async fn batch_filter_content(&self, requests: Vec<FilterRequest>) -> Result<Vec<FilterResult>, Error> {
        // 验证请求
        for request in &requests {
            self.validate_request(request)?;
        }

        // 批量过滤
        self.filter_service.batch_filter_content(requests).await
            .map_err(Error::from)
    }

    // 添加规则
    pub async fn add_rule(&self, mut rule: FilterRule) -> Result<FilterRule, Error> {
        // 验证规则
        self.validate_rule(&rule)?;

        // 设置默认值
        if rule.id == Uuid::nil() {
            rule.id = Uuid::new_v4();
        }
        rule.created_at = Utc::now();
        rule.updated_at = Utc::now();

        // 保存规则
        self.filter_repository.save_rule(rule.clone()).await?;

        Ok(rule)
    }

    // 更新规则
    pub async fn update_rule(&self, mut rule: FilterRule) -> Result<FilterRule, Error> {
        // 验证规则存在
        if self.filter_repository.get_rule(&rule.id.to_string()).await?.is_none() {
            return Err(Error::NotFound(format!("Rule {} not found", rule.id)));
        }

        // 验证规则
        self.validate_rule(&rule)?;

        // 更新时间戳
        rule.updated_at = Utc::now();

        // 保存规则
        self.filter_repository.save_rule(rule.clone()).await?;

        Ok(rule)
    }

    // 删除规则
    pub async fn delete_rule(&self, rule_id: &str) -> Result<(), Error> {
        self.filter_repository.delete_rule(rule_id).await
            .map_err(Error::from)
    }

    // 启用规则
    pub async fn enable_rule(&self, rule_id: &str) -> Result<(), Error> {
        self.filter_repository.enable_rule(rule_id).await
            .map_err(Error::from)
    }

    // 禁用规则
    pub async fn disable_rule(&self, rule_id: &str) -> Result<(), Error> {
        self.filter_repository.disable_rule(rule_id).await
            .map_err(Error::from)
    }

    // 导入规则
    pub async fn import_rules(&self, rules: Vec<FilterRule>) -> Result<(), Error> {
        // 验证规则
        for rule in &rules {
            self.validate_rule(rule)?;
        }

        self.filter_service.import_rules(rules).await
            .map_err(Error::from)
    }

    // 导出规则
    pub async fn export_rules(&self, rule_type: Option<RuleType>) -> Result<Vec<FilterRule>, Error> {
        self.filter_service.export_rules(rule_type).await
            .map_err(Error::from)
    }

    // 验证过滤请求
    fn validate_request(&self, request: &FilterRequest) -> Result<(), Error> {
        if request.content.is_empty() {
            return Err(Error::ValidationError("Content is required".to_string()));
        }
        if request.content_type.is_empty() {
            return Err(Error::ValidationError("Content type is required".to_string()));
        }
        if request.metadata.user_id.is_empty() {
            return Err(Error::ValidationError("User ID is required".to_string()));
        }
        if request.metadata.session_id.is_empty() {
            return Err(Error::ValidationError("Session ID is required".to_string()));
        }
        Ok(())
    }

    // 验证过滤规则
    fn validate_rule(&self, rule: &FilterRule) -> Result<(), Error> {
        if rule.name.is_empty() {
            return Err(Error::ValidationError("Rule name is required".to_string()));
        }
        if rule.pattern.is_empty() {
            return Err(Error::ValidationError("Rule pattern is required".to_string()));
        }
        if rule.priority < 0 {
            return Err(Error::ValidationError("Priority must be non-negative".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(#[from] crate::domain::repositories::filter_repository::Error),
    
    #[error("Service error: {0}")]
    Service(#[from] crate::domain::services::filter_service::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
} 