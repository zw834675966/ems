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
use ems_protocol::{ModbusPointDetail, TcpPointDetail};
use serde_json::Value;
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

    if let Err(res) = validate_point_mapping_protocol_detail(
        &record.source_type,
        record.protocol_detail.as_deref(),
    ) {
        return res;
    }

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

    // 若更新中带了 source_type/protocol_detail，按最终 source_type 做校验
    let effective_source_type = if let Some(st) = update.source_type.as_deref() {
        st.to_string()
    } else {
        match state
            .point_mapping_store
            .find_point_mapping(&ctx, &path.project_id, &path.source_id)
            .await
        {
            Ok(Some(m)) => m.source_type,
            Ok(None) => return not_found_error(),
            Err(err) => return storage_error(err),
        }
    };
    if let Err(res) = validate_point_mapping_protocol_detail(
        &effective_source_type,
        update.protocol_detail.as_deref(),
    ) {
        return res;
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

#[allow(clippy::result_large_err)]
fn validate_point_mapping_protocol_detail(
    source_type: &str,
    detail_str: Option<&str>,
) -> Result<(), Response> {
    let detail_str = match detail_str {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };

    match source_type {
        "modbus" => {
            let detail: ModbusPointDetail = serde_json::from_str(detail_str).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        api_contract::error_codes::INVALID_REQUEST,
                        "Modbus protocolDetail 不符合规范（functionCode/registerAddress/registerCount/dataType/wordOrder）",
                    )),
                )
                    .into_response()
            })?;

            if detail.register_count == 0 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        api_contract::error_codes::INVALID_REQUEST,
                        "Modbus registerCount/quantity 必须大于 0",
                    )),
                )
                    .into_response());
            }

            match detail.function_code {
                1 | 2 => {
                    if detail.data_type != ems_protocol::ModbusDataType::Bool {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error(
                                api_contract::error_codes::INVALID_REQUEST,
                                "Modbus 功能码 01/02 点位 dataType 必须为 bool",
                            )),
                        )
                            .into_response());
                    }
                    if detail.register_count > 2000 {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error(
                                api_contract::error_codes::INVALID_REQUEST,
                                "Modbus 功能码 01/02 quantity 建议不超过 2000",
                            )),
                        )
                            .into_response());
                    }
                }
                3 | 4 => {
                    if detail.data_type == ems_protocol::ModbusDataType::Bool {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error(
                                api_contract::error_codes::INVALID_REQUEST,
                                "Modbus 功能码 03/04 点位 dataType 不能为 bool",
                            )),
                        )
                            .into_response());
                    }
                    if detail.register_count > 125 {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error(
                                api_contract::error_codes::INVALID_REQUEST,
                                "Modbus 功能码 03/04 registerCount/quantity 不能超过 125",
                            )),
                        )
                            .into_response());
                    }
                }
                _ => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::<()>::error(
                            api_contract::error_codes::INVALID_REQUEST,
                            "当前仅支持 Modbus 01/02/03/04（读）功能码",
                        )),
                    )
                        .into_response());
                }
            }

            // 32-bit 必须有 wordOrder（在 ems-protocol 解析阶段也会强校验，这里提前拦截）
            match detail.data_type {
                ems_protocol::ModbusDataType::Int32
                | ems_protocol::ModbusDataType::Uint32
                | ems_protocol::ModbusDataType::Float32 => {
                    if detail.word_order.is_none() {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error(
                                api_contract::error_codes::INVALID_REQUEST,
                                "Modbus 32-bit 点位必须设置 wordOrder",
                            )),
                        )
                            .into_response());
                    }
                }
                _ => {}
            }
        }
        "tcp" => {
            let _detail: TcpPointDetail = serde_json::from_str(detail_str).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        api_contract::error_codes::INVALID_REQUEST,
                        "TCP protocolDetail 不符合规范（tag/valueType/endian）",
                    )),
                )
                    .into_response()
            })?;
        }
        // mqtt：保留为自由 JSON（由 normalize 模块按 jsonPath 等规则解析）
        "mqtt" => {
            serde_json::from_str::<Value>(detail_str).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        api_contract::error_codes::INVALID_REQUEST,
                        "无效的 protocolDetail JSON 格式",
                    )),
                )
                    .into_response()
            })?;
        }
        _ => {}
    }

    Ok(())
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
