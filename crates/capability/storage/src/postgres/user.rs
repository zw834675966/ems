//! Postgres 用户存储实现
//!
//! 通过 SQL 查询实现用户查找功能。
//!
//! 设计要点：
//! - 支持全局用户查询（忽略租户）
//! - 支持租户隔离查询

use crate::error::StorageError;
use crate::models::{PermissionRecord, RbacRoleCreate, RbacRoleRecord, RbacUserCreate, RbacUserRecord, RbacUserUpdate, UserRecord};
use crate::traits::{RbacStore, UserStore};
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
            sqlx::query_scalar("select role_code from tenant_user_roles where tenant_id = $1 and user_id = $2")
                .bind(&tenant_id)
                .bind(&user_id)
                .fetch_all(&self.pool)
                .await?;

        let permissions: Vec<String> = sqlx::query_scalar(
            "select distinct permission_code \
             from tenant_role_permissions rp \
             join tenant_user_roles ur \
               on ur.tenant_id = rp.tenant_id and ur.role_code = rp.role_code \
             where ur.tenant_id = $1 and ur.user_id = $2",
        )
        .bind(&tenant_id)
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

    async fn update_password_hash(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        password_hash: &str,
    ) -> Result<bool, StorageError> {
        let result = if ctx.tenant_id.is_empty() {
            sqlx::query("update users set password_hash = $2 where user_id = $1")
                .bind(user_id)
                .bind(password_hash)
                .execute(&self.pool)
                .await?
        } else {
            sqlx::query("update users set password_hash = $3 where tenant_id = $1 and user_id = $2")
                .bind(&ctx.tenant_id)
                .bind(user_id)
                .bind(password_hash)
                .execute(&self.pool)
                .await?
        };
        Ok(result.rows_affected() > 0)
    }

    async fn get_refresh_jti(
        &self,
        ctx: &TenantContext,
        user_id: &str,
    ) -> Result<Option<String>, StorageError> {
        let value: Option<String> = if ctx.tenant_id.is_empty() {
            sqlx::query_scalar("select refresh_jti from users where user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?
        } else {
            sqlx::query_scalar("select refresh_jti from users where tenant_id = $1 and user_id = $2")
                .bind(&ctx.tenant_id)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?
        };
        Ok(value)
    }

    async fn set_refresh_jti(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        refresh_jti: Option<&str>,
    ) -> Result<bool, StorageError> {
        let result = if ctx.tenant_id.is_empty() {
            sqlx::query("update users set refresh_jti = $2 where user_id = $1")
                .bind(user_id)
                .bind(refresh_jti)
                .execute(&self.pool)
                .await?
        } else {
            sqlx::query("update users set refresh_jti = $3 where tenant_id = $1 and user_id = $2")
                .bind(&ctx.tenant_id)
                .bind(user_id)
                .bind(refresh_jti)
                .execute(&self.pool)
                .await?
        };
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait::async_trait]
impl RbacStore for PgUserStore {
    async fn list_users(&self, ctx: &TenantContext) -> Result<Vec<RbacUserRecord>, StorageError> {
        let rows = sqlx::query("select user_id, username, status from users where tenant_id = $1 order by created_at asc")
            .bind(&ctx.tenant_id)
            .fetch_all(&self.pool)
            .await?;

        let mut users: Vec<RbacUserRecord> = Vec::with_capacity(rows.len());
        let mut user_ids: Vec<String> = Vec::with_capacity(rows.len());
        for row in rows {
            let user_id: String = row.try_get("user_id")?;
            let username: String = row.try_get("username")?;
            let status: String = row.try_get("status")?;
            user_ids.push(user_id.clone());
            users.push(RbacUserRecord {
                tenant_id: ctx.tenant_id.clone(),
                user_id,
                username,
                status,
                roles: Vec::new(),
            });
        }

        if user_ids.is_empty() {
            return Ok(users);
        }

        let rows = sqlx::query(
            "select user_id, role_code from tenant_user_roles where tenant_id = $1 and user_id = any($2)",
        )
        .bind(&ctx.tenant_id)
        .bind(&user_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut role_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for row in rows {
            let user_id: String = row.try_get("user_id")?;
            let role_code: String = row.try_get("role_code")?;
            role_map.entry(user_id).or_default().push(role_code);
        }

        for user in &mut users {
            if let Some(roles) = role_map.get(&user.user_id) {
                user.roles = roles.clone();
            }
        }

        Ok(users)
    }

    async fn create_user(
        &self,
        ctx: &TenantContext,
        record: RbacUserCreate,
    ) -> Result<RbacUserRecord, StorageError> {
        sqlx::query(
            "insert into users (user_id, tenant_id, username, password_hash, status) values ($1,$2,$3,$4,$5)",
        )
        .bind(&record.user_id)
        .bind(&record.tenant_id)
        .bind(&record.username)
        .bind(&record.password)
        .bind(&record.status)
        .execute(&self.pool)
        .await?;

        let mut created = RbacUserRecord {
            tenant_id: record.tenant_id,
            user_id: record.user_id,
            username: record.username,
            status: record.status,
            roles: Vec::new(),
        };

        if !record.roles.is_empty() {
            if let Some(updated) = self
                .set_user_roles(ctx, &created.user_id, record.roles)
                .await?
            {
                created = updated;
            }
        }
        Ok(created)
    }

    async fn update_user(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        update: RbacUserUpdate,
    ) -> Result<Option<RbacUserRecord>, StorageError> {
        let row = sqlx::query(
            "update users set password_hash = coalesce($3, password_hash), status = coalesce($4, status), \
             refresh_jti = case when $3 is null then refresh_jti else null end \
             where tenant_id = $1 and user_id = $2 returning user_id, username, status",
        )
        .bind(&ctx.tenant_id)
        .bind(user_id)
        .bind(update.password)
        .bind(update.status)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let user_id: String = row.try_get("user_id")?;
        let username: String = row.try_get("username")?;
        let status: String = row.try_get("status")?;

        let roles: Vec<String> = sqlx::query_scalar(
            "select role_code from tenant_user_roles where tenant_id = $1 and user_id = $2 order by role_code asc",
        )
        .bind(&ctx.tenant_id)
        .bind(&user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(RbacUserRecord {
            tenant_id: ctx.tenant_id.clone(),
            user_id,
            username,
            status,
            roles,
        }))
    }

    async fn set_user_roles(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        roles: Vec<String>,
    ) -> Result<Option<RbacUserRecord>, StorageError> {
        let exists: Option<i32> = sqlx::query_scalar(
            "select 1 from users where tenant_id = $1 and user_id = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        if exists.is_none() {
            return Ok(None);
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("delete from tenant_user_roles where tenant_id = $1 and user_id = $2")
            .bind(&ctx.tenant_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        for role_code in &roles {
            sqlx::query(
                "insert into tenant_user_roles (tenant_id, user_id, role_code) values ($1,$2,$3) on conflict do nothing",
            )
            .bind(&ctx.tenant_id)
            .bind(user_id)
            .bind(role_code)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        let row = sqlx::query("select user_id, username, status from users where tenant_id = $1 and user_id = $2")
            .bind(&ctx.tenant_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        let username: String = row.try_get("username")?;
        let status: String = row.try_get("status")?;

        Ok(Some(RbacUserRecord {
            tenant_id: ctx.tenant_id.clone(),
            user_id: user_id.to_string(),
            username,
            status,
            roles,
        }))
    }

    async fn list_roles(&self, ctx: &TenantContext) -> Result<Vec<RbacRoleRecord>, StorageError> {
        let rows = sqlx::query(
            "select role_code, name from tenant_roles where tenant_id = $1 order by role_code asc",
        )
        .bind(&ctx.tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let mut roles: Vec<RbacRoleRecord> = Vec::with_capacity(rows.len());
        let mut role_codes: Vec<String> = Vec::with_capacity(rows.len());
        for row in rows {
            let role_code: String = row.try_get("role_code")?;
            let name: String = row.try_get("name")?;
            role_codes.push(role_code.clone());
            roles.push(RbacRoleRecord {
                tenant_id: ctx.tenant_id.clone(),
                role_code,
                name,
                permissions: Vec::new(),
            });
        }

        if role_codes.is_empty() {
            return Ok(roles);
        }

        let rows = sqlx::query(
            "select role_code, permission_code from tenant_role_permissions where tenant_id = $1 and role_code = any($2)",
        )
        .bind(&ctx.tenant_id)
        .bind(&role_codes)
        .fetch_all(&self.pool)
        .await?;

        let mut perm_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for row in rows {
            let role_code: String = row.try_get("role_code")?;
            let permission_code: String = row.try_get("permission_code")?;
            perm_map.entry(role_code).or_default().push(permission_code);
        }

        for role in &mut roles {
            if let Some(perms) = perm_map.get(&role.role_code) {
                let mut perms = perms.clone();
                perms.sort();
                role.permissions = perms;
            }
        }

        Ok(roles)
    }

    async fn create_role(
        &self,
        _ctx: &TenantContext,
        record: RbacRoleCreate,
    ) -> Result<RbacRoleRecord, StorageError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("insert into tenant_roles (tenant_id, role_code, name) values ($1,$2,$3)")
            .bind(&record.tenant_id)
            .bind(&record.role_code)
            .bind(&record.name)
            .execute(&mut *tx)
            .await?;

        for permission_code in &record.permissions {
            sqlx::query(
                "insert into tenant_role_permissions (tenant_id, role_code, permission_code) values ($1,$2,$3) on conflict do nothing",
            )
            .bind(&record.tenant_id)
            .bind(&record.role_code)
            .bind(permission_code)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(RbacRoleRecord {
            tenant_id: record.tenant_id,
            role_code: record.role_code,
            name: record.name,
            permissions: record.permissions,
        })
    }

    async fn delete_role(
        &self,
        ctx: &TenantContext,
        role_code: &str,
    ) -> Result<bool, StorageError> {
        let result = sqlx::query("delete from tenant_roles where tenant_id = $1 and role_code = $2")
            .bind(&ctx.tenant_id)
            .bind(role_code)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_role_permissions(
        &self,
        ctx: &TenantContext,
        role_code: &str,
        permissions: Vec<String>,
    ) -> Result<Option<RbacRoleRecord>, StorageError> {
        let row = sqlx::query("select role_code, name from tenant_roles where tenant_id = $1 and role_code = $2")
            .bind(&ctx.tenant_id)
            .bind(role_code)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let name: String = row.try_get("name")?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "delete from tenant_role_permissions where tenant_id = $1 and role_code = $2",
        )
        .bind(&ctx.tenant_id)
        .bind(role_code)
        .execute(&mut *tx)
        .await?;
        for permission_code in &permissions {
            sqlx::query(
                "insert into tenant_role_permissions (tenant_id, role_code, permission_code) values ($1,$2,$3) on conflict do nothing",
            )
            .bind(&ctx.tenant_id)
            .bind(role_code)
            .bind(permission_code)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        Ok(Some(RbacRoleRecord {
            tenant_id: ctx.tenant_id.clone(),
            role_code: role_code.to_string(),
            name,
            permissions,
        }))
    }

    async fn list_permissions(
        &self,
        _ctx: &TenantContext,
    ) -> Result<Vec<PermissionRecord>, StorageError> {
        let rows = sqlx::query("select permission_code, description from permissions order by permission_code asc")
            .fetch_all(&self.pool)
            .await?;
        let mut permissions = Vec::with_capacity(rows.len());
        for row in rows {
            let permission_code: String = row.try_get("permission_code")?;
            let description: String = row.try_get("description")?;
            permissions.push(PermissionRecord {
                permission_code,
                description,
            });
        }
        Ok(permissions)
    }
}
