//! 命令回执内存实现
//!
//! 仅用于本地测试和占位。

use crate::error::StorageError;
use crate::models::CommandReceiptRecord;
use crate::traits::{CommandReceiptStore, CommandReceiptWriteResult};
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::sync::RwLock;

/// 命令回执内存存储
pub struct InMemoryCommandReceiptStore {
    receipts: RwLock<Vec<CommandReceiptRecord>>,
}

impl InMemoryCommandReceiptStore {
    /// 创建新的回执存储
    pub fn new() -> Self {
        Self {
            receipts: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl CommandReceiptStore for InMemoryCommandReceiptStore {
    async fn create_receipt(
        &self,
        ctx: &TenantContext,
        record: CommandReceiptRecord,
    ) -> Result<CommandReceiptWriteResult, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut receipts = self
            .receipts
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        let inserted = !receipts.iter().any(|item| item.receipt_id == record.receipt_id);
        if inserted {
            receipts.push(record.clone());
        }
        Ok(CommandReceiptWriteResult { record, inserted })
    }

    async fn list_receipts(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        command_id: &str,
    ) -> Result<Vec<CommandReceiptRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let receipts = self
            .receipts
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mut items: Vec<CommandReceiptRecord> = receipts
            .iter()
            .filter(|item| {
                item.tenant_id == ctx.tenant_id
                    && item.project_id == project_id
                    && item.command_id == command_id
            })
            .cloned()
            .collect();
        items.sort_by(|a, b| b.ts_ms.cmp(&a.ts_ms));
        Ok(items)
    }
}
