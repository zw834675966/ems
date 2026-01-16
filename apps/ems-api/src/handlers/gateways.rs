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
//!
//! 数据隔离：
//! - 所有操作都通过 TenantContext 进行多租户隔离
//! - 存储层会根据 tenant_id 和 project_id 过滤数据
//!
//! 数据转换：
//! - GatewayRecord（存储层）→ GatewayDto（API 响应）
//! - tenant_id 字段在 DTO 中被排除，确保租户信息不泄露

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::response::gateway_to_dto;
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use crate::utils::{normalize_optional, normalize_required};
use api_contract::{ApiResponse, CreateGatewayRequest, GatewayDto, UpdateGatewayRequest};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use uuid::Uuid;

/// 项目路径参数
///
/// 从 URL 路径中提取项目 ID，例如 `/projects/{project_id}/gateways`
#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

/// 网关路径参数
///
/// 从 URL 路径中提取项目 ID 和网关 ID，例如 `/projects/{project_id}/gateways/{gateway_id}`
#[derive(serde::Deserialize)]
pub struct GatewayPath {
    project_id: String,
    gateway_id: String,
}

/// 列出网关
///
/// # HTTP 方法与路径
/// `GET /projects/{project_id}/gateways`
///
/// # 功能描述
/// 获取指定项目下的所有网关列表，返回网关的基本信息（不含租户 ID）。
///
/// # 认证与授权
/// 1. **认证**：通过 `require_project_scope` 验证 Bearer token
/// 2. **授权**：验证项目是否属于当前租户
/// 3. **多租户隔离**：`list_gateways` 根据 `tenant_id` 和 `project_id` 过滤数据
///
/// # 请求流程
/// ```text
/// 1. 从请求头提取 Authorization: Bearer <token>
/// 2. 调用 require_project_scope:
///    a. 验证 token 并提取 TenantContext（包含 tenant_id, user_id, roles, permissions）
///    b. 调用 project_belongs_to_tenant 验证项目归属
///    c. 设置 ctx.project_scope = Some(project_id)
/// 3. 调用 gateway_store.list_gateways(&ctx, &project_id) 查询网关列表
/// 4. 将 GatewayRecord 转换为 GatewayDto（排除 tenant_id）
/// 5. 返回 200 OK + JSON 响应
/// ```
///
/// # 成功响应示例 (200 OK)
/// ```json
/// {
///   "success": true,
///   "data": [
///     {
///       "gatewayId": "550e8400-e29b-41d4-a716-446655440000",
///       "projectId": "project-1",
///       "name": "Gateway-1",
///       "status": "online"
///     }
///   ],
///   "error": null
/// }
/// ```
///
/// # 错误响应
/// - `401 UNAUTHORIZED`：Bearer token 无效或缺失
/// - `403 FORBIDDEN`：项目不属于当前租户
/// - `500 INTERNAL SERVER ERROR`：存储层查询失败
///
/// # 存储层接口
/// `GatewayStore::list_gateways(ctx, project_id) -> Result<Vec<GatewayRecord>, StorageError>`
pub async fn list_gateways(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    // 步骤 1: 验证项目归属，获取增强的租户上下文
    // - 如果 token 无效 → 返回 401 UNAUTHORIZED
    // - 如果项目不属于当前租户 → 返回 403 FORBIDDEN
    // - 如果存储层错误 → 返回 500 INTERNAL SERVER ERROR
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_GATEWAY_READ) {
        return response;
    }

    // 步骤 2: 查询网关列表
    // - 存储层会根据 ctx.tenant_id 和 project_id 自动过滤数据
    // - 返回的 GatewayRecord 包含 tenant_id，但不会暴露给客户端
    match state
        .gateway_store
        .list_gateways(&ctx, &path.project_id)
        .await
    {
        Ok(items) => {
            let gateway_ids: Vec<String> =
                items.iter().map(|item| item.gateway_id.clone()).collect();
            let online = state
                .online_store
                .list_gateways_last_seen_at_ms(&ctx, &path.project_id, &gateway_ids)
                .await
                .unwrap_or_default();
            let data: Vec<GatewayDto> = items
                .into_iter()
                .map(|record| {
                    let mut dto = gateway_to_dto(record);
                    if let Some(ts_ms) = online.get(&dto.gateway_id).copied() {
                        dto.online = true;
                        dto.last_seen_at_ms = Some(ts_ms);
                    }
                    dto
                })
                .collect();
            // 步骤 4: 返回成功响应
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 创建网关
///
/// # HTTP 方法与路径
/// `POST /projects/{project_id}/gateways`
///
/// # 功能描述
/// 在指定项目下创建新网关。网关 ID 自动生成（UUID v4），状态默认为 "offline"。
///
/// # 认证与授权
/// - **认证**：通过 `require_project_scope` 验证 Bearer token
/// - **授权**：验证项目是否属于当前租户
/// - **多租户隔离**：创建时自动注入 `tenant_id`，确保数据隔离
///
/// # 请求体示例
/// ```json
/// {
///   "name": "Gateway-1",
///   "status": "offline"  // 可选，默认 "offline"
/// }
/// ```
///
/// # 验证规则
/// - `name`：必填，去除首尾空格后不能为空
/// - `status`：可选，默认 "offline"
///
/// # 成功响应示例 (200 OK)
/// ```json
/// {
///   "success": true,
///   "data": {
///     "gatewayId": "550e8400-e29b-41d4-a716-446655440000",
///     "projectId": "project-1",
///     "name": "Gateway-1",
///     "status": "offline"
///   },
///   "error": null
/// }
/// ```
///
/// # 错误响应
/// - `400 BAD REQUEST`：name 字段为空或仅包含空格
/// - `401 UNAUTHORIZED`：Bearer token 无效或缺失
/// - `403 FORBIDDEN`：项目不属于当前租户
/// - `500 INTERNAL SERVER ERROR`：存储层创建失败
pub async fn create_gateway(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<CreateGatewayRequest>,
) -> Response {
    // 步骤 1: 验证项目归属，获取租户上下文
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_GATEWAY_WRITE) {
        return response;
    }

    // 步骤 2: 验证必填字段 name（去除空格并检查非空）
    let name = match normalize_required(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };

    // 步骤 3: 处理可选字段 status，默认值为 "offline"
    let status = req.status.unwrap_or_else(|| "offline".to_string());

    // 步骤 4: 构建网关记录
    // - gateway_id: 自动生成 UUID v4
    // - tenant_id: 从上下文获取（多租户隔离）
    // - project_id: 从路径参数获取
    // - protocol_type: 默认 mqtt
    let record = ems_storage::GatewayRecord {
        gateway_id: Uuid::new_v4().to_string(),
        tenant_id: ctx.tenant_id.clone(),
        project_id: path.project_id,
        name,
        status,
        protocol_type: req.protocol_type.unwrap_or_else(|| "mqtt".to_string()),
        protocol_config: req.protocol_config,
    };

    // 步骤 5: 创建网关并返回
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
///
/// # HTTP 方法与路径
/// `GET /projects/{project_id}/gateways/{gateway_id}`
///
/// # 功能描述
/// 根据项目 ID 和网关 ID 获取指定网关的详细信息。
///
/// # 认证与授权
/// - **认证**：通过 `require_project_scope` 验证 Bearer token
/// - **授权**：验证项目是否属于当前租户
/// - **多租户隔离**：查询时根据 `tenant_id` 和 `project_id` 过滤数据
///
/// # 成功响应示例 (200 OK)
/// ```json
/// {
///   "success": true,
///   "data": {
///     "gatewayId": "550e8400-e29b-41d4-a716-446655440000",
///     "projectId": "project-1",
///     "name": "Gateway-1",
///     "status": "online"
///   },
///   "error": null
/// }
/// ```
///
/// # 错误响应
/// - `401 UNAUTHORIZED`：Bearer token 无效或缺失
/// - `403 FORBIDDEN`：项目不属于当前租户
/// - `404 NOT FOUND`：网关不存在或不属于当前租户/项目
/// - `500 INTERNAL SERVER ERROR`：存储层查询失败
pub async fn get_gateway(
    State(state): State<AppState>,
    Path(path): Path<GatewayPath>,
    headers: HeaderMap,
) -> Response {
    // 步骤 1: 验证项目归属，获取租户上下文
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_GATEWAY_READ) {
        return response;
    }

    // 步骤 2: 查询网关详情
    // - 存储层根据 ctx.tenant_id 和 project_id 过滤数据
    // - 返回 Option<GatewayRecord> 表示可能不存在
    match state
        .gateway_store
        .find_gateway(&ctx, &path.project_id, &path.gateway_id)
        .await
    {
        Ok(Some(item)) => {
            let last_seen_at_ms = state
                .online_store
                .get_gateway_last_seen_at_ms(&ctx, &path.project_id, &path.gateway_id)
                .await
                .ok()
                .flatten();
            let mut dto = gateway_to_dto(item);
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

/// 更新网关
///
/// # HTTP 方法与路径
/// `PUT /projects/{project_id}/gateways/{gateway_id}`
///
/// # 功能描述
/// 部分更新指定网关的字段（name 或 status）。支持部分更新，只更新提供的字段。
///
/// # 认证与授权
/// - **认证**：通过 `require_project_scope` 验证 Bearer token
/// - **授权**：验证项目是否属于当前租户
/// - **多租户隔离**：更新时根据 `tenant_id` 和 `project_id` 过滤数据
///
/// # 请求体示例
/// ```json
/// {
///   "name": "Updated-Gateway-1",
///   "status": "online"
/// }
/// ```
///
/// # 验证规则
/// - `name`：可选，如果提供则去除首尾空格后不能为空
/// - `status`：可选，如果提供则去除首尾空格后不能为空
/// - 至少提供一个字段，否则返回 400 BAD REQUEST
///
/// # 成功响应示例 (200 OK)
/// ```json
/// {
///   "success": true,
///   "data": {
///     "gatewayId": "550e8400-e29b-41d4-a716-446655440000",
///     "projectId": "project-1",
///     "name": "Updated-Gateway-1",
///     "status": "online"
///   },
///   "error": null
/// }
/// ```
///
/// # 错误响应
/// - `400 BAD REQUEST`：未提供任何更新字段，或字段为空
/// - `401 UNAUTHORIZED`：Bearer token 无效或缺失
/// - `403 FORBIDDEN`：项目不属于当前租户
/// - `404 NOT FOUND`：网关不存在或不属于当前租户/项目
/// - `500 INTERNAL SERVER ERROR`：存储层更新失败
pub async fn update_gateway(
    State(state): State<AppState>,
    Path(path): Path<GatewayPath>,
    headers: HeaderMap,
    Json(req): Json<UpdateGatewayRequest>,
) -> Response {
    // 步骤 1: 验证项目归属，获取租户上下文
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_GATEWAY_WRITE) {
        return response;
    }

    // 步骤 2: 验证并处理可选字段 name
    let name = match normalize_optional(req.name, "name") {
        Ok(value) => value,
        Err(response) => return response,
    };

    // 步骤 3: 验证并处理可选字段 status
    let status = match normalize_optional(req.status, "status") {
        Ok(value) => value,
        Err(response) => return response,
    };

    // 步骤 4: 检查是否至少提供了一个更新字段
    let protocol_type = req.protocol_type;
    let protocol_config = req.protocol_config;
    if name.is_none() && status.is_none() && protocol_type.is_none() && protocol_config.is_none() {
        return bad_request_error("empty update");
    }

    // 步骤 5: 构建更新对象
    let update = ems_storage::GatewayUpdate {
        name,
        status,
        protocol_type,
        protocol_config,
    };

    // 步骤 6: 执行更新并返回
    match state
        .gateway_store
        .update_gateway(&ctx, &path.project_id, &path.gateway_id, update)
        .await
    {
        Ok(Some(item)) => {
            let last_seen_at_ms = state
                .online_store
                .get_gateway_last_seen_at_ms(&ctx, &path.project_id, &path.gateway_id)
                .await
                .ok()
                .flatten();
            let mut dto = gateway_to_dto(item);
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

/// 删除网关
///
/// # HTTP 方法与路径
/// `DELETE /projects/{project_id}/gateways/{gateway_id}`
///
/// # 功能描述
/// 删除指定网关。删除操作不可逆，请谨慎使用。
///
/// # 认证与授权
/// - **认证**：通过 `require_project_scope` 验证 Bearer token
/// - **授权**：验证项目是否属于当前租户
/// - **多租户隔离**：删除时根据 `tenant_id` 和 `project_id` 过滤数据
///
/// # 成功响应示例 (200 OK)
/// ```json
/// {
///   "success": true,
///   "data": null,
///   "error": null
/// }
/// ```
///
/// # 错误响应
/// - `401 UNAUTHORIZED`：Bearer token 无效或缺失
/// - `403 FORBIDDEN`：项目不属于当前租户
/// - `404 NOT FOUND`：网关不存在或不属于当前租户/项目
/// - `500 INTERNAL SERVER ERROR`：存储层删除失败
///
/// # 注意事项
/// - 删除操作不可逆
/// - 删除网关可能会影响关联的设备和点位数据
/// - 建议在删除前检查是否有依赖关系
pub async fn delete_gateway(
    State(state): State<AppState>,
    Path(path): Path<GatewayPath>,
    headers: HeaderMap,
) -> Response {
    // 步骤 1: 验证项目归属，获取租户上下文
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_GATEWAY_WRITE) {
        return response;
    }

    // 步骤 2: 执行删除操作
    // - 存储层根据 ctx.tenant_id 和 project_id 过滤数据
    // - 返回 bool 表示是否成功删除
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
