//! 设备 CRUD handlers
//!
//! 提供设备资源的增删改查接口：
//! - GET /projects/{id}/devices - 列出设备
//! - POST /projects/{id}/devices - 创建设备（需验证网关存在）
//! - GET /projects/{id}/devices/{did} - 获取设备详情
//! - PUT /projects/{id}/devices/{did} - 更新设备
//! - DELETE /projects/{id}/devices/{did} - 删除设备
//!
//! 权限要求：
//! - 所有接口需要 Bearer token 认证
//! - 需验证项目归属当前租户
//! - 创建设备时需验证网关属于该项目

use crate::AppState;
use crate::middleware::require_project_scope;
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use crate::utils::{device_to_dto, normalize_optional, normalize_required};
use api_contract::{ApiResponse, CreateDeviceRequest, DeviceDto, UpdateDeviceRequest};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

#[derive(serde::Deserialize)]
pub struct DevicePath {
    project_id: String,
    device_id: String,
}

/// 列出设备
pub async fn list_devices(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match state
        .device_store
        .list_devices(&ctx, &path.project_id)
        .await
    {
        Ok(items) => {
            let data: Vec<DeviceDto> = items.into_iter().map(device_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 创建设备
pub async fn create_device(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<CreateDeviceRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let gateway_id = match normalize_required(req.gateway_id, "gatewayId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let name = match normalize_required(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let exists = state
        .gateway_store
        .find_gateway(&ctx, &path.project_id, &gateway_id)
        .await;
    match exists {
        Ok(Some(_)) => {}
        Ok(None) => return bad_request_error("gateway not found"),
        Err(err) => return storage_error(err),
    }
    let record = ems_storage::DeviceRecord {
        device_id: Uuid::new_v4().to_string(),
        tenant_id: ctx.tenant_id.clone(),
        project_id: path.project_id,
        gateway_id,
        name,
        model: req.model,
    };
    match state.device_store.create_device(&ctx, record).await {
        Ok(item) => (
            StatusCode::OK,
            Json(ApiResponse::success(device_to_dto(item))),
        )
            .into_response(),
        Err(err) => storage_error(err),
    }
}

/// 获取设备详情
pub async fn get_device(
    State(state): State<AppState>,
    Path(path): Path<DevicePath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match state
        .device_store
        .find_device(&ctx, &path.project_id, &path.device_id)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(device_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 更新设备
pub async fn update_device(
    State(state): State<AppState>,
    Path(path): Path<DevicePath>,
    headers: HeaderMap,
    Json(req): Json<UpdateDeviceRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let name = match normalize_optional(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let model = match normalize_optional(req.model, "model") {
        Ok(value) => value,
        Err(response) => return response,
    };
    if name.is_none() && model.is_none() {
        return bad_request_error("empty update");
    }
    let update = ems_storage::DeviceUpdate { name, model };
    match state
        .device_store
        .update_device(&ctx, &path.project_id, &path.device_id, update)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(device_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 删除设备
pub async fn delete_device(
    State(state): State<AppState>,
    Path(path): Path<DevicePath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match state
        .device_store
        .delete_device(&ctx, &path.project_id, &path.device_id)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(false) => not_found_error(),
        Err(err) => storage_error(err),
    }
}
