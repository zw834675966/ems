//! # 路由定义模块
//!
//! 集中管理所有 HTTP API 路由，将 URL 路径映射到对应的请求处理器（handlers）。
//!
//! ## 技术栈
//!
//! - 使用 **Axum** Web 框架构建路由器
//! - 支持路由组合、路径参数提取、多方法绑定等特性
//!
//! ## 路由分类
//!
//! ```text
//! ├─ 系统健康检查
//! │   ├─ /health          - 基础健康检查
//! │   ├─ /livez           - 存活检查（Liveness Probe）
//! │   ├─ /readyz          - 就绪检查（Readiness Probe）
//! │   └─ /metrics         - Prometheus 指标端点
//!
//! ├─ 认证与授权
//! │   ├─ /login                   - 用户登录（获取 JWT 令牌）
//! │   ├─ /refresh-token           - 刷新访问令牌
//! │   └─ /get-async-routes        - 获取异步路由配置
//!
//! ├─ RBAC 权限管理（基于角色的访问控制）
//! │   ├─ /rbac/users                      - 用户列表（GET）/ 创建用户（POST）
//! │   ├─ /rbac/users/:user_id            - 更新用户信息（PUT）
//! │   ├─ /rbac/users/:user_id/roles      - 设置用户角色（PUT）
//! │   ├─ /rbac/roles                      - 角色列表（GET）/ 创建角色（POST）
//! │   ├─ /rbac/roles/:role_code           - 删除角色（DELETE）
//! │   ├─ /rbac/roles/:role_code/permissions - 设置角色权限（PUT）
//! │   └─ /rbac/permissions                - 权限列表（GET）
//!
//! ├─ 项目管理
//! │   ├─ /projects               - 项目列表（GET）/ 创建项目（POST）
//! │   └─ /projects/:project_id   - 获取项目（GET）/ 更新（PUT）/ 删除（DELETE）
//!
//! ├─ 网关管理
//! │   ├─ /projects/:project_id/gateways                 - 网关列表（GET）/ 创建网关（POST）
//! │   ├─ /projects/:project_id/gateways/:gateway_id      - 获取网关（GET）/ 更新（PUT）/ 删除（DELETE）
//! │   └─ /projects/:project_id/gateways/:gateway_id/test  - 测试网关连接（POST）
//!
//! ├─ 设备管理
//! │   ├─ /projects/:project_id/devices                 - 设备列表（GET）/ 创建设备（POST）
//! │   └─ /projects/:project_id/devices/:device_id      - 获取设备（GET）/ 更新（PUT）/ 删除（DELETE）
//!
//! ├─ 测点管理
//! │   ├─ /projects/:project_id/points                 - 测点列表（GET）/ 创建测点（POST）
//! │   ├─ /projects/:project_id/points/:point_id       - 获取测点（GET）/ 更新（PUT）/ 删除（DELETE）
//! │   ├─ /points/batch                                 - 批量获取测点（GET）
//! │   └─ /projects/:project_id/points/:point_id/test  - 测试测点读写（POST）
//!
//! ├─ 测点映射管理
//! │   ├─ /projects/:project_id/point-mappings                 - 映射列表（GET）/ 创建映射（POST）
//! │   └─ /projects/:project_id/point-mappings/:source_id      - 获取映射（GET）/ 更新（PUT）/ 删除（DELETE）
//!
//! ├─ 实时与历史数据
//! │   ├─ /projects/:project_id/realtime        - 获取实时数据（最新值）
//! │   └─ /projects/:project_id/measurements    - 获取历史测量数据
//!
//! ├─ 控制命令管理
//! │   ├─ /projects/:project_id/commands                        - 指令列表（GET）/ 创建指令（POST）
//! │   └─ /projects/:project_id/commands/:command_id/receipts   - 获取指令回执列表（GET）
//!
//! ├─ 日志管理
//! │   ├─ /projects/:project_id/audit                       - 审计日志（GET）
//! │   ├─ /projects/:project_id/system-logs                  - 系统日志（GET）
//! │   ├─ /projects/:project_id/system-logs/unread           - 获取未读日志数量（GET）
//! │   └─ /projects/:project_id/system-logs/read             - 标记为已读（POST）
//!
//! └─ 采集策略管理
//!     ├─ /projects/:project_id/collection-strategies                    - 策略列表（GET）/ 创建/更新策略（POST）
//!     ├─ /projects/:project_id/collection-strategies/batch-enabled     - 批量更新启用状态（POST）
//!     └─ /projects/:project_id/collection-strategies/:strategy_id      - 删除策略（DELETE）
//! ```

use super::AppState;
use super::handlers::*;
use axum::{
    Router,
    routing::{delete, get, post, put},
};

/// 创建 API 路由器
///
/// 返回包含所有 API 端点的 Router，支持 / 和 /api/ 两种前缀
pub fn create_api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
        .route("/metrics", get(get_metrics))
        .route("/login", post(login))
        .route("/refresh-token", post(refresh_token))
        .route("/get-async-routes", get(get_async_routes))
        .route("/rbac/users", get(list_rbac_users).post(create_rbac_user))
        .route("/rbac/users/:user_id", put(update_rbac_user))
        .route("/rbac/users/:user_id/roles", put(set_rbac_user_roles))
        .route("/rbac/roles", get(list_rbac_roles).post(create_rbac_role))
        .route("/rbac/roles/:role_code", delete(delete_rbac_role))
        .route(
            "/rbac/roles/:role_code/permissions",
            put(set_rbac_role_permissions),
        )
        .route("/rbac/permissions", get(list_rbac_permissions))
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/:project_id",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route(
            "/projects/:project_id/gateways",
            get(list_gateways).post(create_gateway),
        )
        .route(
            "/projects/:project_id/gateways/:gateway_id",
            get(get_gateway).put(update_gateway).delete(delete_gateway),
        )
        .route(
            "/projects/:project_id/gateways/:gateway_id/test",
            post(test_gateway),
        )
        .route(
            "/projects/:project_id/devices",
            get(list_devices).post(create_device),
        )
        .route(
            "/projects/:project_id/devices/:device_id",
            get(get_device).put(update_device).delete(delete_device),
        )
        .route(
            "/projects/:project_id/points",
            get(list_points).post(create_point),
        )
        .route("/projects/:project_id/realtime", get(get_realtime))
        .route("/projects/:project_id/measurements", get(list_measurements))
        .route(
            "/projects/:project_id/points/:point_id",
            get(get_point).put(update_point).delete(delete_point),
        )
        .route("/points/batch", get(list_points_batch))
        .route(
            "/projects/:project_id/points/:point_id/test",
            post(test_point),
        )
        .route(
            "/projects/:project_id/modbus/snapshot",
            post(modbus_snapshot),
        )
        .route(
            "/projects/:project_id/point-mappings",
            get(list_point_mappings).post(create_point_mapping),
        )
        .route(
            "/projects/:project_id/point-mappings/:source_id",
            get(get_point_mapping)
                .put(update_point_mapping)
                .delete(delete_point_mapping),
        )
        .route(
            "/projects/:project_id/commands",
            get(list_commands).post(create_command),
        )
        .route(
            "/projects/:project_id/commands/:command_id/receipts",
            get(list_command_receipts),
        )
        .route("/projects/:project_id/audit", get(list_audit_logs))
        .route("/projects/:project_id/system-logs", get(list_system_logs))
        .route(
            "/projects/:project_id/system-logs/unread",
            get(get_unread_count),
        )
        .route("/projects/:project_id/system-logs/read", post(mark_as_read))
        .route(
            "/projects/:project_id/collection-strategies",
            get(list_strategies).post(upsert_strategies),
        )
        .route(
            "/projects/:project_id/collection-strategies/batch-enabled",
            post(batch_update_enabled),
        )
        .route(
            "/projects/:project_id/collection-strategies/:strategy_id",
            axum::routing::delete(delete_strategy),
        )
}
