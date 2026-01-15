//! 认证相关 handlers：登录、刷新 token、获取动态路由
//!
//! 提供以下端点：
//! - GET /health - 健康检查
//! - POST /login - 登录认证
//! - POST /refresh-token - 刷新 access token
//! - GET /get-async-routes - 获取动态路由（需认证）
//!
//! 认证逻辑：
//! - 登录：验证用户名密码，返回 access/refresh token
//! - 刷新：验证 refresh token，签发新的 access/refresh token
//! - 动态路由：根据用户角色和权限返回前端路由配置

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

/// 健康检查
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

/// 登录接口
pub async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Response {
    match state.auth.login(&req.username, &req.password).await {
        Ok((user, tokens)) => {
            let response = LoginResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires: tokens.expires_at.saturating_mul(1000),
                username: user.username.clone(),
                nickname: user.username,
                avatar: "".to_string(),
                roles: user.roles,
                permissions: user.permissions,
            };
            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        Err(AuthError::InvalidCredentials) => auth_error(StatusCode::UNAUTHORIZED),
        Err(err) => internal_auth_error(err),
    }
}

/// 刷新 access token
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Response {
    match state.auth.refresh(&req.refresh_token) {
        Ok(tokens) => {
            let response = RefreshTokenResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires: tokens.expires_at.saturating_mul(1000),
            };
            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        Err(AuthError::TokenInvalid | AuthError::TokenExpired) => {
            auth_error(StatusCode::UNAUTHORIZED)
        }
        Err(err) => internal_auth_error(err),
    }
}

/// 获取动态路由
pub async fn get_async_routes(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    let roles = if ctx.roles.is_empty() {
        None
    } else {
        Some(ctx.roles.clone())
    };
    let meta = |title: &str, rank: i32, auths: Option<Vec<String>>| RouteMeta {
        title: title.to_string(),
        icon: "monitor".to_string(),
        rank,
        roles: roles.clone(),
        auths,
    };
    let routes = vec![AsyncRoute {
        path: "/ems".to_string(),
        name: "EMS".to_string(),
        component: "Layout".to_string(),
        meta: meta("EMS", 1, None),
        children: vec![
            AsyncRoute {
                path: "/ems/projects".to_string(),
                name: "EmsProjects".to_string(),
                component: "ems/projects/index".to_string(),
                meta: meta(
                    "项目",
                    1,
                    Some(vec![
                        permissions::PROJECT_READ.to_string(),
                        permissions::PROJECT_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            AsyncRoute {
                path: "/ems/gateways".to_string(),
                name: "EmsGateways".to_string(),
                component: "ems/gateways/index".to_string(),
                meta: meta(
                    "网关",
                    2,
                    Some(vec![
                        permissions::ASSET_GATEWAY_READ.to_string(),
                        permissions::ASSET_GATEWAY_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            AsyncRoute {
                path: "/ems/devices".to_string(),
                name: "EmsDevices".to_string(),
                component: "ems/devices/index".to_string(),
                meta: meta(
                    "设备",
                    3,
                    Some(vec![
                        permissions::ASSET_DEVICE_READ.to_string(),
                        permissions::ASSET_DEVICE_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            AsyncRoute {
                path: "/ems/points".to_string(),
                name: "EmsPoints".to_string(),
                component: "ems/points/index".to_string(),
                meta: meta(
                    "点位",
                    4,
                    Some(vec![
                        permissions::ASSET_POINT_READ.to_string(),
                        permissions::ASSET_POINT_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
            AsyncRoute {
                path: "/ems/point-mappings".to_string(),
                name: "EmsPointMappings".to_string(),
                component: "ems/point-mappings/index".to_string(),
                meta: meta(
                    "点位映射",
                    5,
                    Some(vec![
                        permissions::ASSET_POINT_READ.to_string(),
                        permissions::ASSET_POINT_WRITE.to_string(),
                    ]),
                ),
                children: Vec::new(),
            },
        ],
    }];

    (StatusCode::OK, Json(ApiResponse::success(routes))).into_response()
}

#[cfg(test)]
mod tests {
    use crate::middleware::bearer_token;
    use axum::http::{HeaderMap, HeaderValue, header};

    #[test]
    fn bearer_token_extracts() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token-1"),
        );
        assert_eq!(bearer_token(&headers), Some("token-1"));
    }
}
