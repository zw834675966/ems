//! 追踪与请求 ID 生成。

use tracing_subscriber::{fmt, EnvFilter};

/// 请求级追踪标识。
#[derive(Debug, Clone)]
pub struct RequestIds {
    pub request_id: String,
    pub trace_id: String,
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
