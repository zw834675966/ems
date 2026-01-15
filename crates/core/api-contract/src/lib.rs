//! 稳定的 DTO 与 API 响应契约。

use serde::{Deserialize, Serialize};

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
}

/// 网关更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGatewayRequest {
    pub name: Option<String>,
    pub status: Option<String>,
}

/// 网关返回结构。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayDto {
    pub gateway_id: String,
    pub project_id: String,
    pub name: String,
    pub status: String,
}

/// 设备创建请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeviceRequest {
    pub gateway_id: String,
    pub name: String,
    pub model: Option<String>,
}

/// 设备更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub model: Option<String>,
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
}

/// 点位映射更新请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePointMappingRequest {
    pub source_type: Option<String>,
    pub address: Option<String>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
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
}
