use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: Uuid,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub bucket: String,
    pub key: String,
    pub url: String,
    pub status: MediaStatus,
    pub metadata: MediaMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<f64>,
    pub format: Option<String>,
    pub bitrate: Option<u32>,
    pub hash: Option<String>,
    pub user_id: String,
    pub upload_id: Option<String>,
    pub parts: Vec<PartInfo>,
    pub tags: Vec<String>,
    pub custom_data: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartInfo {
    pub part_number: i32,
    pub size: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaStatus {
    Pending,    // 等待上传
    Uploading,  // 上传中
    Processing, // 处理中
    Complete,   // 完成
    Failed,     // 失败
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequest {
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub bucket: String,
    pub user_id: String,
    pub tags: Vec<String>,
    pub custom_data: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub media_id: Uuid,
    pub upload_id: String,
    pub key: String,
    pub urls: Vec<PartUploadUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartUploadUrl {
    pub part_number: i32,
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteMultipartUpload {
    pub media_id: Uuid,
    pub upload_id: String,
    pub parts: Vec<PartInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub media_id: Uuid,
    pub user_id: String,
    pub expires_in: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResponse {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaQuery {
    pub bucket: Option<String>,
    pub user_id: Option<String>,
    pub mime_type: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<MediaStatus>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: u32,
    pub offset: u32,
} 