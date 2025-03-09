use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::Utc;
use crate::{
    domain::entities::notification::*,
    infrastructure::services::notification_service_impl::{
        DeviceRepository, TemplateRepository, NotificationRepository,
    },
    domain::services::notification_service::{Error as ServiceError, PlatformStatistics},
};

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // 创建数据库表
    pub async fn create_tables(&self) -> Result<(), ServiceError> {
        // 创建设备表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS devices (
                user_id VARCHAR(255) NOT NULL,
                device_id VARCHAR(255) NOT NULL,
                platform VARCHAR(50) NOT NULL,
                push_token TEXT NOT NULL,
                app_version VARCHAR(50) NOT NULL,
                provider VARCHAR(50) NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT true,
                last_active_at TIMESTAMP WITH TIME ZONE NOT NULL,
                PRIMARY KEY (user_id, device_id)
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        // 创建模板表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS notification_templates (
                id UUID PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                title_template TEXT NOT NULL,
                content_template TEXT NOT NULL,
                category VARCHAR(100) NOT NULL,
                platform JSONB NOT NULL,
                metadata JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        // 创建通知表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS notifications (
                id UUID PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                notification_type VARCHAR(50) NOT NULL,
                priority VARCHAR(20) NOT NULL,
                target_type VARCHAR(50) NOT NULL,
                target_users JSONB NOT NULL,
                platform JSONB NOT NULL,
                status VARCHAR(50) NOT NULL,
                metadata JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
                scheduled_at TIMESTAMP WITH TIME ZONE,
                expired_at TIMESTAMP WITH TIME ZONE
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        // 创建通知结果表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS notification_results (
                notification_id UUID PRIMARY KEY,
                success BOOLEAN NOT NULL,
                platform_results JSONB NOT NULL,
                sent_count INTEGER NOT NULL,
                failed_count INTEGER NOT NULL,
                error TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl DeviceRepository for PostgresRepository {
    async fn save_device(&self, device: DeviceInfo) -> Result<(), ServiceError> {
        sqlx::query(r#"
            INSERT INTO devices (
                user_id, device_id, platform, push_token, app_version,
                provider, is_active, last_active_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id, device_id) DO UPDATE
            SET platform = $3,
                push_token = $4,
                app_version = $5,
                provider = $6,
                is_active = $7,
                last_active_at = $8
        "#)
        .bind(&device.user_id)
        .bind(&device.device_id)
        .bind(format!("{:?}", device.platform))
        .bind(&device.push_token)
        .bind(&device.app_version)
        .bind(&device.provider)
        .bind(device.is_active)
        .bind(device.last_active_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        Ok(())
    }

    async fn delete_device(&self, user_id: &str, device_id: &str) -> Result<(), ServiceError> {
        sqlx::query("DELETE FROM devices WHERE user_id = $1 AND device_id = $2")
            .bind(user_id)
            .bind(device_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))?;

        Ok(())
    }

    async fn update_device_token(&self, user_id: &str, device_id: &str, new_token: &str) -> Result<(), ServiceError> {
        sqlx::query(r#"
            UPDATE devices
            SET push_token = $3,
                last_active_at = $4
            WHERE user_id = $1 AND device_id = $2
        "#)
        .bind(user_id)
        .bind(device_id)
        .bind(new_token)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        Ok(())
    }

    async fn get_user_devices(&self, user_id: &str) -> Result<Vec<DeviceInfo>, ServiceError> {
        let rows = sqlx::query(r#"
            SELECT user_id, device_id, platform, push_token, app_version,
                   provider, is_active, last_active_at
            FROM devices
            WHERE user_id = $1 AND is_active = true
        "#)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(DeviceInfo {
                user_id: row.get("user_id"),
                device_id: row.get("device_id"),
                platform: serde_json::from_value(row.get("platform"))
                    .map_err(|e| ServiceError::Service(e.to_string()))?,
                push_token: row.get("push_token"),
                app_version: row.get("app_version"),
                provider: row.get("provider"),
                is_active: row.get("is_active"),
                last_active_at: row.get("last_active_at"),
            });
        }

        Ok(devices)
    }
}

#[async_trait]
impl TemplateRepository for PostgresRepository {
    async fn save_template(&self, template: NotificationTemplate) -> Result<NotificationTemplate, ServiceError> {
        sqlx::query(r#"
            INSERT INTO notification_templates (
                id, name, title_template, content_template, category,
                platform, metadata, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE
            SET name = $2,
                title_template = $3,
                content_template = $4,
                category = $5,
                platform = $6,
                metadata = $7,
                updated_at = $9
            RETURNING *
        "#)
        .bind(template.id)
        .bind(&template.name)
        .bind(&template.title_template)
        .bind(&template.content_template)
        .bind(&template.category)
        .bind(serde_json::to_value(&template.platform).unwrap())
        .bind(serde_json::to_value(&template.metadata).unwrap())
        .bind(template.created_at)
        .bind(template.updated_at)
        .fetch_one(&self.pool)
        .await
        .map(|row| NotificationTemplate {
            id: row.get("id"),
            name: row.get("name"),
            title_template: row.get("title_template"),
            content_template: row.get("content_template"),
            category: row.get("category"),
            platform: serde_json::from_value(row.get("platform")).unwrap(),
            metadata: serde_json::from_value(row.get("metadata")).unwrap(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn delete_template(&self, template_id: &str) -> Result<(), ServiceError> {
        let uuid = Uuid::parse_str(template_id)
            .map_err(|e| ServiceError::InvalidRequest(e.to_string()))?;

        sqlx::query("DELETE FROM notification_templates WHERE id = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Service(e.to_string()))?;

        Ok(())
    }

    async fn get_template(&self, template_id: &str) -> Result<Option<NotificationTemplate>, ServiceError> {
        let uuid = Uuid::parse_str(template_id)
            .map_err(|e| ServiceError::InvalidRequest(e.to_string()))?;

        sqlx::query(r#"
            SELECT id, name, title_template, content_template, category,
                   platform, metadata, created_at, updated_at
            FROM notification_templates
            WHERE id = $1
        "#)
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await
        .map(|row_opt| {
            row_opt.map(|row| NotificationTemplate {
                id: row.get("id"),
                name: row.get("name"),
                title_template: row.get("title_template"),
                content_template: row.get("content_template"),
                category: row.get("category"),
                platform: serde_json::from_value(row.get("platform")).unwrap(),
                metadata: serde_json::from_value(row.get("metadata")).unwrap(),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
        })
        .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn get_templates_by_category(&self, category: &str) -> Result<Vec<NotificationTemplate>, ServiceError> {
        sqlx::query(r#"
            SELECT id, name, title_template, content_template, category,
                   platform, metadata, created_at, updated_at
            FROM notification_templates
            WHERE category = $1
        "#)
        .bind(category)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| NotificationTemplate {
                    id: row.get("id"),
                    name: row.get("name"),
                    title_template: row.get("title_template"),
                    content_template: row.get("content_template"),
                    category: row.get("category"),
                    platform: serde_json::from_value(row.get("platform")).unwrap(),
                    metadata: serde_json::from_value(row.get("metadata")).unwrap(),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
                .collect()
        })
        .map_err(|e| ServiceError::Service(e.to_string()))
    }
}

#[async_trait]
impl NotificationRepository for PostgresRepository {
    async fn save_notification(&self, notification: &Notification) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| ServiceError::Service(e.to_string()))?;

        // 保存通知
        sqlx::query(r#"
            INSERT INTO notifications (
                id, title, content, notification_type, priority,
                target_type, target_users, platform, status, metadata,
                created_at, updated_at, scheduled_at, expired_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (id) DO UPDATE
            SET title = $2,
                content = $3,
                notification_type = $4,
                priority = $5,
                target_type = $6,
                target_users = $7,
                platform = $8,
                status = $9,
                metadata = $10,
                updated_at = $12,
                scheduled_at = $13,
                expired_at = $14
        "#)
        .bind(notification.id)
        .bind(&notification.title)
        .bind(&notification.content)
        .bind(format!("{:?}", notification.notification_type))
        .bind(format!("{:?}", notification.priority))
        .bind(format!("{:?}", notification.target_type))
        .bind(serde_json::to_value(&notification.target_users).unwrap())
        .bind(serde_json::to_value(&notification.platform).unwrap())
        .bind(format!("{:?}", notification.status))
        .bind(serde_json::to_value(&notification.metadata).unwrap())
        .bind(notification.created_at)
        .bind(notification.updated_at)
        .bind(notification.scheduled_at)
        .bind(notification.expired_at)
        .execute(&mut tx)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        tx.commit().await
            .map_err(|e| ServiceError::Service(e.to_string()))?;

        Ok(())
    }

    async fn get_notification(&self, notification_id: &str) -> Result<Option<NotificationResult>, ServiceError> {
        let uuid = Uuid::parse_str(notification_id)
            .map_err(|e| ServiceError::InvalidRequest(e.to_string()))?;

        sqlx::query(r#"
            SELECT n.*, r.success, r.platform_results, r.sent_count,
                   r.failed_count, r.error
            FROM notifications n
            LEFT JOIN notification_results r ON n.id = r.notification_id
            WHERE n.id = $1
        "#)
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await
        .map(|row_opt| {
            row_opt.map(|row| NotificationResult {
                notification_id: row.get("id"),
                success: row.get("success"),
                platform_results: serde_json::from_value(row.get("platform_results")).unwrap(),
                sent_count: row.get("sent_count"),
                failed_count: row.get("failed_count"),
                error: row.get("error"),
            })
        })
        .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn update_notification_status(
        &self,
        notification_id: &str,
        status: NotificationStatus,
    ) -> Result<(), ServiceError> {
        let uuid = Uuid::parse_str(notification_id)
            .map_err(|e| ServiceError::InvalidRequest(e.to_string()))?;

        sqlx::query(r#"
            UPDATE notifications
            SET status = $2,
                updated_at = $3
            WHERE id = $1
        "#)
        .bind(uuid)
        .bind(format!("{:?}", status))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        Ok(())
    }

    async fn get_user_notifications(
        &self,
        user_id: &str,
        notification_type: Option<NotificationType>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Notification>, ServiceError> {
        let query = match notification_type {
            Some(nt) => {
                sqlx::query(r#"
                    SELECT *
                    FROM notifications
                    WHERE target_users ? $1
                    AND notification_type = $2
                    ORDER BY created_at DESC
                    LIMIT $3 OFFSET $4
                "#)
                .bind(user_id)
                .bind(format!("{:?}", nt))
                .bind(limit as i64)
                .bind(offset as i64)
            },
            None => {
                sqlx::query(r#"
                    SELECT *
                    FROM notifications
                    WHERE target_users ? $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                "#)
                .bind(user_id)
                .bind(limit as i64)
                .bind(offset as i64)
            }
        };

        query.fetch_all(&self.pool)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| Notification {
                        id: row.get("id"),
                        title: row.get("title"),
                        content: row.get("content"),
                        notification_type: serde_json::from_value(row.get("notification_type")).unwrap(),
                        priority: serde_json::from_value(row.get("priority")).unwrap(),
                        target_type: serde_json::from_value(row.get("target_type")).unwrap(),
                        target_users: serde_json::from_value(row.get("target_users")).unwrap(),
                        platform: serde_json::from_value(row.get("platform")).unwrap(),
                        status: serde_json::from_value(row.get("status")).unwrap(),
                        metadata: serde_json::from_value(row.get("metadata")).unwrap(),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                        scheduled_at: row.get("scheduled_at"),
                        expired_at: row.get("expired_at"),
                    })
                    .collect()
            })
            .map_err(|e| ServiceError::Service(e.to_string()))
    }

    async fn get_platform_statistics(
        &self,
        platform: Platform,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<PlatformStatistics, ServiceError> {
        let rows = sqlx::query(r#"
            SELECT COUNT(*) as total,
                   SUM(CASE WHEN success THEN 1 ELSE 0 END) as success_count,
                   SUM(CASE WHEN NOT success THEN 1 ELSE 0 END) as failed_count,
                   AVG(EXTRACT(EPOCH FROM (updated_at - created_at))) as avg_latency,
                   jsonb_object_agg(COALESCE(error, 'none'), COUNT(*)) as error_counts
            FROM notification_results r
            JOIN notifications n ON r.notification_id = n.id
            WHERE n.platform ? $1
            AND n.created_at BETWEEN $2 AND $3
            GROUP BY n.platform
        "#)
        .bind(format!("{:?}", platform))
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServiceError::Service(e.to_string()))?;

        if rows.is_empty() {
            return Ok(PlatformStatistics {
                total_sent: 0,
                total_failed: 0,
                success_rate: 0.0,
                average_latency: 0.0,
                error_counts: std::collections::HashMap::new(),
            });
        }

        let row = &rows[0];
        let total: i64 = row.get("total");
        let success_count: i64 = row.get("success_count");
        let failed_count: i64 = row.get("failed_count");
        let avg_latency: f64 = row.get("avg_latency");
        let error_counts: serde_json::Value = row.get("error_counts");

        Ok(PlatformStatistics {
            total_sent: success_count as u64,
            total_failed: failed_count as u64,
            success_rate: if total > 0 {
                success_count as f64 / total as f64
            } else {
                0.0
            },
            average_latency: avg_latency,
            error_counts: serde_json::from_value(error_counts)
                .unwrap_or_else(|_| std::collections::HashMap::new()),
        })
    }
} 