//! 存储能力的最小内存实现（仅用于 M0 演示）。

use async_trait::async_trait;
use domain::TenantContext;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::RwLock;

/// 用户记录（后续应替换为数据库模型）。
#[derive(Debug, Clone)]
pub struct UserRecord {
    pub tenant_id: String,
    pub user_id: String,
    pub username: String,
    pub password: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl UserRecord {
    /// 将用户记录转换为 TenantContext。
    pub fn to_tenant_context(&self) -> TenantContext {
        TenantContext::new(
            self.tenant_id.clone(),
            self.user_id.clone(),
            self.roles.clone(),
            self.permissions.clone(),
            None,
        )
    }
}

/// 存储层错误（最小封装）。
#[derive(Debug)]
pub struct StorageError {
    message: String,
}

impl StorageError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StorageError {}

impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// 用户读取接口（禁止在 handler 中直连 SQL）。
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn find_by_username(
        &self,
        ctx: &TenantContext,
        username: &str,
    ) -> Result<Option<UserRecord>, StorageError>;
}

/// 仅用于本地 M0 的内存实现。
pub struct InMemoryUserStore {
    users: RwLock<HashMap<String, UserRecord>>,
}

impl InMemoryUserStore {
    /// 内置 admin 账户。
    pub fn with_default_admin() -> Self {
        let mut users = HashMap::new();
        users.insert(
            "admin".to_string(),
            UserRecord {
                tenant_id: "tenant-1".to_string(),
                user_id: "user-1".to_string(),
                username: "admin".to_string(),
                password: "admin123".to_string(),
                roles: vec!["admin".to_string()],
                permissions: Vec::new(),
            },
        );
        Self {
            users: RwLock::new(users),
        }
    }
}

#[async_trait]
impl UserStore for InMemoryUserStore {
    async fn find_by_username(
        &self,
        _ctx: &TenantContext,
        username: &str,
    ) -> Result<Option<UserRecord>, StorageError> {
        Ok(self
            .users
            .read()
            .ok()
            .and_then(|map| map.get(username).cloned()))
    }
}

/// Postgres 用户存储实现（用于替换内存版本）。
pub struct PgUserStore {
    pool: PgPool,
}

impl PgUserStore {
    /// 通过数据库 URL 建立连接池。
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
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

        let roles: Vec<String> = sqlx::query_scalar(
            "select role_code from user_roles where user_id = $1",
        )
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
