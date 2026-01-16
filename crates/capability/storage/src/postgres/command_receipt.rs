//! Postgres 命令回执实现

use crate::error::StorageError;
use crate::models::CommandReceiptRecord;
use crate::traits::{CommandReceiptStore, CommandReceiptWriteResult};
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgCommandReceiptStore {
    pub pool: PgPool,
}

impl PgCommandReceiptStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CommandReceiptStore for PgCommandReceiptStore {
    async fn create_receipt(
        &self,
        ctx: &TenantContext,
        record: CommandReceiptRecord,
    ) -> Result<CommandReceiptWriteResult, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let result = sqlx::query(
            "insert into command_receipts \
             (receipt_id, tenant_id, project_id, command_id, ts, status, message) \
             values ($1, $2, $3, $4, to_timestamp($5 / 1000.0), $6, $7) \
             on conflict (receipt_id) do nothing",
        )
        .bind(&record.receipt_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.command_id)
        .bind(record.ts_ms as f64)
        .bind(&record.status)
        .bind(&record.message)
        .execute(&self.pool)
        .await?;
        Ok(CommandReceiptWriteResult {
            record,
            inserted: result.rows_affected() > 0,
        })
    }

    async fn list_receipts(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        command_id: &str,
    ) -> Result<Vec<CommandReceiptRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let rows = sqlx::query(
            "select receipt_id, tenant_id, project_id, command_id, \
             (extract(epoch from ts) * 1000)::bigint as ts_ms, status, message \
             from command_receipts \
             where tenant_id = $1 and project_id = $2 and command_id = $3 \
             order by ts desc",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(command_id)
        .fetch_all(&self.pool)
        .await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(CommandReceiptRecord {
                receipt_id: row.try_get("receipt_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                command_id: row.try_get("command_id")?,
                ts_ms: row.try_get("ts_ms")?,
                status: row.try_get("status")?,
                message: row.try_get("message")?,
            });
        }
        Ok(items)
    }
}
