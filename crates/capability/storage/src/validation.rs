//! 验证辅助函数
//!
//! 提供统一的验证逻辑，确保数据一致性：
//! - ensure_tenant：验证租户 ID 非空
//! - ensure_project_scope：验证项目归属（租户 + 项目作用域）
//!
//! 使用场景：
//! - 所有数据访问前验证租户上下文
//! - 项目资源访问前验证项目归属权限

use crate::error::StorageError;
use domain::TenantContext;

/// 验证租户 ID 非空
///
/// 确保所有数据访问都有有效的租户上下文。
pub fn ensure_tenant(ctx: &TenantContext) -> Result<(), StorageError> {
    if ctx.tenant_id.is_empty() {
        return Err(StorageError::new("tenant_id required"));
    }
    Ok(())
}

/// 验证项目归属
///
/// 确保在正确的项目作用域内访问项目资源。
pub fn ensure_project_scope(ctx: &TenantContext, project_id: &str) -> Result<(), StorageError> {
    ensure_tenant(ctx)?;
    if let Some(scope) = ctx.project_scope.as_deref() {
        if scope != project_id {
            return Err(StorageError::new("project scope mismatch"));
        }
    }
    Ok(())
}
