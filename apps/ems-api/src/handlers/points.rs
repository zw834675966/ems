//! 点 CRUD handlers
//!
//! 提供点资源的增删改查接口：
//! - GET /projects/{id}/points - 列出点
//! - POST /projects/{id}/points - 创建点（需验证设备存在）
//! - GET /projects/{id}/points/{pid} - 获取点详情
//! - PUT /projects/{id}/points/{pid} - 更新点
//! - DELETE /projects/{id}/points/{pid} - 删除点
//!
//! 权限要求：
//! - 所有接口需要 Bearer token 认证
//! - 需验证项目归属当前租户
//! - 创建点时需验证设备属于该项目

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use crate::utils::{normalize_optional, normalize_required, point_to_dto};
use api_contract::{ApiResponse, CreatePointRequest, PointDto, UpdatePointRequest};
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

#[derive(serde::Deserialize)]
pub struct PointPath {
    project_id: String,
    point_id: String,
}

/// 列出点
pub async fn list_points(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_READ) {
        return response;
    }
    match state.point_store.list_points(&ctx, &path.project_id).await {
        Ok(items) => {
            let data: Vec<PointDto> = items.into_iter().map(point_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 创建点
pub async fn create_point(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<CreatePointRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }
    let device_id = match normalize_required(req.device_id, "deviceId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let key = match normalize_required(req.key, "key") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let data_type = match normalize_required(req.data_type, "dataType") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let exists = state
        .device_store
        .find_device(&ctx, &path.project_id, &device_id)
        .await;
    match exists {
        Ok(Some(_)) => {}
        Ok(None) => return bad_request_error("device not found"),
        Err(err) => return storage_error(err),
    }
    let record = ems_storage::PointRecord {
        point_id: Uuid::new_v4().to_string(),
        tenant_id: ctx.tenant_id.clone(),
        project_id: path.project_id,
        device_id,
        key,
        data_type,
        unit: req.unit,
    };
    match state.point_store.create_point(&ctx, record).await {
        Ok(item) => (
            StatusCode::OK,
            Json(ApiResponse::success(point_to_dto(item))),
        )
            .into_response(),
        Err(err) => storage_error(err),
    }
}

/// 获取点详情
pub async fn get_point(
    State(state): State<AppState>,
    Path(path): Path<PointPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_READ) {
        return response;
    }
    match state
        .point_store
        .find_point(&ctx, &path.project_id, &path.point_id)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(point_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 更新点
pub async fn update_point(
    State(state): State<AppState>,
    Path(path): Path<PointPath>,
    headers: HeaderMap,
    Json(req): Json<UpdatePointRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }
    let key = match normalize_optional(req.key, "key") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let data_type = match normalize_optional(req.data_type, "dataType") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let unit = match normalize_optional(req.unit, "unit") {
        Ok(value) => value,
        Err(response) => return response,
    };
    if key.is_none() && data_type.is_none() && unit.is_none() {
        return bad_request_error("empty update");
    }
    let update = ems_storage::PointUpdate {
        key,
        data_type,
        unit,
    };
    match state
        .point_store
        .update_point(&ctx, &path.project_id, &path.point_id, update)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(point_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 删除点
pub async fn delete_point(
    State(state): State<AppState>,
    Path(path): Path<PointPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }
    match state
        .point_store
        .delete_point(&ctx, &path.project_id, &path.point_id)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(false) => not_found_error(),
        Err(err) => storage_error(err),
    }
}
