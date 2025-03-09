use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::{
    domain::entities::media::*,
    infrastructure::services::s3_storage_service::{MediaRepository, Error},
};

pub struct PostgresMediaRepository {
    pool: PgPool,
}

impl PostgresMediaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_tables(&self) -> Result<(), Error> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS media (
                id UUID PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                size BIGINT NOT NULL,
                mime_type VARCHAR(100) NOT NULL,
                bucket VARCHAR(100) NOT NULL,
                key VARCHAR(255) NOT NULL,
                url TEXT NOT NULL,
                status VARCHAR(50) NOT NULL,
                metadata JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Service(e.to_string()))?;

        // 创建索引
        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_media_user_id ON media ((metadata->>'user_id'));
            CREATE INDEX IF NOT EXISTS idx_media_bucket ON media (bucket);
            CREATE INDEX IF NOT EXISTS idx_media_status ON media (status);
            CREATE INDEX IF NOT EXISTS idx_media_created_at ON media (created_at);
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Service(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl MediaRepository for PostgresMediaRepository {
    async fn save_media(&self, media: &Media) -> Result<(), Error> {
        sqlx::query(r#"
            INSERT INTO media (
                id, name, size, mime_type, bucket, key, url,
                status, metadata, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE
            SET name = $2,
                size = $3,
                mime_type = $4,
                bucket = $5,
                key = $6,
                url = $7,
                status = $8,
                metadata = $9,
                updated_at = $11
        "#)
        .bind(media.id)
        .bind(&media.name)
        .bind(media.size as i64)
        .bind(&media.mime_type)
        .bind(&media.bucket)
        .bind(&media.key)
        .bind(&media.url)
        .bind(format!("{:?}", media.status))
        .bind(serde_json::to_value(&media.metadata).unwrap())
        .bind(media.created_at)
        .bind(media.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Service(e.to_string()))?;

        Ok(())
    }

    async fn delete_media(&self, media_id: &Uuid) -> Result<(), Error> {
        sqlx::query("DELETE FROM media WHERE id = $1")
            .bind(media_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Service(e.to_string()))?;

        Ok(())
    }

    async fn get_media(&self, media_id: &Uuid) -> Result<Option<Media>, Error> {
        let row = sqlx::query(r#"
            SELECT id, name, size, mime_type, bucket, key, url,
                   status, metadata, created_at, updated_at
            FROM media
            WHERE id = $1
        "#)
        .bind(media_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Service(e.to_string()))?;

        Ok(row.map(|row| Media {
            id: row.get("id"),
            name: row.get("name"),
            size: row.get::<i64, _>("size") as u64,
            mime_type: row.get("mime_type"),
            bucket: row.get("bucket"),
            key: row.get("key"),
            url: row.get("url"),
            status: serde_json::from_value(row.get("status")).unwrap(),
            metadata: serde_json::from_value(row.get("metadata")).unwrap(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    async fn query_media(&self, query: MediaQuery) -> Result<Vec<Media>, Error> {
        let mut sql = String::from(r#"
            SELECT id, name, size, mime_type, bucket, key, url,
                   status, metadata, created_at, updated_at
            FROM media
            WHERE 1=1
        "#);

        let mut params: Vec<String> = Vec::new();
        let mut param_count = 1;

        if let Some(bucket) = query.bucket {
            sql.push_str(&format!(" AND bucket = ${}", param_count));
            params.push(bucket);
            param_count += 1;
        }

        if let Some(user_id) = query.user_id {
            sql.push_str(&format!(" AND metadata->>'user_id' = ${}", param_count));
            params.push(user_id);
            param_count += 1;
        }

        if let Some(mime_type) = query.mime_type {
            sql.push_str(&format!(" AND mime_type = ${}", param_count));
            params.push(mime_type);
            param_count += 1;
        }

        if let Some(status) = query.status {
            sql.push_str(&format!(" AND status = ${}", param_count));
            params.push(format!("{:?}", status));
            param_count += 1;
        }

        if let Some(start_time) = query.start_time {
            sql.push_str(&format!(" AND created_at >= ${}", param_count));
            params.push(start_time.to_rfc3339());
            param_count += 1;
        }

        if let Some(end_time) = query.end_time {
            sql.push_str(&format!(" AND created_at <= ${}", param_count));
            params.push(end_time.to_rfc3339());
            param_count += 1;
        }

        if let Some(tags) = query.tags {
            sql.push_str(&format!(" AND metadata->'tags' ?| ${}", param_count));
            params.push(serde_json::to_string(&tags).unwrap());
            param_count += 1;
        }

        sql.push_str(" ORDER BY created_at DESC");
        sql.push_str(&format!(" LIMIT ${} OFFSET ${}", param_count, param_count + 1));

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        query_builder = query_builder.bind(query.limit as i64);
        query_builder = query_builder.bind(query.offset as i64);

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Service(e.to_string()))?;

        let media = rows.into_iter()
            .map(|row| Media {
                id: row.get("id"),
                name: row.get("name"),
                size: row.get::<i64, _>("size") as u64,
                mime_type: row.get("mime_type"),
                bucket: row.get("bucket"),
                key: row.get("key"),
                url: row.get("url"),
                status: serde_json::from_value(row.get("status")).unwrap(),
                metadata: serde_json::from_value(row.get("metadata")).unwrap(),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(media)
    }
} 