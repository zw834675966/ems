//! Postgres 点存储实现
//!
//! 通过 SQL 查询实现点位 CRUD 操作。
//!
//! 设计要点：
//! - 所有操作都带有租户和项目作用域验证
//! - 使用参数化 SQL 防止注入

use crate::error::StorageError;
use crate::models::{PointRecord, PointUpdate};
use crate::traits::PointStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgPointStore {
    pub pool: PgPool,
}

impl PgPointStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = crate::connection::connect_pool(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl PointStore for PgPointStore {
    async fn list_points(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<PointRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let rows = sqlx::query(
            "select point_id, tenant_id, project_id, device_id, key, data_type, unit \
             from points where tenant_id = $1 and project_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        let mut points = Vec::with_capacity(rows.len());
        for row in rows {
            points.push(PointRecord {
                point_id: row.try_get("point_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                device_id: row.try_get("device_id")?,
                key: row.try_get("key")?,
                data_type: row.try_get("data_type")?,
                unit: row.try_get("unit")?,
            });
        }
        Ok(points)
    }

    async fn find_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<PointRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "select point_id, tenant_id, project_id, device_id, key, data_type, unit \
             from points where tenant_id = $1 and project_id = $2 and point_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(point_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(PointRecord {
            point_id: row.try_get("point_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            device_id: row.try_get("device_id")?,
            key: row.try_get("key")?,
            data_type: row.try_get("data_type")?,
            unit: row.try_get("unit")?,
        }))
    }

    async fn create_point(
        &self,
        ctx: &TenantContext,
        record: PointRecord,
    ) -> Result<PointRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        sqlx::query(
            "insert into points (point_id, tenant_id, project_id, device_id, key, data_type, unit) \
             values ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&record.point_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.device_id)
        .bind(&record.key)
        .bind(&record.data_type)
        .bind(&record.unit)
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn update_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
        update: PointUpdate,
    ) -> Result<Option<PointRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "update points set \
             key = coalesce($1, key), \
             data_type = coalesce($2, data_type), \
             unit = coalesce($3, unit) \
             where tenant_id = $4 and project_id = $5 and point_id = $6 \
             returning point_id, tenant_id, project_id, device_id, key, data_type, unit",
        )
        .bind(update.key)
        .bind(update.data_type)
        .bind(update.unit)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(point_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(PointRecord {
            point_id: row.try_get("point_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            device_id: row.try_get("device_id")?,
            key: row.try_get("key")?,
            data_type: row.try_get("data_type")?,
            unit: row.try_get("unit")?,
        }))
    }

    /// 删除点位（级联删除所有关联资源）
    ///
    /// 删除顺序：
    /// 1. 点位映射 (point_sources) - 属于该点位的所有映射
    /// 2. 点位 (points) - 点位本身
    async fn delete_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;

        // 使用事务确保级联删除的原子性
        let mut tx = self.pool.begin().await?;

        // 1. 删除点位映射（属于该点位的所有映射）
        sqlx::query(
            "DELETE FROM point_sources WHERE tenant_id = $1 AND project_id = $2 AND point_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(point_id)
        .execute(&mut *tx)
        .await?;

        // 2. 删除点位本身
        let result = sqlx::query(
            "DELETE FROM points WHERE tenant_id = $1 AND project_id = $2 AND point_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(point_id)
        .execute(&mut *tx)
        .await?;

        // 提交事务
        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }
}
