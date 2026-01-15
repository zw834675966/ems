//! Postgres 用户存储实现
//!
//! 通过 SQL 查询实现用户查找功能。
//!
//! 设计要点：
//! - 支持全局用户查询（忽略租户）
//! - 支持租户隔离查询

use crate::error::StorageError;
use crate::models::UserRecord;
use crate::traits::UserStore;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgUserStore {
    pub pool: PgPool,
}

impl PgUserStore {
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
impl UserStore for PgUserStore {
    async fn find_by_username(
        &self,
        ctx: &TenantContext,
        username: &str,
    ) -> Result<Option<UserRecord>, StorageError> {
        let row = if ctx.tenant_id.is_empty() {
            sqlx::query(
                "select user_id, tenant_id, username, password_hash \
                 from users where username = $1",
            )
            .bind(username)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                "select user_id, tenant_id, username, password_hash \
                 from users where username = $1 and tenant_id = $2",
            )
            .bind(username)
            .bind(&ctx.tenant_id)
            .fetch_optional(&self.pool)
            .await?
        };

        let Some(row) = row else {
            return Ok(None);
        };

        let user_id: String = row.try_get("user_id")?;
        let tenant_id: String = row.try_get("tenant_id")?;
        let username: String = row.try_get("username")?;
        let password: String = row.try_get("password_hash")?;

        let roles: Vec<String> =
            sqlx::query_scalar("select role_code from user_roles where user_id = $1")
                .bind(&user_id)
                .fetch_all(&self.pool)
                .await?;

        let permissions: Vec<String> = sqlx::query_scalar(
            "select distinct permission_code \
             from role_permissions rp \
             join user_roles ur on ur.role_code = rp.role_code \
             where ur.user_id = $1",
        )
        .bind(&user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(UserRecord {
            tenant_id,
            user_id,
            username,
            password,
            roles,
            permissions,
        }))
    }
}
