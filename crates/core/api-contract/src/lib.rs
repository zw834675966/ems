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
