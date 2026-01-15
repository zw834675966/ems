//! Postgres 网关存储实现
//!
//! 通过 SQL 查询实现网关 CRUD 操作。
//!
//! 设计要点：
//! - 所有操作都带有租户和项目作用域验证
//! - 使用参数化 SQL 防止注入

use crate::error::StorageError;
use crate::models::{GatewayRecord, GatewayUpdate};
use crate::traits::GatewayStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgGatewayStore {
    pub pool: PgPool,
}

impl PgGatewayStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 通过数据库 URL 建立连接池
    ///
    /// # 参数
    /// - `database_url`：Postgres 连接字符串
    ///
    /// # 返回
    /// - `Result<Self, StorageError>`：连接池或错误
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = crate::connection::connect_pool(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl GatewayStore for PgGatewayStore {
    /// 列出指定项目的所有网关
    async fn list_gateways(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<GatewayRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let rows = sqlx::query(
            "select gateway_id, tenant_id, project_id, name, status \
             from gateways where tenant_id = $1 and project_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        let mut gateways = Vec::with_capacity(rows.len());
        for row in rows {
            gateways.push(GatewayRecord {
                gateway_id: row.try_get("gateway_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                name: row.try_get("name")?,
                status: row.try_get("status")?,
            });
        }
        Ok(gateways)
    }

    /// 查找指定网关
    async fn find_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<Option<GatewayRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "select gateway_id, tenant_id, project_id, name, status \
             from gateways where tenant_id = $1 and project_id = $2 and gateway_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(gateway_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(GatewayRecord {
            gateway_id: row.try_get("gateway_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            name: row.try_get("name")?,
            status: row.try_get("status")?,
        }))
    }

    /// 创建新网关
    async fn create_gateway(
        &self,
        ctx: &TenantContext,
        record: GatewayRecord,
    ) -> Result<GatewayRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        sqlx::query(
            "insert into gateways (gateway_id, tenant_id, project_id, name, status) \
             values ($1, $2, $3, $4, $5)",
        )
        .bind(&record.gateway_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.name)
        .bind(&record.status)
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    /// 更新网关
    async fn update_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
        update: GatewayUpdate,
    ) -> Result<Option<GatewayRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let row = sqlx::query(
            "update gateways set \
             name = coalesce($1, name), \
             status = coalesce($2, status) \
             where tenant_id = $3 and project_id = $4 and gateway_id = $5 \
             returning gateway_id, tenant_id, project_id, name, status",
        )
        .bind(update.name)
        .bind(update.status)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(gateway_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(GatewayRecord {
            gateway_id: row.try_get("gateway_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            name: row.try_get("name")?,
            status: row.try_get("status")?,
        }))
    }

    /// 删除网关
    async fn delete_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let result = sqlx::query(
            "delete from gateways where tenant_id = $1 and project_id = $2 and gateway_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(gateway_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
