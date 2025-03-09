use async_trait::async_trait;
use regex::Regex;
use crate::domain::{
    entities::filter::{
        FilterRule, FilterRequest, FilterResult, RuleType, FilterAction,
        MatchedRule, Position, RiskLevel,
    },
    services::filter_service::{FilterService, Error as ServiceError},
};

pub struct FilterServiceImpl {
    rules_cache: std::sync::Arc<tokio::sync::RwLock<Vec<FilterRule>>>,
}

impl FilterServiceImpl {
    pub fn new() -> Self {
        Self {
            rules_cache: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    // 更新规则缓存
    pub async fn update_rules_cache(&self, rules: Vec<FilterRule>) {
        let mut cache = self.rules_cache.write().await;
        *cache = rules;
    }

    // 应用关键词规则
    async fn apply_keyword_rule(&self, content: &str, rule: &FilterRule) -> Option<MatchedRule> {
        if content.contains(&rule.pattern) {
            Some(MatchedRule {
                rule_id: rule.id,
                rule_name: rule.name.clone(),
                rule_type: rule.rule_type.clone(),
                action: rule.action.clone(),
                matched_content: rule.pattern.clone(),
                position: None, // TODO: 实现位置查找
            })
        } else {
            None
        }
    }

    // 应用正则表达式规则
    async fn apply_regex_rule(&self, content: &str, rule: &FilterRule) -> Option<MatchedRule> {
        if let Ok(regex) = Regex::new(&rule.pattern) {
            if let Some(mat) = regex.find(content) {
                Some(MatchedRule {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    rule_type: rule.rule_type.clone(),
                    action: rule.action.clone(),
                    matched_content: mat.as_str().to_string(),
                    position: Some(Position {
                        start: mat.start(),
                        end: mat.end(),
                    }),
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    // 应用词典规则
    async fn apply_dictionary_rule(&self, content: &str, rule: &FilterRule) -> Option<MatchedRule> {
        // TODO: 实现词典匹配
        None
    }

    // 应用图片哈希规则
    async fn apply_image_hash_rule(&self, content: &str, rule: &FilterRule) -> Option<MatchedRule> {
        // TODO: 实现图片哈希匹配
        None
    }

    // 应用媒体类型规则
    async fn apply_media_type_rule(&self, content: &str, rule: &FilterRule) -> Option<MatchedRule> {
        // TODO: 实现媒体类型匹配
        None
    }

    // 应用自定义规则
    async fn apply_custom_rule(&self, content: &str, rule: &FilterRule) -> Option<MatchedRule> {
        // TODO: 实现自定义规则匹配
        None
    }

    // 计算风险等级
    fn calculate_risk_level(&self, matched_rules: &[MatchedRule]) -> RiskLevel {
        let mut highest_risk = RiskLevel::Safe;
        for rule in matched_rules {
            let risk = match rule.action {
                FilterAction::Block => RiskLevel::Critical,
                FilterAction::Replace => RiskLevel::High,
                FilterAction::Warn => RiskLevel::Medium,
                FilterAction::Log => RiskLevel::Low,
                FilterAction::Review => RiskLevel::Medium,
            };
            if risk > highest_risk {
                highest_risk = risk;
            }
        }
        highest_risk
    }

    // 替换内容
    fn replace_content(&self, content: &str, matched_rules: &[MatchedRule]) -> Option<String> {
        let mut modified = content.to_string();
        let mut has_changes = false;

        for rule in matched_rules {
            if let FilterAction::Replace = rule.action {
                if let Some(replacement) = &rule.matched_content {
                    modified = modified.replace(&rule.matched_content, replacement);
                    has_changes = true;
                }
            }
        }

        if has_changes {
            Some(modified)
        } else {
            None
        }
    }
}

#[async_trait]
impl FilterService for FilterServiceImpl {
    async fn filter_content(&self, request: FilterRequest) -> Result<FilterResult, ServiceError> {
        let rules = self.rules_cache.read().await;
        let mut matched_rules = Vec::new();

        // 应用所有启用的规则
        for rule in rules.iter().filter(|r| r.is_enabled) {
            let matched = match rule.rule_type {
                RuleType::Keyword => self.apply_keyword_rule(&request.content, rule).await,
                RuleType::Regex => self.apply_regex_rule(&request.content, rule).await,
                RuleType::Dictionary => self.apply_dictionary_rule(&request.content, rule).await,
                RuleType::ImageHash => self.apply_image_hash_rule(&request.content, rule).await,
                RuleType::MediaType => self.apply_media_type_rule(&request.content, rule).await,
                RuleType::Custom => self.apply_custom_rule(&request.content, rule).await,
            };

            if let Some(matched_rule) = matched {
                matched_rules.push(matched_rule);
            }
        }

        // 计算风险等级
        let risk_level = self.calculate_risk_level(&matched_rules);

        // 检查是否需要阻止
        let is_blocked = matched_rules.iter()
            .any(|r| matches!(r.action, FilterAction::Block));

        // 检查是否需要人工审核
        let review_required = matched_rules.iter()
            .any(|r| matches!(r.action, FilterAction::Review));

        // 替换内容
        let modified_content = self.replace_content(&request.content, &matched_rules);

        Ok(FilterResult {
            is_blocked,
            matched_rules,
            modified_content,
            review_required,
            risk_level,
        })
    }

    async fn batch_filter_content(&self, requests: Vec<FilterRequest>) -> Result<Vec<FilterResult>, ServiceError> {
        let mut results = Vec::new();
        for request in requests {
            results.push(self.filter_content(request).await?);
        }
        Ok(results)
    }

    async fn add_rule(&self, rule: FilterRule) -> Result<FilterRule, ServiceError> {
        let mut rules = self.rules_cache.write().await;
        rules.push(rule.clone());
        Ok(rule)
    }

    async fn update_rule(&self, rule: FilterRule) -> Result<FilterRule, ServiceError> {
        let mut rules = self.rules_cache.write().await;
        if let Some(index) = rules.iter().position(|r| r.id == rule.id) {
            rules[index] = rule.clone();
            Ok(rule)
        } else {
            Err(ServiceError::NotFound(format!("Rule {} not found", rule.id)))
        }
    }

    async fn delete_rule(&self, rule_id: &str) -> Result<(), ServiceError> {
        let mut rules = self.rules_cache.write().await;
        if let Some(index) = rules.iter().position(|r| r.id.to_string() == rule_id) {
            rules.remove(index);
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!("Rule {} not found", rule_id)))
        }
    }

    async fn get_rule(&self, rule_id: &str) -> Result<Option<FilterRule>, ServiceError> {
        let rules = self.rules_cache.read().await;
        Ok(rules.iter()
            .find(|r| r.id.to_string() == rule_id)
            .cloned())
    }

    async fn get_rules_by_type(&self, rule_type: RuleType) -> Result<Vec<FilterRule>, ServiceError> {
        let rules = self.rules_cache.read().await;
        Ok(rules.iter()
            .filter(|r| r.rule_type == rule_type)
            .cloned()
            .collect())
    }

    async fn enable_rule(&self, rule_id: &str) -> Result<(), ServiceError> {
        let mut rules = self.rules_cache.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id.to_string() == rule_id) {
            rule.is_enabled = true;
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!("Rule {} not found", rule_id)))
        }
    }

    async fn disable_rule(&self, rule_id: &str) -> Result<(), ServiceError> {
        let mut rules = self.rules_cache.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id.to_string() == rule_id) {
            rule.is_enabled = false;
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!("Rule {} not found", rule_id)))
        }
    }

    async fn import_rules(&self, rules: Vec<FilterRule>) -> Result<(), ServiceError> {
        let mut cache = self.rules_cache.write().await;
        cache.extend(rules);
        Ok(())
    }

    async fn export_rules(&self, rule_type: Option<RuleType>) -> Result<Vec<FilterRule>, ServiceError> {
        let rules = self.rules_cache.read().await;
        Ok(match rule_type {
            Some(rt) => rules.iter()
                .filter(|r| r.rule_type == rt)
                .cloned()
                .collect(),
            None => rules.clone(),
        })
    }
} 