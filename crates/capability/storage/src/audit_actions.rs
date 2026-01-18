//! 审计日志操作类型常量
//!
//! 定义所有审计操作的字符串常量，用于构建审计日志记录。
//!
//! ## 操作分类
//!
//! - `AUTH.*`: 认证相关事件
//! - `RBAC.*`: 权限/角色管理变更
//! - `RESOURCE.*`: 资源 CRUD 操作
//! - `CONTROL.*`: 控制命令相关

// ============================================================================
// 认证事件
// ============================================================================

/// 用户登录成功
pub const AUTH_LOGIN: &str = "AUTH.LOGIN";

/// 用户登录失败
pub const AUTH_LOGIN_FAILED: &str = "AUTH.LOGIN.FAILED";

/// Token 刷新成功
pub const AUTH_REFRESH: &str = "AUTH.REFRESH";

/// Token 刷新失败
pub const AUTH_REFRESH_FAILED: &str = "AUTH.REFRESH.FAILED";

/// 用户登出（预留）
pub const AUTH_LOGOUT: &str = "AUTH.LOGOUT";

// ============================================================================
// RBAC 事件
// ============================================================================

/// 创建用户
pub const RBAC_USER_CREATE: &str = "RBAC.USER.CREATE";

/// 更新用户信息
pub const RBAC_USER_UPDATE: &str = "RBAC.USER.UPDATE";

/// 用户角色变更
pub const RBAC_USER_ROLES_CHANGE: &str = "RBAC.USER.ROLES.CHANGE";

/// 用户状态变更（启用/禁用）
pub const RBAC_USER_STATUS_CHANGE: &str = "RBAC.USER.STATUS.CHANGE";

/// 创建角色
pub const RBAC_ROLE_CREATE: &str = "RBAC.ROLE.CREATE";

/// 删除角色
pub const RBAC_ROLE_DELETE: &str = "RBAC.ROLE.DELETE";

/// 角色权限变更
pub const RBAC_ROLE_PERMISSIONS_CHANGE: &str = "RBAC.ROLE.PERMISSIONS.CHANGE";

// ============================================================================
// 资源 CRUD 事件
// ============================================================================

/// 创建项目
pub const RESOURCE_PROJECT_CREATE: &str = "RESOURCE.PROJECT.CREATE";

/// 更新项目
pub const RESOURCE_PROJECT_UPDATE: &str = "RESOURCE.PROJECT.UPDATE";

/// 删除项目
pub const RESOURCE_PROJECT_DELETE: &str = "RESOURCE.PROJECT.DELETE";

/// 创建网关
pub const RESOURCE_GATEWAY_CREATE: &str = "RESOURCE.GATEWAY.CREATE";

/// 更新网关
pub const RESOURCE_GATEWAY_UPDATE: &str = "RESOURCE.GATEWAY.UPDATE";

/// 删除网关
pub const RESOURCE_GATEWAY_DELETE: &str = "RESOURCE.GATEWAY.DELETE";

/// 创建设备
pub const RESOURCE_DEVICE_CREATE: &str = "RESOURCE.DEVICE.CREATE";

/// 更新设备
pub const RESOURCE_DEVICE_UPDATE: &str = "RESOURCE.DEVICE.UPDATE";

/// 删除设备
pub const RESOURCE_DEVICE_DELETE: &str = "RESOURCE.DEVICE.DELETE";

/// 创建点位
pub const RESOURCE_POINT_CREATE: &str = "RESOURCE.POINT.CREATE";

/// 更新点位
pub const RESOURCE_POINT_UPDATE: &str = "RESOURCE.POINT.UPDATE";

/// 删除点位
pub const RESOURCE_POINT_DELETE: &str = "RESOURCE.POINT.DELETE";

/// 创建点位映射
pub const RESOURCE_POINT_MAPPING_CREATE: &str = "RESOURCE.POINT_MAPPING.CREATE";

/// 更新点位映射
pub const RESOURCE_POINT_MAPPING_UPDATE: &str = "RESOURCE.POINT_MAPPING.UPDATE";

/// 删除点位映射
pub const RESOURCE_POINT_MAPPING_DELETE: &str = "RESOURCE.POINT_MAPPING.DELETE";

// ============================================================================
// 控制命令事件
// ============================================================================

/// 下发控制命令
pub const CONTROL_COMMAND_ISSUE: &str = "CONTROL.COMMAND.ISSUE";

/// 控制命令回执（已有）
pub const CONTROL_COMMAND_RECEIPT: &str = "CONTROL.COMMAND.RECEIPT";

// ============================================================================
// 审计结果
// ============================================================================

/// 操作成功
pub const RESULT_SUCCESS: &str = "SUCCESS";

/// 操作失败
pub const RESULT_FAILURE: &str = "FAILURE";

/// 操作被拒绝（权限不足）
pub const RESULT_DENIED: &str = "DENIED";
