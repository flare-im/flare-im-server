use std::sync::Arc;
use aws_sdk_s3::Client as S3Client;
use anyhow::Result;
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};
use flare_rpc_core::minio::{MinioClient, MinioConfig};
use flare_rpc_core::redis::{RedisClient, RedisConfig};
use log::{error, info};
use std::time::Duration;
use tonic::{Request, Response, Status};
use uuid::Uuid;

// 包含生成的 proto 代码
tonic::include_proto!("api.im.media");

pub mod media {
    use aws_sdk_s3::primitives::ByteStream;
    use bytes::Bytes;
    use futures::Stream;
    use proto::api::im::media::media_server::Media;
    use proto::api::im::media::{
        UploadFileRequest, UploadFileResponse,
        DownloadFileRequest, DownloadFileResponse,
        DeleteFileRequest, DeleteFileResponse,
        GetFileInfoRequest, GetFileInfoResponse,
    };

    #[derive(Debug)]
    pub struct MediaService {
        s3_client: Arc<S3Client>,
        bucket: String,
    }

    impl MediaService {
        pub async fn new(endpoint: &str, bucket: &str) -> anyhow::Result<Self> {
            let config = aws_config::from_env()
                .endpoint_url(endpoint)
                .load()
                .await;
            
            let s3_client = Arc::new(S3Client::new(&config));

            Ok(Self {
                s3_client,
                bucket: bucket.to_string(),
            })
        }

        async fn upload_part(&self, key: &str, part_number: i32, data: Bytes) -> anyhow::Result<String> {
            let resp = self.s3_client
                .upload_part()
                .bucket(&self.bucket)
                .key(key)
                .part_number(part_number as i32)
                .body(ByteStream::from(data))
                .send()
                .await?;

            Ok(resp.e_tag().unwrap_or_default().to_string())
        }
    }

    #[tonic::async_trait]
    impl Media for MediaService {
        /// 上传文件
        async fn upload_file(
            &self,
            request: Request<UploadFileRequest>,
        ) -> Result<Response<UploadFileResponse>, Status> {
            todo!("Implement upload_file")
        }

        /// 下载文件
        async fn download_file(
            &self,
            request: Request<DownloadFileRequest>,
        ) -> Result<Response<DownloadFileResponse>, Status> {
            todo!("Implement download_file")
        }

        /// 删除文件
        async fn delete_file(
            &self,
            request: Request<DeleteFileRequest>,
        ) -> Result<Response<DeleteFileResponse>, Status> {
            todo!("Implement delete_file")
        }

        /// 获取文件信息
        async fn get_file_info(
            &self,
            request: Request<GetFileInfoRequest>,
        ) -> Result<Response<GetFileInfoResponse>, Status> {
            todo!("Implement get_file_info")
        }
    }
}

// 媒体服务
pub struct MediaService {
    // MinIO 客户端，用于对象存储
    minio: Arc<MinioClient>,
    // Redis 客户端，用于缓存
    redis: Arc<RedisClient>,
}

impl MediaService {
    pub async fn new(config: &common::config::Config) -> Result<Self> {
        // 初始化 MinIO 客户端
        let minio_config = MinioConfig {
            endpoint: config.minio.endpoint.clone(),
            access_key: config.minio.access_key.clone(),
            secret_key: config.minio.secret_key.clone(),
            bucket: config.minio.bucket.clone(),
            region: config.minio.region.clone(),
        };
        let minio = Arc::new(MinioClient::new(minio_config).await?);

        // 初始化 Redis 客户端
        let redis_config = RedisConfig {
            host: config.redis.host.clone(),
            port: config.redis.port,
            password: config.redis.password.clone(),
            database: config.redis.database,
            pool_size: config.redis.pool_size,
        };
        let redis = Arc::new(RedisClient::new(redis_config).await?);

        Ok(Self { minio, redis })
    }

    // 生成上传 URL
    async fn generate_upload_url(&self, file_type: &str) -> Result<(String, String)> {
        let file_id = Uuid::new_v4().to_string();
        let key = format!("{}/{}", file_type, file_id);
        let url = self.minio.generate_presigned_url(&key, 3600).await?;
        Ok((file_id, url))
    }

    // 生成下载 URL
    async fn generate_download_url(&self, file_id: &str) -> Result<String> {
        let url = self.minio.generate_presigned_url(file_id, 3600).await?;
        Ok(url)
    }

    // 处理图片
    async fn process_image(&self, file_id: &str, options: &ImageProcessOptions) -> Result<String> {
        // TODO: 实现图片处理
        // 1. 下载原图
        // 2. 根据选项处理图片
        // 3. 上传处理后的图片
        // 4. 返回新图片的 URL
        Ok(format!("processed_{}", file_id))
    }

    // 处理视频
    async fn process_video(&self, file_id: &str, options: &VideoProcessOptions) -> Result<String> {
        // TODO: 实现视频处理
        // 1. 下载原视频
        // 2. 根据选项处理视频
        // 3. 上传处理后的视频
        // 4. 返回新视频的 URL
        Ok(format!("processed_{}", file_id))
    }
}

#[tonic::async_trait]
impl media_server::Media for MediaService {
    // 获取上传 URL
    async fn get_upload_url(
        &self,
        request: Request<GetUploadUrlRequest>,
    ) -> Result<Response<GetUploadUrlResponse>, Status> {
        let req = request.into_inner();
        info!("收到获取上传 URL 请求: {:?}", req);

        match self.generate_upload_url(&req.file_type).await {
            Ok((file_id, url)) => {
                let response = GetUploadUrlResponse {
                    file_id,
                    upload_url: url,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("生成上传 URL 失败: {}", e);
                let response = GetUploadUrlResponse {
                    file_id: "".to_string(),
                    upload_url: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 获取下载 URL
    async fn get_download_url(
        &self,
        request: Request<GetDownloadUrlRequest>,
    ) -> Result<Response<GetDownloadUrlResponse>, Status> {
        let req = request.into_inner();
        info!("收到获取下载 URL 请求: {:?}", req);

        match self.generate_download_url(&req.file_id).await {
            Ok(url) => {
                let response = GetDownloadUrlResponse {
                    download_url: url,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("生成下载 URL 失败: {}", e);
                let response = GetDownloadUrlResponse {
                    download_url: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 处理图片
    async fn process_image(
        &self,
        request: Request<ProcessImageRequest>,
    ) -> Result<Response<ProcessImageResponse>, Status> {
        let req = request.into_inner();
        info!("收到处理图片请求: {:?}", req);

        let options = req.options.ok_or_else(|| Status::invalid_argument("处理选项不能为空"))?;

        match self.process_image(&req.file_id, &options).await {
            Ok(processed_file_id) => {
                let response = ProcessImageResponse {
                    processed_file_id,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("处理图片失败: {}", e);
                let response = ProcessImageResponse {
                    processed_file_id: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }

    // 处理视频
    async fn process_video(
        &self,
        request: Request<ProcessVideoRequest>,
    ) -> Result<Response<ProcessVideoResponse>, Status> {
        let req = request.into_inner();
        info!("收到处理视频请求: {:?}", req);

        let options = req.options.ok_or_else(|| Status::invalid_argument("处理选项不能为空"))?;

        match self.process_video(&req.file_id, &options).await {
            Ok(processed_file_id) => {
                let response = ProcessVideoResponse {
                    processed_file_id,
                    status: 0,
                    error: "".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("处理视频失败: {}", e);
                let response = ProcessVideoResponse {
                    processed_file_id: "".to_string(),
                    status: 1,
                    error: e.to_string(),
                };
                Ok(Response::new(response))
            }
        }
    }
}

/// 启动媒体服务
pub async fn start_media_service(config: common::config::Config) -> Result<()> {
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

    // 创建媒体服务
    let media_service = MediaService::new(&config).await?;
    let media_server = media_server::MediaServer::new(media_service);

    // 启动服务
    app.run(&config.service.host, config.service.port, |mut server| async move {
        server
            .add_service(media_server)
            .serve(format!("{}:{}", config.service.host, config.service.port).parse()?)
            .await
            .map_err(|e| e.into())
    })
    .await?;

    Ok(())
}
