//! 追踪与请求 ID 生成。

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing_subscriber::{EnvFilter, fmt};

/// 请求级追踪标识。
#[derive(Debug, Clone)]
pub struct RequestIds {
    pub request_id: String,
    pub trace_id: String,
}

/// 基础指标快照（MVP）。
#[derive(Debug, Clone, Copy, Default)]
pub struct MetricsSnapshot {
    pub raw_events: u64,
    pub normalized_values: u64,
    pub write_success: u64,
    pub write_failure: u64,
    pub dropped_duplicate: u64,
    pub dropped_invalid: u64,
    pub dropped_stale: u64,
    pub dropped_unmapped: u64,
    pub backpressure: u64,
    pub write_latency_ms_total: u64,
    pub write_latency_ms_count: u64,
    pub end_to_end_latency_ms_total: u64,
    pub end_to_end_latency_ms_count: u64,
    pub commands_issued: u64,
    pub command_dispatch_success: u64,
    pub command_dispatch_failure: u64,
    pub command_issue_latency_ms_total: u64,
    pub command_issue_latency_ms_count: u64,
    pub receipts_processed: u64,
}

/// 基础指标（MVP）。
pub struct TelemetryMetrics {
    raw_events: AtomicU64,
    normalized_values: AtomicU64,
    write_success: AtomicU64,
    write_failure: AtomicU64,
    dropped_duplicate: AtomicU64,
    dropped_invalid: AtomicU64,
    dropped_stale: AtomicU64,
    dropped_unmapped: AtomicU64,
    backpressure: AtomicU64,
    write_latency_ms_total: AtomicU64,
    write_latency_ms_count: AtomicU64,
    end_to_end_latency_ms_total: AtomicU64,
    end_to_end_latency_ms_count: AtomicU64,
    commands_issued: AtomicU64,
    command_dispatch_success: AtomicU64,
    command_dispatch_failure: AtomicU64,
    command_issue_latency_ms_total: AtomicU64,
    command_issue_latency_ms_count: AtomicU64,
    receipts_processed: AtomicU64,
}

impl TelemetryMetrics {
    pub fn new() -> Self {
        Self {
            raw_events: AtomicU64::new(0),
            normalized_values: AtomicU64::new(0),
            write_success: AtomicU64::new(0),
            write_failure: AtomicU64::new(0),
            dropped_duplicate: AtomicU64::new(0),
            dropped_invalid: AtomicU64::new(0),
            dropped_stale: AtomicU64::new(0),
            dropped_unmapped: AtomicU64::new(0),
            backpressure: AtomicU64::new(0),
            write_latency_ms_total: AtomicU64::new(0),
            write_latency_ms_count: AtomicU64::new(0),
            end_to_end_latency_ms_total: AtomicU64::new(0),
            end_to_end_latency_ms_count: AtomicU64::new(0),
            commands_issued: AtomicU64::new(0),
            command_dispatch_success: AtomicU64::new(0),
            command_dispatch_failure: AtomicU64::new(0),
            command_issue_latency_ms_total: AtomicU64::new(0),
            command_issue_latency_ms_count: AtomicU64::new(0),
            receipts_processed: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            raw_events: self.raw_events.load(Ordering::Relaxed),
            normalized_values: self.normalized_values.load(Ordering::Relaxed),
            write_success: self.write_success.load(Ordering::Relaxed),
            write_failure: self.write_failure.load(Ordering::Relaxed),
            dropped_duplicate: self.dropped_duplicate.load(Ordering::Relaxed),
            dropped_invalid: self.dropped_invalid.load(Ordering::Relaxed),
            dropped_stale: self.dropped_stale.load(Ordering::Relaxed),
            dropped_unmapped: self.dropped_unmapped.load(Ordering::Relaxed),
            backpressure: self.backpressure.load(Ordering::Relaxed),
            write_latency_ms_total: self.write_latency_ms_total.load(Ordering::Relaxed),
            write_latency_ms_count: self.write_latency_ms_count.load(Ordering::Relaxed),
            end_to_end_latency_ms_total: self.end_to_end_latency_ms_total.load(Ordering::Relaxed),
            end_to_end_latency_ms_count: self.end_to_end_latency_ms_count.load(Ordering::Relaxed),
            commands_issued: self.commands_issued.load(Ordering::Relaxed),
            command_dispatch_success: self.command_dispatch_success.load(Ordering::Relaxed),
            command_dispatch_failure: self.command_dispatch_failure.load(Ordering::Relaxed),
            command_issue_latency_ms_total: self
                .command_issue_latency_ms_total
                .load(Ordering::Relaxed),
            command_issue_latency_ms_count: self
                .command_issue_latency_ms_count
                .load(Ordering::Relaxed),
            receipts_processed: self.receipts_processed.load(Ordering::Relaxed),
        }
    }
}

static METRICS: OnceLock<TelemetryMetrics> = OnceLock::new();

/// 获取全局指标实例（MVP）。
pub fn metrics() -> &'static TelemetryMetrics {
    METRICS.get_or_init(TelemetryMetrics::new)
}

/// 初始化 tracing（默认 info）。
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

/// 生成新的 request_id 与 trace_id。
pub fn new_request_ids() -> RequestIds {
    RequestIds {
        request_id: uuid::Uuid::new_v4().to_string(),
        trace_id: uuid::Uuid::new_v4().to_string(),
    }
}

/// 记录 RawEvent 接收次数。
pub fn record_raw_event() {
    metrics().raw_events.fetch_add(1, Ordering::Relaxed);
}

/// 记录规范化输出次数。
pub fn record_normalized_value() {
    metrics().normalized_values.fetch_add(1, Ordering::Relaxed);
}

/// 记录写入成功次数。
pub fn record_write_success() {
    metrics().write_success.fetch_add(1, Ordering::Relaxed);
}

/// 记录写入失败次数。
pub fn record_write_failure() {
    metrics().write_failure.fetch_add(1, Ordering::Relaxed);
}

/// 记录重复值丢弃次数。
pub fn record_dropped_duplicate() {
    metrics().dropped_duplicate.fetch_add(1, Ordering::Relaxed);
}

/// 记录非法值丢弃次数。
pub fn record_dropped_invalid() {
    metrics().dropped_invalid.fetch_add(1, Ordering::Relaxed);
}

/// 记录过期值丢弃次数。
pub fn record_dropped_stale() {
    metrics().dropped_stale.fetch_add(1, Ordering::Relaxed);
}

/// 记录未映射丢弃次数。
pub fn record_dropped_unmapped() {
    metrics().dropped_unmapped.fetch_add(1, Ordering::Relaxed);
}

/// 记录背压次数。
pub fn record_backpressure() {
    metrics().backpressure.fetch_add(1, Ordering::Relaxed);
}

/// 记录写入延迟（毫秒）。
pub fn record_write_latency_ms(latency_ms: u64) {
    let metrics = metrics();
    metrics
        .write_latency_ms_total
        .fetch_add(latency_ms, Ordering::Relaxed);
    metrics
        .write_latency_ms_count
        .fetch_add(1, Ordering::Relaxed);
}

/// 记录端到端延迟（毫秒）。
pub fn record_end_to_end_latency_ms(latency_ms: u64) {
    let metrics = metrics();
    metrics
        .end_to_end_latency_ms_total
        .fetch_add(latency_ms, Ordering::Relaxed);
    metrics
        .end_to_end_latency_ms_count
        .fetch_add(1, Ordering::Relaxed);
}

/// 记录命令下发请求次数。
pub fn record_command_issued() {
    metrics().commands_issued.fetch_add(1, Ordering::Relaxed);
}

/// 记录命令下发成功次数（MQTT 发布成功）。
pub fn record_command_dispatch_success() {
    metrics()
        .command_dispatch_success
        .fetch_add(1, Ordering::Relaxed);
}

/// 记录命令下发失败次数（MQTT 发布失败）。
pub fn record_command_dispatch_failure() {
    metrics()
        .command_dispatch_failure
        .fetch_add(1, Ordering::Relaxed);
}

/// 记录命令下发处理耗时（毫秒，包含写库+下发+状态更新）。
pub fn record_command_issue_latency_ms(latency_ms: u64) {
    let metrics = metrics();
    metrics
        .command_issue_latency_ms_total
        .fetch_add(latency_ms, Ordering::Relaxed);
    metrics
        .command_issue_latency_ms_count
        .fetch_add(1, Ordering::Relaxed);
}

/// 记录回执处理次数（MQTT 回执成功写入）。
pub fn record_receipt_processed() {
    metrics()
        .receipts_processed
        .fetch_add(1, Ordering::Relaxed);
}
