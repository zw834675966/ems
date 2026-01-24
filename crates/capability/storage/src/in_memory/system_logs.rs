use crate::error::StorageError;
use crate::models::{LogCategory, SystemLogQuery, SystemLogRecord, UnreadStats};
use crate::traits::SystemLogStore;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// 内存实现：系统日志存储
pub struct InMemorySystemLogStore {
    logs: RwLock<HashMap<String, SystemLogRecord>>,
}

impl InMemorySystemLogStore {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySystemLogStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SystemLogStore for InMemorySystemLogStore {
    async fn create_system_log(
        &self,
        ctx: &TenantContext,
        record: SystemLogRecord,
    ) -> Result<SystemLogRecord, StorageError> {
        // 验证租户匹配
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }

        let mut logs = self.logs.write().unwrap();
        logs.insert(record.log_id.clone(), record.clone());
        Ok(record)
    }

    async fn list_system_logs(
        &self,
        ctx: &TenantContext,
        query: SystemLogQuery,
    ) -> Result<Vec<SystemLogRecord>, StorageError> {
        let logs = self.logs.read().unwrap();

        let mut results: Vec<SystemLogRecord> = logs
            .values()
            .filter(|log| {
                if log.tenant_id != ctx.tenant_id {
                    return false;
                }
                if let Some(ref cat) = query.category
                    && log.category.as_str() != cat.as_str()
                {
                    return false;
                }
                if let Some(ref lvl) = query.level && log.level.as_str() != lvl.as_str() {
                    return false;
                }
                if query.unread_only && log.is_read {
                    return false;
                }
                if let Some(from) = query.from_ms && log.created_at_ms < from {
                    return false;
                }
                if let Some(to) = query.to_ms && log.created_at_ms > to {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        // 按创建时间倒序排序
        results.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));

        // 分页限制
        Ok(results.into_iter().take(query.limit as usize).collect())
    }

    async fn get_unread_count(
        &self,
        ctx: &TenantContext,
        category: Option<LogCategory>,
    ) -> Result<UnreadStats, StorageError> {
        let logs = self.logs.read().unwrap();
        let mut stats = UnreadStats {
            operation: 0,
            error: 0,
            warning: 0,
            total: 0,
        };

        for log in logs.values() {
            if log.tenant_id != ctx.tenant_id || log.is_read {
                continue;
            }

            if let Some(ref cat) = category {
                if log.category == *cat {
                    match log.category {
                        LogCategory::Operation => stats.operation += 1,
                        LogCategory::Error => stats.error += 1,
                        LogCategory::Warning => stats.warning += 1,
                    }
                }
            } else {
                match log.category {
                    LogCategory::Operation => stats.operation += 1,
                    LogCategory::Error => stats.error += 1,
                    LogCategory::Warning => stats.warning += 1,
                }
            }
        }
        stats.total = stats.operation + stats.error + stats.warning;
        Ok(stats)
    }

    async fn mark_as_read(
        &self,
        ctx: &TenantContext,
        log_ids: Vec<String>,
    ) -> Result<usize, StorageError> {
        let mut logs = self.logs.write().unwrap();
        let mut count = 0;

        for log_id in log_ids {
            if let Some(log) = logs.get_mut(&log_id)
                && log.tenant_id == ctx.tenant_id
                && !log.is_read
            {
                log.is_read = true;
                log.read_at_ms = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                );
                count += 1;
            }
        }
        Ok(count)
    }

    async fn cleanup_old_logs(&self, older_than_ms: i64) -> Result<usize, StorageError> {
        let mut logs = self.logs.write().unwrap();
        let initial_len = logs.len();
        logs.retain(|_, log| log.created_at_ms >= older_than_ms);
        Ok(initial_len - logs.len())
    }
}
