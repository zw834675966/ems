//! 采集策略 PostgreSQL 存储实现
//!
//! 提供采集策略的 CRUD 操作，支持多项目批量查询和采集状态更新。

use crate::error::StorageError;
use crate::models::{CollectionStrategyCreate, CollectionStrategyRecord, CollectionStrategyUpdate};
use crate::traits::CollectionStrategyStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

/// PostgreSQL 采集策略存储实现
#[derive(Debug, Clone)]
pub struct PgCollectionStrategyStore {
    pool: PgPool,
}

impl PgCollectionStrategyStore {
    /// 创建新的采集策略存储实例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 从数据库行构造 CollectionStrategyRecord
    /// 注意：时间戳字段暂不解析，返回 None（后续可添加 chrono 依赖完善）
    fn row_to_record(row: &sqlx::postgres::PgRow) -> Result<CollectionStrategyRecord, sqlx::Error> {
        Ok(CollectionStrategyRecord {
            strategy_id: row.try_get("strategy_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            point_id: row.try_get("point_id")?,
            enabled: row.try_get("enabled")?,
            interval_value: row.try_get("interval_value")?,
            interval_unit: row.try_get("interval_unit")?,
            write_to_db: row.try_get("write_to_db")?,
            // 时间戳字段暂不解析，后续可添加 chrono 依赖完善
            last_collected_at: None,
            last_value: row.try_get("last_value")?,
            last_error: row.try_get("last_error")?,
            created_at: None,
            updated_at: None,
        })
    }
}

#[async_trait::async_trait]
impl CollectionStrategyStore for PgCollectionStrategyStore {
    async fn list_strategies(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;

        let rows = sqlx::query(
            "SELECT strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at \
             FROM collection_strategies \
             WHERE tenant_id = $1 AND project_id = $2 \
             ORDER BY created_at DESC",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            records.push(Self::row_to_record(row)?);
        }
        Ok(records)
    }

    async fn list_strategies_by_projects(
        &self,
        ctx: &TenantContext,
        project_ids: &[String],
    ) -> Result<Vec<CollectionStrategyRecord>, StorageError> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            "SELECT strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at \
             FROM collection_strategies \
             WHERE tenant_id = $1 AND project_id = ANY($2) \
             ORDER BY project_id, created_at DESC",
        )
        .bind(&ctx.tenant_id)
        .bind(project_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            records.push(Self::row_to_record(row)?);
        }
        Ok(records)
    }

    async fn list_enabled_strategies(
        &self,
        ctx: &TenantContext,
    ) -> Result<Vec<CollectionStrategyRecord>, StorageError> {
        let rows = sqlx::query(
            "SELECT strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at \
             FROM collection_strategies \
             WHERE tenant_id = $1 AND enabled = true \
             ORDER BY project_id, created_at DESC",
        )
        .bind(&ctx.tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            records.push(Self::row_to_record(row)?);
        }
        Ok(records)
    }

    async fn find_by_point_id(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;

        let row = sqlx::query(
            "SELECT strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at \
             FROM collection_strategies \
             WHERE tenant_id = $1 AND project_id = $2 AND point_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(point_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(Self::row_to_record(&r)?)),
            None => Ok(None),
        }
    }

    async fn find_strategy(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        strategy_id: &str,
    ) -> Result<Option<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;

        let row = sqlx::query(
            "SELECT strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at \
             FROM collection_strategies \
             WHERE tenant_id = $1 AND project_id = $2 AND strategy_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(strategy_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(Self::row_to_record(&r)?)),
            None => Ok(None),
        }
    }

    async fn create_strategy(
        &self,
        ctx: &TenantContext,
        record: CollectionStrategyCreate,
    ) -> Result<CollectionStrategyRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;

        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }

        let row = sqlx::query(
            "INSERT INTO collection_strategies \
             (strategy_id, tenant_id, project_id, point_id, enabled, interval_value, interval_unit, write_to_db) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             RETURNING strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at",
        )
        .bind(&record.strategy_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.point_id)
        .bind(record.enabled)
        .bind(record.interval_value)
        .bind(&record.interval_unit)
        .bind(record.write_to_db)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_record(&row)?)
    }

    async fn update_strategy(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        strategy_id: &str,
        update: CollectionStrategyUpdate,
    ) -> Result<Option<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;

        let row = sqlx::query(
            "UPDATE collection_strategies SET \
             enabled = COALESCE($4, enabled), \
             interval_value = COALESCE($5, interval_value), \
             interval_unit = COALESCE($6, interval_unit), \
             write_to_db = COALESCE($7, write_to_db), \
             updated_at = NOW() \
             WHERE tenant_id = $1 AND project_id = $2 AND strategy_id = $3 \
             RETURNING strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(strategy_id)
        .bind(update.enabled)
        .bind(update.interval_value)
        .bind(update.interval_unit)
        .bind(update.write_to_db)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(Self::row_to_record(&r)?)),
            None => Ok(None),
        }
    }

    async fn upsert_strategy(
        &self,
        ctx: &TenantContext,
        record: CollectionStrategyCreate,
    ) -> Result<CollectionStrategyRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;

        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }

        let row = sqlx::query(
            "INSERT INTO collection_strategies \
             (strategy_id, tenant_id, project_id, point_id, enabled, interval_value, interval_unit, write_to_db) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (tenant_id, project_id, point_id) DO UPDATE SET \
             enabled = EXCLUDED.enabled, \
             interval_value = EXCLUDED.interval_value, \
             interval_unit = EXCLUDED.interval_unit, \
             write_to_db = EXCLUDED.write_to_db, \
             updated_at = NOW() \
             RETURNING strategy_id, tenant_id, project_id, point_id, \
             enabled, interval_value, interval_unit, write_to_db, \
             last_collected_at, last_value, last_error, created_at, updated_at",
        )
        .bind(&record.strategy_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.point_id)
        .bind(record.enabled)
        .bind(record.interval_value)
        .bind(&record.interval_unit)
        .bind(record.write_to_db)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_record(&row)?)
    }

    async fn batch_update_enabled(
        &self,
        ctx: &TenantContext,
        strategy_ids: &[String],
        enabled: bool,
    ) -> Result<usize, StorageError> {
        if strategy_ids.is_empty() {
            return Ok(0);
        }

        let result = sqlx::query(
            "UPDATE collection_strategies \
             SET enabled = $3, updated_at = NOW() \
             WHERE tenant_id = $1 AND strategy_id = ANY($2)",
        )
        .bind(&ctx.tenant_id)
        .bind(strategy_ids)
        .bind(enabled)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    async fn delete_strategy(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        strategy_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;

        let result = sqlx::query(
            "DELETE FROM collection_strategies \
             WHERE tenant_id = $1 AND project_id = $2 AND strategy_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(strategy_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_collection_status(
        &self,
        ctx: &TenantContext,
        strategy_id: &str,
        last_value: Option<String>,
        last_error: Option<String>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE collection_strategies \
             SET last_collected_at = NOW(), \
             last_value = $3, \
             last_error = $4, \
             updated_at = NOW() \
             WHERE tenant_id = $1 AND strategy_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(strategy_id)
        .bind(last_value)
        .bind(last_error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_millis() {
        let record = CollectionStrategyRecord {
            strategy_id: "test".to_string(),
            tenant_id: "t1".to_string(),
            project_id: "p1".to_string(),
            point_id: "pt1".to_string(),
            enabled: true,
            interval_value: 1000,
            interval_unit: "ms".to_string(),
            write_to_db: true,
            last_collected_at: None,
            last_value: None,
            last_error: None,
            created_at: None,
            updated_at: None,
        };

        assert_eq!(record.interval_millis(), 1000);

        let record_sec = CollectionStrategyRecord {
            interval_value: 5,
            interval_unit: "s".to_string(),
            ..record.clone()
        };
        assert_eq!(record_sec.interval_millis(), 5000);

        let record_min = CollectionStrategyRecord {
            interval_value: 2,
            interval_unit: "min".to_string(),
            ..record
        };
        assert_eq!(record_min.interval_millis(), 120000);
    }
}
