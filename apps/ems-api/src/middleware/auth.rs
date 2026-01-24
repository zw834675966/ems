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

pub fn has_permission(ctx: &TenantContext, permission: &str) -> bool {
    ctx.permissions.iter().any(|item| item == permission)
}

#[allow(clippy::result_large_err)]
pub fn require_permission(ctx: &TenantContext, permission: &str) -> Result<(), Response> {
    if has_permission(ctx, permission) {
        Ok(())
    } else {
        Err(forbidden_error())
    }
}

#[allow(clippy::result_large_err)]
pub fn require_any_permission(ctx: &TenantContext, permissions: &[&str]) -> Result<(), Response> {
    if permissions.is_empty() {
        return Ok(());
    }
    if permissions
        .iter()
        .any(|permission| has_permission(ctx, permission))
    {
        Ok(())
    } else {
        Err(forbidden_error())
    }
}

/// 请求上下文中间件：注入 request_id/trace_id
#[allow(dead_code)]
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
#[allow(clippy::result_large_err)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use ems_auth::{AuthService, JwtManager};
    use ems_control::{CommandService, NoopDispatcher};
    use std::sync::Arc;

    fn build_state() -> AppState {
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
        let command_service = Arc::new(CommandService::new(
            command_store.clone(),
            audit_log_store.clone(),
            Arc::new(NoopDispatcher),
        ));
        let collection_strategy_store: Arc<dyn ems_storage::CollectionStrategyStore> =
            Arc::new(ems_storage::InMemoryCollectionStrategyStore::new());
        let system_log_store: Arc<dyn ems_storage::SystemLogStore> =
            Arc::new(ems_storage::InMemorySystemLogStore::default());

        AppState::new(
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
        )
    }

    #[test]
    fn require_tenant_context_rejects_missing_token() {
        unsafe { std::env::set_var("EMS_JWT_SECRET", "test-secret") };
        let state = build_state();
        let headers = HeaderMap::new();
        let response = require_tenant_context(&state, &headers).unwrap_err();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_project_scope_sets_scope() {
        unsafe { std::env::set_var("EMS_JWT_SECRET", "test-secret") };
        let state = build_state();
        let ctx = TenantContext::new(
            "tenant-1",
            "user-1",
            Vec::new(),
            Vec::new(),
            None,
        );
        let jwt = JwtManager::new("test-secret".to_string(), 3600, 7200);
        let tokens = jwt.issue_tokens(&ctx).expect("tokens");

        let mut headers = HeaderMap::new();
        let header_value = format!("Bearer {}", tokens.access_token);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&header_value).expect("header"),
        );

        let ctx = require_project_scope(&state, &headers, "project-1")
            .await
            .expect("context");
        assert_eq!(ctx.project_scope.as_deref(), Some("project-1"));
    }
}
