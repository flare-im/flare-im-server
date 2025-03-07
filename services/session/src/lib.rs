use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::redis::{RedisClient, RedisConfig};
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tonic::{Request, Response, Status};

// 包含生成的 proto 代码
tonic::include_proto!("api.im.session");

// 会话服务
pub struct SessionService {
    // Redis 客户端，用于存储会话状态
    redis: Arc<RedisClient>,
}

impl SessionService {
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

    // 创建会话
    async fn create_session(&self, request: &CreateSessionRequest) -> Result<Session> {
        // 生成会话 ID
        let session_id = uuid::Uuid::new_v4().to_string();

        // 创建会话对象
        let session = Session {
            session_id: session_id.clone(),
            user_id: request.user_id.clone(),
            device_id: request.device_id.clone(),
            device_type: request.device_type,
            online_status: OnlineStatus::Online as i32,
            last_active_time: chrono::Utc::now().timestamp(),
        };

        // 保存会话信息到 Redis
        let key = format!("session:{}", session_id);
        self.redis.set(&key, serde_json::to_string(&session)?).await?;

        // 更新用户的会话列表
        let user_sessions_key = format!("user:sessions:{}", request.user_id);
        self.redis.sadd(&user_sessions_key, &session_id).await?;

        Ok(session)
    }

    // 更新会话状态
    async fn update_session_status(&self, request: &UpdateSessionStatusRequest) -> Result<Session> {
        let key = format!("session:{}", request.session_id);
        
        // 获取会话信息
        let session_str = self.redis.get(&key).await?
            .ok_or_else(|| anyhow::anyhow!("会话不存在"))?;
        let mut session: Session = serde_json::from_str(&session_str)?;

        // 更新状态
        session.online_status = request.online_status;
        session.last_active_time = chrono::Utc::now().timestamp();

        // 保存更新后的会话信息
        self.redis.set(&key, serde_json::to_string(&session)?).await?;

        Ok(session)
    }

    // 获取用户的会话列表
    async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<Session>> {
        let user_sessions_key = format!("user:sessions:{}", user_id);
        
        // 获取用户的所有会话 ID
        let session_ids: Vec<String> = self.redis.smembers(&user_sessions_key).await?;

        // 获取所有会话的详细信息
        let mut sessions = Vec::new();
        for session_id in session_ids {
            let key = format!("session:{}", session_id);
            if let Some(session_str) = self.redis.get(&key).await? {
                if let Ok(session) = serde_json::from_str(&session_str) {
                    sessions.push(session);
                }
            }
        }

        Ok(sessions)
    }

    // 删除会话
    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let key = format!("session:{}", session_id);
        
        // 获取会话信息
        let session_str = self.redis.get(&key).await?
            .ok_or_else(|| anyhow::anyhow!("会话不存在"))?;
        let session: Session = serde_json::from_str(&session_str)?;

        // 从用户的会话列表中移除
        let user_sessions_key = format!("user:sessions:{}", session.user_id);
        self.redis.srem(&user_sessions_key, session_id).await?;

        // 删除会话信息
        self.redis.del(&key).await?;

        Ok(())
    }
}

#[tonic::async_trait]
impl session_server::Session for SessionService {
    // 创建会话
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        let req = request.into_inner();
        info!("收到创建会话请求: {:?}", req);

        match self.create_session(&req).await {
            Ok(session) => {
                let response = CreateSessionResponse {
                    session: Some(session),
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("创建会话失败: {}", e);
                let response = CreateSessionResponse {
                    session: None,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 更新会话状态
    async fn update_session_status(
        &self,
        request: Request<UpdateSessionStatusRequest>,
    ) -> Result<Response<UpdateSessionStatusResponse>, Status> {
        let req = request.into_inner();
        info!("收到更新会话状态请求: {:?}", req);

        match self.update_session_status(&req).await {
            Ok(session) => {
                let response = UpdateSessionStatusResponse {
                    session: Some(session),
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("更新会话状态失败: {}", e);
                let response = UpdateSessionStatusResponse {
                    session: None,
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 获取用户的会话列表
    async fn get_user_sessions(
        &self,
        request: Request<GetUserSessionsRequest>,
    ) -> Result<Response<GetUserSessionsResponse>, Status> {
        let req = request.into_inner();
        info!("收到获取用户会话列表请求: {:?}", req);

        match self.get_user_sessions(&req.user_id).await {
            Ok(sessions) => {
                let response = GetUserSessionsResponse {
                    sessions,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("获取用户会话列表失败: {}", e);
                let response = GetUserSessionsResponse {
                    sessions: vec![],
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 删除会话
    async fn delete_session(
        &self,
        request: Request<DeleteSessionRequest>,
    ) -> Result<Response<DeleteSessionResponse>, Status> {
        let req = request.into_inner();
        info!("收到删除会话请求: {:?}", req);

        match self.delete_session(&req.session_id).await {
            Ok(()) => {
                let response = DeleteSessionResponse {
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("删除会话失败: {}", e);
                let response = DeleteSessionResponse {
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动会话服务
pub async fn start_session_service(config: common::config::Config) -> Result<()> {
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

    // 创建会话服务
    let session_service = SessionService::new(&config).await?;
    let session_server = session_server::SessionServer::new(session_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(session_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
} 