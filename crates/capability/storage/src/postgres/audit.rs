//! Postgres 审计日志实现

use crate::error::StorageError;
use crate::models::AuditLogRecord;
use crate::traits::AuditLogStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use sqlx::{PgPool, Row};

pub struct PgAuditLogStore {
    pub pool: PgPool,
}

impl PgAuditLogStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AuditLogStore for PgAuditLogStore {
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
        sqlx::query(
            "insert into audit_logs \
             (audit_id, tenant_id, project_id, actor, action, resource, result, detail, ts) \
             values ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9 / 1000.0))",
        )
        .bind(&record.audit_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.actor)
        .bind(&record.action)
        .bind(&record.resource)
        .bind(&record.result)
        .bind(&record.detail)
        .bind(record.ts_ms as f64)
        .execute(&self.pool)
        .await?;
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
        let rows = sqlx::query(
            "select audit_id, tenant_id, project_id, actor, action, resource, result, detail, \
             (extract(epoch from ts) * 1000)::bigint as ts_ms \
             from audit_logs \
             where tenant_id = $1 \
             and project_id = $2 \
             and ($3 is null or ts >= to_timestamp($3 / 1000.0)) \
             and ($4 is null or ts <= to_timestamp($4 / 1000.0)) \
             order by ts desc \
             limit $5",
        )
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(from_ms)
        .bind(to_ms)
        .bind(limit.max(0))
        .fetch_all(&self.pool)
        .await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(AuditLogRecord {
                audit_id: row.try_get("audit_id")?,
                tenant_id: row.try_get("tenant_id")?,
                project_id: row.try_get("project_id")?,
                actor: row.try_get("actor")?,
                action: row.try_get("action")?,
                resource: row.try_get("resource")?,
                result: row.try_get("result")?,
                detail: row.try_get("detail")?,
                ts_ms: row.try_get("ts_ms")?,
            });
        }
        Ok(items)
    }
}
