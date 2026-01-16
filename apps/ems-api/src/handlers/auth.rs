//! 认证相关 handlers：登录、刷新 token、获取动态路由
//!
//! ## 提供的端点
//!
//! ### 公开端点（无需认证）
//! - `GET /health` - 健康检查，返回 `{"ok": true}`
//! - `POST /login` - 用户登录，验证用户名密码后返回 access/refresh token
//! - `POST /refresh-token` - 使用 refresh token 刷新 access token
//!
//! ### 私有端点（需 Bearer token 认证）
//! - `GET /get-async-routes` - 根据用户角色和权限返回前端路由配置
//!
//! ## 认证流程
//!
//! ### 登录流程
//! 1. 客户端发送用户名密码
//! 2. 服务端调用 `AuthService::login()` 验证凭据
//! 3. 验证成功后，返回：
//!    - `access_token`: 短期有效的访问令牌（用于 API 调用）
//!    - `refresh_token`: 长期有效的刷新令牌（用于换取新的 access token）
//!    - `expires`: 过期时间（Unix 毫秒时间戳）
//!    - 用户基本信息：用户名、昵称、角色、权限列表
//!
//! ### Token 刷新流程
//! 1. 客户端使用 refresh token 请求新 token
//! 2. 服务端验证 refresh token 的有效性
//! 3. 验证通过后，签发新的 access/refresh token 对
//!
//! ### 动态路由流程
//! 1. 客户端携带 Bearer access token 请求路由配置
//! 2. 中间件 `require_tenant_context` 验证 token 并提取用户上下文（TenantContext）
//! 3. 根据用户的角色和权限动态构建路由树
//! 4. 返回符合前端框架（pure-admin-thin）要求的路由配置

use crate::AppState;
use crate::middleware::require_tenant_context;
use crate::utils::response::{auth_error, internal_auth_error};
use api_contract::{
    ApiResponse, AsyncRoute, LoginRequest, LoginResponse, RefreshTokenRequest,
    RefreshTokenResponse, RouteMeta,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use ems_auth::AuthError;

/// 健康检查端点
///
/// 无需认证，返回简单的健康状态。可用于负载均衡器健康探针或服务监控。
///
/// # Returns
///
/// JSON 响应：`{"ok": true}`
///
/// # HTTP 状态码
///
/// - `200 OK`: 服务正常运行
pub async fn health() -> impl IntoResponse {
    livez().await
}

/// Liveness 探针：只反映进程存活，不做外部依赖检查。
pub async fn livez() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

/// Readiness 探针：用于反映关键依赖是否就绪（当前检查 Postgres 连接）。
pub async fn readyz(State(state): State<AppState>) -> Response {
    let Some(pool) = state.db_pool.as_ref() else {
        return (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response();
    };

    match sqlx::query_scalar::<_, i32>("select 1").fetch_one(pool).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(err) => {
            tracing::warn!(error = %err, "readyz check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "ok": false })),
            )
                .into_response()
        }
    }
}

/// 登录接口
///
/// 验证用户名和密码，成功后返回 access token、refresh token 和用户信息。
///
/// # Arguments
///
/// * `state` - 应用状态，包含认证服务实例
/// * `req` - 登录请求，包含 `username` 和 `password`
///
/// # Returns
///
/// 成功时返回 `200 OK` 和包含以下字段的 `LoginResponse`：
/// - `access_token`: 访问令牌（短期有效）
/// - `refresh_token`: 刷新令牌（长期有效）
/// - `expires`: 过期时间（Unix 毫秒时间戳）
/// - `username`: 用户名
/// - `nickname`: 昵称（当前与 username 相同）
/// - `avatar`: 头像 URL（当前为空字符串）
/// - `roles`: 角色列表
/// - `permissions`: 权限列表
///
/// # Errors
///
/// - `401 UNAUTHORIZED`: 用户名或密码错误（`InvalidCredentials`）
/// - `500 INTERNAL SERVER ERROR`: 认证服务内部错误
pub async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Response {
    // 调用认证服务的登录方法验证用户凭据
    match state.auth.login(&req.username, &req.password).await {
        Ok((user, tokens)) => {
            // 登录成功，构建响应
            let response = LoginResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                // 将秒级时间戳转换为毫秒级（前端期望的时间戳格式）
                expires: tokens.expires_at.saturating_mul(1000),
                username: user.username.clone(),
                nickname: user.username,
                avatar: "".to_string(), // 当前版本未实现头像功能
                roles: user.roles,
                permissions: user.permissions,
            };
            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        // 用户名或密码错误，返回 401
        Err(AuthError::InvalidCredentials) => auth_error(StatusCode::UNAUTHORIZED),
        // 其他认证服务错误，返回 500
        Err(err) => internal_auth_error(err),
    }
}

/// 刷新 access token
///
/// 使用 refresh token 换取新的 access token 和 refresh token。
///
/// # Arguments
///
/// * `state` - 应用状态，包含认证服务实例
/// * `req` - 刷新 token 请求，包含 `refresh_token`
///
/// # Returns
///
/// 成功时返回 `200 OK` 和包含以下字段的 `RefreshTokenResponse`：
/// - `access_token`: 新的访问令牌
/// - `refresh_token`: 新的刷新令牌（旧 refresh token 同时失效）
/// - `expires`: 过期时间（Unix 毫秒时间戳）
///
/// # Errors
///
/// - `401 UNAUTHORIZED`: refresh token 无效或已过期
/// - `500 INTERNAL SERVER ERROR`: 认证服务内部错误
///
/// # Note
///
/// 每次刷新都会返回新的 refresh token，这是推荐的安全实践，称为 "refresh token rotation"。
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Response {
    // 验证 refresh token 并生成新的 token 对
    match state.auth.refresh(&req.refresh_token).await {
        Ok(tokens) => {
            let response = RefreshTokenResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                // 将秒级时间戳转换为毫秒级（前端期望的时间戳格式）
                expires: tokens.expires_at.saturating_mul(1000),
            };
            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        // token 无效或已过期，返回 401
        Err(AuthError::TokenInvalid | AuthError::TokenExpired) => {
            auth_error(StatusCode::UNAUTHORIZED)
        }
        // 其他认证服务错误，返回 500
        Err(err) => internal_auth_error(err),
    }
}

/// 获取动态路由
///
/// 根据用户的角色和权限动态生成前端路由配置。前端使用返回的路由配置构建导航菜单和页面路由。
///
/// # Arguments
///
/// * `state` - 应用状态
/// * `headers` - HTTP 请求头，用于提取 Bearer token
///
/// # Returns
///
/// 成功时返回 `200 OK` 和路由配置数组。每个路由包含：
/// - `path`: 路由路径
/// - `name`: 路由名称（用于路由组件）
/// - `component`: 组件路径
/// - `meta`: 路由元数据（标题、图标、排序、角色限制、权限要求）
/// - `children`: 子路由数组
///
/// # Authentication
///
/// 需要 Bearer access token，通过 `require_tenant_context` 中间件验证。
///
/// # Route Structure
///
/// ```
/// /ems (根路由)
/// ├── /ems/projects (项目管理) - 需要 PROJECT.READ 或 PROJECT.WRITE 权限
/// ├── /ems/gateways (网关管理) - 需要 ASSET.GATEWAY.READ 或 ASSET.GATEWAY.WRITE 权限
/// ├── /ems/devices (设备管理) - 需要 ASSET.DEVICE.READ 或 ASSET.DEVICE.WRITE 权限
/// ├── /ems/points (点位管理) - 需要 ASSET.POINT.READ 或 ASSET.POINT.WRITE 权限
/// ├── /ems/point-mappings (点位映射) - 需要 ASSET.POINT.READ 或 ASSET.POINT.WRITE 权限
/// ├── /ems/realtime (实时查询) - 需要 DATA.REALTIME.READ 权限
/// ├── /ems/measurements (历史查询) - 需要 DATA.MEASUREMENTS.READ 权限
/// ├── /ems/commands (控制命令) - 需要 CONTROL.COMMAND.ISSUE 或 CONTROL.COMMAND.READ 权限
/// └── /ems/audit (审计日志) - 需要 CONTROL.COMMAND.READ 权限
/// ```
///
/// # Frontend Compatibility
///
/// 路由格式兼容 [pure-admin-thin](https://github.com/pure-admin/pure-admin-thin) 框架。
/// 叶子节点省略 `children` 字段（使用空数组 `Vec::new()` 以避免前端菜单过滤）。
///
/// # Errors
///
/// - `401 UNAUTHORIZED`: 未提供 token 或 token 无效/已过期
pub async fn get_async_routes(State(state): State<AppState>, headers: HeaderMap) -> Response {
    // 验证 Bearer token 并提取租户上下文（包含用户角色和权限）
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    // 准备用户角色数据：如果角色列表为空则返回 None，否则克隆列表
    let roles = if ctx.roles.is_empty() {
        None
    } else {
        Some(ctx.roles.clone())
    };

    // 路由元数据构造闭包
    // title: 路由标题
    // icon: 图标标识符 (Iconify 格式，如 "ri:xyz")
    // rank: 路由排序（数字越小越靠前）
    // auths: 访问此路由所需的权限列表（None 表示无权限要求）
    let meta = |title: &str, icon: &str, rank: i32, auths: Option<Vec<String>>| RouteMeta {
        title: title.to_string(),
        icon: icon.to_string(),
        rank,
        roles: roles.clone(), // 将用户角色绑定到路由元数据（前端用于角色过滤）
        auths,
    };

    // 构建路由树
    let routes = vec![AsyncRoute {
        path: "/ems".to_string(),
        name: "EMS".to_string(),
        component: "Layout".to_string(), // 根路由使用布局组件
        meta: meta("EMS", "ri:flashlight-line", 1, None), // 根路由无权限限制
        children: vec![
            // 项目管理路由
            AsyncRoute {
                path: "/ems/projects".to_string(),
                name: "EmsProjects".to_string(),
                component: "ems/projects/index".to_string(),
                meta: meta(
                    "项目",
                    "ri:folders-line",
                    1,
                    // 拥有任一权限即可访问（前端逻辑：auths 为权限数组，满足其一即可）
                    Some(vec![
                        permissions::PROJECT_READ.to_string(),
                        permissions::PROJECT_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(), // 叶子节点使用空数组而非省略
            },
            // 网关管理路由
            AsyncRoute {
                path: "/ems/gateways".to_string(),
                name: "EmsGateways".to_string(),
                component: "ems/gateways/index".to_string(),
                meta: meta(
                    "网关",
                    "ri:router-line",
                    2,
                    Some(vec![
                        permissions::ASSET_GATEWAY_READ.to_string(),
                        permissions::ASSET_GATEWAY_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            // 设备管理路由
            AsyncRoute {
                path: "/ems/devices".to_string(),
                name: "EmsDevices".to_string(),
                component: "ems/devices/index".to_string(),
                meta: meta(
                    "设备",
                    "ri:cpu-line",
                    3,
                    Some(vec![
                        permissions::ASSET_DEVICE_READ.to_string(),
                        permissions::ASSET_DEVICE_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            // 点位管理路由
            AsyncRoute {
                path: "/ems/points".to_string(),
                name: "EmsPoints".to_string(),
                component: "ems/points/index".to_string(),
                meta: meta(
                    "点位",
                    "ri:node-tree",
                    4,
                    Some(vec![
                        permissions::ASSET_POINT_READ.to_string(),
                        permissions::ASSET_POINT_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            // 点位映射路由
            AsyncRoute {
                path: "/ems/point-mappings".to_string(),
                name: "EmsPointMappings".to_string(),
                component: "ems/point-mappings/index".to_string(),
                meta: meta(
                    "点位映射",
                    "ri:links-line",
                    5,
                    Some(vec![
                        permissions::ASSET_POINT_READ.to_string(),
                        permissions::ASSET_POINT_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            // 实时查询路由
            AsyncRoute {
                path: "/ems/realtime".to_string(),
                name: "EmsRealtime".to_string(),
                component: "ems/realtime/index".to_string(),
                meta: meta(
                    "实时",
                    "ri:pulse-line",
                    6,
                    Some(vec![permissions::DATA_REALTIME_READ.to_string()]),
                ),
                children: Vec::new(),
            },
            // 历史查询路由
            AsyncRoute {
                path: "/ems/measurements".to_string(),
                name: "EmsMeasurements".to_string(),
                component: "ems/measurements/index".to_string(),
                meta: meta(
                    "历史",
                    "ri:history-line",
                    7,
                    Some(vec![permissions::DATA_MEASUREMENTS_READ.to_string()]),
                ),
                children: Vec::new(),
            },
            // 控制命令路由
            AsyncRoute {
                path: "/ems/commands".to_string(),
                name: "EmsCommands".to_string(),
                component: "ems/commands/index".to_string(),
                meta: meta(
                    "控制",
                    "ri:terminal-window-line",
                    8,
                    Some(vec![
                        permissions::CONTROL_COMMAND_ISSUE.to_string(),
                        permissions::CONTROL_COMMAND_READ.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            // 审计日志路由
            AsyncRoute {
                path: "/ems/audit".to_string(),
                name: "EmsAudit".to_string(),
                component: "ems/audit/index".to_string(),
                meta: meta(
                    "审计",
                    "ri:shield-user-line",
                    9,
                    Some(vec![permissions::CONTROL_COMMAND_READ.to_string()]),
                ),
                children: Vec::new(),
            },
            // RBAC 用户管理（tenant 级）
            AsyncRoute {
                path: "/ems/rbac-users".to_string(),
                name: "EmsRbacUsers".to_string(),
                component: "ems/rbac/users/index".to_string(),
                meta: meta(
                    "用户",
                    "ri:user-settings-line",
                    10,
                    Some(vec![
                        permissions::RBAC_USER_READ.to_string(),
                        permissions::RBAC_USER_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            // RBAC 角色管理（tenant 级）
            AsyncRoute {
                path: "/ems/rbac-roles".to_string(),
                name: "EmsRbacRoles".to_string(),
                component: "ems/rbac/roles/index".to_string(),
                meta: meta(
                    "角色",
                    "ri:team-line",
                    11,
                    Some(vec![
                        permissions::RBAC_ROLE_READ.to_string(),
                        permissions::RBAC_ROLE_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
        ],
    }];

    (StatusCode::OK, Json(ApiResponse::success(routes))).into_response()
}

/// 单元测试模块
#[cfg(test)]
mod tests {
    use crate::middleware::bearer_token;
    use axum::http::{HeaderMap, HeaderValue, header};

    /// 测试 `bearer_token` 函数能正确从 Authorization 头提取 Bearer token
    #[test]
    fn bearer_token_extracts() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token-1"),
        );
        // 验证能正确提取 "Bearer " 前缀后的 token
        assert_eq!(bearer_token(&headers), Some("token-1"));
    }
}
