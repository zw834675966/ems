//! 认证和授权中间件
//!
//! 提供以下中间件和辅助函数：
//! - request_context：请求上下文中间件，注入 request_id/trace_id
//! - bearer_token：从 Authorization 头提取 Bearer token
//! - require_tenant_context：验证 token 并提取租户上下文
//! - require_project_scope：验证项目归属（带租户上下文）
//!
//! 认证流程：
//! 1. request_context：在所有请求前注入追踪 ID
//! 2. bearer_token：从请求头提取 token
//! 3. require_tenant_context：验证 JWT 签名，获取 TenantContext
//! 4. require_project_scope：验证 project_id 属于当前租户

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderValue, header},
    middleware::Next,
    response::Response,
};
use ems_auth::AuthError;
use ems_telemetry::new_request_ids;
use tracing::{Instrument, info_span};

use crate::AppState;
use crate::utils::response::{auth_error, forbidden_error, storage_error};
use domain::TenantContext;

/// 请求上下文中间件：注入 request_id/trace_id
pub async fn request_context(mut req: Request<Body>, next: Next) -> Response {
    let ids = new_request_ids();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    req.extensions_mut().insert(ids.clone());

    let span = info_span!(
        "request",
        request_id = %ids.request_id,
        trace_id = %ids.trace_id,
        method = %method,
        path = %path
    );

    let mut response: axum::response::Response = next.run(req).instrument(span).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&ids.request_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    response.headers_mut().insert(
        "x-trace-id",
        HeaderValue::from_str(&ids.trace_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    response
}

/// 从请求头中提取 Bearer token
pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let header_value = headers.get(header::AUTHORIZATION)?;
    let auth_str = header_value.to_str().ok()?;
    auth_str.strip_prefix("Bearer ")
}

/// 验证并提取租户上下文
pub fn require_tenant_context(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<TenantContext, Response> {
    let token = match bearer_token(headers) {
        Some(token) => token,
        None => return Err(auth_error(axum::http::StatusCode::UNAUTHORIZED)),
    };
    match state.auth.verify_access_token(token) {
        Ok(ctx) => Ok(ctx),
        Err(AuthError::TokenInvalid | AuthError::TokenExpired) => {
            Err(auth_error(axum::http::StatusCode::UNAUTHORIZED))
        }
        Err(err) => Err(crate::utils::response::internal_auth_error(err)),
    }
}

/// 验证项目归属权限
pub async fn require_project_scope(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<TenantContext, Response> {
    let mut ctx = match require_tenant_context(state, headers) {
        Ok(ctx) => ctx,
        Err(response) => return Err(response),
    };
    match state
        .project_store
        .project_belongs_to_tenant(&ctx, project_id)
        .await
    {
        Ok(true) => {
            ctx.project_scope = Some(project_id.to_string());
            Ok(ctx)
        }
        Ok(false) => Err(forbidden_error()),
        Err(err) => Err(storage_error(err)),
    }
}
