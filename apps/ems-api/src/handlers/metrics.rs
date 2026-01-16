//! Telemetry 指标快照（MVP）。
//!
//! - GET /metrics

use api_contract::{ApiResponse, MetricsSnapshotDto};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use ems_telemetry::metrics;
use domain::permissions;

use crate::{AppState, middleware::{require_permission, require_tenant_context}};

pub async fn get_metrics(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::SYSTEM_METRICS_READ) {
        return response;
    }

    let snapshot = metrics().snapshot();
    (
        StatusCode::OK,
        Json(ApiResponse::success(MetricsSnapshotDto {
        raw_events: snapshot.raw_events,
        normalized_values: snapshot.normalized_values,
        write_success: snapshot.write_success,
        write_failure: snapshot.write_failure,
        dropped_duplicate: snapshot.dropped_duplicate,
        dropped_invalid: snapshot.dropped_invalid,
        dropped_stale: snapshot.dropped_stale,
        dropped_unmapped: snapshot.dropped_unmapped,
        backpressure: snapshot.backpressure,
        write_latency_ms_total: snapshot.write_latency_ms_total,
        write_latency_ms_count: snapshot.write_latency_ms_count,
        end_to_end_latency_ms_total: snapshot.end_to_end_latency_ms_total,
        end_to_end_latency_ms_count: snapshot.end_to_end_latency_ms_count,
        commands_issued: snapshot.commands_issued,
        command_dispatch_success: snapshot.command_dispatch_success,
        command_dispatch_failure: snapshot.command_dispatch_failure,
        command_issue_latency_ms_total: snapshot.command_issue_latency_ms_total,
        command_issue_latency_ms_count: snapshot.command_issue_latency_ms_count,
        receipts_processed: snapshot.receipts_processed,
        })),
    )
        .into_response()
}
