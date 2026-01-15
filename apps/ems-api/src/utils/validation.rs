//! 输入验证辅助函数
//!
//! 提供统一的输入验证函数：
//! - normalize_required：验证必填字段，去除空格并检查非空
//! - normalize_optional：验证可选字段，如果提供则去除空格并检查非空
//!
//! 验证规则：
//! - 去除首尾空格
//! - 非空字符串才通过验证
//! - 失败返回 bad_request_error 响应

use crate::utils::response::bad_request_error;
use axum::response::Response;

/// 验证必填字段，去除空格并检查非空
pub fn normalize_required(value: String, field: &str) -> Result<String, Response> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(bad_request_error(format!("{field} required")));
    }
    Ok(trimmed.to_string())
}

/// 验证可选字段，如果提供则去除空格并检查非空
pub fn normalize_optional(value: Option<String>, field: &str) -> Result<Option<String>, Response> {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(bad_request_error(format!("{field} required")));
            }
            Ok(Some(trimmed.to_string()))
        }
        None => Ok(None),
    }
}
