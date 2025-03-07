use anyhow::Result;
use aws_sdk_s3::{Client, Endpoint};
use aws_types::region::Region;
use aws_credential_types::Credentials;
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;

use crate::config::S3Config;

pub async fn init_s3_client(config: &S3Config) -> Result<Client> {
    let creds = Credentials::new(
        &config.access_key_id,
        &config.secret_access_key,
        None,
        None,
        "static",
    );

    let endpoint = Endpoint::immutable(config.endpoint.parse()?);
    let region = Region::new(config.region.clone());

    let s3_config = aws_sdk_s3::Config::builder()
        .endpoint_resolver(endpoint)
        .region(Some(region))
        .credentials_provider(creds)
        .http_client(HyperClientBuilder::new().build())
        .force_path_style(true)
        .build();

    Ok(Client::from_conf(s3_config))
}

pub async fn upload_file(
    client: &Client,
    bucket: &str,
    key: &str,
    content_type: &str,
    data: bytes::Bytes,
) -> Result<String> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(data.into())
        .content_type(content_type)
        .send()
        .await?;

    Ok(format!("/{}/{}", bucket, key))
}

pub async fn download_file(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<bytes::Bytes> {
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let data = resp.body.collect().await?;
    Ok(data.into_bytes())
}

pub async fn delete_file(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    Ok(())
} 