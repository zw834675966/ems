//! 实时查询 handlers
//!
//! - GET /projects/{id}/realtime

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::normalize_optional;
use crate::utils::response::storage_error;
use api_contract::{ApiResponse, RealtimeQuery, RealtimeValueDto};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    pub(crate) project_id: String,
}

pub async fn get_realtime(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    Query(query): Query<RealtimeQuery>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::DATA_REALTIME_READ) {
        return response;
    }
    let point_id = match normalize_optional(query.point_id, "pointId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let records = if let Some(point_id) = point_id {
        match state
            .realtime_store
            .get_last_value(&ctx, &path.project_id, &point_id)
            .await
        {
            Ok(Some(item)) => vec![item],
            Ok(None) => Vec::new(),
            Err(err) => return storage_error(err),
        }
    } else {
        match state
            .realtime_store
            .list_last_values(&ctx, &path.project_id)
            .await
        {
            Ok(items) => items,
            Err(err) => return storage_error(err),
        }
    };
    let data: Vec<RealtimeValueDto> = records
        .into_iter()
        .map(|record| RealtimeValueDto {
            project_id: record.project_id,
            point_id: record.point_id,
            ts_ms: record.ts_ms,
            value: record.value,
            quality: record.quality,
        })
        .collect();
    (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
}
