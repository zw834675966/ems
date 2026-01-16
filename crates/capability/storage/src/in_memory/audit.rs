//! 审计日志内存实现
//!
//! 仅用于本地测试和占位。

use crate::error::StorageError;
use crate::models::AuditLogRecord;
use crate::traits::AuditLogStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::sync::RwLock;

/// 审计日志内存存储
pub struct InMemoryAuditLogStore {
    logs: RwLock<Vec<AuditLogRecord>>,
}

impl InMemoryAuditLogStore {
    /// 创建新的审计日志存储
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl AuditLogStore for InMemoryAuditLogStore {
    async fn create_audit_log(
        &self,
        ctx: &TenantContext,
        record: AuditLogRecord,
    ) -> Result<AuditLogRecord, StorageError> {
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        if let Some(project_id) = record.project_id.as_deref() {
            ensure_project_scope(ctx, project_id)?;
        }
        let mut logs = self
            .logs
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        logs.push(record.clone());
        Ok(record)
    }

    async fn list_audit_logs(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        from_ms: Option<i64>,
        to_ms: Option<i64>,
        limit: i64,
    ) -> Result<Vec<AuditLogRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let limit = limit.max(0) as usize;
        let logs = self
            .logs
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mut items: Vec<AuditLogRecord> = logs
            .iter()
            .filter(|item| {
                item.tenant_id == ctx.tenant_id && item.project_id.as_deref() == Some(project_id)
            })
            .filter(|item| match from_ms {
                Some(from) => item.ts_ms >= from,
                None => true,
            })
            .filter(|item| match to_ms {
                Some(to) => item.ts_ms <= to,
                None => true,
            })
            .cloned()
            .collect();
        items.sort_by(|a, b| b.ts_ms.cmp(&a.ts_ms));
        if limit > 0 && items.len() > limit {
            items.truncate(limit);
        }
        Ok(items)
    }
}
