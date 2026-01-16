//! RBAC 管理面接口（tenant 级）

use crate::AppState;
use crate::middleware::{require_permission, require_tenant_context};
use crate::utils::response::{bad_request_error, internal_auth_error, not_found_error, storage_error};
use api_contract::{
    ApiResponse, CreateRbacRoleRequest, CreateRbacUserRequest, PermissionDto, RbacRoleDto,
    RbacUserDto, SetRolePermissionsRequest, SetUserRolesRequest, UpdateRbacUserRequest,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use ems_auth::hash_password;
use ems_storage::{PermissionRecord, RbacRoleRecord, RbacUserRecord};
use uuid::Uuid;

fn user_to_dto(record: RbacUserRecord) -> RbacUserDto {
    RbacUserDto {
        user_id: record.user_id,
        username: record.username,
        status: record.status,
        roles: record.roles,
    }
}

fn role_to_dto(record: RbacRoleRecord) -> RbacRoleDto {
    RbacRoleDto {
        role_code: record.role_code,
        name: record.name,
        permissions: record.permissions,
    }
}

fn permission_to_dto(record: PermissionRecord) -> PermissionDto {
    PermissionDto {
        permission_code: record.permission_code,
        description: record.description,
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct UserPath {
    pub user_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct RolePath {
    pub role_code: String,
}

pub async fn list_rbac_users(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_USER_READ) {
        return response;
    }

    match state.rbac_store.list_users(&ctx).await {
        Ok(items) => {
            let items = items.into_iter().map(user_to_dto).collect::<Vec<_>>();
            (StatusCode::OK, Json(ApiResponse::success(items))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

pub async fn create_rbac_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateRbacUserRequest>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_USER_WRITE) {
        return response;
    }

    let username = req.username.trim().to_string();
    if username.is_empty() {
        return bad_request_error("username is required");
    }
    if req.password.trim().is_empty() {
        return bad_request_error("password is required");
    }
    let status = req.status.unwrap_or_else(|| "active".to_string());
    let roles = req.roles.unwrap_or_default();

    if !roles.is_empty() {
        let known = match state.rbac_store.list_roles(&ctx).await {
            Ok(items) => items
                .into_iter()
                .map(|item| item.role_code)
                .collect::<std::collections::HashSet<_>>(),
            Err(err) => return storage_error(err),
        };
        let unknown: Vec<String> = roles
            .iter()
            .filter(|role| !known.contains(*role))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return bad_request_error(format!("unknown roles: {}", unknown.join(",")));
        }
    }

    let password_hash = match hash_password(&req.password) {
        Ok(value) => value,
        Err(err) => return internal_auth_error(err),
    };

    let record = ems_storage::RbacUserCreate {
        tenant_id: ctx.tenant_id.clone(),
        user_id: Uuid::new_v4().to_string(),
        username,
        password: password_hash,
        status,
        roles,
    };
    match state.rbac_store.create_user(&ctx, record).await {
        Ok(created) => (StatusCode::OK, Json(ApiResponse::success(user_to_dto(created))))
            .into_response(),
        Err(err) => storage_error(err),
    }
}

pub async fn update_rbac_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<UserPath>,
    Json(req): Json<UpdateRbacUserRequest>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_USER_WRITE) {
        return response;
    }

    if req.password.is_none() && req.status.is_none() {
        return bad_request_error("no fields to update");
    }

    let password_hash = match req.password {
        None => None,
        Some(password) => {
            if password.trim().is_empty() {
                return bad_request_error("password is required");
            }
            match hash_password(&password) {
                Ok(value) => Some(value),
                Err(err) => return internal_auth_error(err),
            }
        }
    };

    match state
        .rbac_store
        .update_user(
            &ctx,
            &path.user_id,
            ems_storage::RbacUserUpdate {
                password: password_hash,
                status: req.status,
            },
        )
        .await
    {
        Ok(Some(updated)) => (StatusCode::OK, Json(ApiResponse::success(user_to_dto(updated))))
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

pub async fn set_rbac_user_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<UserPath>,
    Json(req): Json<SetUserRolesRequest>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_USER_WRITE) {
        return response;
    }

    let known = match state.rbac_store.list_roles(&ctx).await {
        Ok(items) => items
            .into_iter()
            .map(|item| item.role_code)
            .collect::<std::collections::HashSet<_>>(),
        Err(err) => return storage_error(err),
    };
    let unknown: Vec<String> = req
        .roles
        .iter()
        .filter(|role| !known.contains(*role))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return bad_request_error(format!("unknown roles: {}", unknown.join(",")));
    }

    match state
        .rbac_store
        .set_user_roles(&ctx, &path.user_id, req.roles)
        .await
    {
        Ok(Some(updated)) => (StatusCode::OK, Json(ApiResponse::success(user_to_dto(updated))))
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

pub async fn list_rbac_roles(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_ROLE_READ) {
        return response;
    }

    match state.rbac_store.list_roles(&ctx).await {
        Ok(items) => {
            let items = items.into_iter().map(role_to_dto).collect::<Vec<_>>();
            (StatusCode::OK, Json(ApiResponse::success(items))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

pub async fn create_rbac_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateRbacRoleRequest>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_ROLE_WRITE) {
        return response;
    }

    let role_code = req.role_code.trim().to_string();
    if role_code.is_empty() {
        return bad_request_error("roleCode is required");
    }
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return bad_request_error("name is required");
    }
    let permissions = req.permissions.unwrap_or_default();

    if !permissions.is_empty() {
        let known = match state.rbac_store.list_permissions(&ctx).await {
            Ok(items) => items
                .into_iter()
                .map(|item| item.permission_code)
                .collect::<std::collections::HashSet<_>>(),
            Err(err) => return storage_error(err),
        };
        let unknown: Vec<String> = permissions
            .iter()
            .filter(|p| !known.contains(*p))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return bad_request_error(format!(
                "unknown permissions: {}",
                unknown.join(",")
            ));
        }
    }

    let record = ems_storage::RbacRoleCreate {
        tenant_id: ctx.tenant_id.clone(),
        role_code,
        name,
        permissions,
    };

    match state.rbac_store.create_role(&ctx, record).await {
        Ok(created) => (StatusCode::OK, Json(ApiResponse::success(role_to_dto(created))))
            .into_response(),
        Err(err) => storage_error(err),
    }
}

pub async fn delete_rbac_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<RolePath>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_ROLE_WRITE) {
        return response;
    }

    match state.rbac_store.delete_role(&ctx, &path.role_code).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(false) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

pub async fn set_rbac_role_permissions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<RolePath>,
    Json(req): Json<SetRolePermissionsRequest>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_ROLE_WRITE) {
        return response;
    }

    let known = match state.rbac_store.list_permissions(&ctx).await {
        Ok(items) => items
            .into_iter()
            .map(|item| item.permission_code)
            .collect::<std::collections::HashSet<_>>(),
        Err(err) => return storage_error(err),
    };
    let unknown: Vec<String> = req
        .permissions
        .iter()
        .filter(|p| !known.contains(*p))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return bad_request_error(format!(
            "unknown permissions: {}",
            unknown.join(",")
        ));
    }

    match state
        .rbac_store
        .set_role_permissions(&ctx, &path.role_code, req.permissions)
        .await
    {
        Ok(Some(updated)) => (StatusCode::OK, Json(ApiResponse::success(role_to_dto(updated))))
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

pub async fn list_rbac_permissions(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::RBAC_ROLE_READ) {
        return response;
    }

    match state.rbac_store.list_permissions(&ctx).await {
        Ok(items) => {
            let items = items
                .into_iter()
                .map(permission_to_dto)
                .collect::<Vec<_>>();
            (StatusCode::OK, Json(ApiResponse::success(items))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, header};
    use ems_auth::{AuthService, JwtManager};
    use std::sync::Arc;

    fn build_state() -> AppState {
        let user_store: Arc<ems_storage::InMemoryUserStore> =
            Arc::new(ems_storage::InMemoryUserStore::with_default_admin());
        let jwt = JwtManager::new("secret".to_string(), 3600, 3600);
        let auth: Arc<AuthService> = Arc::new(AuthService::new(user_store.clone(), jwt));
        let rbac_store: Arc<dyn ems_storage::RbacStore> = user_store;

        let project_store: Arc<dyn ems_storage::ProjectStore> =
            Arc::new(ems_storage::InMemoryProjectStore::with_default_project());
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

        AppState {
            auth,
            db_pool: None,
            rbac_store,
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
        }
    }

    #[tokio::test]
    async fn list_users_requires_permission() {
        let state = build_state();
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
        let response = list_rbac_users(State(state), headers).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
