use std::sync::Arc;
use aws_sdk_s3::Client;
use tonic::{Request, Response, Status};
use log::{info, error, debug};
use uuid::Uuid;

use proto_crate::media::{
    media_server::Media,
    UploadRequest, UploadResponse,
    DownloadRequest, DownloadResponse,
    DeleteRequest, DeleteResponse,
};

use crate::config::Config;
use crate::storage;

#[derive(Debug)]
pub struct MediaService {
    config: Arc<Config>,
    s3_client: Client,
}

impl MediaService {
    pub fn new(config: Config, s3_client: Client) -> Self {
        Self {
            config: Arc::new(config),
            s3_client,
        }
    }

    fn generate_key(&self, user_id: &str, filename: &str) -> String {
        let uuid = Uuid::new_v4();
        format!("{}/{}-{}", user_id, uuid, filename)
    }
}

#[tonic::async_trait]
impl Media for MediaService {
    async fn upload(
        &self,
        request: Request<UploadRequest>
    ) -> Result<Response<UploadResponse>, Status> {
        let req = request.into_inner();
        debug!("Received upload request from user_id: {}", req.user_id);

        // 检查文件大小
        if req.data.len() > self.config.server.max_file_size {
            return Err(Status::invalid_argument("File too large"));
        }

        // 生成文件键
        let key = self.generate_key(&req.user_id, &req.filename);

        // 上传文件
        match storage::upload_file(
            &self.s3_client,
            &self.config.s3.bucket,
            &key,
            &req.content_type,
            req.data.into(),
        ).await {
            Ok(url) => {
                info!("File uploaded successfully: {}", url);
                Ok(Response::new(UploadResponse { url }))
            }
            Err(e) => {
                error!("Failed to upload file: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }

    async fn download(
        &self,
        request: Request<DownloadRequest>
    ) -> Result<Response<DownloadResponse>, Status> {
        let req = request.into_inner();
        debug!("Received download request for key: {}", req.key);

        match storage::download_file(
            &self.s3_client,
            &self.config.s3.bucket,
            &req.key,
        ).await {
            Ok(data) => {
                info!("File downloaded successfully: {}", req.key);
                Ok(Response::new(DownloadResponse {
                    data: data.to_vec(),
                }))
            }
            Err(e) => {
                error!("Failed to download file: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }

    async fn delete(
        &self,
        request: Request<DeleteRequest>
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();
        debug!("Received delete request for key: {}", req.key);

        match storage::delete_file(
            &self.s3_client,
            &self.config.s3.bucket,
            &req.key,
        ).await {
            Ok(_) => {
                info!("File deleted successfully: {}", req.key);
                Ok(Response::new(DeleteResponse {}))
            }
            Err(e) => {
                error!("Failed to delete file: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
} 