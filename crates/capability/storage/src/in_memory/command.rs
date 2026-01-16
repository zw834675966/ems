//! 命令存储内存实现
//!
//! 仅用于本地测试和占位。

use crate::error::StorageError;
use crate::models::CommandRecord;
use crate::traits::CommandStore;
use crate::validation::{ensure_project_scope, ensure_tenant};
use domain::TenantContext;
use std::sync::RwLock;

/// 命令内存存储
pub struct InMemoryCommandStore {
    commands: RwLock<Vec<CommandRecord>>,
}

impl InMemoryCommandStore {
    /// 创建新的命令存储
    pub fn new() -> Self {
        Self {
            commands: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl CommandStore for InMemoryCommandStore {
    async fn create_command(
        &self,
        ctx: &TenantContext,
        record: CommandRecord,
    ) -> Result<CommandRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut commands = self
            .commands
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        commands.push(record.clone());
        Ok(record)
    }

    async fn update_command_status(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        command_id: &str,
        status: &str,
    ) -> Result<Option<CommandRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        ensure_tenant(ctx)?;
        let mut commands = self
            .commands
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        for command in commands.iter_mut() {
            if command.command_id == command_id {
                if command.tenant_id != ctx.tenant_id || command.project_id != project_id {
                    return Ok(None);
                }
                command.status = status.to_string();
                return Ok(Some(command.clone()));
            }
        }
        Ok(None)
    }

    async fn transition_command_status(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        command_id: &str,
        from_status: &str,
        to_status: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        ensure_tenant(ctx)?;
        let mut commands = self
            .commands
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        for command in commands.iter_mut() {
            if command.command_id == command_id {
                if command.tenant_id != ctx.tenant_id || command.project_id != project_id {
                    return Ok(false);
                }
                if command.status != from_status {
                    return Ok(false);
                }
                command.status = to_status.to_string();
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn list_commands(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<CommandRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let limit = limit.max(0) as usize;
        let commands = self
            .commands
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mut items: Vec<CommandRecord> = commands
            .iter()
            .filter(|item| item.tenant_id == ctx.tenant_id && item.project_id == project_id)
            .cloned()
            .collect();
        items.sort_by(|a, b| b.issued_at_ms.cmp(&a.issued_at_ms));
        if limit > 0 && items.len() > limit {
            items.truncate(limit);
        }
        Ok(items)
    }
}
