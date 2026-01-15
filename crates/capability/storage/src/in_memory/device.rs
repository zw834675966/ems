//! 设备内存存储实现
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 功能：
//! - 设备 CRUD 操作
//! - 项目级资源过滤
//! - 租户隔离验证

use crate::error::StorageError;
use crate::models::{DeviceRecord, DeviceUpdate};
use crate::traits::DeviceStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;

/// 设备内存存储
///
/// 使用 RwLock + HashMap 提供线程安全的内存存储。
pub struct InMemoryDeviceStore {
    devices: RwLock<HashMap<String, DeviceRecord>>,
}

impl InMemoryDeviceStore {
    /// 创建新的设备存储
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl DeviceStore for InMemoryDeviceStore {
    /// 列出指定项目的所有设备
    async fn list_devices(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<DeviceRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let items = self
            .devices
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

    /// 查找指定设备
    async fn find_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<DeviceRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let item = self
            .devices
            .read()
            .ok()
            .and_then(|map| map.get(device_id).cloned())
            .filter(|item| item.tenant_id == ctx.tenant_id && item.project_id == project_id);
        Ok(item)
    }

    /// 创建新设备
    async fn create_device(
        &self,
        ctx: &TenantContext,
        record: DeviceRecord,
    ) -> Result<DeviceRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut map = self
            .devices
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        if map.contains_key(&record.device_id) {
            return Err(StorageError::new("device exists"));
        }
        map.insert(record.device_id.clone(), record.clone());
        Ok(record)
    }

    /// 更新设备
    async fn update_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
        update: DeviceUpdate,
    ) -> Result<Option<DeviceRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .devices
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        let device = match map.get_mut(device_id) {
            Some(device) => device,
            None => return Ok(None),
        };
        if device.tenant_id != ctx.tenant_id || device.project_id != project_id {
            return Ok(None);
        }
        if let Some(name) = update.name {
            device.name = name;
        }
        if let Some(model) = update.model {
            device.model = Some(model);
        }
        Ok(Some(device.clone()))
    }

    /// 删除设备
    async fn delete_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .devices
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        match map.get(device_id) {
            Some(item) if item.tenant_id == ctx.tenant_id && item.project_id == project_id => {
                map.remove(device_id);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
