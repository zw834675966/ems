//! Postgres 项目存储实现
//!
//! 通过 SQL 查询实现项目 CRUD 操作。
//!
//! 设计要点：
//! - 所有操作都带有租户验证
//! - 使用参数化 SQL 防止注入

use crate::error::StorageError;
use crate::models::{ProjectRecord, ProjectUpdate};
use crate::traits::ProjectStore;
use crate::validation::ensure_tenant;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgProjectStore {
    pub pool: PgPool,
}

impl PgProjectStore {
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
impl ProjectStore for PgProjectStore {
    /// 列出当前租户的所有项目
    async fn list_projects(&self, ctx: &TenantContext) -> Result<Vec<ProjectRecord>, StorageError> {
        ensure_tenant(ctx)?;
        let rows = sqlx::query(
            "select project_id, tenant_id, name, timezone \
             from projects where tenant_id = $1",
        )
        .bind(&ctx.tenant_id)
        .fetch_all(&self.pool)
        .await?;
        let mut projects = Vec::with_capacity(rows.len());
        for row in rows {
            projects.push(ProjectRecord {
                project_id: row.try_get("project_id")?,
                tenant_id: row.try_get("tenant_id")?,
                name: row.try_get("name")?,
                timezone: row.try_get("timezone")?,
            });
        }
        Ok(projects)
    }

    /// 查找指定项目
    async fn find_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Option<ProjectRecord>, StorageError> {
        ensure_tenant(ctx)?;
        let row = sqlx::query(
            "select project_id, tenant_id, name, timezone \
             from projects where tenant_id = $1 and project_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(ProjectRecord {
            project_id: row.try_get("project_id")?,
            tenant_id: row.try_get("tenant_id")?,
            name: row.try_get("name")?,
            timezone: row.try_get("timezone")?,
        }))
    }

    /// 创建新项目
    async fn create_project(
        &self,
        ctx: &TenantContext,
        record: ProjectRecord,
    ) -> Result<ProjectRecord, StorageError> {
        ensure_tenant(ctx)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        sqlx::query(
            "insert into projects (project_id, tenant_id, name, timezone) \
             values ($1, $2, $3, $4)",
        )
        .bind(&record.project_id)
        .bind(&record.tenant_id)
        .bind(&record.name)
        .bind(&record.timezone)
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    /// 更新项目
    async fn update_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        update: ProjectUpdate,
    ) -> Result<Option<ProjectRecord>, StorageError> {
        ensure_tenant(ctx)?;
        let row = sqlx::query(
            "update projects set \
             name = coalesce($1, name), \
             timezone = coalesce($2, timezone) \
             where tenant_id = $3 and project_id = $4 \
             returning project_id, tenant_id, name, timezone",
        )
        .bind(update.name)
        .bind(update.timezone)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(ProjectRecord {
            project_id: row.try_get("project_id")?,
            tenant_id: row.try_get("tenant_id")?,
            name: row.try_get("name")?,
            timezone: row.try_get("timezone")?,
        }))
    }

    /// 删除项目
    async fn delete_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_tenant(ctx)?;
        let result = sqlx::query("delete from projects where tenant_id = $1 and project_id = $2")
            .bind(&ctx.tenant_id)
            .bind(project_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// 验证项目归属当前租户
    async fn project_belongs_to_tenant(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_tenant(ctx)?;
        let exists: Option<i32> =
            sqlx::query_scalar("select 1 from projects where project_id = $1 and tenant_id = $2")
                .bind(project_id)
                .bind(&ctx.tenant_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(exists.is_some())
    }
}
