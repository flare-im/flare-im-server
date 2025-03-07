use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::elasticsearch::{ElasticsearchClient, ElasticsearchConfig};
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.search");

// 搜索服务
pub struct SearchService {
    // Elasticsearch 客户端
    es: Arc<ElasticsearchClient>,
}

impl SearchService {
    pub async fn new(config: &common::config::Config) -> Result<Self> {
        // 初始化 Elasticsearch 客户端
        let es_config = ElasticsearchConfig {
            hosts: config.elasticsearch.hosts.clone(),
            username: config.elasticsearch.username.clone(),
            password: config.elasticsearch.password.clone(),
        };
        let es = Arc::new(ElasticsearchClient::new(es_config).await?);

        Ok(Self { es })
    }

    // 搜索消息
    async fn search_messages(&self, request: &SearchMessagesRequest) -> Result<Vec<Message>> {
        let mut query = json!({
            "query": {
                "bool": {
                    "must": [
                        {
                            "match": {
                                "content": request.keyword
                            }
                        }
                    ],
                    "filter": [
                        {
                            "term": {
                                "conversation_id": request.conversation_id
                            }
                        }
                    ]
                }
            },
            "sort": [
                {
                    "timestamp": {
                        "order": "desc"
                    }
                }
            ],
            "from": request.offset,
            "size": request.limit
        });

        // 添加时间范围过滤
        if let Some(time_range) = &request.time_range {
            query["query"]["bool"]["filter"].as_array_mut().unwrap().push(json!({
                "range": {
                    "timestamp": {
                        "gte": time_range.start_time,
                        "lte": time_range.end_time
                    }
                }
            }));
        }

        // 执行搜索
        let result = self.es.search("messages", query).await?;
        
        // 解析结果
        let messages = result.hits.hits.into_iter()
            .filter_map(|hit| serde_json::from_value(hit.source).ok())
            .collect();

        Ok(messages)
    }

    // 搜索文件
    async fn search_files(&self, request: &SearchFilesRequest) -> Result<Vec<File>> {
        let query = json!({
            "query": {
                "bool": {
                    "must": [
                        {
                            "match": {
                                "name": request.keyword
                            }
                        }
                    ],
                    "filter": [
                        {
                            "term": {
                                "conversation_id": request.conversation_id
                            }
                        }
                    ]
                }
            },
            "sort": [
                {
                    "timestamp": {
                        "order": "desc"
                    }
                }
            ],
            "from": request.offset,
            "size": request.limit
        });

        // 执行搜索
        let result = self.es.search("files", query).await?;
        
        // 解析结果
        let files = result.hits.hits.into_iter()
            .filter_map(|hit| serde_json::from_value(hit.source).ok())
            .collect();

        Ok(files)
    }
}

#[tonic::async_trait]
impl search_server::Search for SearchService {
    // 搜索消息
    async fn search_messages(
        &self,
        request: Request<SearchMessagesRequest>,
    ) -> Result<Response<SearchMessagesResponse>, Status> {
        let req = request.into_inner();
        info!("收到搜索消息请求: {:?}", req);

        match self.search_messages(&req).await {
            Ok(messages) => {
                let response = SearchMessagesResponse {
                    messages,
                    total: messages.len() as i32,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("搜索消息失败: {}", e);
                let response = SearchMessagesResponse {
                    messages: vec![],
                    total: 0,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 搜索文件
    async fn search_files(
        &self,
        request: Request<SearchFilesRequest>,
    ) -> Result<Response<SearchFilesResponse>, Status> {
        let req = request.into_inner();
        info!("收到搜索文件请求: {:?}", req);

        match self.search_files(&req).await {
            Ok(files) => {
                let response = SearchFilesResponse {
                    files,
                    total: files.len() as i32,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("搜索文件失败: {}", e);
                let response = SearchFilesResponse {
                    files: vec![],
                    total: 0,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动搜索服务
pub async fn start_search_service(config: common::config::Config) -> Result<()> {
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

    // 创建搜索服务
    let search_service = SearchService::new(&config).await?;
    let search_server = search_server::SearchServer::new(search_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(search_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
}
