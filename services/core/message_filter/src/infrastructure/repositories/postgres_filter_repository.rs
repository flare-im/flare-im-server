use async_trait::async_trait;
use sqlx::{PgPool, Row};
use crate::domain::{
    entities::filter::{FilterRule, RuleType, FilterAction, RuleMetadata},
    repositories::filter_repository::{FilterRepository, Error as RepoError},
};
use chrono::Utc;
use uuid::Uuid;

pub struct PostgresFilterRepository {
    pool: PgPool,
}

impl PostgresFilterRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // 构建规则表 SQL
    async fn create_tables(&self) -> Result<(), RepoError> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS filter_rules (
                id UUID PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                rule_type VARCHAR(50) NOT NULL,
                pattern TEXT NOT NULL,
                action VARCHAR(50) NOT NULL,
                priority INTEGER NOT NULL,
                is_enabled BOOLEAN NOT NULL DEFAULT true,
                description TEXT,
                category VARCHAR(100),
                replacement TEXT,
                custom_config JSONB,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    // 转换规则类型
    fn convert_rule_type(rule_type: &str) -> RuleType {
        match rule_type {
            "Keyword" => RuleType::Keyword,
            "Regex" => RuleType::Regex,
            "Dictionary" => RuleType::Dictionary,
            "ImageHash" => RuleType::ImageHash,
            "MediaType" => RuleType::MediaType,
            _ => RuleType::Custom,
        }
    }

    // 转换过滤动作
    fn convert_action(action: &str) -> FilterAction {
        match action {
            "Block" => FilterAction::Block,
            "Replace" => FilterAction::Replace,
            "Warn" => FilterAction::Warn,
            "Log" => FilterAction::Log,
            _ => FilterAction::Review,
        }
    }
}

#[async_trait]
impl FilterRepository for PostgresFilterRepository {
    async fn save_rule(&self, rule: FilterRule) -> Result<(), RepoError> {
        sqlx::query(r#"
            INSERT INTO filter_rules (
                id, name, rule_type, pattern, action, priority, is_enabled,
                description, category, replacement, custom_config,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO UPDATE
            SET name = $2, rule_type = $3, pattern = $4, action = $5,
                priority = $6, is_enabled = $7, description = $8,
                category = $9, replacement = $10, custom_config = $11,
                updated_at = $13
        "#)
        .bind(rule.id)
        .bind(&rule.name)
        .bind(format!("{:?}", rule.rule_type))
        .bind(&rule.pattern)
        .bind(format!("{:?}", rule.action))
        .bind(rule.priority)
        .bind(rule.is_enabled)
        .bind(&rule.metadata.description)
        .bind(&rule.metadata.category)
        .bind(&rule.metadata.replacement)
        .bind(serde_json::to_value(&rule.metadata.custom_config).unwrap_or_default())
        .bind(rule.created_at)
        .bind(rule.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn get_rule(&self, rule_id: &str) -> Result<Option<FilterRule>, RepoError> {
        let uuid = Uuid::parse_str(rule_id)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;

        let row = sqlx::query(r#"
            SELECT id, name, rule_type, pattern, action, priority, is_enabled,
                   description, category, replacement, custom_config,
                   created_at, updated_at
            FROM filter_rules
            WHERE id = $1
        "#)
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        match row {
            Some(row) => {
                let custom_config: serde_json::Value = row.get("custom_config");
                Ok(Some(FilterRule {
                    id: row.get("id"),
                    name: row.get("name"),
                    rule_type: Self::convert_rule_type(row.get("rule_type")),
                    pattern: row.get("pattern"),
                    action: Self::convert_action(row.get("action")),
                    priority: row.get("priority"),
                    is_enabled: row.get("is_enabled"),
                    metadata: RuleMetadata {
                        description: row.get("description"),
                        category: row.get("category"),
                        replacement: row.get("replacement"),
                        custom_config: serde_json::from_value(custom_config)
                            .unwrap_or_default(),
                    },
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }))
            }
            None => Ok(None),
        }
    }

    async fn delete_rule(&self, rule_id: &str) -> Result<(), RepoError> {
        let uuid = Uuid::parse_str(rule_id)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;

        sqlx::query("DELETE FROM filter_rules WHERE id = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn get_rules_by_type(&self, rule_type: RuleType) -> Result<Vec<FilterRule>, RepoError> {
        let rows = sqlx::query(r#"
            SELECT id, name, rule_type, pattern, action, priority, is_enabled,
                   description, category, replacement, custom_config,
                   created_at, updated_at
            FROM filter_rules
            WHERE rule_type = $1
            ORDER BY priority DESC
        "#)
        .bind(format!("{:?}", rule_type))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        let mut rules = Vec::new();
        for row in rows {
            let custom_config: serde_json::Value = row.get("custom_config");
            rules.push(FilterRule {
                id: row.get("id"),
                name: row.get("name"),
                rule_type: Self::convert_rule_type(row.get("rule_type")),
                pattern: row.get("pattern"),
                action: Self::convert_action(row.get("action")),
                priority: row.get("priority"),
                is_enabled: row.get("is_enabled"),
                metadata: RuleMetadata {
                    description: row.get("description"),
                    category: row.get("category"),
                    replacement: row.get("replacement"),
                    custom_config: serde_json::from_value(custom_config)
                        .unwrap_or_default(),
                },
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(rules)
    }

    async fn get_enabled_rules(&self) -> Result<Vec<FilterRule>, RepoError> {
        let rows = sqlx::query(r#"
            SELECT id, name, rule_type, pattern, action, priority, is_enabled,
                   description, category, replacement, custom_config,
                   created_at, updated_at
            FROM filter_rules
            WHERE is_enabled = true
            ORDER BY priority DESC
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        let mut rules = Vec::new();
        for row in rows {
            let custom_config: serde_json::Value = row.get("custom_config");
            rules.push(FilterRule {
                id: row.get("id"),
                name: row.get("name"),
                rule_type: Self::convert_rule_type(row.get("rule_type")),
                pattern: row.get("pattern"),
                action: Self::convert_action(row.get("action")),
                priority: row.get("priority"),
                is_enabled: row.get("is_enabled"),
                metadata: RuleMetadata {
                    description: row.get("description"),
                    category: row.get("category"),
                    replacement: row.get("replacement"),
                    custom_config: serde_json::from_value(custom_config)
                        .unwrap_or_default(),
                },
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(rules)
    }

    async fn get_rules_by_category(&self, category: &str) -> Result<Vec<FilterRule>, RepoError> {
        let rows = sqlx::query(r#"
            SELECT id, name, rule_type, pattern, action, priority, is_enabled,
                   description, category, replacement, custom_config,
                   created_at, updated_at
            FROM filter_rules
            WHERE category = $1
            ORDER BY priority DESC
        "#)
        .bind(category)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        let mut rules = Vec::new();
        for row in rows {
            let custom_config: serde_json::Value = row.get("custom_config");
            rules.push(FilterRule {
                id: row.get("id"),
                name: row.get("name"),
                rule_type: Self::convert_rule_type(row.get("rule_type")),
                pattern: row.get("pattern"),
                action: Self::convert_action(row.get("action")),
                priority: row.get("priority"),
                is_enabled: row.get("is_enabled"),
                metadata: RuleMetadata {
                    description: row.get("description"),
                    category: row.get("category"),
                    replacement: row.get("replacement"),
                    custom_config: serde_json::from_value(custom_config)
                        .unwrap_or_default(),
                },
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(rules)
    }

    async fn batch_save_rules(&self, rules: Vec<FilterRule>) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        for rule in rules {
            sqlx::query(r#"
                INSERT INTO filter_rules (
                    id, name, rule_type, pattern, action, priority, is_enabled,
                    description, category, replacement, custom_config,
                    created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ON CONFLICT (id) DO UPDATE
                SET name = $2, rule_type = $3, pattern = $4, action = $5,
                    priority = $6, is_enabled = $7, description = $8,
                    category = $9, replacement = $10, custom_config = $11,
                    updated_at = $13
            "#)
            .bind(rule.id)
            .bind(&rule.name)
            .bind(format!("{:?}", rule.rule_type))
            .bind(&rule.pattern)
            .bind(format!("{:?}", rule.action))
            .bind(rule.priority)
            .bind(rule.is_enabled)
            .bind(&rule.metadata.description)
            .bind(&rule.metadata.category)
            .bind(&rule.metadata.replacement)
            .bind(serde_json::to_value(&rule.metadata.custom_config).unwrap_or_default())
            .bind(rule.created_at)
            .bind(rule.updated_at)
            .execute(&mut tx)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        }

        tx.commit().await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn batch_delete_rules(&self, rule_ids: Vec<String>) -> Result<(), RepoError> {
        let uuids: Result<Vec<Uuid>, _> = rule_ids.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();

        let uuids = uuids.map_err(|e| RepoError::InvalidData(e.to_string()))?;

        sqlx::query("DELETE FROM filter_rules WHERE id = ANY($1)")
            .bind(&uuids)
            .execute(&self.pool)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn enable_rule(&self, rule_id: &str) -> Result<(), RepoError> {
        let uuid = Uuid::parse_str(rule_id)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;

        sqlx::query(r#"
            UPDATE filter_rules
            SET is_enabled = true,
                updated_at = $2
            WHERE id = $1
        "#)
        .bind(uuid)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn disable_rule(&self, rule_id: &str) -> Result<(), RepoError> {
        let uuid = Uuid::parse_str(rule_id)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;

        sqlx::query(r#"
            UPDATE filter_rules
            SET is_enabled = false,
                updated_at = $2
            WHERE id = $1
        "#)
        .bind(uuid)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn count_rules(&self) -> Result<u64, RepoError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM filter_rules")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(row.get::<i64, _>("count") as u64)
    }

    async fn count_rules_by_type(&self, rule_type: RuleType) -> Result<u64, RepoError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM filter_rules WHERE rule_type = $1")
            .bind(format!("{:?}", rule_type))
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(row.get::<i64, _>("count") as u64)
    }
} 