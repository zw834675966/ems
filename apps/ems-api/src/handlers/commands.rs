//! 控制命令 handlers
//!
//! - GET /projects/{id}/commands
//! - POST /projects/{id}/commands

use crate::AppState;
use crate::middleware::{require_any_permission, require_permission, require_project_scope};
use crate::utils::response::{command_receipt_to_dto, command_to_dto, storage_error};
use crate::utils::validation::normalize_required;
use api_contract::{
    ApiResponse, CommandDto, CommandQuery, CommandReceiptDto, CreateCommandRequest,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use ems_control::CommandRequest;

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

#[derive(serde::Deserialize)]
pub struct CommandPath {
    project_id: String,
    command_id: String,
}

/// 列出命令
pub async fn list_commands(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    Query(query): Query<CommandQuery>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_any_permission(
        &ctx,
        &[permissions::CONTROL_COMMAND_READ, permissions::CONTROL_COMMAND_ISSUE],
    ) {
        return response;
    }
    let limit = query.limit.unwrap_or(100).max(0);
    match state
        .command_store
        .list_commands(&ctx, &path.project_id, limit)
        .await
    {
        Ok(items) => {
            let data: Vec<CommandDto> = items.into_iter().map(command_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 下发命令
pub async fn create_command(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<CreateCommandRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::CONTROL_COMMAND_ISSUE) {
        return response;
    }
    let target = match normalize_required(req.target, "target") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let now_ms = now_epoch_ms();
    let request = CommandRequest {
        project_id: path.project_id,
        target,
        payload: req.payload,
        issued_at_ms: now_ms,
    };
    match state.command_service.issue_command(&ctx, request).await {
        Ok(command) => (
            StatusCode::OK,
            Json(ApiResponse::success(command_to_dto(command))),
        )
            .into_response(),
        Err(err) => storage_error(ems_storage::StorageError::new(err.to_string())),
    }
}

/// 列出命令回执
pub async fn list_command_receipts(
    State(state): State<AppState>,
    Path(path): Path<CommandPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_any_permission(
        &ctx,
        &[permissions::CONTROL_COMMAND_READ, permissions::CONTROL_COMMAND_ISSUE],
    ) {
        return response;
    }
    match state
        .command_receipt_store
        .list_receipts(&ctx, &path.project_id, &path.command_id)
        .await
    {
        Ok(items) => {
            let data: Vec<CommandReceiptDto> =
                items.into_iter().map(command_receipt_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

fn now_epoch_ms() -> i64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as i64
}
