//! 审计日志辅助工具
//!
//! 提供从 HTTP 请求中提取审计相关信息的辅助函数。

use axum::http::HeaderMap;

/// 从 HTTP HeaderMap 提取客户端 IP
///
/// 检查顺序：X-Forwarded-For -> X-Real-IP
/// X-Forwarded-For 可能包含多个 IP（经过多级代理），取第一个作为原始客户端 IP。
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // 优先检查 X-Forwarded-For（反向代理场景）
    if let Some(forwarded) = headers.get("x-forwarded-for")
        && let Ok(value) = forwarded.to_str()
    {
        // X-Forwarded-For 可能包含多个 IP，取第一个（原始客户端）
        if let Some(first_ip) = value.split(',').next() {
            return Some(first_ip.trim().to_string());
        }
    }

    // 检查 X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip")
        && let Ok(value) = real_ip.to_str()
    {
        return Some(value.to_string());
    }

    None
}

/// 从 HTTP HeaderMap 提取 trace_id
///
/// trace_id 由请求上下文中间件注入到响应头，但在请求处理时也会存在于扩展中。
/// 这里从请求头中提取（如果客户端或网关传递了 trace_id）。
#[allow(dead_code)]
pub fn extract_trace_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-trace-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// 从 HTTP HeaderMap 提取 request_id
#[allow(dead_code)]
pub fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}
