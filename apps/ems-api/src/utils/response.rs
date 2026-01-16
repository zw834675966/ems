//! HTTP 响应辅助函数和 DTO 转换
//!
//! 提供统一的错误响应构造函数和 DTO 转换函数：
//! - 错误响应：auth_error, forbidden_error, bad_request_error, not_found_error, internal_auth_error, storage_error
//! - DTO 转换：project_to_dto, gateway_to_dto, device_to_dto, point_to_dto, point_mapping_to_dto, command_to_dto, audit_log_to_dto
//!
//! 设计原则：
//! - 所有错误返回统一的 ApiResponse 格式
//! - HTTP 状态码与错误码对应
//! - DTO 转换保持 Record 和 DTO 字段一致

use api_contract::{
    ApiResponse, AuditLogDto, CommandDto, CommandReceiptDto, DeviceDto, GatewayDto, PointDto,
    PointMappingDto, ProjectDto, error_codes,
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ems_auth::AuthError;
use ems_storage::{
    AuditLogRecord, CommandReceiptRecord, CommandRecord, DeviceRecord, GatewayRecord,
    PointMappingRecord, PointRecord, ProjectRecord, StorageError,
};

/// 认证错误响应
pub fn auth_error(status: StatusCode) -> Response {
    (
        status,
        Json(ApiResponse::<()>::error(
            error_codes::AUTH_UNAUTHORIZED,
            "unauthorized",
        )),
    )
        .into_response()
}

/// 禁止访问错误响应
pub fn forbidden_error() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ApiResponse::<()>::error(
            error_codes::AUTH_FORBIDDEN,
            "forbidden",
        )),
    )
        .into_response()
}

/// 错误请求响应
pub fn bad_request_error(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::<()>::error(
            error_codes::INVALID_REQUEST,
            message.into(),
        )),
    )
        .into_response()
}

/// 资源未找到错误响应
pub fn not_found_error() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::<()>::error(
            error_codes::RESOURCE_NOT_FOUND,
            "not found",
        )),
    )
        .into_response()
}

/// 认证内部错误响应
pub fn internal_auth_error(err: AuthError) -> Response {
    tracing::error!(error = ?err, "internal auth error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<()>::error(
            error_codes::INTERNAL_ERROR,
            "internal error",
        )),
    )
        .into_response()
}

/// 存储错误响应
pub fn storage_error(err: StorageError) -> Response {
    tracing::error!(error = %err, "storage error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<()>::error(
            error_codes::INTERNAL_ERROR,
            "internal error",
        )),
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
        online: false,
        last_seen_at_ms: None,
        protocol_type: record.protocol_type,
        protocol_config: record.protocol_config,
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
        online: false,
        last_seen_at_ms: None,
        room_id: record.room_id,
        address_config: record.address_config,
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
        protocol_detail: record.protocol_detail,
    }
}

/// CommandRecord 转 CommandDto
pub fn command_to_dto(record: CommandRecord) -> CommandDto {
    let payload = serde_json::from_str(&record.payload)
        .unwrap_or_else(|_| serde_json::Value::String(record.payload.clone()));
    CommandDto {
        command_id: record.command_id,
        project_id: record.project_id,
        target: record.target,
        payload,
        status: record.status,
        issued_by: record.issued_by,
        issued_at_ms: record.issued_at_ms,
    }
}

/// AuditLogRecord 转 AuditLogDto
pub fn audit_log_to_dto(record: AuditLogRecord) -> AuditLogDto {
    AuditLogDto {
        audit_id: record.audit_id,
        project_id: record.project_id,
        actor: record.actor,
        action: record.action,
        resource: record.resource,
        result: record.result,
        detail: record.detail,
        ts_ms: record.ts_ms,
    }
}

/// CommandReceiptRecord 转 CommandReceiptDto
pub fn command_receipt_to_dto(record: CommandReceiptRecord) -> CommandReceiptDto {
    CommandReceiptDto {
        receipt_id: record.receipt_id,
        command_id: record.command_id,
        project_id: record.project_id,
        status: record.status,
        message: record.message,
        ts_ms: record.ts_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;

    async fn response_json(response: Response) -> serde_json::Value {
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn forbidden_error_contract() {
        let response = forbidden_error();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = response_json(response).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], error_codes::AUTH_FORBIDDEN);
    }

    #[tokio::test]
    async fn unauthorized_error_contract() {
        let response = auth_error(StatusCode::UNAUTHORIZED);
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = response_json(response).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], error_codes::AUTH_UNAUTHORIZED);
    }
}
