//! Telemetry 指标快照（MVP）。
//!
//! - GET /metrics        - JSON 格式（默认，需要权限）
//! - GET /metrics/prometheus - Prometheus 文本格式（需要权限）
//!
//! Prometheus 格式示例：
//! ```text
//! # HELP ems_raw_events_total Total number of raw events received
//! # TYPE ems_raw_events_total counter
//! ems_raw_events_total 12345
//!
//! # HELP ems_write_success_total Total number of successful writes
//! # TYPE ems_write_success_total counter
//! ems_write_success_total 12000
//! ```

use api_contract::{ApiResponse, MetricsSnapshotDto};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use ems_telemetry::metrics;

use crate::{
    AppState,
    middleware::{require_permission, require_tenant_context},
};

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

/// Prometheus 格式指标端点
pub async fn get_metrics_prometheus(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::SYSTEM_METRICS_READ) {
        return response;
    }

    let snapshot = metrics().snapshot();
    let output = format_prometheus_metrics(&snapshot);
    (
        StatusCode::OK,
        [(
            "Content-Type",
            "text/plain; version=0.0.4; charset=utf-8".to_string(),
        )],
        output,
    )
        .into_response()
}

fn format_prometheus_metrics(snapshot: &ems_telemetry::MetricsSnapshot) -> String {
    let mut output = String::new();

    // 采集指标
    output.push_str("# HELP ems_raw_events_total Total number of raw events received\n");
    output.push_str("# TYPE ems_raw_events_total counter\n");
    output.push_str(&format!("ems_raw_events_total {}\n\n", snapshot.raw_events));

    output.push_str("# HELP ems_normalized_values_total Total number of normalized values\n");
    output.push_str("# TYPE ems_normalized_values_total counter\n");
    output.push_str(&format!(
        "ems_normalized_values_total {}\n\n",
        snapshot.normalized_values
    ));

    output.push_str("# HELP ems_write_success_total Total number of successful writes\n");
    output.push_str("# TYPE ems_write_success_total counter\n");
    output.push_str(&format!("ems_write_success_total {}\n\n", snapshot.write_success));

    output.push_str("# HELP ems_write_failure_total Total number of failed writes\n");
    output.push_str("# TYPE ems_write_failure_total counter\n");
    output.push_str(&format!("ems_write_failure_total {}\n\n", snapshot.write_failure));

    // 丢弃指标
    output.push_str("# HELP ems_dropped_duplicate_total Total number of duplicate values dropped\n");
    output.push_str("# TYPE ems_dropped_duplicate_total counter\n");
    output.push_str(&format!(
        "ems_dropped_duplicate_total {}\n\n",
        snapshot.dropped_duplicate
    ));

    output.push_str("# HELP ems_dropped_invalid_total Total number of invalid values dropped\n");
    output.push_str("# TYPE ems_dropped_invalid_total counter\n");
    output.push_str(&format!(
        "ems_dropped_invalid_total {}\n\n",
        snapshot.dropped_invalid
    ));

    output.push_str("# HELP ems_dropped_stale_total Total number of stale values dropped\n");
    output.push_str("# TYPE ems_dropped_stale_total counter\n");
    output.push_str(&format!("ems_dropped_stale_total {}\n\n", snapshot.dropped_stale));

    output.push_str(
        "# HELP ems_dropped_unmapped_total Total number of unmapped values dropped\n",
    );
    output.push_str("# TYPE ems_dropped_unmapped_total counter\n");
    output.push_str(&format!(
        "ems_dropped_unmapped_total {}\n\n",
        snapshot.dropped_unmapped
    ));

    output.push_str("# HELP ems_backpressure_total Total number of backpressure events\n");
    output.push_str("# TYPE ems_backpressure_total counter\n");
    output.push_str(&format!("ems_backpressure_total {}\n\n", snapshot.backpressure));

    // 延迟指标
    output.push_str("# HELP ems_write_latency_ms_total Total write latency in milliseconds\n");
    output.push_str("# TYPE ems_write_latency_ms_total counter\n");
    output.push_str(&format!(
        "ems_write_latency_ms_total {}\n\n",
        snapshot.write_latency_ms_total
    ));

    output.push_str("# HELP ems_write_latency_ms_count Total number of write operations\n");
    output.push_str("# TYPE ems_write_latency_ms_count counter\n");
    output.push_str(&format!(
        "ems_write_latency_ms_count {}\n\n",
        snapshot.write_latency_ms_count
    ));

    output.push_str("# HELP ems_end_to_end_latency_ms_total Total end-to-end latency in milliseconds\n");
    output.push_str("# TYPE ems_end_to_end_latency_ms_total counter\n");
    output.push_str(&format!(
        "ems_end_to_end_latency_ms_total {}\n\n",
        snapshot.end_to_end_latency_ms_total
    ));

    output.push_str("# HELP ems_end_to_end_latency_ms_count Total number of end-to-end operations\n");
    output.push_str("# TYPE ems_end_to_end_latency_ms_count counter\n");
    output.push_str(&format!(
        "ems_end_to_end_latency_ms_count {}\n\n",
        snapshot.end_to_end_latency_ms_count
    ));

    // 控制指标
    output.push_str("# HELP ems_commands_issued_total Total number of commands issued\n");
    output.push_str("# TYPE ems_commands_issued_total counter\n");
    output.push_str(&format!("ems_commands_issued_total {}\n\n", snapshot.commands_issued));

    output.push_str(
        "# HELP ems_command_dispatch_success_total Total number of successful command dispatches\n",
    );
    output.push_str("# TYPE ems_command_dispatch_success_total counter\n");
    output.push_str(&format!(
        "ems_command_dispatch_success_total {}\n\n",
        snapshot.command_dispatch_success
    ));

    output.push_str(
        "# HELP ems_command_dispatch_failure_total Total number of failed command dispatches\n",
    );
    output.push_str("# TYPE ems_command_dispatch_failure_total counter\n");
    output.push_str(&format!(
        "ems_command_dispatch_failure_total {}\n\n",
        snapshot.command_dispatch_failure
    ));

    output.push_str(
        "# HELP ems_command_issue_latency_ms_total Total command issue latency in milliseconds\n",
    );
    output.push_str("# TYPE ems_command_issue_latency_ms_total counter\n");
    output.push_str(&format!(
        "ems_command_issue_latency_ms_total {}\n\n",
        snapshot.command_issue_latency_ms_total
    ));

    output.push_str(
        "# HELP ems_command_issue_latency_ms_count Total number of command issue operations\n",
    );
    output.push_str("# TYPE ems_command_issue_latency_ms_count counter\n");
    output.push_str(&format!(
        "ems_command_issue_latency_ms_count {}\n\n",
        snapshot.command_issue_latency_ms_count
    ));

    output.push_str("# HELP ems_receipts_processed_total Total number of receipts processed\n");
    output.push_str("# TYPE ems_receipts_processed_total counter\n");
    output.push_str(&format!(
        "ems_receipts_processed_total {}\n",
        snapshot.receipts_processed
    ));

    output
}
