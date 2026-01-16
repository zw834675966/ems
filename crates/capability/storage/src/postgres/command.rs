//! Postgres 控制命令实现

use crate::error::StorageError;
use crate::models::CommandRecord;
use crate::traits::CommandStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgCommandStore {
    pub pool: PgPool,
}

impl PgCommandStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CommandStore for PgCommandStore {
    async fn create_command(
        &self,
        ctx: &TenantContext,
        record: CommandRecord,
    ) -> Result<CommandRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        sqlx::query(
            "insert into commands \
             (command_id, tenant_id, project_id, target, payload, status, issued_by, issued_at) \
             values ($1, $2, $3, $4, $5::jsonb, $6, $7, to_timestamp($8 / 1000.0))",
        )
        .bind(&record.command_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.target)
        .bind(&record.payload)
        .bind(&record.status)
        .bind(&record.issued_by)
        .bind(record.issued_at_ms as f64)
        .execute(&self.pool)
        .await?;
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
        let row = sqlx::query(
            "update commands set status = $1 \
             where tenant_id = $2 and project_id = $3 and command_id = $4 \
             returning command_id, tenant_id, project_id, target, payload::text as payload, \
             status, issued_by, (extract(epoch from issued_at) * 1000)::bigint as issued_at_ms",
        )
        .bind(status)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(command_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(CommandRecord {
            command_id: row.try_get("command_id")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            target: row.try_get("target")?,
            payload: row.try_get("payload")?,
            status: row.try_get("status")?,
            issued_by: row.try_get("issued_by")?,
            issued_at_ms: row.try_get("issued_at_ms")?,
        }))
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
        let result = sqlx::query(
            "update commands set status = $1 \
             where tenant_id = $2 and project_id = $3 and command_id = $4 and status = $5",
        )
        .bind(to_status)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(command_id)
        .bind(from_status)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_commands(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<CommandRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let rows = sqlx::query(
            "select command_id, tenant_id, project_id, target, payload::text as payload, status, \
             issued_by, (extract(epoch from issued_at) * 1000)::bigint as issued_at_ms \
             from commands \
             where tenant_id = $1 and project_id = $2 \
             order by issued_at desc \
             limit $3",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(limit.max(0))
        .fetch_all(&self.pool)
        .await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(CommandRecord {
                command_id: row.try_get("command_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                target: row.try_get("target")?,
                payload: row.try_get("payload")?,
                status: row.try_get("status")?,
                issued_by: row.try_get("issued_by")?,
                issued_at_ms: row.try_get("issued_at_ms")?,
            });
        }
        Ok(items)
    }
}
