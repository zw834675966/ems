//! Postgres 点位映射存储实现
//!
//! 通过 SQL 查询实现点映射 CRUD 操作。
//!
//! 设计要点：
//! - 所有操作都带有租户和项目作用域验证
//! - 使用参数化 SQL 防止注入

use crate::error::StorageError;
use crate::models::{PointMappingRecord, PointMappingUpdate};
use crate::traits::PointMappingStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgPointMappingStore {
    pub pool: PgPool,
}

impl PgPointMappingStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = crate::connection::connect_pool(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl PointMappingStore for PgPointMappingStore {
    async fn list_point_mappings(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<PointMappingRecord>, StorageError> {
        let rows = sqlx::query(
            "select source_id, tenant_id, project_id, point_id, source_type, address, scale, offset_value, protocol_detail \
             from point_sources where tenant_id = $1 and project_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        let mut mappings = Vec::with_capacity(rows.len());
        for row in rows {
            mappings.push(PointMappingRecord {
                source_id: row.try_get("source_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                point_id: row.try_get("point_id")?,
                source_type: row.try_get("source_type")?,
                address: row.try_get("address")?,
                scale: row.try_get("scale")?,
                offset: row.try_get("offset_value")?,
                protocol_detail: row.try_get("protocol_detail")?,
            });
        }
        Ok(mappings)
    }

    async fn find_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
    ) -> Result<Option<PointMappingRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "select source_id, tenant_id, project_id, point_id, source_type, address, scale, offset_value, protocol_detail \
             from point_sources where tenant_id = $1 and project_id = $2 and source_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(PointMappingRecord {
            source_id: row.try_get("source_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            point_id: row.try_get("point_id")?,
            source_type: row.try_get("source_type")?,
            address: row.try_get("address")?,
            scale: row.try_get("scale")?,
            offset: row.try_get("offset_value")?,
            protocol_detail: row.try_get("protocol_detail")?,
        }))
    }

    async fn create_point_mapping(
        &self,
        ctx: &TenantContext,
        record: PointMappingRecord,
    ) -> Result<PointMappingRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        sqlx::query(
            "insert into point_sources (source_id, tenant_id, project_id, point_id, source_type, address, scale, offset_value, protocol_detail) \
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&record.source_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.point_id)
        .bind(&record.source_type)
        .bind(&record.address)
        .bind(&record.scale)
        .bind(&record.offset)
        .bind(&record.protocol_detail)
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn update_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
        update: PointMappingUpdate,
    ) -> Result<Option<PointMappingRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "update point_sources set \
             source_type = coalesce($1, source_type), \
             address = coalesce($2, address), \
             scale = coalesce($3, scale), \
             offset_value = coalesce($4, offset_value), \
             protocol_detail = coalesce($5, protocol_detail) \
             where tenant_id = $6 and project_id = $7 and source_id = $8 \
             returning source_id, tenant_id, project_id, point_id, source_type, address, scale, offset_value, protocol_detail",
        )
        .bind(update.source_type)
        .bind(update.address)
        .bind(update.scale)
        .bind(update.offset)
        .bind(update.protocol_detail)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(PointMappingRecord {
            source_id: row.try_get("source_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            point_id: row.try_get("point_id")?,
            source_type: row.try_get("source_type")?,
            address: row.try_get("address")?,
            scale: row.try_get("scale")?,
            offset: row.try_get("offset_value")?,
            protocol_detail: row.try_get("protocol_detail")?,
        }))
    }

    async fn delete_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let result = sqlx::query(
            "delete from point_sources where tenant_id = $1 and project_id = $2 and source_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(source_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
