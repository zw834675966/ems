//! 网关内存存储实现
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 功能：
//! - 网关 CRUD 操作
//! - 项目级资源过滤
//! - 租户隔离验证

use crate::error::StorageError;
use crate::models::{GatewayRecord, GatewayUpdate};
use crate::traits::GatewayStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;

/// 网关内存存储
///
/// 使用 RwLock + HashMap 提供线程安全的内存存储。
pub struct InMemoryGatewayStore {
    gateways: RwLock<HashMap<String, GatewayRecord>>,
}

impl InMemoryGatewayStore {
    /// 创建新的网关存储
    pub fn new() -> Self {
        Self {
            gateways: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl GatewayStore for InMemoryGatewayStore {
    /// 列出指定项目的所有网关
    async fn list_gateways(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<GatewayRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let items = self
            .gateways
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

    /// 查找指定网关
    async fn find_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<Option<GatewayRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let item = self
            .gateways
            .read()
            .ok()
            .and_then(|map| map.get(gateway_id).cloned())
            .filter(|item| item.tenant_id == ctx.tenant_id && item.project_id == project_id);
        Ok(item)
    }

    /// 创建新网关
    async fn create_gateway(
        &self,
        ctx: &TenantContext,
        record: GatewayRecord,
    ) -> Result<GatewayRecord, StorageError> {
        ensure_project_scope(ctx, &record.project_id)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut map = self
            .gateways
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        if map.contains_key(&record.gateway_id) {
            return Err(StorageError::new("gateway exists"));
        }
        map.insert(record.gateway_id.clone(), record.clone());
        Ok(record)
    }

    /// 更新网关
    async fn update_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
        update: GatewayUpdate,
    ) -> Result<Option<GatewayRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .gateways
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        let gateway = match map.get_mut(gateway_id) {
            Some(gateway) => gateway,
            None => return Ok(None),
        };
        if gateway.tenant_id != ctx.tenant_id || gateway.project_id != project_id {
            return Ok(None);
        }
        if let Some(name) = update.name {
            gateway.name = name;
        }
        if let Some(status) = update.status {
            gateway.status = status;
        }
        Ok(Some(gateway.clone()))
    }

    /// 删除网关
    async fn delete_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .gateways
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        match map.get(gateway_id) {
            Some(item) if item.tenant_id == ctx.tenant_id && item.project_id == project_id => {
                map.remove(gateway_id);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
