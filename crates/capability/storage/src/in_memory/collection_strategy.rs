//! 采集策略内存实现
//!
//! 仅用于本地测试和占位。

use crate::error::StorageError;
use crate::models::{CollectionStrategyCreate, CollectionStrategyRecord, CollectionStrategyUpdate};
use crate::traits::CollectionStrategyStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;

/// 采集策略内存存储
pub struct InMemoryCollectionStrategyStore {
    strategies: RwLock<HashMap<String, CollectionStrategyRecord>>,
}

impl InMemoryCollectionStrategyStore {
    /// 创建新的采集策略存储
    pub fn new() -> Self {
        Self {
            strategies: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCollectionStrategyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CollectionStrategyStore for InMemoryCollectionStrategyStore {
    async fn list_strategies(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let strategies = self
            .strategies
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let items: Vec<CollectionStrategyRecord> = strategies
            .values()
            .filter(|s| s.tenant_id == ctx.tenant_id && s.project_id == project_id)
            .cloned()
            .collect();
        Ok(items)
    }

    async fn list_strategies_by_projects(
        &self,
        ctx: &TenantContext,
        project_ids: &[String],
    ) -> Result<Vec<CollectionStrategyRecord>, StorageError> {
        let strategies = self
            .strategies
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let items: Vec<CollectionStrategyRecord> = strategies
            .values()
            .filter(|s| s.tenant_id == ctx.tenant_id && project_ids.contains(&s.project_id))
            .cloned()
            .collect();
        Ok(items)
    }

    async fn list_enabled_strategies(
        &self,
        ctx: &TenantContext,
    ) -> Result<Vec<CollectionStrategyRecord>, StorageError> {
        let strategies = self
            .strategies
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let items: Vec<CollectionStrategyRecord> = strategies
            .values()
            .filter(|s| s.tenant_id == ctx.tenant_id && s.enabled)
            .cloned()
            .collect();
        Ok(items)
    }

    async fn find_by_point_id(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let strategies = self
            .strategies
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let record = strategies
            .values()
            .find(|s| {
                s.tenant_id == ctx.tenant_id && s.project_id == project_id && s.point_id == point_id
            })
            .cloned();
        Ok(record)
    }

    async fn find_strategy(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        strategy_id: &str,
    ) -> Result<Option<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let strategies = self
            .strategies
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let record = strategies
            .get(strategy_id)
            .filter(|s| s.tenant_id == ctx.tenant_id && s.project_id == project_id)
            .cloned();
        Ok(record)
    }

    async fn create_strategy(
        &self,
        ctx: &TenantContext,
        record: CollectionStrategyCreate,
    ) -> Result<CollectionStrategyRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }

        let strategy_record = CollectionStrategyRecord {
            strategy_id: record.strategy_id.clone(),
            tenant_id: record.tenant_id,
            project_id: record.project_id,
            point_id: record.point_id,
            enabled: record.enabled,
            interval_value: record.interval_value,
            interval_unit: record.interval_unit,
            write_to_db: record.write_to_db,
            last_collected_at: None,
            last_value: None,
            last_error: None,
            created_at: None,
            updated_at: None,
        };

        let mut strategies = self
            .strategies
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        strategies.insert(record.strategy_id, strategy_record.clone());
        Ok(strategy_record)
    }

    async fn update_strategy(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        strategy_id: &str,
        update: CollectionStrategyUpdate,
    ) -> Result<Option<CollectionStrategyRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut strategies = self
            .strategies
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;

        let Some(record) = strategies.get_mut(strategy_id) else {
            return Ok(None);
        };

        if record.tenant_id != ctx.tenant_id || record.project_id != project_id {
            return Ok(None);
        }

        if let Some(enabled) = update.enabled {
            record.enabled = enabled;
        }
        if let Some(interval_value) = update.interval_value {
            record.interval_value = interval_value;
        }
        if let Some(interval_unit) = update.interval_unit {
            record.interval_unit = interval_unit;
        }
        if let Some(write_to_db) = update.write_to_db {
            record.write_to_db = write_to_db;
        }

        Ok(Some(record.clone()))
    }

    async fn upsert_strategy(
        &self,
        ctx: &TenantContext,
        record: CollectionStrategyCreate,
    ) -> Result<CollectionStrategyRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }

        let mut strategies = self
            .strategies
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;

        // Check if exists by point_id
        let existing = strategies
            .values()
            .find(|s| {
                s.tenant_id == ctx.tenant_id
                    && s.project_id == record.project_id
                    && s.point_id == record.point_id
            })
            .map(|s| s.strategy_id.clone());

        let strategy_id = existing.unwrap_or_else(|| record.strategy_id.clone());

        let strategy_record = CollectionStrategyRecord {
            strategy_id: strategy_id.clone(),
            tenant_id: record.tenant_id,
            project_id: record.project_id,
            point_id: record.point_id,
            enabled: record.enabled,
            interval_value: record.interval_value,
            interval_unit: record.interval_unit,
            write_to_db: record.write_to_db,
            last_collected_at: None,
            last_value: None,
            last_error: None,
            created_at: None,
            updated_at: None,
        };

        strategies.insert(strategy_id, strategy_record.clone());
        Ok(strategy_record)
    }

    async fn batch_update_enabled(
        &self,
        ctx: &TenantContext,
        strategy_ids: &[String],
        enabled: bool,
    ) -> Result<usize, StorageError> {
        let mut strategies = self
            .strategies
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;

        let mut count = 0;
        for id in strategy_ids {
            if let Some(record) = strategies.get_mut(id)
                && record.tenant_id == ctx.tenant_id
            {
                record.enabled = enabled;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_strategy(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        strategy_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut strategies = self
            .strategies
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;

        if let Some(record) = strategies.get(strategy_id)
            && record.tenant_id == ctx.tenant_id
            && record.project_id == project_id
        {
            strategies.remove(strategy_id);
            return Ok(true);
        }
        Ok(false)
    }

    async fn update_collection_status(
        &self,
        ctx: &TenantContext,
        strategy_id: &str,
        last_value: Option<String>,
        last_error: Option<String>,
    ) -> Result<(), StorageError> {
        let mut strategies = self
            .strategies
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;

        if let Some(record) = strategies.get_mut(strategy_id)
            && record.tenant_id == ctx.tenant_id
        {
            record.last_value = last_value;
            record.last_error = last_error;
        }
        Ok(())
    }

    async fn update_collection_status_by_point_id(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
        last_value: Option<String>,
        last_error: Option<String>,
    ) -> Result<(), StorageError> {
        let mut strategies = self
            .strategies
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;

        for record in strategies.values_mut() {
            if record.tenant_id == ctx.tenant_id
                && record.project_id == project_id
                && record.point_id == point_id
            {
                record.last_value = last_value.clone();
                record.last_error = last_error.clone();
            }
        }
        Ok(())
    }
}
