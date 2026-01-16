//! 实时数据内存实现
//!
//! 仅用于本地测试和占位。

use crate::error::StorageError;
use crate::models::RealtimeRecord;
use crate::traits::RealtimeStore;
use crate::validation::ensure_project_scope;
use domain::{PointValue, TenantContext};
use std::collections::HashMap;
use std::sync::RwLock;

fn last_value_key(value: &PointValue) -> String {
    format!(
        "tenant:{}:project:{}:point:{}",
        value.tenant_id, value.project_id, value.point_id
    )
}

/// 实时数据内存存储
pub struct InMemoryRealtimeStore {
    last_values: RwLock<HashMap<String, PointValue>>,
}

impl InMemoryRealtimeStore {
    /// 创建新的实时存储
    pub fn new() -> Self {
        Self {
            last_values: RwLock::new(HashMap::new()),
        }
    }

    /// 获取 last_value 数量（用于测试）
    pub fn len(&self) -> usize {
        self.last_values.read().map(|m| m.len()).unwrap_or(0)
    }
}

#[async_trait::async_trait]
impl RealtimeStore for InMemoryRealtimeStore {
    async fn upsert_last_value(
        &self,
        ctx: &TenantContext,
        value: &PointValue,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, &value.project_id)?;
        if value.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut values = self
            .last_values
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        values.insert(last_value_key(value), value.clone());
        Ok(())
    }

    async fn get_last_value(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<RealtimeRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let values = self
            .last_values
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let key = format!(
            "tenant:{}:project:{}:point:{}",
            ctx.tenant_id, project_id, point_id
        );
        let value = values.get(&key);
        Ok(value.map(|value| RealtimeRecord {
            tenant_id: value.tenant_id.clone(),
            project_id: value.project_id.clone(),
            point_id: value.point_id.clone(),
            ts_ms: value.ts_ms,
            value: match &value.value {
                domain::PointValueData::I64(v) => v.to_string(),
                domain::PointValueData::F64(v) => v.to_string(),
                domain::PointValueData::Bool(v) => v.to_string(),
                domain::PointValueData::String(v) => v.clone(),
            },
            quality: value.quality.clone(),
        }))
    }

    async fn list_last_values(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<RealtimeRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let values = self
            .last_values
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mut items = Vec::new();
        for value in values.values() {
            if value.tenant_id != ctx.tenant_id || value.project_id != project_id {
                continue;
            }
            items.push(RealtimeRecord {
                tenant_id: value.tenant_id.clone(),
                project_id: value.project_id.clone(),
                point_id: value.point_id.clone(),
                ts_ms: value.ts_ms,
                value: match &value.value {
                    domain::PointValueData::I64(v) => v.to_string(),
                    domain::PointValueData::F64(v) => v.to_string(),
                    domain::PointValueData::Bool(v) => v.to_string(),
                    domain::PointValueData::String(v) => v.clone(),
                },
                quality: value.quality.clone(),
            });
        }
        Ok(items)
    }
}
