//! 历史查询 handlers
//!
//! - GET /projects/{id}/measurements

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::normalize_required;
use crate::utils::response::{bad_request_error, storage_error};
use api_contract::{ApiResponse, MeasurementValueDto, MeasurementsQuery};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use ems_storage::{MeasurementAggFn, MeasurementAggregation, MeasurementsQueryOptions, TimeOrder};

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    pub(crate) project_id: String,
}

pub async fn list_measurements(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    Query(query): Query<MeasurementsQuery>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::DATA_MEASUREMENTS_READ) {
        return response;
    }
    let point_id = match normalize_required(query.point_id, "pointId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let (Some(from), Some(to)) = (query.from, query.to) {
        if from > to {
            return bad_request_error("from must be <= to");
        }
    }
    let limit = query.limit.unwrap_or(1000);
    if limit <= 0 || limit > 5000 {
        return bad_request_error("limit out of range");
    }
    let order = match parse_order(query.order.as_deref()) {
        Ok(order) => order,
        Err(response) => return response,
    };
    let aggregation = match parse_aggregation(query.bucket_ms, query.agg.as_deref()) {
        Ok(aggregation) => aggregation,
        Err(response) => return response,
    };
    match state
        .measurement_store
        .query_measurements(
            &ctx,
            &path.project_id,
            &point_id,
            MeasurementsQueryOptions {
                from_ms: query.from,
                to_ms: query.to,
                cursor_ts_ms: query.cursor_ts_ms,
                order,
                limit,
                aggregation,
            },
        )
        .await
    {
        Ok(items) => {
            let data: Vec<MeasurementValueDto> = items
                .into_iter()
                .map(|record| MeasurementValueDto {
                    project_id: record.project_id,
                    point_id: record.point_id,
                    ts_ms: record.ts_ms,
                    value: record.value,
                    quality: record.quality,
                })
                .collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

fn parse_order(value: Option<&str>) -> Result<TimeOrder, Response> {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(TimeOrder::Asc),
        Some(value) if value.is_empty() => Ok(TimeOrder::Asc),
        Some(value) if value == "asc" => Ok(TimeOrder::Asc),
        Some(value) if value == "desc" => Ok(TimeOrder::Desc),
        Some(_) => Err(bad_request_error("order must be asc|desc")),
    }
}

fn parse_aggregation(
    bucket_ms: Option<i64>,
    agg: Option<&str>,
) -> Result<Option<MeasurementAggregation>, Response> {
    let Some(bucket_ms) = bucket_ms else {
        if agg.is_some() {
            return Err(bad_request_error("agg requires bucketMs"));
        }
        return Ok(None);
    };
    if bucket_ms <= 0 {
        return Err(bad_request_error("bucketMs must be > 0"));
    }
    let func = match agg.map(|value| value.trim().to_ascii_lowercase()) {
        None => MeasurementAggFn::Avg,
        Some(value) if value.is_empty() => MeasurementAggFn::Avg,
        Some(value) if value == "avg" => MeasurementAggFn::Avg,
        Some(value) if value == "min" => MeasurementAggFn::Min,
        Some(value) if value == "max" => MeasurementAggFn::Max,
        Some(value) if value == "sum" => MeasurementAggFn::Sum,
        Some(value) if value == "count" => MeasurementAggFn::Count,
        Some(_) => return Err(bad_request_error("agg must be avg|min|max|sum|count")),
    };
    Ok(Some(MeasurementAggregation { bucket_ms, func }))
}
