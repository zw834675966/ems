//! 用户内存存储实现
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 功能：
//! - 内置 admin 账户（用户名：admin，密码：admin123）
//! - 根据用户名查找用户

use crate::error::StorageError;
use crate::models::UserRecord;
use crate::traits::UserStore;
use domain::TenantContext;

/// 用户内存存储
///
/// 使用 RwLock + HashMap 提供线程安全的内存存储。
pub struct InMemoryUserStore {
    users: std::sync::RwLock<std::collections::HashMap<String, UserRecord>>,
}

impl InMemoryUserStore {
    /// 内置 admin 账户
    ///
    /// 创建包含默认 admin 用户的存储。
    pub fn with_default_admin() -> Self {
        let mut users = std::collections::HashMap::new();
        users.insert(
            "admin".to_string(),
            UserRecord {
                tenant_id: "tenant-1".to_string(),
                user_id: "user-1".to_string(),
                username: "admin".to_string(),
                password: "admin123".to_string(),
                roles: vec![domain::permissions::ROLE_ADMIN.to_string()],
                permissions: domain::permissions::PERMISSION_CODES
                    .iter()
                    .map(|code| (*code).to_string())
                    .collect(),
            },
        );
        Self {
            users: std::sync::RwLock::new(users),
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
        Ok(self
            .users
            .read()
            .ok()
            .and_then(|map| map.get(username).cloned()))
    }
}
