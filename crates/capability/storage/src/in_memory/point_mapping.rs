//! 点位映射内存存储实现
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 功能：
//! - 点映射 CRUD 操作
//! - 项目级资源过滤
//! - 租户隔离验证

use crate::error::StorageError;
use crate::models::{PointMappingRecord, PointMappingUpdate};
use crate::traits::PointMappingStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;

/// 点位映射内存存储
///
/// 使用 RwLock + HashMap 提供线程安全的内存存储。
pub struct InMemoryPointMappingStore {
    mappings: RwLock<HashMap<String, PointMappingRecord>>,
}

impl InMemoryPointMappingStore {
    /// 创建新的点映射存储
    pub fn new() -> Self {
        Self {
            mappings: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl PointMappingStore for InMemoryPointMappingStore {
    /// 列出指定项目的所有点映射
    async fn list_point_mappings(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<PointMappingRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let items = self
            .mappings
            .read()
            .map(|map| {
                map.values()
                    .filter(|item| item.tenant_id == ctx.tenant_id && item.project_id == project_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(items)
    }

    /// 查找指定点映射
    async fn find_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
    ) -> Result<Option<PointMappingRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let item = self
            .mappings
            .read()
            .ok()
            .and_then(|map| map.get(source_id).cloned())
            .filter(|item| item.tenant_id == ctx.tenant_id && item.project_id == project_id);
        Ok(item)
    }

    /// 创建新点映射
    async fn create_point_mapping(
        &self,
        ctx: &TenantContext,
        record: PointMappingRecord,
    ) -> Result<PointMappingRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut map = self
            .mappings
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        if map.contains_key(&record.source_id) {
            return Err(StorageError::new("mapping exists"));
        }
        map.insert(record.source_id.clone(), record.clone());
        Ok(record)
    }

    /// 更新点映射
    async fn update_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
        update: PointMappingUpdate,
    ) -> Result<Option<PointMappingRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .mappings
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mapping = match map.get_mut(source_id) {
            Some(mapping) => mapping,
            None => return Ok(None),
        };
        if mapping.tenant_id != ctx.tenant_id || mapping.project_id != project_id {
            return Ok(None);
        }
        if let Some(source_type) = update.source_type {
            mapping.source_type = source_type;
        }
        if let Some(address) = update.address {
            mapping.address = address;
        }
        if let Some(scale) = update.scale {
            mapping.scale = Some(scale);
        }
        if let Some(offset) = update.offset {
            mapping.offset = Some(offset);
        }
        Ok(Some(mapping.clone()))
    }

    /// 删除点映射
    async fn delete_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .mappings
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        match map.get(source_id) {
            Some(item) if item.tenant_id == ctx.tenant_id && item.project_id == project_id => {
                map.remove(source_id);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
