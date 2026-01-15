//! Postgres 设备存储实现
//!
//! 通过 SQL 查询实现设备 CRUD 操作。
//!
//! 设计要点：
//! - 所有操作都带有租户和项目作用域验证
//! - 使用参数化 SQL 防止注入

use crate::error::StorageError;
use crate::models::{DeviceRecord, DeviceUpdate};
use crate::traits::DeviceStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgDeviceStore {
    pub pool: PgPool,
}

impl PgDeviceStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = crate::connection::connect_pool(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl DeviceStore for PgDeviceStore {
    async fn list_devices(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<DeviceRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let rows = sqlx::query(
            "select device_id, tenant_id, project_id, gateway_id, name, model \
             from devices where tenant_id = $1 and project_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        let mut devices = Vec::with_capacity(rows.len());
        for row in rows {
            devices.push(DeviceRecord {
                device_id: row.try_get("device_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                gateway_id: row.try_get("gateway_id")?,
                name: row.try_get("name")?,
                model: row.try_get("model")?,
            });
        }
        Ok(devices)
    }

    async fn find_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<DeviceRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "select device_id, tenant_id, project_id, gateway_id, name, model \
             from devices where tenant_id = $1 and project_id = $2 and device_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(DeviceRecord {
            device_id: row.try_get("device_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            gateway_id: row.try_get("gateway_id")?,
            name: row.try_get("name")?,
            model: row.try_get("model")?,
        }))
    }

    async fn create_device(
        &self,
        ctx: &TenantContext,
        record: DeviceRecord,
    ) -> Result<DeviceRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        sqlx::query(
            "insert into devices (device_id, tenant_id, project_id, gateway_id, name, model) \
             values ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&record.device_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.gateway_id)
        .bind(&record.name)
        .bind(&record.model)
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn update_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
        update: DeviceUpdate,
    ) -> Result<Option<DeviceRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "update devices set \
             name = coalesce($1, name), \
             model = coalesce($2, model) \
             where tenant_id = $3 and project_id = $4 and device_id = $5 \
             returning device_id, tenant_id, project_id, gateway_id, name, model",
        )
        .bind(update.name)
        .bind(update.model)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(DeviceRecord {
            device_id: row.try_get("device_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            gateway_id: row.try_get("gateway_id")?,
            name: row.try_get("name")?,
            model: row.try_get("model")?,
        }))
    }

    async fn delete_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let result = sqlx::query(
            "delete from devices where tenant_id = $1 and project_id = $2 and device_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
