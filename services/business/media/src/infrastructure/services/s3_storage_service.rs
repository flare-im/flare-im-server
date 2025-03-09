use async_trait::async_trait;
use aws_sdk_s3::{
    Client as S3Client,
    config::{Credentials, Region},
    types::{CompletedMultipartUpload, CompletedPart},
};
use chrono::Utc;
use uuid::Uuid;
use std::sync::Arc;
use crate::domain::{
    entities::media::*,
    services::storage_service::{StorageService, Error, StorageStats},
};

pub struct S3StorageService {
    client: S3Client,
    repository: Arc<dyn MediaRepository + Send + Sync>,
    endpoint: String,
    region: String,
}

impl S3StorageService {
    pub async fn new(
        endpoint: String,
        region: String,
        access_key: String,
        secret_key: String,
        repository: Arc<dyn MediaRepository + Send + Sync>,
    ) -> Result<Self, Error> {
        let creds = Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "s3-storage-service",
        );

        let region = Region::new(region.clone());
        let config = aws_sdk_s3::Config::builder()
            .region(region)
            .endpoint_url(endpoint.clone())
            .credentials_provider(creds)
            .build();

        let client = S3Client::from_conf(config);

        Ok(Self {
            client,
            repository,
            endpoint,
            region,
        })
    }

    // 生成对象键
    fn generate_key(&self, file_name: &str, user_id: &str) -> String {
        let extension = std::path::Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        format!("{}/{}.{}", user_id, Uuid::new_v4(), extension)
    }

    // 构建完整URL
    fn build_url(&self, bucket: &str, key: &str) -> String {
        format!("{}/{}/{}", self.endpoint, bucket, key)
    }
}

#[async_trait]
impl StorageService for S3StorageService {
    async fn init_multipart_upload(&self, request: UploadRequest) -> Result<UploadResponse, Error> {
        let key = self.generate_key(&request.file_name, &request.user_id);

        // 创建媒体记录
        let media = Media {
            id: Uuid::new_v4(),
            name: request.file_name.clone(),
            size: request.file_size,
            mime_type: request.mime_type.clone(),
            bucket: request.bucket.clone(),
            key: key.clone(),
            url: self.build_url(&request.bucket, &key),
            status: MediaStatus::Pending,
            metadata: MediaMetadata {
                width: None,
                height: None,
                duration: None,
                format: None,
                bitrate: None,
                hash: None,
                user_id: request.user_id.clone(),
                upload_id: None,
                parts: Vec::new(),
                tags: request.tags.clone(),
                custom_data: request.custom_data.clone(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.repository.save_media(&media).await
            .map_err(|e| Error::Service(e.to_string()))?;

        // 初始化分片上传
        let output = self.client.create_multipart_upload()
            .bucket(&request.bucket)
            .key(&key)
            .content_type(&request.mime_type)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        let upload_id = output.upload_id()
            .ok_or_else(|| Error::Storage("No upload ID returned".to_string()))?
            .to_string();

        // 更新上传ID
        let mut media = media;
        media.status = MediaStatus::Uploading;
        media.metadata.upload_id = Some(upload_id.clone());
        self.repository.save_media(&media).await
            .map_err(|e| Error::Service(e.to_string()))?;

        // 生成分片上传URL
        let chunk_size = 5 * 1024 * 1024; // 5MB
        let total_parts = (request.file_size + chunk_size - 1) / chunk_size;
        let mut urls = Vec::new();

        for part_number in 1..=total_parts as i32 {
            let presigned_url = self.client.generate_presigned_url()
                .bucket(&request.bucket)
                .key(&key)
                .upload_id(&upload_id)
                .part_number(part_number)
                .build()
                .map_err(|e| Error::Storage(e.to_string()))?;

            urls.push(PartUploadUrl {
                part_number,
                url: presigned_url.to_string(),
                headers: std::collections::HashMap::new(),
            });
        }

        Ok(UploadResponse {
            media_id: media.id,
            upload_id,
            key,
            urls,
        })
    }

    async fn complete_multipart_upload(&self, request: CompleteMultipartUpload) -> Result<Media, Error> {
        let media = self.repository.get_media(&request.media_id).await
            .map_err(|e| Error::Service(e.to_string()))?
            .ok_or_else(|| Error::NotFound("Media not found".to_string()))?;

        let completed_parts: Vec<CompletedPart> = request.parts.iter()
            .map(|part| {
                CompletedPart::builder()
                    .e_tag(part.etag.clone())
                    .part_number(part.part_number)
                    .build()
            })
            .collect();

        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client.complete_multipart_upload()
            .bucket(&media.bucket)
            .key(&media.key)
            .upload_id(&request.upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        // 更新媒体状态
        let mut media = media;
        media.status = MediaStatus::Complete;
        media.metadata.parts = request.parts;
        media.updated_at = Utc::now();

        self.repository.save_media(&media).await
            .map_err(|e| Error::Service(e.to_string()))?;

        Ok(media)
    }

    async fn abort_multipart_upload(&self, media_id: Uuid, upload_id: &str) -> Result<(), Error> {
        let media = self.repository.get_media(&media_id).await
            .map_err(|e| Error::Service(e.to_string()))?
            .ok_or_else(|| Error::NotFound("Media not found".to_string()))?;

        self.client.abort_multipart_upload()
            .bucket(&media.bucket)
            .key(&media.key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        // 更新媒体状态
        let mut media = media;
        media.status = MediaStatus::Failed;
        media.updated_at = Utc::now();

        self.repository.save_media(&media).await
            .map_err(|e| Error::Service(e.to_string()))?;

        Ok(())
    }

    async fn get_download_url(&self, request: DownloadRequest) -> Result<DownloadResponse, Error> {
        let media = self.repository.get_media(&request.media_id).await
            .map_err(|e| Error::Service(e.to_string()))?
            .ok_or_else(|| Error::NotFound("Media not found".to_string()))?;

        // 检查权限
        if media.metadata.user_id != request.user_id {
            return Err(Error::PermissionDenied("No permission to access this media".to_string()));
        }

        let expires_in = request.expires_in.unwrap_or(3600); // 默认1小时
        let presigned_url = self.client.generate_presigned_url()
            .bucket(&media.bucket)
            .key(&media.key)
            .expires_in(std::time::Duration::from_secs(expires_in as u64))
            .build()
            .map_err(|e| Error::Storage(e.to_string()))?;

        Ok(DownloadResponse {
            url: presigned_url.to_string(),
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in),
        })
    }

    async fn delete_media(&self, media_id: Uuid) -> Result<(), Error> {
        let media = self.repository.get_media(&media_id).await
            .map_err(|e| Error::Service(e.to_string()))?
            .ok_or_else(|| Error::NotFound("Media not found".to_string()))?;

        self.client.delete_object()
            .bucket(&media.bucket)
            .key(&media.key)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        self.repository.delete_media(&media_id).await
            .map_err(|e| Error::Service(e.to_string()))?;

        Ok(())
    }

    async fn get_media(&self, media_id: Uuid) -> Result<Option<Media>, Error> {
        self.repository.get_media(&media_id).await
            .map_err(|e| Error::Service(e.to_string()))
    }

    async fn query_media(&self, query: MediaQuery) -> Result<Vec<Media>, Error> {
        self.repository.query_media(query).await
            .map_err(|e| Error::Service(e.to_string()))
    }

    async fn get_storage_stats(&self, bucket: &str) -> Result<StorageStats, Error> {
        let objects = self.client.list_objects_v2()
            .bucket(bucket)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        let mut total_size = 0;
        let mut total_files = 0;

        if let Some(contents) = objects.contents() {
            for object in contents {
                total_size += object.size() as u64;
                total_files += 1;
            }
        }

        Ok(StorageStats {
            total_size,
            total_files,
            total_buckets: 1,
            used_size: total_size,
            used_files: total_files,
        })
    }
}

#[async_trait]
pub trait MediaRepository {
    async fn save_media(&self, media: &Media) -> Result<(), Error>;
    async fn delete_media(&self, media_id: &Uuid) -> Result<(), Error>;
    async fn get_media(&self, media_id: &Uuid) -> Result<Option<Media>, Error>;
    async fn query_media(&self, query: MediaQuery) -> Result<Vec<Media>, Error>;
} 