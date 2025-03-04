use std::sync::Arc;
use aws_sdk_s3::Client as S3Client;

pub mod media {
    use aws_sdk_s3::primitives::ByteStream;
    use bytes::Bytes;
    use futures::Stream;
    use tonic::{Request, Response, Status};
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
