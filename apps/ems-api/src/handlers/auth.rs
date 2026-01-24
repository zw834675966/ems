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
use crate::utils::audit::extract_client_ip;
use crate::utils::response::{auth_error, internal_auth_error};
use api_contract::{
    ApiResponse, AsyncRoute, LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use domain::{TenantContext, permissions};
use ems_auth::AuthError;
use ems_storage::{AuditBuilder, audit_actions};

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

    match sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(pool)
        .await
    {
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
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Response {
    // 提取客户端 IP（用于审计日志）
    let client_ip = extract_client_ip(&headers).unwrap_or_else(|| "unknown".to_string());

    // 调用认证服务的登录方法验证用户凭据
    match state.auth.login(&req.username, &req.password).await {
        Ok((user, tokens)) => {
            // 登录成功，记录审计日志
            let ctx = user.to_tenant_context();
            let audit_record = AuditBuilder::new(&ctx, audit_actions::AUTH_LOGIN)
                .tenant_scope() // 登录是租户级操作，无项目作用域
                .resource_typed("user", &user.username)
                .success()
                .with_client_ip(&client_ip)
                .build();

            // 异步写入审计日志（不阻塞响应）
            let audit_store = state.audit_log_store.clone();
            tokio::spawn(async move {
                if let Err(e) = audit_store.create_audit_log(&ctx, audit_record).await {
                    tracing::warn!(error = %e, "failed to write login audit log");
                }
            });

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
        Err(AuthError::InvalidCredentials) => {
            // 登录失败，记录审计日志
            let audit_record =
                AuditBuilder::anonymous("default", &req.username, audit_actions::AUTH_LOGIN_FAILED)
                    .resource_typed("user", &req.username)
                    .failure()
                    .with_client_ip(&client_ip)
                    .with_field("reason", "invalid_credentials")
                    .build();

            // 异步写入审计日志
            let audit_store = state.audit_log_store.clone();
            let ctx = domain::TenantContext::default();
            tokio::spawn(async move {
                if let Err(e) = audit_store.create_audit_log(&ctx, audit_record).await {
                    tracing::warn!(error = %e, "failed to write login failure audit log");
                }
            });

            auth_error(StatusCode::UNAUTHORIZED)
        }
        // 其他认证服务错误
        Err(err) => {
            // 记录详细的内部错误原因
            tracing::error!(error = %err, username = %req.username, "login failed due to internal error");

            // 如果是内部错误，尝试判断是否可能是数据库连接问题
            // 这里我们简单地返回 500，但有了上面的 error 日志，排查会更容易
            // 注意：AuthError::Internal 包含了底层的错误信息字符串
            internal_auth_error(err)
        }
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
    headers: HeaderMap,
    Json(req): Json<RefreshTokenRequest>,
) -> Response {
    // 提取客户端 IP（用于审计日志）
    let client_ip = extract_client_ip(&headers).unwrap_or_else(|| "unknown".to_string());

    // 验证 refresh token 并生成新的 token 对
    match state.auth.refresh(&req.refresh_token).await {
        Ok(tokens) => {
            // 刷新成功，尝试从新 token 中获取用户信息记录审计
            // 注意：refresh 返回的 tokens 不包含用户信息，我们仍记录操作
            let response = RefreshTokenResponse {
                access_token: tokens.access_token.clone(),
                refresh_token: tokens.refresh_token,
                // 将秒级时间戳转换为毫秒级（前端期望的时间戳格式）
                expires: tokens.expires_at.saturating_mul(1000),
            };

            // 尝试从新 access_token 解析用户信息用于审计
            if let Ok(ctx) = state.auth.verify_access_token(&tokens.access_token) {
                let audit_record = AuditBuilder::new(&ctx, audit_actions::AUTH_REFRESH)
                    .tenant_scope()
                    .resource_typed("user", &ctx.user_id)
                    .success()
                    .with_client_ip(&client_ip)
                    .build();

                let audit_store = state.audit_log_store.clone();
                tokio::spawn(async move {
                    if let Err(e) = audit_store.create_audit_log(&ctx, audit_record).await {
                        tracing::warn!(error = %e, "failed to write refresh audit log");
                    }
                });
            }

            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        // token 无效或已过期，返回 401
        Err(AuthError::TokenInvalid | AuthError::TokenExpired) => {
            // Token 刷新失败，记录审计日志
            let audit_record =
                AuditBuilder::anonymous("default", "unknown", audit_actions::AUTH_REFRESH_FAILED)
                    .resource("refresh_token")
                    .failure()
                    .with_client_ip(&client_ip)
                    .with_field("reason", "token_invalid_or_expired")
                    .build();

            let audit_store = state.audit_log_store.clone();
            let ctx = domain::TenantContext::default();
            tokio::spawn(async move {
                if let Err(e) = audit_store.create_audit_log(&ctx, audit_record).await {
                    tracing::warn!(error = %e, "failed to write refresh failure audit log");
                }
            });

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
/// ```text
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
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    fn has_any_permission(ctx: &TenantContext, permissions: &[&str]) -> bool {
        if permissions.is_empty() {
            return true;
        }
        permissions
            .iter()
            .any(|permission| ctx.permissions.iter().any(|item| item == permission))
    }

    fn route_meta(
        title: &str,
        icon: &str,
        rank: i32,
        auths: Option<Vec<&str>>,
    ) -> api_contract::RouteMeta {
        api_contract::RouteMeta {
            title: title.to_string(),
            icon: icon.to_string(),
            rank,
            roles: None,
            auths: auths.map(|items| items.into_iter().map(|s| s.to_string()).collect()),
        }
    }

    fn page_route(
        ctx: &TenantContext,
        path: &str,
        name: &str,
        component: &str,
        title: &str,
        icon: &str,
        rank: i32,
        required_permissions: &[&str],
    ) -> Option<AsyncRoute> {
        if !has_any_permission(ctx, required_permissions) {
            return None;
        }
        Some(AsyncRoute {
            path: path.to_string(),
            name: name.to_string(),
            component: component.to_string(),
            meta: route_meta(
                title,
                icon,
                rank,
                if required_permissions.is_empty() {
                    None
                } else {
                    Some(required_permissions.to_vec())
                },
            ),
            children: Vec::new(),
        })
    }

    fn group_route(
        path: &str,
        name: &str,
        component: &str,
        title: &str,
        icon: &str,
        rank: i32,
        children: Vec<AsyncRoute>,
    ) -> Option<AsyncRoute> {
        if children.is_empty() {
            return None;
        }
        Some(AsyncRoute {
            path: path.to_string(),
            name: name.to_string(),
            component: component.to_string(),
            meta: route_meta(title, icon, rank, None),
            children,
        })
    }

    let rbac_children: Vec<AsyncRoute> = [
        page_route(
            &ctx,
            "/ems/rbac/users",
            "EmsRbacUsers",
            "/src/views/ems/rbac/users/index.vue",
            "用户管理",
            "",
            0,
            &[permissions::RBAC_USER_READ, permissions::RBAC_USER_WRITE],
        ),
        page_route(
            &ctx,
            "/ems/rbac/roles",
            "EmsRbacRoles",
            "/src/views/ems/rbac/roles/index.vue",
            "角色管理",
            "",
            0,
            &[permissions::RBAC_ROLE_READ, permissions::RBAC_ROLE_WRITE],
        ),
    ]
    .into_iter()
    .flatten()
    .collect();

    let ems_children: Vec<AsyncRoute> = [
        page_route(
            &ctx,
            "/ems/projects",
            "EmsProjects",
            "/src/views/ems/projects/index.vue",
            "项目管理",
            "ep:folder",
            0,
            &[permissions::PROJECT_READ, permissions::PROJECT_WRITE],
        ),
        page_route(
            &ctx,
            "/ems/gateways",
            "EmsGateways",
            "/src/views/ems/gateways/index.vue",
            "网关管理",
            "ep:connection",
            0,
            &[
                permissions::ASSET_GATEWAY_READ,
                permissions::ASSET_GATEWAY_WRITE,
            ],
        ),
        page_route(
            &ctx,
            "/ems/devices",
            "EmsDevices",
            "/src/views/ems/devices/index.vue",
            "设备管理",
            "ep:cpu",
            0,
            &[
                permissions::ASSET_DEVICE_READ,
                permissions::ASSET_DEVICE_WRITE,
            ],
        ),
        page_route(
            &ctx,
            "/ems/points",
            "EmsPoints",
            "/src/views/ems/points/index.vue",
            "点位管理",
            "ep:aim",
            0,
            &[
                permissions::ASSET_POINT_READ,
                permissions::ASSET_POINT_WRITE,
            ],
        ),
        page_route(
            &ctx,
            "/ems/point-mappings",
            "EmsPointMappings",
            "/src/views/ems/point-mappings/index.vue",
            "点位映射",
            "ep:share",
            0,
            &[
                permissions::ASSET_POINT_READ,
                permissions::ASSET_POINT_WRITE,
            ],
        ),
        page_route(
            &ctx,
            "/ems/collection-strategies",
            "EmsCollectionStrategies",
            "/src/views/ems/collection-strategies/index.vue",
            "采集策略",
            "ep:timer",
            0,
            &[
                permissions::DATA_REALTIME_READ,
                permissions::ASSET_POINT_WRITE,
            ],
        ),
        page_route(
            &ctx,
            "/ems/realtime",
            "EmsRealtime",
            "/src/views/ems/realtime/index.vue",
            "实时监控",
            "ep:data-line",
            0,
            &[permissions::DATA_REALTIME_READ],
        ),
        page_route(
            &ctx,
            "/ems/measurements",
            "EmsMeasurements",
            "/src/views/ems/measurements/index.vue",
            "历史数据",
            "ep:trend-charts",
            0,
            &[permissions::DATA_MEASUREMENTS_READ],
        ),
        page_route(
            &ctx,
            "/ems/commands",
            "EmsCommands",
            "/src/views/ems/commands/index.vue",
            "指令下发",
            "ep:promotion",
            0,
            &[
                permissions::CONTROL_COMMAND_READ,
                permissions::CONTROL_COMMAND_ISSUE,
            ],
        ),
        page_route(
            &ctx,
            "/ems/audit",
            "EmsAudit",
            "/src/views/ems/audit/index.vue",
            "审计日志",
            "ep:document-copy",
            0,
            &[permissions::CONTROL_COMMAND_READ],
        ),
        group_route(
            "/ems/rbac",
            "EmsRbac",
            "ParentView",
            "权限管理",
            "ep:lock",
            0,
            rbac_children,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();

    let routes: Vec<AsyncRoute> = group_route(
        "/ems",
        "Ems",
        "ParentView",
        "能源管理",
        "ep:monitor",
        10,
        ems_children,
    )
    .into_iter()
    .collect();

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

    #[tokio::test]
    async fn get_async_routes_filters_by_permissions() {
        use super::get_async_routes;
        use crate::AppState;
        use axum::extract::State;
        use domain::{TenantContext, permissions};
        use ems_auth::{AuthService, JwtManager};
        use ems_control::{CommandService, NoopDispatcher};
        use http_body_util::BodyExt;
        use serde_json::Value;
        use std::sync::Arc;

        let user_store: Arc<ems_storage::InMemoryUserStore> =
            Arc::new(ems_storage::InMemoryUserStore::with_default_admin());
        let jwt = JwtManager::new("test-secret".to_string(), 3600, 7200);
        let auth: Arc<AuthService> = Arc::new(AuthService::new(user_store.clone(), jwt));
        let rbac_store: Arc<dyn ems_storage::RbacStore> = user_store.clone();

        let project_store: Arc<dyn ems_storage::ProjectStore> =
            Arc::new(ems_storage::InMemoryProjectStore::with_default_project());
        let gateway_store: Arc<dyn ems_storage::GatewayStore> =
            Arc::new(ems_storage::InMemoryGatewayStore::new());
        let device_store: Arc<dyn ems_storage::DeviceStore> =
            Arc::new(ems_storage::InMemoryDeviceStore::new());
        let point_store: Arc<dyn ems_storage::PointStore> =
            Arc::new(ems_storage::InMemoryPointStore::new());
        let point_mapping_store: Arc<dyn ems_storage::PointMappingStore> =
            Arc::new(ems_storage::InMemoryPointMappingStore::new());
        let measurement_store: Arc<dyn ems_storage::MeasurementStore> =
            Arc::new(ems_storage::InMemoryMeasurementStore::new());
        let realtime_store: Arc<dyn ems_storage::RealtimeStore> =
            Arc::new(ems_storage::InMemoryRealtimeStore::new());
        let online_store: Arc<dyn ems_storage::OnlineStore> =
            Arc::new(ems_storage::InMemoryOnlineStore::new());
        let command_store: Arc<dyn ems_storage::CommandStore> =
            Arc::new(ems_storage::InMemoryCommandStore::new());
        let command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore> =
            Arc::new(ems_storage::InMemoryCommandReceiptStore::new());
        let audit_log_store: Arc<dyn ems_storage::AuditLogStore> =
            Arc::new(ems_storage::InMemoryAuditLogStore::new());
        let collection_strategy_store: Arc<dyn ems_storage::CollectionStrategyStore> =
            Arc::new(ems_storage::InMemoryCollectionStrategyStore::new());
        let system_log_store: Arc<dyn ems_storage::SystemLogStore> =
            Arc::new(ems_storage::InMemorySystemLogStore::default());

        let command_service = Arc::new(CommandService::new(
            command_store.clone(),
            audit_log_store.clone(),
            Arc::new(NoopDispatcher),
        ));

        let state = AppState::new(
            auth,
            None,
            rbac_store,
            project_store,
            gateway_store,
            device_store,
            point_store,
            point_mapping_store,
            measurement_store,
            realtime_store,
            online_store,
            command_store,
            command_receipt_store,
            audit_log_store,
            command_service,
            collection_strategy_store,
            system_log_store,
        );

        let ctx = TenantContext::new(
            "tenant-1",
            "user-1",
            vec!["admin".to_string()],
            vec![permissions::PROJECT_READ.to_string()],
            None,
        );
        let jwt = JwtManager::new("test-secret".to_string(), 3600, 7200);
        let tokens = jwt.issue_tokens(&ctx).expect("tokens");

        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).expect("header"),
        );

        let response = get_async_routes(State(state), headers).await;
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let json: Value = serde_json::from_slice(&bytes).expect("json");
        let data = json["data"].as_array().expect("data array");
        assert!(
            data.iter().any(|item| item["path"] == "/ems"),
            "expected /ems root route"
        );
        let ems_route = data
            .iter()
            .find(|item| item["path"] == "/ems")
            .expect("/ems route should exist");

        assert_eq!(
            ems_route["component"], "ParentView",
            "/ems route component should be ParentView"
        );

        let ems_children = ems_route["children"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        assert!(
            ems_children
                .iter()
                .any(|item| item["path"] == "/ems/projects"),
            "expected /ems/projects to be present when PROJECT.READ is granted"
        );
    }
}
