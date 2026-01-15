//! 网关 CRUD handlers
//!
//! 提供网关资源的增删改查接口：
//! - GET /projects/{id}/gateways - 列出网关
//! - POST /projects/{id}/gateways - 创建网关
//! - GET /projects/{id}/gateways/{gid} - 获取网关详情
//! - PUT /projects/{id}/gateways/{gid} - 更新网关
//! - DELETE /projects/{id}/gateways/{gid} - 删除网关
//!
//! 权限要求：
//! - 所有接口需要 Bearer token 认证
//! - 需验证项目归属当前租户（require_project_scope）

use crate::AppState;
use crate::middleware::require_project_scope;
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use crate::utils::{gateway_to_dto, normalize_optional, normalize_required};
use api_contract::{ApiResponse, CreateGatewayRequest, GatewayDto, UpdateGatewayRequest};
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
pub struct GatewayPath {
    project_id: String,
    gateway_id: String,
}

/// 列出网关
pub async fn list_gateways(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match state
        .gateway_store
        .list_gateways(&ctx, &path.project_id)
        .await
    {
        Ok(items) => {
            let data: Vec<GatewayDto> = items.into_iter().map(gateway_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 创建网关
pub async fn create_gateway(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<CreateGatewayRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let name = match normalize_required(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let status = req.status.unwrap_or_else(|| "offline".to_string());
    let record = ems_storage::GatewayRecord {
        gateway_id: Uuid::new_v4().to_string(),
        tenant_id: ctx.tenant_id.clone(),
        project_id: path.project_id,
        name,
        status,
    };
    match state.gateway_store.create_gateway(&ctx, record).await {
        Ok(item) => (
            StatusCode::OK,
            Json(ApiResponse::success(gateway_to_dto(item))),
        )
            .into_response(),
        Err(err) => storage_error(err),
    }
}

/// 获取网关详情
pub async fn get_gateway(
    State(state): State<AppState>,
    Path(path): Path<GatewayPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match state
        .gateway_store
        .find_gateway(&ctx, &path.project_id, &path.gateway_id)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(gateway_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 更新网关
pub async fn update_gateway(
    State(state): State<AppState>,
    Path(path): Path<GatewayPath>,
    headers: HeaderMap,
    Json(req): Json<UpdateGatewayRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let name = match normalize_optional(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let status = match normalize_optional(req.status, "status") {
        Ok(value) => value,
        Err(response) => return response,
    };
    if name.is_none() && status.is_none() {
        return bad_request_error("empty update");
    }
    let update = ems_storage::GatewayUpdate { name, status };
    match state
        .gateway_store
        .update_gateway(&ctx, &path.project_id, &path.gateway_id, update)
        .await
    {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(ApiResponse::success(gateway_to_dto(item))),
        )
            .into_response(),
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 删除网关
pub async fn delete_gateway(
    State(state): State<AppState>,
    Path(path): Path<GatewayPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match state
        .gateway_store
        .delete_gateway(&ctx, &path.project_id, &path.gateway_id)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(false) => not_found_error(),
        Err(err) => storage_error(err),
    }
}
