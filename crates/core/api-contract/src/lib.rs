//! 稳定的 DTO 与 API 响应契约。

use serde::{Deserialize, Serialize};

/// 稳定错误码清单（跨前后端对齐）。
pub mod error_codes {
    pub const AUTH_UNAUTHORIZED: &str = "AUTH.UNAUTHORIZED";
    pub const AUTH_FORBIDDEN: &str = "AUTH.FORBIDDEN";
    pub const INVALID_REQUEST: &str = "INVALID.REQUEST";
    pub const RESOURCE_NOT_FOUND: &str = "RESOURCE.NOT_FOUND";
    pub const INTERNAL_ERROR: &str = "INTERNAL.ERROR";
}

/// 标准 API 响应封装。
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

/// 失败响应的错误体。
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

/// 登录请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应体。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: u64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 刷新 token 请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    #[serde(alias = "refresh_token")]
    pub refresh_token: String,
}

/// 刷新 token 响应体。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: u64,
}

/// 动态路由返回结构（兼容 pure-admin-thin）。
#[derive(Debug, Serialize)]
pub struct AsyncRoute {
    pub path: String,
    pub name: String,
    pub component: String,
    pub meta: RouteMeta,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AsyncRoute>,
}

/// 路由元数据。
#[derive(Debug, Serialize)]
pub struct RouteMeta {
    pub title: String,
    pub icon: String,
    pub rank: i32,
    pub roles: Option<Vec<String>>,
    pub auths: Option<Vec<String>>,
}

/// RBAC 用户返回结构（tenant 级）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RbacUserDto {
    pub user_id: String,
    pub username: String,
    pub status: String,
    pub roles: Vec<String>,
}

/// RBAC 创建用户请求体（tenant 级）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRbacUserRequest {
    pub username: String,
    pub password: String,
    pub status: Option<String>,
    pub roles: Option<Vec<String>>,
}

/// RBAC 更新用户请求体（tenant 级）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRbacUserRequest {
    pub password: Option<String>,
    pub status: Option<String>,
}

/// RBAC 设置用户角色请求体（tenant 级，替换模式）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserRolesRequest {
    pub roles: Vec<String>,
}

/// RBAC 角色返回结构（tenant 级）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RbacRoleDto {
    pub role_code: String,
    pub name: String,
    pub permissions: Vec<String>,
}

/// RBAC 创建角色请求体（tenant 级）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRbacRoleRequest {
    pub role_code: String,
    pub name: String,
    pub permissions: Option<Vec<String>>,
}

/// RBAC 设置角色权限请求体（tenant 级，替换模式）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRolePermissionsRequest {
    pub permissions: Vec<String>,
}

/// 权限码返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDto {
    pub permission_code: String,
    pub description: String,
}

/// 项目创建请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    pub timezone: Option<String>,
}

/// 项目更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub timezone: Option<String>,
}

/// 项目返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub project_id: String,
    pub name: String,
    pub timezone: String,
}

/// 网关创建请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGatewayRequest {
    pub name: String,
    pub status: Option<String>,
    /// 协议类型: mqtt | modbus_tcp | tcp_server | tcp_client
    pub protocol_type: Option<String>,
    /// 协议配置（JSON 字符串）
    pub protocol_config: Option<String>,
}

/// 网关更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGatewayRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub protocol_type: Option<String>,
    pub protocol_config: Option<String>,
}

/// 网关返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayDto {
    pub gateway_id: String,
    pub project_id: String,
    pub name: String,
    pub status: String,
    pub online: bool,
    pub last_seen_at_ms: Option<i64>,
    pub protocol_type: String,
    pub protocol_config: Option<String>,
}

/// 设备创建请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeviceRequest {
    pub gateway_id: String,
    pub name: String,
    pub model: Option<String>,
    /// 设备所在房间 ID
    pub room_id: Option<String>,
    /// 协议地址配置（JSON 字符串）
    pub address_config: Option<String>,
}

/// 设备更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub model: Option<String>,
    pub room_id: Option<String>,
    pub address_config: Option<String>,
}

/// 设备返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDto {
    pub device_id: String,
    pub project_id: String,
    pub gateway_id: String,
    pub name: String,
    pub model: Option<String>,
    pub online: bool,
    pub last_seen_at_ms: Option<i64>,
    pub room_id: Option<String>,
    pub address_config: Option<String>,
}

/// 点位创建请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePointRequest {
    pub device_id: String,
    pub key: String,
    pub data_type: String,
    pub unit: Option<String>,
}

/// 点位更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePointRequest {
    pub key: Option<String>,
    pub data_type: Option<String>,
    pub unit: Option<String>,
}

/// 点位返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PointDto {
    pub point_id: String,
    pub project_id: String,
    pub device_id: String,
    pub key: String,
    pub data_type: String,
    pub unit: Option<String>,
}

/// 点位映射创建请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePointMappingRequest {
    pub point_id: String,
    pub source_type: String,
    pub address: String,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    /// 协议细节配置（JSON 字符串）
    pub protocol_detail: Option<String>,
}

/// 点位映射更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePointMappingRequest {
    pub source_type: Option<String>,
    pub address: Option<String>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub protocol_detail: Option<String>,
}

/// 点位映射返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PointMappingDto {
    pub source_id: String,
    pub project_id: String,
    pub point_id: String,
    pub source_type: String,
    pub address: String,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub protocol_detail: Option<String>,
}

/// 实时查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeQuery {
    pub point_id: Option<String>,
}

/// 实时返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeValueDto {
    pub project_id: String,
    pub point_id: String,
    pub ts_ms: i64,
    pub value: String,
    pub quality: Option<String>,
}

/// 历史查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasurementsQuery {
    pub point_id: String,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<i64>,
    /// 可选游标（毫秒时间戳），用于按时间做 keyset 分页。
    pub cursor_ts_ms: Option<i64>,
    /// 排序方式：`asc`/`desc`，默认 `asc`。
    pub order: Option<String>,
    /// 聚合桶大小（毫秒）。提供该字段将返回聚合结果（value 为聚合值字符串，tsMs 为桶起始）。
    pub bucket_ms: Option<i64>,
    /// 聚合函数：`avg`/`min`/`max`/`sum`/`count`。默认 `avg`。
    pub agg: Option<String>,
}

/// 历史返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasurementValueDto {
    pub project_id: String,
    pub point_id: String,
    pub ts_ms: i64,
    pub value: String,
    pub quality: Option<String>,
}

/// 命令创建请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommandRequest {
    pub target: String,
    pub payload: serde_json::Value,
}

/// 命令查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandQuery {
    pub limit: Option<i64>,
}

/// 命令返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDto {
    pub command_id: String,
    pub project_id: String,
    pub target: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub issued_by: String,
    pub issued_at_ms: i64,
}

/// 命令回执返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandReceiptDto {
    pub receipt_id: String,
    pub command_id: String,
    pub project_id: String,
    pub status: String,
    pub message: Option<String>,
    pub ts_ms: i64,
}

/// 审计查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<i64>,
}

/// 审计日志返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogDto {
    pub audit_id: String,
    pub project_id: Option<String>,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub result: String,
    pub detail: Option<String>,
    pub ts_ms: i64,
}

/// Telemetry 指标快照（MVP，聚合计数）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSnapshotDto {
    pub raw_events: u64,
    pub normalized_values: u64,
    pub write_success: u64,
    pub write_failure: u64,
    pub dropped_duplicate: u64,
    pub dropped_invalid: u64,
    pub dropped_stale: u64,
    pub dropped_unmapped: u64,
    pub backpressure: u64,
    pub write_latency_ms_total: u64,
    pub write_latency_ms_count: u64,
    pub end_to_end_latency_ms_total: u64,
    pub end_to_end_latency_ms_count: u64,
    pub commands_issued: u64,
    pub command_dispatch_success: u64,
    pub command_dispatch_failure: u64,
    pub command_issue_latency_ms_total: u64,
    pub command_issue_latency_ms_count: u64,
    pub receipts_processed: u64,
}
