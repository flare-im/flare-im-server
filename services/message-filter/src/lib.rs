use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::redis::{RedisClient, RedisConfig};
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.filter");

// 消息过滤服务
pub struct MessageFilterService {
    // Redis 客户端，用于存储过滤规则
    redis: Arc<RedisClient>,
}

impl MessageFilterService {
    pub async fn new(config: &common::config::Config) -> Result<Self> {
        // 初始化 Redis 客户端
        let redis_config = RedisConfig {
            host: config.redis.host.clone(),
            port: config.redis.port,
            password: config.redis.password.clone(),
            database: config.redis.database,
            pool_size: config.redis.pool_size,
        };
        let redis = Arc::new(RedisClient::new(redis_config).await?);

        Ok(Self { redis })
    }

    // 加载过滤规则
    async fn load_filter_rules(&self, rule_type: &str) -> Result<Vec<FilterRule>> {
        let key = format!("filter:rules:{}", rule_type);
        let rules: Option<String> = self.redis.get(&key).await?;
        
        if let Some(rules_str) = rules {
            let rules: Vec<FilterRule> = serde_json::from_str(&rules_str)?;
            Ok(rules)
        } else {
            Ok(vec![])
        }
    }

    // 更新过滤规则
    async fn update_filter_rules(&self, rule_type: &str, rules: &[FilterRule]) -> Result<()> {
        let key = format!("filter:rules:{}", rule_type);
        let rules_str = serde_json::to_string(rules)?;
        self.redis.set(&key, &rules_str, Some(3600 * 24)).await?;
        Ok(())
    }

    // 检查文本内容
    async fn check_text_content(&self, content: &str) -> Result<FilterResult> {
        // 加载敏感词规则
        let rules = self.load_filter_rules("sensitive_words").await?;
        
        let mut result = FilterResult {
            passed: true,
            risk_level: 0,
            matched_rules: vec![],
            suggestion: "".to_string(),
        };

        // 检查每个规则
        for rule in rules {
            if content.contains(&rule.pattern) {
                result.passed = false;
                result.risk_level = rule.risk_level;
                result.matched_rules.push(rule.clone());
            }
        }

        // 如果未通过，生成建议
        if !result.passed {
            result.suggestion = "包含敏感词，建议修改".to_string();
        }

        Ok(result)
    }

    // 检查图片内容
    async fn check_image_content(&self, image_url: &str) -> Result<FilterResult> {
        // TODO: 实现图片内容检查
        // 1. 调用图片识别服务
        // 2. 检查违规内容
        // 3. 返回检查结果
        Ok(FilterResult {
            passed: true,
            risk_level: 0,
            matched_rules: vec![],
            suggestion: "".to_string(),
        })
    }
}

#[tonic::async_trait]
impl message_filter_server::MessageFilter for MessageFilterService {
    // 过滤消息
    async fn filter_message(
        &self,
        request: Request<FilterMessageRequest>,
    ) -> Result<Response<FilterMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到消息过滤请求: {:?}", req);

        let message = req.message.ok_or_else(|| Status::invalid_argument("消息不能为空"))?;
        let mut filter_results = Vec::new();

        // 根据消息类型进行不同的过滤
        match message.message_type {
            0 => { // 文本消息
                let result = self.check_text_content(&message.content).await
                    .map_err(|e| Status::internal(e.to_string()))?;
                filter_results.push(result);
            }
            2 | 3 | 4 => { // 图片、语音、视频消息
                let result = self.check_image_content(&message.content).await
                    .map_err(|e| Status::internal(e.to_string()))?;
                filter_results.push(result);
            }
            _ => {}
        }

        // 汇总过滤结果
        let passed = filter_results.iter().all(|r| r.passed);
        let max_risk_level = filter_results.iter().map(|r| r.risk_level).max().unwrap_or(0);
        let all_matched_rules: Vec<FilterRule> = filter_results
            .iter()
            .flat_map(|r| r.matched_rules.clone())
            .collect();
        let suggestions: Vec<String> = filter_results
            .iter()
            .filter(|r| !r.suggestion.is_empty())
            .map(|r| r.suggestion.clone())
            .collect();

        let response = FilterMessageResponse {
            message_id: message.message_id,
            passed,
            risk_level: max_risk_level,
            matched_rules: all_matched_rules,
            suggestion: suggestions.join("; "),
            status: 0,
            error: "".to_string(),
        };

        Ok(Response::new(response))
    }

    // 批量过滤消息
    async fn batch_filter_messages(
        &self,
        request: Request<BatchFilterMessageRequest>,
    ) -> Result<Response<BatchFilterMessageResponse>, Status> {
        let req = request.into_inner();
        info!("收到批量消息过滤请求: {:?}", req);

        let mut results = Vec::new();
        for message in req.messages {
            let filter_req = FilterMessageRequest {
                message: Some(message),
            };
            match self.filter_message(Request::new(filter_req)).await {
                Ok(response) => {
                    results.push(response.into_inner());
                }
                Err(status) => {
                    results.push(FilterMessageResponse {
                        message_id: "".to_string(),
                        passed: false,
                        risk_level: 3,
                        matched_rules: vec![],
                        suggestion: status.message().to_string(),
                        status: 1,
                        error: status.message().to_string(),
                    });
                }
            }
        }

        let response = BatchFilterMessageResponse {
            results,
            status: 0,
            error: "".to_string(),
        };

        Ok(Response::new(response))
    }

    // 更新过滤规则
    async fn update_filter_rules(
        &self,
        request: Request<UpdateFilterRulesRequest>,
    ) -> Result<Response<UpdateFilterRulesResponse>, Status> {
        let req = request.into_inner();
        info!("收到更新过滤规则请求: {:?}", req);

        match self.update_filter_rules(&req.rule_type, &req.rules).await {
            Ok(_) => {
                let response = UpdateFilterRulesResponse {
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("更新过滤规则失败: {}", e);
                let response = UpdateFilterRulesResponse {
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 获取过滤规则
    async fn get_filter_rules(
        &self,
        request: Request<GetFilterRulesRequest>,
    ) -> Result<Response<GetFilterRulesResponse>, Status> {
        let req = request.into_inner();
        info!("收到获取过滤规则请求: {:?}", req);

        match self.load_filter_rules(&req.rule_type).await {
            Ok(rules) => {
                let response = GetFilterRulesResponse {
                    rules,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("获取过滤规则失败: {}", e);
                let response = GetFilterRulesResponse {
                    rules: vec![],
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动消息过滤服务
pub async fn start_message_filter(config: common::config::Config) -> Result<()> {
    // 初始化日志
    common::log::init_logger(&config.log)?;

    // 创建 Consul 配置
    let consul_config = ConsulConfig {
        addr: format!("{}:{}", config.consul.host, config.consul.port),
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };

    // 创建 Consul 注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(15)).await?;

    // 创建并配置应用
    let app = AppBuilder::new(&config.service.name)
        .version(&config.service.metadata.get("version").unwrap_or(&"1.0.0".to_string()))
        .tags(&config.service.tags)
        .meta("protocol", "grpc")
        .weight(10)
        .register(registry)
        .build();

    // 创建消息过滤服务
    let filter_service = MessageFilterService::new(&config).await?;
    let filter_server = message_filter_server::MessageFilterServer::new(filter_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(filter_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 