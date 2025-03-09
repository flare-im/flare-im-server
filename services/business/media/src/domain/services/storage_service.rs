use async_trait::async_trait;
use crate::domain::entities::media::{
    Media, UploadRequest, UploadResponse, CompleteMultipartUpload,
    DownloadRequest, DownloadResponse, MediaQuery,
};

#[async_trait]
pub trait StorageService: Send + Sync {
    // 初始化分片上传
    async fn init_multipart_upload(&self, request: UploadRequest) -> Result<UploadResponse, Error>;

    // 完成分片上传
    async fn complete_multipart_upload(&self, request: CompleteMultipartUpload) -> Result<Media, Error>;

    // 取消分片上传
    async fn abort_multipart_upload(&self, media_id: uuid::Uuid, upload_id: &str) -> Result<(), Error>;

    // 获取下载URL
    async fn get_download_url(&self, request: DownloadRequest) -> Result<DownloadResponse, Error>;

    // 删除媒体
    async fn delete_media(&self, media_id: uuid::Uuid) -> Result<(), Error>;

    // 获取媒体信息
    async fn get_media(&self, media_id: uuid::Uuid) -> Result<Option<Media>, Error>;

    // 查询媒体列表
    async fn query_media(&self, query: MediaQuery) -> Result<Vec<Media>, Error>;

    // 获取存储统计信息
    async fn get_storage_stats(&self, bucket: &str) -> Result<StorageStats, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Service error: {0}")]
    Service(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_size: u64,
    pub total_files: u64,
    pub total_buckets: u64,
    pub used_size: u64,
    pub used_files: u64,
} 