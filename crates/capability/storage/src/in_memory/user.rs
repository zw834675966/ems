//! 用户内存存储实现
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 功能：
//! - 内置 admin 账户（用户名：admin，密码：admin123）
//! - 根据用户名查找用户

use crate::error::StorageError;
use crate::models::{
    PermissionRecord, RbacRoleCreate, RbacRoleRecord, RbacUserCreate, RbacUserRecord,
    RbacUserUpdate, UserRecord,
};
use crate::traits::{RbacStore, UserStore};
use domain::TenantContext;

/// 用户内存存储
///
/// 使用 RwLock + HashMap 提供线程安全的内存存储。
pub struct InMemoryUserStore {
    users: std::sync::RwLock<std::collections::HashMap<String, UserInternal>>,
    usernames: std::sync::RwLock<std::collections::HashMap<String, String>>,
    roles: std::sync::RwLock<std::collections::HashMap<String, RoleInternal>>,
}

#[derive(Debug, Clone)]
struct UserInternal {
    tenant_id: String,
    user_id: String,
    username: String,
    password: String,
    refresh_jti: Option<String>,
    status: String,
    roles: Vec<String>,
}

#[derive(Debug, Clone)]
struct RoleInternal {
    tenant_id: String,
    role_code: String,
    name: String,
    permissions: Vec<String>,
}

fn tenant_role_key(tenant_id: &str, role_code: &str) -> String {
    format!("tenant:{}:role:{}", tenant_id, role_code)
}

impl InMemoryUserStore {
    /// 内置 admin 账户
    ///
    /// 创建包含默认 admin 用户的存储。
    pub fn with_default_admin() -> Self {
        let tenant_id = "tenant-1".to_string();
        let role_code = domain::permissions::ROLE_ADMIN.to_string();
        let role_key = tenant_role_key(&tenant_id, &role_code);
        let role = RoleInternal {
            tenant_id: tenant_id.clone(),
            role_code: role_code.clone(),
            name: "Administrator".to_string(),
            permissions: domain::permissions::PERMISSION_CODES
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
        };
        let mut roles = std::collections::HashMap::new();
        roles.insert(role_key, role);

        let user = UserInternal {
            tenant_id: tenant_id.clone(),
            user_id: "user-1".to_string(),
            username: "admin".to_string(),
            password: "admin123".to_string(),
            refresh_jti: None,
            status: "active".to_string(),
            roles: vec![role_code],
        };
        let mut users = std::collections::HashMap::new();
        users.insert(user.user_id.clone(), user.clone());
        let mut usernames = std::collections::HashMap::new();
        usernames.insert(user.username.clone(), user.user_id.clone());
        Self {
            users: std::sync::RwLock::new(users),
            usernames: std::sync::RwLock::new(usernames),
            roles: std::sync::RwLock::new(roles),
        }
    }
}

#[async_trait::async_trait]
impl UserStore for InMemoryUserStore {
    async fn find_by_username(
        &self,
        _ctx: &TenantContext,
        username: &str,
    ) -> Result<Option<UserRecord>, StorageError> {
        let user_id = self
            .usernames
            .read()
            .ok()
            .and_then(|map| map.get(username).cloned());
        let Some(user_id) = user_id else {
            return Ok(None);
        };
        let user = self
            .users
            .read()
            .ok()
            .and_then(|map| map.get(&user_id).cloned());
        let Some(user) = user else {
            return Ok(None);
        };

        let roles = user.roles.clone();
        let role_map = self.roles.read().ok();
        let mut permissions: std::collections::HashSet<String> = std::collections::HashSet::new();
        if let Some(role_map) = role_map {
            for role_code in &roles {
                let key = tenant_role_key(&user.tenant_id, role_code);
                if let Some(role) = role_map.get(&key) {
                    for permission in &role.permissions {
                        permissions.insert(permission.clone());
                    }
                }
            }
        }
        let mut permissions: Vec<String> = permissions.into_iter().collect();
        permissions.sort();

        Ok(Some(UserRecord {
            tenant_id: user.tenant_id,
            user_id: user.user_id,
            username: user.username,
            password: user.password,
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
        let mut users = self.users.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let Some(user) = users.get_mut(user_id) else {
            return Ok(false);
        };
        if !ctx.tenant_id.is_empty() && user.tenant_id != ctx.tenant_id {
            return Ok(false);
        }
        user.password = password_hash.to_string();
        Ok(true)
    }

    async fn get_refresh_jti(
        &self,
        ctx: &TenantContext,
        user_id: &str,
    ) -> Result<Option<String>, StorageError> {
        let users = self.users.read().map_err(|_| StorageError::new("lock poisoned"))?;
        let Some(user) = users.get(user_id) else {
            return Ok(None);
        };
        if !ctx.tenant_id.is_empty() && user.tenant_id != ctx.tenant_id {
            return Ok(None);
        }
        Ok(user.refresh_jti.clone())
    }

    async fn set_refresh_jti(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        refresh_jti: Option<&str>,
    ) -> Result<bool, StorageError> {
        let mut users = self.users.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let Some(user) = users.get_mut(user_id) else {
            return Ok(false);
        };
        if !ctx.tenant_id.is_empty() && user.tenant_id != ctx.tenant_id {
            return Ok(false);
        }
        user.refresh_jti = refresh_jti.map(|value| value.to_string());
        Ok(true)
    }
}

#[async_trait::async_trait]
impl RbacStore for InMemoryUserStore {
    async fn list_users(&self, ctx: &TenantContext) -> Result<Vec<RbacUserRecord>, StorageError> {
        let users = self.users.read().map_err(|_| StorageError::new("lock poisoned"))?;
        let mut result: Vec<RbacUserRecord> = users
            .values()
            .filter(|u| u.tenant_id == ctx.tenant_id)
            .map(|u| RbacUserRecord {
                tenant_id: u.tenant_id.clone(),
                user_id: u.user_id.clone(),
                username: u.username.clone(),
                status: u.status.clone(),
                roles: u.roles.clone(),
            })
            .collect();
        result.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(result)
    }

    async fn create_user(
        &self,
        _ctx: &TenantContext,
        record: RbacUserCreate,
    ) -> Result<RbacUserRecord, StorageError> {
        let mut users = self.users.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let mut usernames =
            self.usernames.write().map_err(|_| StorageError::new("lock poisoned"))?;
        if usernames.contains_key(&record.username) {
            return Err(StorageError::new("username already exists"));
        }
        let user = UserInternal {
            tenant_id: record.tenant_id.clone(),
            user_id: record.user_id.clone(),
            username: record.username.clone(),
            password: record.password.clone(),
            refresh_jti: None,
            status: record.status.clone(),
            roles: record.roles.clone(),
        };
        users.insert(user.user_id.clone(), user);
        usernames.insert(record.username.clone(), record.user_id.clone());
        Ok(RbacUserRecord {
            tenant_id: record.tenant_id,
            user_id: record.user_id,
            username: record.username,
            status: record.status,
            roles: record.roles,
        })
    }

    async fn update_user(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        update: RbacUserUpdate,
    ) -> Result<Option<RbacUserRecord>, StorageError> {
        let mut users = self.users.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let Some(user) = users.get_mut(user_id) else {
            return Ok(None);
        };
        if user.tenant_id != ctx.tenant_id {
            return Ok(None);
        }
        if let Some(password) = update.password {
            user.password = password;
            user.refresh_jti = None;
        }
        if let Some(status) = update.status {
            user.status = status;
        }
        Ok(Some(RbacUserRecord {
            tenant_id: user.tenant_id.clone(),
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            status: user.status.clone(),
            roles: user.roles.clone(),
        }))
    }

    async fn set_user_roles(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        roles: Vec<String>,
    ) -> Result<Option<RbacUserRecord>, StorageError> {
        let mut users = self.users.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let Some(user) = users.get_mut(user_id) else {
            return Ok(None);
        };
        if user.tenant_id != ctx.tenant_id {
            return Ok(None);
        }
        user.roles = roles.clone();
        Ok(Some(RbacUserRecord {
            tenant_id: user.tenant_id.clone(),
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            status: user.status.clone(),
            roles,
        }))
    }

    async fn list_roles(&self, ctx: &TenantContext) -> Result<Vec<RbacRoleRecord>, StorageError> {
        let roles = self.roles.read().map_err(|_| StorageError::new("lock poisoned"))?;
        let mut result: Vec<RbacRoleRecord> = roles
            .values()
            .filter(|r| r.tenant_id == ctx.tenant_id)
            .map(|r| RbacRoleRecord {
                tenant_id: r.tenant_id.clone(),
                role_code: r.role_code.clone(),
                name: r.name.clone(),
                permissions: r.permissions.clone(),
            })
            .collect();
        result.sort_by(|a, b| a.role_code.cmp(&b.role_code));
        Ok(result)
    }

    async fn create_role(
        &self,
        _ctx: &TenantContext,
        record: RbacRoleCreate,
    ) -> Result<RbacRoleRecord, StorageError> {
        let mut roles = self.roles.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let key = tenant_role_key(&record.tenant_id, &record.role_code);
        if roles.contains_key(&key) {
            return Err(StorageError::new("role already exists"));
        }
        roles.insert(
            key,
            RoleInternal {
                tenant_id: record.tenant_id.clone(),
                role_code: record.role_code.clone(),
                name: record.name.clone(),
                permissions: record.permissions.clone(),
            },
        );
        Ok(RbacRoleRecord {
            tenant_id: record.tenant_id,
            role_code: record.role_code,
            name: record.name,
            permissions: record.permissions,
        })
    }

    async fn delete_role(&self, ctx: &TenantContext, role_code: &str) -> Result<bool, StorageError> {
        let mut roles = self.roles.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let key = tenant_role_key(&ctx.tenant_id, role_code);
        let removed = roles.remove(&key).is_some();
        if removed {
            let mut users = self.users.write().map_err(|_| StorageError::new("lock poisoned"))?;
            for user in users.values_mut() {
                if user.tenant_id != ctx.tenant_id {
                    continue;
                }
                user.roles.retain(|r| r != role_code);
            }
        }
        Ok(removed)
    }

    async fn set_role_permissions(
        &self,
        ctx: &TenantContext,
        role_code: &str,
        permissions: Vec<String>,
    ) -> Result<Option<RbacRoleRecord>, StorageError> {
        let mut roles = self.roles.write().map_err(|_| StorageError::new("lock poisoned"))?;
        let key = tenant_role_key(&ctx.tenant_id, role_code);
        let Some(role) = roles.get_mut(&key) else {
            return Ok(None);
        };
        role.permissions = permissions.clone();
        Ok(Some(RbacRoleRecord {
            tenant_id: role.tenant_id.clone(),
            role_code: role.role_code.clone(),
            name: role.name.clone(),
            permissions,
        }))
    }

    async fn list_permissions(
        &self,
        _ctx: &TenantContext,
    ) -> Result<Vec<PermissionRecord>, StorageError> {
        Ok(domain::permissions::PERMISSION_CODES
            .iter()
            .map(|code| PermissionRecord {
                permission_code: (*code).to_string(),
                description: (*code).to_string(),
            })
            .collect())
    }
}
