//! PostgreSQL 系统日志存储实现

use crate::error::StorageError;
use crate::models::{LogCategory, LogLevel, SystemLogQuery, SystemLogRecord, UnreadStats};
use crate::traits::SystemLogStore;
use domain::TenantContext;
use sqlx::{PgPool, Row};

/// PostgreSQL 系统日志存储
pub struct PgSystemLogStore {
    pub pool: PgPool,
}

impl PgSystemLogStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl SystemLogStore for PgSystemLogStore {
    async fn create_system_log(
        &self,
        ctx: &TenantContext,
        record: SystemLogRecord,
    ) -> Result<SystemLogRecord, StorageError> {
        // 验证租户匹配
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }

        sqlx::query(
            "INSERT INTO system_logs \
             (log_id, tenant_id, project_id, category, level, title, message, \
              source, resource, actor, metadata, is_read, created_at, read_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, \
              to_timestamp($13 / 1000.0), NULL)",
        )
        .bind(&record.log_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(record.category.as_str())
        .bind(record.level.as_str())
        .bind(&record.title)
        .bind(&record.message)
        .bind(&record.source)
        .bind(&record.resource)
        .bind(&record.actor)
        .bind(&record.metadata)
        .bind(record.is_read)
        .bind(record.created_at_ms as f64)
        .execute(&self.pool)
        .await?;

        Ok(record)
    }

    async fn list_system_logs(
        &self,
        ctx: &TenantContext,
        query: SystemLogQuery,
    ) -> Result<Vec<SystemLogRecord>, StorageError> {
        // 构建动态查询条件
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut param_idx = 2;

        if query.category.is_some() {
            conditions.push(format!("category = ${}", param_idx));
            param_idx += 1;
        }

        if query.level.is_some() {
            conditions.push(format!("level = ${}", param_idx));
            param_idx += 1;
        }

        if query.unread_only {
            conditions.push(format!("is_read = ${}", param_idx));
            param_idx += 1;
        }

        if query.from_ms.is_some() {
            conditions.push(format!(
                "created_at >= to_timestamp(${} / 1000.0)",
                param_idx
            ));
            param_idx += 1;
        }

        if query.to_ms.is_some() {
            conditions.push(format!(
                "created_at <= to_timestamp(${} / 1000.0)",
                param_idx
            ));
            param_idx += 1;
        }

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT log_id, tenant_id, project_id, category, level, title, message, \
                    source, resource, actor, metadata, is_read, \
                    (extract(epoch from created_at) * 1000)::bigint as created_at_ms, \
                    (extract(epoch from read_at) * 1000)::bigint as read_at_ms \
             FROM system_logs \
             WHERE {} \
             ORDER BY created_at DESC \
             LIMIT ${}",
            where_clause, param_idx
        );

        let mut q = sqlx::query(&sql).bind(&ctx.tenant_id);

        if let Some(cat) = query.category {
            q = q.bind(cat.as_str());
        }
        if let Some(lvl) = query.level {
            q = q.bind(lvl.as_str());
        }
        if query.unread_only {
            q = q.bind(false);
        }
        if let Some(from_ms) = query.from_ms {
            q = q.bind(from_ms as f64);
        }
        if let Some(to_ms) = query.to_ms {
            q = q.bind(to_ms as f64);
        }

        let rows = q.bind(query.limit).fetch_all(&self.pool).await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let category_str: String = row.try_get("category")?;
            let level_str: String = row.try_get("level")?;

            results.push(SystemLogRecord {
                log_id: row.try_get("log_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                category: LogCategory::parse(&category_str)
                    .ok_or_else(|| StorageError::new("invalid category"))?,
                level: LogLevel::parse(&level_str)
                    .ok_or_else(|| StorageError::new("invalid level"))?,
                title: row.try_get("title")?,
                message: row.try_get("message")?,
                source: row.try_get("source")?,
                resource: row.try_get("resource")?,
                actor: row.try_get("actor")?,
                metadata: row.try_get("metadata")?,
                is_read: row.try_get("is_read")?,
                created_at_ms: row.try_get("created_at_ms")?,
                read_at_ms: row.try_get("read_at_ms")?,
            });
        }

        Ok(results)
    }

    async fn get_unread_count(
        &self,
        ctx: &TenantContext,
        category: Option<LogCategory>,
    ) -> Result<UnreadStats, StorageError> {
        let (operation, error, warning) = if let Some(cat) = category {
            // 查询单个分类的未读数量
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM system_logs \
                 WHERE tenant_id = $1 AND category = $2 AND is_read = false",
            )
            .bind(&ctx.tenant_id)
            .bind(cat.as_str())
            .fetch_one(&self.pool)
            .await?;

            match cat {
                LogCategory::Operation => (count, 0, 0),
                LogCategory::Error => (0, count, 0),
                LogCategory::Warning => (0, 0, count),
            }
        } else {
            // 查询所有分类的未读数量
            let row = sqlx::query(
                "SELECT \
                    COUNT(*) FILTER (WHERE category = 'operation') as operation_count, \
                    COUNT(*) FILTER (WHERE category = 'error') as error_count, \
                    COUNT(*) FILTER (WHERE category = 'warning') as warning_count \
                 FROM system_logs \
                 WHERE tenant_id = $1 AND is_read = false",
            )
            .bind(&ctx.tenant_id)
            .fetch_one(&self.pool)
            .await?;

            (
                row.try_get("operation_count")?,
                row.try_get("error_count")?,
                row.try_get("warning_count")?,
            )
        };

        Ok(UnreadStats {
            operation,
            error,
            warning,
            total: operation + error + warning,
        })
    }

    async fn mark_as_read(
        &self,
        ctx: &TenantContext,
        log_ids: Vec<String>,
    ) -> Result<usize, StorageError> {
        if log_ids.is_empty() {
            return Ok(0);
        }

        // 使用 ANY 批量更新
        let result = sqlx::query(
            "UPDATE system_logs \
             SET is_read = true, read_at = CURRENT_TIMESTAMP \
             WHERE tenant_id = $1 AND log_id = ANY($2) AND is_read = false",
        )
        .bind(&ctx.tenant_id)
        .bind(&log_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    async fn cleanup_old_logs(&self, older_than_ms: i64) -> Result<usize, StorageError> {
        let result = sqlx::query(
            "DELETE FROM system_logs \
             WHERE created_at < to_timestamp($1 / 1000.0)",
        )
        .bind(older_than_ms as f64)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }
}
