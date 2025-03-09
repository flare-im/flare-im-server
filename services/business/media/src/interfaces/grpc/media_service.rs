use tonic::{Request, Response, Status};
use std::sync::Arc;
use proto_crate::api::im::service::media::{
    media_server::Media,
    InitUploadRequest,
    InitUploadResponse,
    CompleteUploadRequest,
    CompleteUploadResponse,
    AbortUploadRequest,
    AbortUploadResponse,
    GetDownloadUrlRequest,
    GetDownloadUrlResponse,
    DeleteMediaRequest,
    DeleteMediaResponse,
    GetMediaRequest,
    GetMediaResponse,
    QueryMediaRequest,
    QueryMediaResponse,
    GetStorageStatsRequest,
    GetStorageStatsResponse,
};
use crate::domain::{
    services::storage_service::{StorageService, Error, StorageStats},
    entities::media::*,
};
use uuid::Uuid;
use chrono::Utc;

pub struct MediaManager {
    storage_service: Arc<dyn StorageService + Send + Sync>,
}

impl MediaManager {
    pub fn new(storage_service: Arc<dyn StorageService + Send + Sync>) -> Self {
        Self { storage_service }
    }

    // 初始化上传
    pub async fn init_upload(&self, request: UploadRequest) -> Result<UploadResponse, Error> {
        // 验证请求参数
        self.validate_upload_request(&request)?;
        
        // 调用存储服务初始化上传
        self.storage_service.init_multipart_upload(request).await
    }

    // 完成上传
    pub async fn complete_upload(&self, request: CompleteMultipartUpload) -> Result<Media, Error> {
        // 验证分片信息
        self.validate_parts(&request.parts)?;
        
        // 调用存储服务完成上传
        self.storage_service.complete_multipart_upload(request).await
    }

    // 取消上传
    pub async fn abort_upload(&self, media_id: Uuid, upload_id: &str) -> Result<(), Error> {
        self.storage_service.abort_multipart_upload(media_id, upload_id).await
    }

    // 获取下载链接
    pub async fn get_download_url(&self, request: DownloadRequest) -> Result<DownloadResponse, Error> {
        // 验证下载请求
        self.validate_download_request(&request)?;
        
        self.storage_service.get_download_url(request).await
    }

    // 删除媒体
    pub async fn delete_media(&self, media_id: Uuid, user_id: &str) -> Result<(), Error> {
        // 检查权限
        let media = self.storage_service.get_media(media_id).await?
            .ok_or_else(|| Error::NotFound("Media not found".to_string()))?;
            
        if media.metadata.user_id != user_id {
            return Err(Error::PermissionDenied("No permission to delete this media".to_string()));
        }
        
        self.storage_service.delete_media(media_id).await
    }

    // 查询媒体
    pub async fn query_media(&self, query: MediaQuery) -> Result<Vec<Media>, Error> {
        // 验证查询参数
        self.validate_query(&query)?;
        
        self.storage_service.query_media(query).await
    }

    // 获取存储统计
    pub async fn get_storage_stats(&self, bucket: &str) -> Result<StorageStats, Error> {
        self.storage_service.get_storage_stats(bucket).await
    }

    // 验证上传请求
    fn validate_upload_request(&self, request: &UploadRequest) -> Result<(), Error> {
        if request.file_name.is_empty() {
            return Err(Error::InvalidRequest("File name is required".to_string()));
        }

        if request.file_size == 0 {
            return Err(Error::InvalidRequest("File size must be greater than 0".to_string()));
        }

        if request.mime_type.is_empty() {
            return Err(Error::InvalidRequest("MIME type is required".to_string()));
        }

        if request.bucket.is_empty() {
            return Err(Error::InvalidRequest("Bucket is required".to_string()));
        }

        if request.user_id.is_empty() {
            return Err(Error::InvalidRequest("User ID is required".to_string()));
        }

        Ok(())
    }

    // 验证分片信息
    fn validate_parts(&self, parts: &[PartInfo]) -> Result<(), Error> {
        if parts.is_empty() {
            return Err(Error::InvalidRequest("Parts cannot be empty".to_string()));
        }

        // 检查分片编号是否连续
        let mut part_numbers: Vec<i32> = parts.iter().map(|p| p.part_number).collect();
        part_numbers.sort_unstable();
        
        for (i, &part_number) in part_numbers.iter().enumerate() {
            if part_number != (i + 1) as i32 {
                return Err(Error::InvalidRequest("Part numbers must be consecutive".to_string()));
            }
        }

        Ok(())
    }

    // 验证下载请求
    fn validate_download_request(&self, request: &DownloadRequest) -> Result<(), Error> {
        if request.user_id.is_empty() {
            return Err(Error::InvalidRequest("User ID is required".to_string()));
        }

        if let Some(expires_in) = request.expires_in {
            if expires_in <= 0 || expires_in > 86400 { // 最大24小时
                return Err(Error::InvalidRequest("Invalid expiration time".to_string()));
            }
        }

        Ok(())
    }

    // 验证查询参数
    fn validate_query(&self, query: &MediaQuery) -> Result<(), Error> {
        if query.limit == 0 || query.limit > 100 {
            return Err(Error::InvalidRequest("Invalid limit (1-100)".to_string()));
        }

        if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
            if end < start {
                return Err(Error::InvalidRequest("End time must be after start time".to_string()));
            }
        }

        Ok(())
    }
}

pub struct MediaGrpcService {
    storage_service: Arc<dyn StorageService + Send + Sync>,
}

impl MediaGrpcService {
    pub fn new(storage_service: impl StorageService + Send + Sync + 'static) -> Self {
        Self {
            storage_service: Arc::new(storage_service),
        }
    }

    // 转换请求到领域模型
    fn convert_to_upload_request(&self, request: &InitUploadRequest) -> Result<UploadRequest, Status> {
        Ok(UploadRequest {
            file_name: request.file_name.clone(),
            file_size: request.file_size,
            mime_type: request.mime_type.clone(),
            bucket: request.bucket.clone(),
            user_id: request.user_id.clone(),
            tags: request.tags.clone(),
            custom_data: request.custom_data.clone(),
        })
    }

    // 转换媒体信息到响应
    fn convert_media_to_response(&self, media: &Media) -> proto_crate::api::im::service::media::Media {
        proto_crate::api::im::service::media::Media {
            media_id: media.id.to_string(),
            name: media.name.clone(),
            size: media.size,
            mime_type: media.mime_type.clone(),
            bucket: media.bucket.clone(),
            key: media.key.clone(),
            url: media.url.clone(),
            status: match media.status {
                MediaStatus::Pending => 0,
                MediaStatus::Uploading => 1,
                MediaStatus::Processing => 2,
                MediaStatus::Complete => 3,
                MediaStatus::Failed => 4,
            },
            metadata: Some(proto_crate::api::im::service::media::MediaMetadata {
                width: media.metadata.width.unwrap_or(0),
                height: media.metadata.height.unwrap_or(0),
                duration: media.metadata.duration.unwrap_or(0.0),
                format: media.metadata.format.clone().unwrap_or_default(),
                bitrate: media.metadata.bitrate.unwrap_or(0),
                hash: media.metadata.hash.clone().unwrap_or_default(),
                user_id: media.metadata.user_id.clone(),
                upload_id: media.metadata.upload_id.clone().unwrap_or_default(),
                tags: media.metadata.tags.clone(),
                custom_data: media.metadata.custom_data.clone(),
            }),
            created_at: media.created_at.timestamp() as u64,
            updated_at: media.updated_at.timestamp() as u64,
        }
    }
}

#[tonic::async_trait]
impl Media for MediaGrpcService {
    async fn init_upload(
        &self,
        request: Request<InitUploadRequest>,
    ) -> Result<Response<InitUploadResponse>, Status> {
        let upload_request = self.convert_to_upload_request(request.get_ref())?;
        
        match self.storage_service.init_multipart_upload(upload_request).await {
            Ok(response) => Ok(Response::new(InitUploadResponse {
                media_id: response.media_id.to_string(),
                upload_id: response.upload_id,
                key: response.key,
                urls: response.urls.into_iter().map(|url| {
                    proto_crate::api::im::service::media::PartUploadUrl {
                        part_number: url.part_number,
                        url: url.url,
                        headers: url.headers,
                    }
                }).collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn complete_upload(
        &self,
        request: Request<CompleteUploadRequest>,
    ) -> Result<Response<CompleteUploadResponse>, Status> {
        let req = request.get_ref();
        let media_id = Uuid::parse_str(&req.media_id)
            .map_err(|_| Status::invalid_argument("Invalid media ID"))?;

        let complete_request = CompleteMultipartUpload {
            media_id,
            upload_id: req.upload_id.clone(),
            parts: req.parts.iter().map(|p| PartInfo {
                part_number: p.part_number,
                size: p.size,
                etag: p.etag.clone(),
                last_modified: Utc::now(),
            }).collect(),
        };

        match self.storage_service.complete_multipart_upload(complete_request).await {
            Ok(media) => Ok(Response::new(CompleteUploadResponse {
                media: Some(self.convert_media_to_response(&media)),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn abort_upload(
        &self,
        request: Request<AbortUploadRequest>,
    ) -> Result<Response<AbortUploadResponse>, Status> {
        let req = request.get_ref();
        let media_id = Uuid::parse_str(&req.media_id)
            .map_err(|_| Status::invalid_argument("Invalid media ID"))?;

        match self.storage_service.abort_multipart_upload(media_id, &req.upload_id).await {
            Ok(_) => Ok(Response::new(AbortUploadResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_download_url(
        &self,
        request: Request<GetDownloadUrlRequest>,
    ) -> Result<Response<GetDownloadUrlResponse>, Status> {
        let req = request.get_ref();
        let media_id = Uuid::parse_str(&req.media_id)
            .map_err(|_| Status::invalid_argument("Invalid media ID"))?;

        let download_request = DownloadRequest {
            media_id,
            user_id: req.user_id.clone(),
            expires_in: Some(req.expires_in as i64),
        };

        match self.storage_service.get_download_url(download_request).await {
            Ok(response) => Ok(Response::new(GetDownloadUrlResponse {
                url: response.url,
                expires_at: response.expires_at.timestamp() as u64,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn delete_media(
        &self,
        request: Request<DeleteMediaRequest>,
    ) -> Result<Response<DeleteMediaResponse>, Status> {
        let req = request.get_ref();
        let media_id = Uuid::parse_str(&req.media_id)
            .map_err(|_| Status::invalid_argument("Invalid media ID"))?;

        match self.storage_service.delete_media(media_id).await {
            Ok(_) => Ok(Response::new(DeleteMediaResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_media(
        &self,
        request: Request<GetMediaRequest>,
    ) -> Result<Response<GetMediaResponse>, Status> {
        let req = request.get_ref();
        let media_id = Uuid::parse_str(&req.media_id)
            .map_err(|_| Status::invalid_argument("Invalid media ID"))?;

        match self.storage_service.get_media(media_id).await {
            Ok(Some(media)) => Ok(Response::new(GetMediaResponse {
                media: Some(self.convert_media_to_response(&media)),
            })),
            Ok(None) => Ok(Response::new(GetMediaResponse { media: None })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn query_media(
        &self,
        request: Request<QueryMediaRequest>,
    ) -> Result<Response<QueryMediaResponse>, Status> {
        let req = request.get_ref();
        let query = MediaQuery {
            bucket: req.bucket.clone(),
            user_id: req.user_id.clone(),
            mime_type: req.mime_type.clone(),
            tags: req.tags.clone(),
            status: match req.status {
                0 => Some(MediaStatus::Pending),
                1 => Some(MediaStatus::Uploading),
                2 => Some(MediaStatus::Processing),
                3 => Some(MediaStatus::Complete),
                4 => Some(MediaStatus::Failed),
                _ => None,
            },
            start_time: req.start_time.map(|t| Utc.timestamp_opt(t as i64, 0).unwrap()),
            end_time: req.end_time.map(|t| Utc.timestamp_opt(t as i64, 0).unwrap()),
            limit: req.limit,
            offset: req.offset,
        };

        match self.storage_service.query_media(query).await {
            Ok(media_list) => Ok(Response::new(QueryMediaResponse {
                media: media_list.into_iter()
                    .map(|m| self.convert_media_to_response(&m))
                    .collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_storage_stats(
        &self,
        request: Request<GetStorageStatsRequest>,
    ) -> Result<Response<GetStorageStatsResponse>, Status> {
        let req = request.get_ref();
        match self.storage_service.get_storage_stats(&req.bucket).await {
            Ok(stats) => Ok(Response::new(GetStorageStatsResponse {
                total_size: stats.total_size,
                total_files: stats.total_files,
                total_buckets: stats.total_buckets,
                used_size: stats.used_size,
                used_files: stats.used_files,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
} 