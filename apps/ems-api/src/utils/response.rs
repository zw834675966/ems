//! HTTP 响应辅助函数和 DTO 转换
//!
//! 提供统一的错误响应构造函数和 DTO 转换函数：
//! - 错误响应：auth_error, forbidden_error, bad_request_error, not_found_error, internal_auth_error, storage_error
//! - DTO 转换：project_to_dto, gateway_to_dto, device_to_dto, point_to_dto, point_mapping_to_dto
//!
//! 设计原则：
//! - 所有错误返回统一的 ApiResponse 格式
//! - HTTP 状态码与错误码对应
//! - DTO 转换保持 Record 和 DTO 字段一致

use api_contract::{ApiResponse, DeviceDto, GatewayDto, PointDto, PointMappingDto, ProjectDto};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ems_auth::AuthError;
use ems_storage::{
    DeviceRecord, GatewayRecord, PointMappingRecord, PointRecord, ProjectRecord, StorageError,
};

/// 认证错误响应
pub fn auth_error(status: StatusCode) -> Response {
    (
        status,
        Json(ApiResponse::<()>::error(
            "AUTH.UNAUTHORIZED",
            "unauthorized",
        )),
    )
        .into_response()
}

/// 禁止访问错误响应
pub fn forbidden_error() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ApiResponse::<()>::error("AUTH.FORBIDDEN", "forbidden")),
    )
        .into_response()
}

/// 错误请求响应
pub fn bad_request_error(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::<()>::error("INVALID.REQUEST", message.into())),
    )
        .into_response()
}

/// 资源未找到错误响应
pub fn not_found_error() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::<()>::error("RESOURCE.NOT_FOUND", "not found")),
    )
        .into_response()
}

/// 认证内部错误响应
pub fn internal_auth_error(err: AuthError) -> Response {
    let message = err.to_string();
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<()>::error("INTERNAL.ERROR", message)),
    )
        .into_response()
}

/// 存储错误响应
pub fn storage_error(err: StorageError) -> Response {
    let message = err.to_string();
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<()>::error("INTERNAL.ERROR", message)),
    )
        .into_response()
}

/// ProjectRecord 转 ProjectDto
pub fn project_to_dto(record: ProjectRecord) -> ProjectDto {
    ProjectDto {
        project_id: record.project_id,
        name: record.name,
        timezone: record.timezone,
    }
}

/// GatewayRecord 转 GatewayDto
pub fn gateway_to_dto(record: GatewayRecord) -> GatewayDto {
    GatewayDto {
        gateway_id: record.gateway_id,
        project_id: record.project_id,
        name: record.name,
        status: record.status,
    }
}

/// DeviceRecord 转 DeviceDto
pub fn device_to_dto(record: DeviceRecord) -> DeviceDto {
    DeviceDto {
        device_id: record.device_id,
        project_id: record.project_id,
        gateway_id: record.gateway_id,
        name: record.name,
        model: record.model,
    }
}

/// PointRecord 转 PointDto
pub fn point_to_dto(record: PointRecord) -> PointDto {
    PointDto {
        point_id: record.point_id,
        project_id: record.project_id,
        device_id: record.device_id,
        key: record.key,
        data_type: record.data_type,
        unit: record.unit,
    }
}

/// PointMappingRecord 转 PointMappingDto
pub fn point_mapping_to_dto(record: PointMappingRecord) -> PointMappingDto {
    PointMappingDto {
        source_id: record.source_id,
        project_id: record.project_id,
        point_id: record.point_id,
        source_type: record.source_type,
        address: record.address,
        scale: record.scale,
        offset: record.offset,
    }
}
