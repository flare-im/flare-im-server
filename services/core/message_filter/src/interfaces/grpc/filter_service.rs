use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;
use crate::{
    application::filter_manager::FilterManager,
    domain::{
        entities::filter::{
            FilterRule, FilterRequest, FilterMetadata, RuleType, FilterAction, RuleMetadata,
        },
        repositories::filter_repository::FilterRepository,
        services::filter_service::FilterService,
    },
};
use api::im::service::filter::{
    filter_server::Filter as GrpcFilter,
    FilterContentRequest,
    FilterContentResponse,
    BatchFilterContentRequest,
    BatchFilterContentResponse,
    AddRuleRequest,
    AddRuleResponse,
    UpdateRuleRequest,
    UpdateRuleResponse,
    DeleteRuleRequest,
    DeleteRuleResponse,
    GetRuleRequest,
    GetRuleResponse,
    GetRulesByTypeRequest,
    GetRulesByTypeResponse,
    EnableRuleRequest,
    EnableRuleResponse,
    DisableRuleRequest,
    DisableRuleResponse,
    ImportRulesRequest,
    ImportRulesResponse,
    ExportRulesRequest,
    ExportRulesResponse,
};

pub struct FilterGrpcService<R: FilterRepository, S: FilterService> {
    filter_manager: FilterManager<R, S>,
}

impl<R: FilterRepository, S: FilterService> FilterGrpcService<R, S> {
    pub fn new(filter_manager: FilterManager<R, S>) -> Self {
        Self { filter_manager }
    }

    // 转换规则类型
    fn convert_rule_type(rule_type: i32) -> RuleType {
        match rule_type {
            0 => RuleType::Keyword,
            1 => RuleType::Regex,
            2 => RuleType::Dictionary,
            3 => RuleType::ImageHash,
            4 => RuleType::MediaType,
            _ => RuleType::Custom,
        }
    }

    // 转换过滤动作
    fn convert_action(action: i32) -> FilterAction {
        match action {
            0 => FilterAction::Block,
            1 => FilterAction::Replace,
            2 => FilterAction::Warn,
            3 => FilterAction::Log,
            _ => FilterAction::Review,
        }
    }

    // 转换为 gRPC 规则
    fn to_grpc_rule(rule: &FilterRule) -> api::im::service::filter::Rule {
        api::im::service::filter::Rule {
            id: rule.id.to_string(),
            name: rule.name.clone(),
            rule_type: match rule.rule_type {
                RuleType::Keyword => 0,
                RuleType::Regex => 1,
                RuleType::Dictionary => 2,
                RuleType::ImageHash => 3,
                RuleType::MediaType => 4,
                RuleType::Custom => 5,
            },
            pattern: rule.pattern.clone(),
            action: match rule.action {
                FilterAction::Block => 0,
                FilterAction::Replace => 1,
                FilterAction::Warn => 2,
                FilterAction::Log => 3,
                FilterAction::Review => 4,
            },
            priority: rule.priority,
            is_enabled: rule.is_enabled,
            metadata: Some(api::im::service::filter::RuleMetadata {
                description: rule.metadata.description.clone(),
                category: rule.metadata.category.clone(),
                replacement: rule.metadata.replacement.clone().unwrap_or_default(),
                custom_config: rule.metadata.custom_config.clone(),
            }),
            created_at: rule.created_at.timestamp_millis(),
            updated_at: rule.updated_at.timestamp_millis(),
        }
    }

    // 从 gRPC 请求转换为规则
    fn from_grpc_request(req: &AddRuleRequest) -> Result<FilterRule, Status> {
        Ok(FilterRule {
            id: Uuid::new_v4(),
            name: req.name.clone(),
            rule_type: Self::convert_rule_type(req.rule_type),
            pattern: req.pattern.clone(),
            action: Self::convert_action(req.action),
            priority: req.priority,
            is_enabled: req.is_enabled,
            metadata: RuleMetadata {
                description: req.metadata.as_ref().map(|m| m.description.clone()).unwrap_or_default(),
                category: req.metadata.as_ref().map(|m| m.category.clone()).unwrap_or_default(),
                replacement: if req.metadata.as_ref().map(|m| m.replacement.is_empty()).unwrap_or(true) {
                    None
                } else {
                    req.metadata.as_ref().map(|m| m.replacement.clone())
                },
                custom_config: req.metadata.as_ref().map(|m| m.custom_config.clone()).unwrap_or_default(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

#[tonic::async_trait]
impl<R: FilterRepository + Send + Sync + 'static, S: FilterService + Send + Sync + 'static> 
    GrpcFilter for FilterGrpcService<R, S> 
{
    async fn filter_content(
        &self,
        request: Request<FilterContentRequest>,
    ) -> Result<Response<FilterContentResponse>, Status> {
        let req = request.into_inner();
        
        let filter_request = FilterRequest {
            content: req.content,
            content_type: req.content_type,
            metadata: FilterMetadata {
                user_id: req.user_id,
                session_id: req.session_id,
                device_info: if req.device_info.is_empty() { None } else { Some(req.device_info) },
                custom_properties: req.custom_properties,
            },
        };

        let result = self.filter_manager.filter_content(filter_request)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(FilterContentResponse {
            is_blocked: result.is_blocked,
            matched_rules: result.matched_rules.into_iter().map(|r| api::im::service::filter::MatchedRule {
                rule_id: r.rule_id.to_string(),
                rule_name: r.rule_name,
                rule_type: match r.rule_type {
                    RuleType::Keyword => 0,
                    RuleType::Regex => 1,
                    RuleType::Dictionary => 2,
                    RuleType::ImageHash => 3,
                    RuleType::MediaType => 4,
                    RuleType::Custom => 5,
                },
                action: match r.action {
                    FilterAction::Block => 0,
                    FilterAction::Replace => 1,
                    FilterAction::Warn => 2,
                    FilterAction::Log => 3,
                    FilterAction::Review => 4,
                },
                matched_content: r.matched_content,
                position: r.position.map(|p| api::im::service::filter::Position {
                    start: p.start as u32,
                    end: p.end as u32,
                }),
            }).collect(),
            modified_content: result.modified_content,
            review_required: result.review_required,
            risk_level: match result.risk_level {
                crate::domain::entities::filter::RiskLevel::Safe => 0,
                crate::domain::entities::filter::RiskLevel::Low => 1,
                crate::domain::entities::filter::RiskLevel::Medium => 2,
                crate::domain::entities::filter::RiskLevel::High => 3,
                crate::domain::entities::filter::RiskLevel::Critical => 4,
            },
            error: None,
        }))
    }

    async fn batch_filter_content(
        &self,
        request: Request<BatchFilterContentRequest>,
    ) -> Result<Response<BatchFilterContentResponse>, Status> {
        let req = request.into_inner();
        
        let filter_requests = req.requests.into_iter()
            .map(|r| FilterRequest {
                content: r.content,
                content_type: r.content_type,
                metadata: FilterMetadata {
                    user_id: r.user_id,
                    session_id: r.session_id,
                    device_info: if r.device_info.is_empty() { None } else { Some(r.device_info) },
                    custom_properties: r.custom_properties,
                },
            })
            .collect();

        let results = self.filter_manager.batch_filter_content(filter_requests)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(BatchFilterContentResponse {
            results: results.into_iter().map(|r| api::im::service::filter::FilterContentResponse {
                is_blocked: r.is_blocked,
                matched_rules: r.matched_rules.into_iter().map(|mr| api::im::service::filter::MatchedRule {
                    rule_id: mr.rule_id.to_string(),
                    rule_name: mr.rule_name,
                    rule_type: match mr.rule_type {
                        RuleType::Keyword => 0,
                        RuleType::Regex => 1,
                        RuleType::Dictionary => 2,
                        RuleType::ImageHash => 3,
                        RuleType::MediaType => 4,
                        RuleType::Custom => 5,
                    },
                    action: match mr.action {
                        FilterAction::Block => 0,
                        FilterAction::Replace => 1,
                        FilterAction::Warn => 2,
                        FilterAction::Log => 3,
                        FilterAction::Review => 4,
                    },
                    matched_content: mr.matched_content,
                    position: mr.position.map(|p| api::im::service::filter::Position {
                        start: p.start as u32,
                        end: p.end as u32,
                    }),
                }).collect(),
                modified_content: r.modified_content,
                review_required: r.review_required,
                risk_level: match r.risk_level {
                    crate::domain::entities::filter::RiskLevel::Safe => 0,
                    crate::domain::entities::filter::RiskLevel::Low => 1,
                    crate::domain::entities::filter::RiskLevel::Medium => 2,
                    crate::domain::entities::filter::RiskLevel::High => 3,
                    crate::domain::entities::filter::RiskLevel::Critical => 4,
                },
                error: None,
            }).collect(),
            error: None,
        }))
    }

    async fn add_rule(
        &self,
        request: Request<AddRuleRequest>,
    ) -> Result<Response<AddRuleResponse>, Status> {
        let req = request.into_inner();
        let rule = Self::from_grpc_request(&req)?;

        let added_rule = self.filter_manager.add_rule(rule)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(AddRuleResponse {
            rule: Some(Self::to_grpc_rule(&added_rule)),
            error: None,
        }))
    }

    async fn update_rule(
        &self,
        request: Request<UpdateRuleRequest>,
    ) -> Result<Response<UpdateRuleResponse>, Status> {
        let req = request.into_inner();
        let rule = FilterRule {
            id: Uuid::parse_str(&req.id).map_err(|e| Status::invalid_argument(e.to_string()))?,
            name: req.name,
            rule_type: Self::convert_rule_type(req.rule_type),
            pattern: req.pattern,
            action: Self::convert_action(req.action),
            priority: req.priority,
            is_enabled: req.is_enabled,
            metadata: RuleMetadata {
                description: req.metadata.as_ref().map(|m| m.description.clone()).unwrap_or_default(),
                category: req.metadata.as_ref().map(|m| m.category.clone()).unwrap_or_default(),
                replacement: if req.metadata.as_ref().map(|m| m.replacement.is_empty()).unwrap_or(true) {
                    None
                } else {
                    req.metadata.as_ref().map(|m| m.replacement.clone())
                },
                custom_config: req.metadata.as_ref().map(|m| m.custom_config.clone()).unwrap_or_default(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let updated_rule = self.filter_manager.update_rule(rule)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(UpdateRuleResponse {
            rule: Some(Self::to_grpc_rule(&updated_rule)),
            error: None,
        }))
    }

    async fn delete_rule(
        &self,
        request: Request<DeleteRuleRequest>,
    ) -> Result<Response<DeleteRuleResponse>, Status> {
        let req = request.into_inner();
        
        self.filter_manager.delete_rule(&req.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DeleteRuleResponse {
            success: true,
            error: None,
        }))
    }

    async fn get_rule(
        &self,
        request: Request<GetRuleRequest>,
    ) -> Result<Response<GetRuleResponse>, Status> {
        let req = request.into_inner();
        
        let rule = self.filter_manager.get_rule(&req.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetRuleResponse {
            rule: rule.map(|r| Self::to_grpc_rule(&r)),
            error: None,
        }))
    }

    async fn get_rules_by_type(
        &self,
        request: Request<GetRulesByTypeRequest>,
    ) -> Result<Response<GetRulesByTypeResponse>, Status> {
        let req = request.into_inner();
        
        let rules = self.filter_manager.get_rules_by_type(Self::convert_rule_type(req.rule_type))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetRulesByTypeResponse {
            rules: rules.into_iter().map(|r| Self::to_grpc_rule(&r)).collect(),
            error: None,
        }))
    }

    async fn enable_rule(
        &self,
        request: Request<EnableRuleRequest>,
    ) -> Result<Response<EnableRuleResponse>, Status> {
        let req = request.into_inner();
        
        self.filter_manager.enable_rule(&req.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(EnableRuleResponse {
            success: true,
            error: None,
        }))
    }

    async fn disable_rule(
        &self,
        request: Request<DisableRuleRequest>,
    ) -> Result<Response<DisableRuleResponse>, Status> {
        let req = request.into_inner();
        
        self.filter_manager.disable_rule(&req.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DisableRuleResponse {
            success: true,
            error: None,
        }))
    }

    async fn import_rules(
        &self,
        request: Request<ImportRulesRequest>,
    ) -> Result<Response<ImportRulesResponse>, Status> {
        let req = request.into_inner();
        
        let rules = req.rules.into_iter()
            .map(|r| FilterRule {
                id: Uuid::parse_str(&r.id).unwrap_or_else(|_| Uuid::new_v4()),
                name: r.name,
                rule_type: Self::convert_rule_type(r.rule_type),
                pattern: r.pattern,
                action: Self::convert_action(r.action),
                priority: r.priority,
                is_enabled: r.is_enabled,
                metadata: RuleMetadata {
                    description: r.metadata.as_ref().map(|m| m.description.clone()).unwrap_or_default(),
                    category: r.metadata.as_ref().map(|m| m.category.clone()).unwrap_or_default(),
                    replacement: if r.metadata.as_ref().map(|m| m.replacement.is_empty()).unwrap_or(true) {
                        None
                    } else {
                        r.metadata.as_ref().map(|m| m.replacement.clone())
                    },
                    custom_config: r.metadata.as_ref().map(|m| m.custom_config.clone()).unwrap_or_default(),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .collect();

        self.filter_manager.import_rules(rules)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ImportRulesResponse {
            success: true,
            error: None,
        }))
    }

    async fn export_rules(
        &self,
        request: Request<ExportRulesRequest>,
    ) -> Result<Response<ExportRulesResponse>, Status> {
        let req = request.into_inner();
        
        let rule_type = if req.rule_type >= 0 {
            Some(Self::convert_rule_type(req.rule_type))
        } else {
            None
        };

        let rules = self.filter_manager.export_rules(rule_type)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ExportRulesResponse {
            rules: rules.into_iter().map(|r| Self::to_grpc_rule(&r)).collect(),
            error: None,
        }))
    }
} 