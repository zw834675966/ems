//! Postgres 设备存储实现
//!
//! 通过 SQL 查询实现设备 CRUD 操作，实现 [`DeviceStore`] trait。
//!
//! ## 设计要点
//!
//! - **多租户隔离**：所有 SQL 查询都包含 `tenant_id` 过滤条件
//! - **项目作用域验证**：所有操作前调用 `ensure_project_scope` 验证项目归属权限
//! - **参数化查询**：使用 sqlx 的参数绑定防止 SQL 注入
//! - **返回更新后数据**：update/delete 操作返回完整记录或受影响行数

use crate::error::StorageError;
use crate::models::{DeviceRecord, DeviceUpdate};
use crate::traits::DeviceStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

/// PostgreSQL 设备存储实现
///
/// 使用 PostgreSQL 连接池执行设备相关的数据库操作。
pub struct PgDeviceStore {
    /// PostgreSQL 连接池
    pub pool: PgPool,
}

impl PgDeviceStore {
    /// 创建新的设备存储实例
    ///
    /// # 参数
    ///
    /// - `pool`: 已初始化的 PostgreSQL 连接池
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let pool = connect_pool("postgresql://user:pass@localhost/db").await?;
    /// let device_store = PgDeviceStore::new(pool);
    /// ```
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 从数据库 URL 创建设备存储实例
    ///
    /// # 参数
    ///
    /// - `database_url`: PostgreSQL 连接字符串
    ///
    /// # 错误
    ///
    /// 如果数据库连接失败，返回 `StorageError`
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = crate::connection::connect_pool(database_url).await?;
        Ok(Self { pool })
    }
}

/// 实现 [`DeviceStore`] trait
///
/// 提供基于 PostgreSQL 的设备 CRUD 操作实现。
#[async_trait::async_trait]
impl DeviceStore for PgDeviceStore {
    /// 列出指定项目的所有设备
    ///
    /// # 安全
    ///
    /// - 首先验证项目归属权限（`ensure_project_scope`）
    /// - SQL 查询包含 `tenant_id` 和 `project_id` 过滤条件，确保租户隔离
    ///
    /// # 返回
    ///
    /// 返回属于指定项目的所有设备列表
    async fn list_devices(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<DeviceRecord>, StorageError> {
        // 验证项目作用域：确保当前上下文有权限访问该项目
        ensure_project_scope(ctx, project_id)?;

        // 查询指定租户和项目下的所有设备
        let rows = sqlx::query(
            "select device_id, tenant_id, project_id, gateway_id, name, model, room_id, address_config \
             from devices where tenant_id = $1 and project_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        // 将查询结果转换为 DeviceRecord 向量
        let mut devices = Vec::with_capacity(rows.len());
        for row in rows {
            devices.push(DeviceRecord {
                device_id: row.try_get("device_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                gateway_id: row.try_get("gateway_id")?,
                name: row.try_get("name")?,
                model: row.try_get("model")?,
                room_id: row.try_get("room_id")?,
                address_config: row.try_get("address_config")?,
            });
        }
        Ok(devices)
    }

    /// 查找指定设备
    ///
    /// # 安全
    ///
    /// - 验证项目归属权限（`ensure_project_scope`）
    /// - SQL 查询同时验证 `tenant_id`、`project_id` 和 `device_id`，三重隔离
    ///
    /// # 返回
    ///
    /// - `Some(DeviceRecord)`：找到设备
    /// - `None`：设备不存在或无权限访问
    async fn find_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<DeviceRecord>, StorageError> {
        // 验证项目作用域
        ensure_project_scope(ctx, project_id)?;

        // 使用三重条件查询：租户 + 项目 + 设备 ID
        let row = sqlx::query(
            "select device_id, tenant_id, project_id, gateway_id, name, model, room_id, address_config \
             from devices where tenant_id = $1 and project_id = $2 and device_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;

        // 如果没有找到记录，返回 None
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
            room_id: row.try_get("room_id")?,
            address_config: row.try_get("address_config")?,
        }))
    }

    /// 创建新设备
    ///
    /// # 安全验证
    ///
    /// 1. 验证项目归属权限（`ensure_project_scope`）
    /// 2. 确保 `record.tenant_id` 与上下文中的 `tenant_id` 一致，防止跨租户创建
    ///
    /// # 返回
    ///
    /// 返回创建的设备记录（与输入相同）
    async fn create_device(
        &self,
        ctx: &TenantContext,
        record: DeviceRecord,
    ) -> Result<DeviceRecord, StorageError> {
        // 验证项目作用域
        ensure_project_scope(ctx, &record.project_id)?;

        // 验证租户 ID 一致性：防止恶意用户创建跨租户数据
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }

        // 执行插入操作
        sqlx::query(
            "insert into devices (device_id, tenant_id, project_id, gateway_id, name, model, room_id, address_config) \
             values ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&record.device_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.gateway_id)
        .bind(&record.name)
        .bind(&record.model)
        .bind(&record.room_id)
        .bind(&record.address_config)
        .execute(&self.pool)
        .await?;

        Ok(record)
    }

    /// 更新设备信息
    ///
    /// # 更新逻辑
    ///
    /// - 使用 `coalesce` 函数实现部分更新：仅更新非 `None` 的字段
    /// - `name = coalesce($1, name)`：如果 `$1` 为 `None`，则保留原有值
    ///
    /// # 安全
    ///
    /// - 验证项目归属权限
    /// - SQL 查询同时验证 `tenant_id`、`project_id` 和 `device_id`
    ///
    /// # 返回
    ///
    /// - `Some(DeviceRecord)`：更新成功，返回更新后的完整记录
    /// - `None`：设备不存在或无权限访问
    async fn update_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
        update: DeviceUpdate,
    ) -> Result<Option<DeviceRecord>, StorageError> {
        // 验证项目作用域
        ensure_project_scope(ctx, project_id)?;

        // 执行更新并返回更新后的记录
        // 使用 coalesce 实现部分更新：如果参数为 None 则保留原值
        let row = sqlx::query(
            "update devices set \
             name = coalesce($1, name), \
             model = coalesce($2, model), \
             room_id = coalesce($3, room_id), \
             address_config = coalesce($4, address_config) \
             where tenant_id = $5 and project_id = $6 and device_id = $7 \
             returning device_id, tenant_id, project_id, gateway_id, name, model, room_id, address_config",
        )
        .bind(update.name)
        .bind(update.model)
        .bind(update.room_id)
        .bind(update.address_config)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;

        // 如果没有找到记录，返回 None
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
            room_id: row.try_get("room_id")?,
            address_config: row.try_get("address_config")?,
        }))
    }

    /// 删除设备（级联删除所有关联资源）
    ///
    /// 删除顺序：
    /// 1. 点位映射 (point_sources) - 属于该设备下的点位
    /// 2. 点位 (points) - 属于该设备
    /// 3. 设备 (devices) - 设备本身
    ///
    /// # 安全
    ///
    /// - 验证项目归属权限
    /// - SQL 查询同时验证 `tenant_id`、`project_id` 和 `device_id`
    /// - 使用事务确保删除操作的原子性
    ///
    /// # 返回
    ///
    /// - `true`：删除成功（至少影响一行）
    /// - `false`：设备不存在或无权限访问
    async fn delete_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<bool, StorageError> {
        // 验证项目作用域
        ensure_project_scope(ctx, project_id)?;

        // 使用事务确保级联删除的原子性
        let mut tx = self.pool.begin().await?;

        // 1. 删除点位映射（属于该设备下点位的所有映射）
        sqlx::query(
            "DELETE FROM point_sources WHERE tenant_id = $1 AND project_id = $2 \
             AND point_id IN (SELECT point_id FROM points WHERE device_id = $3)",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

        // 2. 删除点位（属于该设备的所有点位）
        sqlx::query(
            "DELETE FROM points WHERE tenant_id = $1 AND project_id = $2 AND device_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

        // 3. 删除设备本身
        let result = sqlx::query(
            "DELETE FROM devices WHERE tenant_id = $1 AND project_id = $2 AND device_id = $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

        // 提交事务
        tx.commit().await?;

        // 根据受影响行数判断是否删除成功
        Ok(result.rows_affected() > 0)
    }
}
