//! 点位内存存储实现
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 功能：
//! - 点位 CRUD 操作
//! - 项目级资源过滤
//! - 租户隔离验证

use crate::error::StorageError;
use crate::models::{PointRecord, PointUpdate};
use crate::traits::PointStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;

/// 点位内存存储
///
/// 使用 RwLock + HashMap 提供线程安全的内存存储。
pub struct InMemoryPointStore {
    points: RwLock<HashMap<String, PointRecord>>,
}

impl InMemoryPointStore {
    /// 创建新的点位存储
    pub fn new() -> Self {
        Self {
            points: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl PointStore for InMemoryPointStore {
    /// 列出指定项目的所有点
    async fn list_points(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<PointRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let items = self
            .points
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

    /// 查找指定点
    async fn find_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<PointRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let item = self
            .points
            .read()
            .ok()
            .and_then(|map| map.get(point_id).cloned())
            .filter(|item| item.tenant_id == ctx.tenant_id && item.project_id == project_id);
        Ok(item)
    }

    /// 创建新点
    async fn create_point(
        &self,
        ctx: &TenantContext,
        record: PointRecord,
    ) -> Result<PointRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut map = self
            .points
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        if map.contains_key(&record.point_id) {
            return Err(StorageError::new("point exists"));
        }
        map.insert(record.point_id.clone(), record.clone());
        Ok(record)
    }

    /// 更新点
    async fn update_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
        update: PointUpdate,
    ) -> Result<Option<PointRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .points
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        let point = match map.get_mut(point_id) {
            Some(point) => point,
            None => return Ok(None),
        };
        if point.tenant_id != ctx.tenant_id || point.project_id != project_id {
            return Ok(None);
        }
        if let Some(key) = update.key {
            point.key = key;
        }
        if let Some(data_type) = update.data_type {
            point.data_type = data_type;
        }
        if let Some(unit) = update.unit {
            point.unit = Some(unit);
        }
        Ok(Some(point.clone()))
    }

    /// 删除点
    async fn delete_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .points
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        match map.get(point_id) {
            Some(item) if item.tenant_id == ctx.tenant_id && item.project_id == project_id => {
                map.remove(point_id);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
