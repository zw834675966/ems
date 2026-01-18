//! 审计日志构建器
//!
//! 提供便捷的审计日志记录构建和辅助函数。
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use ems_storage::{AuditBuilder, audit_actions};
//!
//! // 构建登录审计日志
//! let record = AuditBuilder::new(&ctx, audit_actions::AUTH_LOGIN)
//!     .resource("user:admin")
//!     .success()
//!     .with_trace_id("abc123")
//!     .with_client_ip("192.168.1.1")
//!     .build();
//!
//! audit_log_store.create_audit_log(&ctx, record).await?;
//! ```

use crate::models::AuditLogRecord;
use domain::TenantContext;

/// 审计日志构建器
///
/// 使用 Builder 模式构建 AuditLogRecord，简化 handler 中的审计调用。
pub struct AuditBuilder {
    tenant_id: String,
    project_id: Option<String>,
    actor: String,
    action: String,
    resource: String,
    result: String,
    detail: Option<serde_json::Value>,
}

impl AuditBuilder {
    /// 创建新的审计日志构建器
    ///
    /// # Arguments
    ///
    /// * `ctx` - 租户上下文，用于获取 tenant_id、project_id 和 user_id
    /// * `action` - 审计操作类型（使用 audit_actions 常量）
    pub fn new(ctx: &TenantContext, action: &str) -> Self {
        Self {
            tenant_id: ctx.tenant_id.clone(),
            project_id: ctx.project_scope.clone(),
            actor: ctx.user_id.clone(),
            action: action.to_string(),
            resource: String::new(),
            result: crate::audit_actions::RESULT_SUCCESS.to_string(),
            detail: None,
        }
    }

    /// 创建匿名构建器（用于登录失败等无上下文场景）
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - 租户 ID
    /// * `actor` - 操作者标识（用户名或 IP）
    /// * `action` - 审计操作类型
    pub fn anonymous(tenant_id: &str, actor: &str, action: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            project_id: None,
            actor: actor.to_string(),
            action: action.to_string(),
            resource: String::new(),
            result: crate::audit_actions::RESULT_SUCCESS.to_string(),
            detail: None,
        }
    }

    /// 设置资源标识符
    ///
    /// 资源格式建议: `{type}:{id}`，如 `user:admin`, `project:proj-001`
    pub fn resource(mut self, resource: &str) -> Self {
        self.resource = resource.to_string();
        self
    }

    /// 设置资源（类型和 ID 分开）
    pub fn resource_typed(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource = format!("{}:{}", resource_type, resource_id);
        self
    }

    /// 设置项目 ID（覆盖从 ctx 获取的值）
    pub fn project_id(mut self, project_id: &str) -> Self {
        self.project_id = Some(project_id.to_string());
        self
    }

    /// 设置为无项目作用域（租户级操作）
    pub fn tenant_scope(mut self) -> Self {
        self.project_id = None;
        self
    }

    /// 设置操作成功
    pub fn success(mut self) -> Self {
        self.result = crate::audit_actions::RESULT_SUCCESS.to_string();
        self
    }

    /// 设置操作失败
    pub fn failure(mut self) -> Self {
        self.result = crate::audit_actions::RESULT_FAILURE.to_string();
        self
    }

    /// 设置操作被拒绝
    pub fn denied(mut self) -> Self {
        self.result = crate::audit_actions::RESULT_DENIED.to_string();
        self
    }

    /// 设置自定义结果
    pub fn result(mut self, result: &str) -> Self {
        self.result = result.to_string();
        self
    }

    /// 设置详情（原始字符串）
    pub fn detail(mut self, detail: &str) -> Self {
        self.detail = Some(serde_json::json!({"message": detail}));
        self
    }

    /// 设置详情（JSON 值）
    pub fn detail_json(mut self, detail: serde_json::Value) -> Self {
        self.detail = Some(detail);
        self
    }

    /// 追加 trace_id 到详情
    ///
    /// 将 trace_id 合并到详情 JSON 中，便于审计日志关联分布式追踪。
    pub fn with_trace_id(mut self, trace_id: &str) -> Self {
        let detail = self.detail.take().unwrap_or_else(|| serde_json::json!({}));
        if let serde_json::Value::Object(mut map) = detail {
            map.insert("trace_id".to_string(), serde_json::json!(trace_id));
            self.detail = Some(serde_json::Value::Object(map));
        } else {
            self.detail = Some(serde_json::json!({"trace_id": trace_id, "original": detail}));
        }
        self
    }

    /// 追加 client_ip 到详情
    pub fn with_client_ip(mut self, client_ip: &str) -> Self {
        let detail = self.detail.take().unwrap_or_else(|| serde_json::json!({}));
        if let serde_json::Value::Object(mut map) = detail {
            map.insert("client_ip".to_string(), serde_json::json!(client_ip));
            self.detail = Some(serde_json::Value::Object(map));
        } else {
            self.detail = Some(serde_json::json!({"client_ip": client_ip, "original": detail}));
        }
        self
    }

    /// 追加任意键值对到详情
    pub fn with_field(mut self, key: &str, value: impl serde::Serialize) -> Self {
        let detail = self.detail.take().unwrap_or_else(|| serde_json::json!({}));
        if let serde_json::Value::Object(mut map) = detail {
            if let Ok(val) = serde_json::to_value(value) {
                map.insert(key.to_string(), val);
            }
            self.detail = Some(serde_json::Value::Object(map));
        }
        self
    }

    /// 构建 AuditLogRecord
    pub fn build(self) -> AuditLogRecord {
        let detail_str = self.detail.map(|v| v.to_string());

        AuditLogRecord {
            audit_id: generate_uuid(),
            tenant_id: self.tenant_id,
            project_id: self.project_id,
            actor: self.actor,
            action: self.action,
            resource: self.resource,
            result: self.result,
            detail: detail_str,
            ts_ms: current_timestamp_ms(),
        }
    }
}

/// 生成 UUID v4
fn generate_uuid() -> String {
    // 使用简单的伪随机实现，避免额外依赖
    // 在生产中应使用 uuid crate
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = now.as_nanos();
    let random_part: u64 = (nanos as u64) ^ (std::process::id() as u64 * 0x12345678);
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (nanos >> 32) as u32,
        ((nanos >> 16) as u16),
        random_part as u16 & 0x0FFF,
        (random_part >> 16) as u16 & 0x3FFF | 0x8000,
        random_part >> 24
    )
}

/// 获取当前时间戳（毫秒）
fn current_timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> TenantContext {
        TenantContext::new(
            "tenant-1".to_string(),
            "user-1".to_string(),
            vec!["admin".to_string()],
            vec![],
            Some("project-1".to_string()),
        )
    }

    #[test]
    fn test_audit_builder_basic() {
        let ctx = test_ctx();
        let record = AuditBuilder::new(&ctx, crate::audit_actions::AUTH_LOGIN)
            .resource("user:admin")
            .success()
            .build();

        assert_eq!(record.tenant_id, "tenant-1");
        assert_eq!(record.actor, "user-1");
        assert_eq!(record.action, "AUTH.LOGIN");
        assert_eq!(record.resource, "user:admin");
        assert_eq!(record.result, "SUCCESS");
        assert_eq!(record.project_id, Some("project-1".to_string()));
    }

    #[test]
    fn test_audit_builder_with_detail() {
        let ctx = test_ctx();
        let record = AuditBuilder::new(&ctx, crate::audit_actions::RBAC_USER_CREATE)
            .resource_typed("user", "new-user")
            .with_field("username", "testuser")
            .with_trace_id("trace-123")
            .with_client_ip("192.168.1.100")
            .build();

        assert!(record.detail.is_some());
        let detail: serde_json::Value =
            serde_json::from_str(record.detail.as_ref().unwrap()).unwrap();
        assert_eq!(detail["trace_id"], "trace-123");
        assert_eq!(detail["client_ip"], "192.168.1.100");
        assert_eq!(detail["username"], "testuser");
    }

    #[test]
    fn test_audit_builder_tenant_scope() {
        let ctx = test_ctx();
        let record = AuditBuilder::new(&ctx, crate::audit_actions::RBAC_ROLE_CREATE)
            .tenant_scope()
            .resource("role:operator")
            .build();

        assert_eq!(record.project_id, None);
    }

    #[test]
    fn test_anonymous_builder() {
        let record = AuditBuilder::anonymous(
            "tenant-1",
            "unknown",
            crate::audit_actions::AUTH_LOGIN_FAILED,
        )
        .resource("user:baduser")
        .failure()
        .with_client_ip("10.0.0.1")
        .build();

        assert_eq!(record.tenant_id, "tenant-1");
        assert_eq!(record.actor, "unknown");
        assert_eq!(record.result, "FAILURE");
    }
}
