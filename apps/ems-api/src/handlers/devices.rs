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
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::response::device_to_dto;
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use crate::utils::{normalize_optional, normalize_required};
use api_contract::{ApiResponse, CreateDeviceRequest, DeviceDto, UpdateDeviceRequest};
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
pub struct DevicePath {
    project_id: String,
    device_id: String,
}

/// 列出设备
///
/// 查询指定项目下的所有设备列表。
///
/// # 参数
///
/// - `state`: 应用状态，包含 `device_store` 存储实例
/// - `path`: 路径参数，包含 `project_id`
/// - `headers`: HTTP 请求头，用于提取 Bearer token 进行认证
///
/// # 返回
///
/// 成功时返回 `200 OK` 和设备列表，失败时返回相应的错误响应。
///
/// # 流程
///
/// 1. 调用 `require_project_scope` 验证 Bearer token 和项目归属
/// 2. 调用 `device_store.list_devices` 查询该项目的所有设备
/// 3. 将 `DeviceRecord` 列表转换为 `DeviceDto` 列表
/// 4. 返回统一的 API 响应格式
///
/// # 错误处理
///
/// - `401 UNAUTHORIZED`: 认证失败（token 无效或过期）
/// - `403 FORBIDDEN`: 项目归属验证失败（项目不属于当前租户）
/// - `500 INTERNAL SERVER ERROR`: 存储层错误
pub async fn list_devices(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_DEVICE_READ) {
        return response;
    }
    match state
        .device_store
        .list_devices(&ctx, &path.project_id)
        .await
    {
        Ok(items) => {
            let device_ids: Vec<String> = items.iter().map(|item| item.device_id.clone()).collect();
            let online = state
                .online_store
                .list_devices_last_seen_at_ms(&ctx, &path.project_id, &device_ids)
                .await
                .unwrap_or_default();
            let data: Vec<DeviceDto> = items
                .into_iter()
                .map(|record| {
                    let mut dto = device_to_dto(record);
                    if let Some(ts_ms) = online.get(&dto.device_id).copied() {
                        dto.online = true;
                        dto.last_seen_at_ms = Some(ts_ms);
                    }
                    dto
                })
                .collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 创建设备
///
/// 在指定项目下创建新设备。创建前会验证网关是否存在且属于该项目。
///
/// # 参数
///
/// - `state`: 应用状态，包含 `device_store` 和 `gateway_store` 存储实例
/// - `path`: 路径参数，包含 `project_id`
/// - `headers`: HTTP 请求头，用于提取 Bearer token 进行认证
/// - `req`: 请求体，包含设备创建信息（gateway_id、name、model）
///
/// # 返回
///
/// 成功时返回 `200 OK` 和创建的设备信息，失败时返回相应的错误响应。
///
/// # 流程
///
/// 1. 调用 `require_project_scope` 验证 Bearer token 和项目归属
/// 2. 使用 `normalize_required` 验证必填字段（gateway_id、name）
/// 3. 调用 `gateway_store.find_gateway` 验证网关存在且属于该项目
/// 4. 生成新的设备 ID（UUID v4）
/// 5. 创建 `DeviceRecord` 并调用 `device_store.create_device` 保存
/// 6. 将 `DeviceRecord` 转换为 `DeviceDto` 并返回
///
/// # 错误处理
///
/// - `400 BAD REQUEST`: 必填字段缺失或网关不存在
/// - `401 UNAUTHORIZED`: 认证失败
/// - `403 FORBIDDEN`: 项目归属验证失败
/// - `500 INTERNAL SERVER ERROR`: 存储层错误
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
    if let Err(response) = require_permission(&ctx, permissions::ASSET_DEVICE_WRITE) {
        return response;
    }
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
        room_id: req.room_id,
        address_config: req.address_config,
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
///
/// 查询指定设备的详细信息。
///
/// # 参数
///
/// - `state`: 应用状态，包含 `device_store` 存储实例
/// - `path`: 路径参数，包含 `project_id` 和 `device_id`
/// - `headers`: HTTP 请求头，用于提取 Bearer token 进行认证
///
/// # 返回
///
/// 成功时返回 `200 OK` 和设备详情，设备不存在时返回 `404 NOT FOUND`。
///
/// # 流程
///
/// 1. 调用 `require_project_scope` 验证 Bearer token 和项目归属
/// 2. 调用 `device_store.find_device` 查询设备
/// 3. 如果设备存在，转换为 `DeviceDto` 并返回
/// 4. 如果设备不存在，返回 `404 NOT FOUND`
///
/// # 错误处理
///
/// - `401 UNAUTHORIZED`: 认证失败
/// - `403 FORBIDDEN`: 项目归属验证失败
/// - `404 NOT FOUND`: 设备不存在
/// - `500 INTERNAL SERVER ERROR`: 存储层错误
pub async fn get_device(
    State(state): State<AppState>,
    Path(path): Path<DevicePath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_DEVICE_READ) {
        return response;
    }
    match state
        .device_store
        .find_device(&ctx, &path.project_id, &path.device_id)
        .await
    {
        Ok(Some(item)) => {
            let last_seen_at_ms = state
                .online_store
                .get_device_last_seen_at_ms(&ctx, &path.project_id, &path.device_id)
                .await
                .ok()
                .flatten();
            let mut dto = device_to_dto(item);
            if let Some(ts_ms) = last_seen_at_ms {
                dto.online = true;
                dto.last_seen_at_ms = Some(ts_ms);
            }
            (StatusCode::OK, Json(ApiResponse::success(dto))).into_response()
        }
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 更新设备
///
/// 更新指定设备的信息。支持更新名称和型号，至少需要提供一个更新字段。
///
/// # 参数
///
/// - `state`: 应用状态，包含 `device_store` 存储实例
/// - `path`: 路径参数，包含 `project_id` 和 `device_id`
/// - `headers`: HTTP 请求头，用于提取 Bearer token 进行认证
/// - `req`: 请求体，包含要更新的字段（name、model，都是可选的）
///
/// # 返回
///
/// 成功时返回 `200 OK` 和更新后的设备信息，设备不存在时返回 `404 NOT FOUND`。
///
/// # 流程
///
/// 1. 调用 `require_project_scope` 验证 Bearer token 和项目归属
/// 2. 使用 `normalize_optional` 验证可选字段（name、model）
/// 3. 检查是否至少有一个更新字段
/// 4. 调用 `device_store.update_device` 更新设备
/// 5. 如果更新成功，返回更新后的设备信息
/// 6. 如果设备不存在，返回 `404 NOT FOUND`
///
/// # 错误处理
///
/// - `400 BAD REQUEST`: 没有提供更新字段或字段格式错误
/// - `401 UNAUTHORIZED`: 认证失败
/// - `403 FORBIDDEN`: 项目归属验证失败
/// - `404 NOT FOUND`: 设备不存在
/// - `500 INTERNAL SERVER ERROR`: 存储层错误
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
    if let Err(response) = require_permission(&ctx, permissions::ASSET_DEVICE_WRITE) {
        return response;
    }
    let name = match normalize_optional(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let model = match normalize_optional(req.model, "model") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let room_id = req.room_id;
    let address_config = req.address_config;
    if name.is_none() && model.is_none() && room_id.is_none() && address_config.is_none() {
        return bad_request_error("empty update");
    }
    let update = ems_storage::DeviceUpdate {
        name,
        model,
        room_id,
        address_config,
    };
    match state
        .device_store
        .update_device(&ctx, &path.project_id, &path.device_id, update)
        .await
    {
        Ok(Some(item)) => {
            let last_seen_at_ms = state
                .online_store
                .get_device_last_seen_at_ms(&ctx, &path.project_id, &path.device_id)
                .await
                .ok()
                .flatten();
            let mut dto = device_to_dto(item);
            if let Some(ts_ms) = last_seen_at_ms {
                dto.online = true;
                dto.last_seen_at_ms = Some(ts_ms);
            }
            (StatusCode::OK, Json(ApiResponse::success(dto))).into_response()
        }
        Ok(None) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 删除设备
///
/// 删除指定的设备。删除成功后返回空数据。
///
/// # 参数
///
/// - `state`: 应用状态，包含 `device_store` 存储实例
/// - `path`: 路径参数，包含 `project_id` 和 `device_id`
/// - `headers`: HTTP 请求头，用于提取 Bearer token 进行认证
///
/// # 返回
///
/// 成功时返回 `200 OK` 和空数据，设备不存在时返回 `404 NOT FOUND`。
///
/// # 流程
///
/// 1. 调用 `require_project_scope` 验证 Bearer token 和项目归属
/// 2. 调用 `device_store.delete_device` 删除设备
/// 3. 如果删除成功，返回 `200 OK`
/// 4. 如果设备不存在，返回 `404 NOT FOUND`
///
/// # 错误处理
///
/// - `401 UNAUTHORIZED`: 认证失败
/// - `403 FORBIDDEN`: 项目归属验证失败
/// - `404 NOT FOUND`: 设备不存在
/// - `500 INTERNAL SERVER ERROR`: 存储层错误
///
/// # 注意事项
///
/// - 删除设备会级联删除该设备下的所有点位（取决于数据库约束）
/// - 删除操作不可逆，建议在前端增加确认提示
pub async fn delete_device(
    State(state): State<AppState>,
    Path(path): Path<DevicePath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_DEVICE_WRITE) {
        return response;
    }
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
