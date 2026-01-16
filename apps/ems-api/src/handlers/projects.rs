//! 项目 CRUD handlers
//!
//! 提供项目资源的增删改查接口：
//! - GET /projects - 列出项目
//! - POST /projects - 创建项目
//! - GET /projects/{id} - 获取项目详情
//! - PUT /projects/{id} - 更新项目
//! - DELETE /projects/{id} - 删除项目
//!
//! 权限要求：
//! - 所有接口需要 Bearer token 认证
//! - 需验证项目归属当前租户

use crate::AppState;
use crate::middleware::{require_permission, require_tenant_context};
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use crate::utils::{normalize_optional, normalize_required, project_to_dto};
use api_contract::{ApiResponse, CreateProjectRequest, ProjectDto, UpdateProjectRequest};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

/// 列出项目
pub async fn list_projects(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::PROJECT_READ) {
        return response;
    }
    match state.project_store.list_projects(&ctx).await {
        Ok(projects) => {
            let data: Vec<ProjectDto> = projects.into_iter().map(project_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 创建项目
pub async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::PROJECT_WRITE) {
        return response;
    }
    let name = match normalize_required(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let timezone = req.timezone.unwrap_or_else(|| "UTC".to_string());
    let record = ems_storage::ProjectRecord {
        project_id: Uuid::new_v4().to_string(),
        tenant_id: ctx.tenant_id.clone(),
        name,
        timezone,
    };
    match state.project_store.create_project(&ctx, record).await {
        Ok(project) => (
            StatusCode::OK,
            Json(ApiResponse::success(project_to_dto(project))),
        )
            .into_response(),
        Err(err) => storage_error(err),
    }
}

/// 获取项目详情
pub async fn get_project(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::PROJECT_READ) {
        return response;
    }
    match state
        .project_store
        .find_project(&ctx, &path.project_id)
        .await
    {
        Ok(Some(project)) => (
            StatusCode::OK,
            Json(ApiResponse::success(project_to_dto(project))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 更新项目
pub async fn update_project(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<UpdateProjectRequest>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::PROJECT_WRITE) {
        return response;
    }
    let name = match normalize_optional(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let timezone = match normalize_optional(req.timezone, "timezone") {
        Ok(value) => value,
        Err(response) => return response,
    };
    if name.is_none() && timezone.is_none() {
        return bad_request_error("empty update");
    }
    let update = ems_storage::ProjectUpdate { name, timezone };
    match state
        .project_store
        .update_project(&ctx, &path.project_id, update)
        .await
    {
        Ok(Some(project)) => (
            StatusCode::OK,
            Json(ApiResponse::success(project_to_dto(project))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 删除项目
pub async fn delete_project(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::PROJECT_WRITE) {
        return response;
    }
    match state
        .project_store
        .delete_project(&ctx, &path.project_id)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(false) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::require_project_scope;
    use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
    use ems_auth::{AuthService, JwtManager};
    use ems_storage::{InMemoryProjectStore, InMemoryUserStore, ProjectStore};
    use std::sync::Arc;

    #[tokio::test]
    async fn projects_list_requires_permission() {
        let user_store = Arc::new(InMemoryUserStore::with_default_admin());
        let jwt = JwtManager::new("secret".to_string(), 3600, 3600);
        let auth = Arc::new(AuthService::new(user_store.clone(), jwt));
        let project_store: Arc<dyn ProjectStore> =
            Arc::new(InMemoryProjectStore::with_default_project());
        let command_store: Arc<dyn ems_storage::CommandStore> =
            Arc::new(ems_storage::InMemoryCommandStore::new());
        let command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore> =
            Arc::new(ems_storage::InMemoryCommandReceiptStore::new());
        let audit_log_store: Arc<dyn ems_storage::AuditLogStore> =
            Arc::new(ems_storage::InMemoryAuditLogStore::new());
        let dispatcher = Arc::new(ems_control::NoopDispatcher::default());
        let command_service = Arc::new(ems_control::CommandService::new(
            command_store.clone(),
            audit_log_store.clone(),
            dispatcher,
        ));
        let state = AppState {
            auth,
            db_pool: None,
            rbac_store: user_store,
            project_store,
            gateway_store: Arc::new(ems_storage::InMemoryGatewayStore::new()),
            device_store: Arc::new(ems_storage::InMemoryDeviceStore::new()),
            point_store: Arc::new(ems_storage::InMemoryPointStore::new()),
            point_mapping_store: Arc::new(ems_storage::InMemoryPointMappingStore::new()),
            measurement_store: Arc::new(ems_storage::InMemoryMeasurementStore::new()),
            realtime_store: Arc::new(ems_storage::InMemoryRealtimeStore::new()),
            online_store: Arc::new(ems_storage::InMemoryOnlineStore::new()),
            command_store,
            command_receipt_store,
            audit_log_store,
            command_service,
        };

        let jwt = JwtManager::new("secret".to_string(), 3600, 3600);
        let tokens = jwt
            .issue_tokens(&domain::TenantContext::new(
                "tenant-1".to_string(),
                "user-1".to_string(),
                Vec::new(),
                Vec::new(),
                None,
            ))
            .expect("token");
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).expect("header"),
        );
        let response = list_projects(State(state), headers).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn project_scope_sets_context() {
        let user_store = Arc::new(InMemoryUserStore::with_default_admin());
        let jwt = JwtManager::new("secret".to_string(), 3600, 3600);
        let auth = Arc::new(AuthService::new(user_store.clone(), jwt));
        let project_store: Arc<dyn ProjectStore> =
            Arc::new(InMemoryProjectStore::with_default_project());
        let command_store: Arc<dyn ems_storage::CommandStore> =
            Arc::new(ems_storage::InMemoryCommandStore::new());
        let command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore> =
            Arc::new(ems_storage::InMemoryCommandReceiptStore::new());
        let audit_log_store: Arc<dyn ems_storage::AuditLogStore> =
            Arc::new(ems_storage::InMemoryAuditLogStore::new());
        let dispatcher = Arc::new(ems_control::NoopDispatcher::default());
        let command_service = Arc::new(ems_control::CommandService::new(
            command_store.clone(),
            audit_log_store.clone(),
            dispatcher,
        ));
        let state = AppState {
            auth,
            db_pool: None,
            rbac_store: user_store,
            project_store,
            gateway_store: Arc::new(ems_storage::InMemoryGatewayStore::new()),
            device_store: Arc::new(ems_storage::InMemoryDeviceStore::new()),
            point_store: Arc::new(ems_storage::InMemoryPointStore::new()),
            point_mapping_store: Arc::new(ems_storage::InMemoryPointMappingStore::new()),
            measurement_store: Arc::new(ems_storage::InMemoryMeasurementStore::new()),
            realtime_store: Arc::new(ems_storage::InMemoryRealtimeStore::new()),
            online_store: Arc::new(ems_storage::InMemoryOnlineStore::new()),
            command_store,
            command_receipt_store,
            audit_log_store,
            command_service,
        };
        let (_, tokens) = state.auth.login("admin", "admin123").await.expect("login");
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).expect("header"),
        );
        let ctx = require_project_scope(&state, &headers, "project-1")
            .await
            .expect("scope");
        assert_eq!(ctx.project_scope.as_deref(), Some("project-1"));
    }

    #[tokio::test]
    async fn project_scope_rejects_mismatch() {
        let user_store = Arc::new(InMemoryUserStore::with_default_admin());
        let jwt = JwtManager::new("secret".to_string(), 3600, 3600);
        let auth = Arc::new(AuthService::new(user_store.clone(), jwt));
        let project_store: Arc<dyn ProjectStore> =
            Arc::new(InMemoryProjectStore::with_default_project());
        let command_store: Arc<dyn ems_storage::CommandStore> =
            Arc::new(ems_storage::InMemoryCommandStore::new());
        let command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore> =
            Arc::new(ems_storage::InMemoryCommandReceiptStore::new());
        let audit_log_store: Arc<dyn ems_storage::AuditLogStore> =
            Arc::new(ems_storage::InMemoryAuditLogStore::new());
        let dispatcher = Arc::new(ems_control::NoopDispatcher::default());
        let command_service = Arc::new(ems_control::CommandService::new(
            command_store.clone(),
            audit_log_store.clone(),
            dispatcher,
        ));
        let state = AppState {
            auth,
            db_pool: None,
            rbac_store: user_store,
            project_store,
            gateway_store: Arc::new(ems_storage::InMemoryGatewayStore::new()),
            device_store: Arc::new(ems_storage::InMemoryDeviceStore::new()),
            point_store: Arc::new(ems_storage::InMemoryPointStore::new()),
            point_mapping_store: Arc::new(ems_storage::InMemoryPointMappingStore::new()),
            measurement_store: Arc::new(ems_storage::InMemoryMeasurementStore::new()),
            realtime_store: Arc::new(ems_storage::InMemoryRealtimeStore::new()),
            online_store: Arc::new(ems_storage::InMemoryOnlineStore::new()),
            command_store,
            command_receipt_store,
            audit_log_store,
            command_service,
        };
        let (_, tokens) = state.auth.login("admin", "admin123").await.expect("login");
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).expect("header"),
        );
        let response = require_project_scope(&state, &headers, "project-2")
            .await
            .expect_err("forbidden");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
