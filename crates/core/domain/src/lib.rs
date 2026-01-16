pub mod data;
pub mod permissions;

pub use data::{PointValue, PointValueData, RawEvent};

/// 租户上下文：所有模块共享的执行上下文。
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub project_scope: Option<String>,
}

impl TenantContext {
    /// 构造显式身份与权限范围的租户上下文。
    pub fn new(
        tenant_id: impl Into<String>,
        user_id: impl Into<String>,
        roles: Vec<String>,
        permissions: Vec<String>,
        project_scope: Option<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_id: user_id.into(),
            roles,
            permissions,
            project_scope,
        }
    }
}

impl Default for TenantContext {
    /// 空上下文（仅用于测试或占位）。
    fn default() -> Self {
        Self {
            tenant_id: "".to_string(),
            user_id: "".to_string(),
            roles: Vec::new(),
            permissions: Vec::new(),
            project_scope: None,
        }
    }
}
