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
            "select gateway_id, tenant_id, project_id, name, status, protocol_type, protocol_config \
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
                protocol_type: row.try_get("protocol_type")?,
                protocol_config: row.try_get("protocol_config")?,
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
            "select gateway_id, tenant_id, project_id, name, status, protocol_type, protocol_config \
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
            protocol_type: row.try_get("protocol_type")?,
            protocol_config: row.try_get("protocol_config")?,
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
            "insert into gateways (gateway_id, tenant_id, project_id, name, status, protocol_type, protocol_config) \
             values ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&record.gateway_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.name)
        .bind(&record.status)
        .bind(&record.protocol_type)
        .bind(&record.protocol_config)
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
             status = coalesce($2, status), \
             protocol_type = coalesce($3, protocol_type), \
             protocol_config = coalesce($4, protocol_config) \
             where tenant_id = $5 and project_id = $6 and gateway_id = $7 \
             returning gateway_id, tenant_id, project_id, name, status, protocol_type, protocol_config",
        )
        .bind(update.name)
        .bind(update.status)
        .bind(update.protocol_type)
        .bind(update.protocol_config)
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
            protocol_type: row.try_get("protocol_type")?,
            protocol_config: row.try_get("protocol_config")?,
        }))
    }

    /// 删除网关
    /// 删除网关（级联删除所有关联资源）
    ///
    /// 删除顺序：
    /// 1. 点位映射 (point_sources) - 属于该网关下设备的点位
    /// 2. 点位 (points) - 属于该网关下的设备
    /// 3. 设备 (devices) - 属于该网关
    /// 4. 网关 (gateways) - 网关本身
    async fn delete_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;

        // 使用事务确保级联删除的原子性
        let mut tx = self.pool.begin().await?;

        // 1. 删除点位映射（通过 points JOIN devices 找到该网关下的所有点位映射）
        sqlx::query(
            "DELETE FROM point_sources WHERE tenant_id = $1 AND project_id = $2 \
             AND point_id IN (SELECT p.point_id FROM points p \
             JOIN devices d ON p.device_id = d.device_id \
             WHERE d.gateway_id = $3)",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(gateway_id)
        .execute(&mut *tx)
        .await?;

        // 2. 删除点位（属于该网关下设备的所有点位）
        sqlx::query(
            "DELETE FROM points WHERE tenant_id = $1 AND project_id = $2 \
             AND device_id IN (SELECT device_id FROM devices WHERE gateway_id = $3)",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(gateway_id)
        .execute(&mut *tx)
        .await?;

        // 3. 删除设备（属于该网关的所有设备）
        sqlx::query(
            "DELETE FROM devices WHERE tenant_id = $1 AND project_id = $2 AND gateway_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(gateway_id)
        .execute(&mut *tx)
        .await?;

        // 4. 删除网关本身
        let result = sqlx::query(
            "DELETE FROM gateways WHERE tenant_id = $1 AND project_id = $2 AND gateway_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(gateway_id)
        .execute(&mut *tx)
        .await?;

        // 提交事务
        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }
}
