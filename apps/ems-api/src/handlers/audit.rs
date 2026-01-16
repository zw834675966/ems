//! 审计日志 handlers
//!
//! - GET /projects/{id}/audit

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::response::{audit_log_to_dto, storage_error};
use api_contract::{ApiResponse, AuditLogDto, AuditLogQuery};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;

/// 路径参数提取器
///
/// 从 URL 路径中提取项目 ID
#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

/// 查询审计日志
///
/// 路由: GET /projects/{id}/audit
/// 权限要求: 需要项目访问权限
/// 查询参数:
///   - from: 可选，开始时间戳（毫秒）
///   - to: 可选，结束时间戳（毫秒）
///   - limit: 可选，返回数量限制（默认 100）
pub async fn list_audit_logs(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    Query(query): Query<AuditLogQuery>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::CONTROL_COMMAND_READ) {
        return response;
    }
    let limit = query.limit.unwrap_or(100).max(0);
    match state
        .audit_log_store
        .list_audit_logs(&ctx, &path.project_id, query.from, query.to, limit)
        .await
    {
        Ok(items) => {
            let data: Vec<AuditLogDto> = items.into_iter().map(audit_log_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}
