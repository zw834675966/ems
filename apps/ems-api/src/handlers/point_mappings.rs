//! 点映射 CRUD handlers
//!
//! 提供点映射资源的增删改查接口：
//! - GET /projects/{id}/point-mappings - 列出点映射
//! - POST /projects/{id}/point-mappings - 创建点映射（需验证点存在）
//! - GET /projects/{id}/point-mappings/{sid} - 获取点映射详情
//! - PUT /projects/{id}/point-mappings/{sid} - 更新点映射
//! - DELETE /projects/{id}/point-mappings/{sid} - 删除点映射
//!
//! 权限要求：
//! - 所有接口需要 Bearer token 认证
//! - 需验证项目归属当前租户
//! - 创建点映射时需验证点属于该项目

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use crate::utils::{normalize_optional, normalize_required, point_mapping_to_dto};
use api_contract::{
    ApiResponse, CreatePointMappingRequest, PointMappingDto, UpdatePointMappingRequest,
};
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
pub struct PointMappingPath {
    project_id: String,
    source_id: String,
}

/// 列出点映射
pub async fn list_point_mappings(
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
    match state
        .point_mapping_store
        .list_point_mappings(&ctx, &path.project_id)
        .await
    {
        Ok(items) => {
            let data: Vec<PointMappingDto> = items.into_iter().map(point_mapping_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 创建点映射
pub async fn create_point_mapping(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<CreatePointMappingRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }
    let point_id = match normalize_required(req.point_id, "pointId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let source_type = match normalize_required(req.source_type, "sourceType") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let address = match normalize_required(req.address, "address") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let exists = state
        .point_store
        .find_point(&ctx, &path.project_id, &point_id)
        .await;
    match exists {
        Ok(Some(_)) => {}
        Ok(None) => return bad_request_error("point not found"),
        Err(err) => return storage_error(err),
    }
    let record = ems_storage::PointMappingRecord {
        source_id: Uuid::new_v4().to_string(),
        tenant_id: ctx.tenant_id.clone(),
        project_id: path.project_id,
        point_id,
        source_type,
        address,
        scale: req.scale,
        offset: req.offset,
        protocol_detail: req.protocol_detail,
    };
    match state
        .point_mapping_store
        .create_point_mapping(&ctx, record)
        .await
    {
        Ok(item) => (
            StatusCode::OK,
            Json(ApiResponse::success(point_mapping_to_dto(item))),
        )
            .into_response(),
        Err(err) => storage_error(err),
    }
}

/// 获取点映射详情
pub async fn get_point_mapping(
    State(state): State<AppState>,
    Path(path): Path<PointMappingPath>,
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
        .point_mapping_store
        .find_point_mapping(&ctx, &path.project_id, &path.source_id)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(point_mapping_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 更新点映射
pub async fn update_point_mapping(
    State(state): State<AppState>,
    Path(path): Path<PointMappingPath>,
    headers: HeaderMap,
    Json(req): Json<UpdatePointMappingRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }
    let source_type = match normalize_optional(req.source_type, "sourceType") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let address = match normalize_optional(req.address, "address") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let protocol_detail = req.protocol_detail;
    let update = ems_storage::PointMappingUpdate {
        source_type,
        address,
        scale: req.scale,
        offset: req.offset,
        protocol_detail: protocol_detail.clone(),
    };
    if update.source_type.is_none()
        && update.address.is_none()
        && update.scale.is_none()
        && update.offset.is_none()
        && protocol_detail.is_none()
    {
        return bad_request_error("empty update");
    }
    match state
        .point_mapping_store
        .update_point_mapping(&ctx, &path.project_id, &path.source_id, update)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(point_mapping_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 删除点映射
pub async fn delete_point_mapping(
    State(state): State<AppState>,
    Path(path): Path<PointMappingPath>,
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
        .point_mapping_store
        .delete_point_mapping(&ctx, &path.project_id, &path.source_id)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(false) => not_found_error(),
        Err(err) => storage_error(err),
    }
}
