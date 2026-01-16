//! Online 状态内存实现（用于测试与占位）。

use crate::error::StorageError;
use crate::online::OnlineStore;
use crate::validation::ensure_project_scope;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Clone, Copy)]
struct Entry {
    last_seen_at_ms: i64,
}

pub struct InMemoryOnlineStore {
    gateway: RwLock<HashMap<String, Entry>>,
    device: RwLock<HashMap<String, Entry>>,
}

impl InMemoryOnlineStore {
    pub fn new() -> Self {
        Self {
            gateway: RwLock::new(HashMap::new()),
            device: RwLock::new(HashMap::new()),
        }
    }
}

fn gateway_key(tenant_id: &str, project_id: &str, gateway_id: &str) -> String {
    format!(
        "tenant:{}:project:{}:gateway:{}:online",
        tenant_id, project_id, gateway_id
    )
}

fn device_key(tenant_id: &str, project_id: &str, device_id: &str) -> String {
    format!(
        "tenant:{}:project:{}:device:{}:online",
        tenant_id, project_id, device_id
    )
}

#[async_trait::async_trait]
impl OnlineStore for InMemoryOnlineStore {
    async fn touch_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
        ts_ms: i64,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .gateway
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        map.insert(
            gateway_key(&ctx.tenant_id, project_id, gateway_id),
            Entry {
                last_seen_at_ms: ts_ms,
            },
        );
        Ok(())
    }

    async fn touch_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
        ts_ms: i64,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut map = self
            .device
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        map.insert(
            device_key(&ctx.tenant_id, project_id, device_id),
            Entry {
                last_seen_at_ms: ts_ms,
            },
        );
        Ok(())
    }

    async fn get_gateway_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<Option<i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let map = self
            .gateway
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        Ok(map
            .get(&gateway_key(&ctx.tenant_id, project_id, gateway_id))
            .map(|item| item.last_seen_at_ms))
    }

    async fn get_device_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let map = self
            .device
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        Ok(map
            .get(&device_key(&ctx.tenant_id, project_id, device_id))
            .map(|item| item.last_seen_at_ms))
    }

    async fn list_gateways_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_ids: &[String],
    ) -> Result<HashMap<String, i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let map = self
            .gateway
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mut result = HashMap::new();
        for gateway_id in gateway_ids {
            if let Some(item) = map.get(&gateway_key(&ctx.tenant_id, project_id, gateway_id)) {
                result.insert(gateway_id.clone(), item.last_seen_at_ms);
            }
        }
        Ok(result)
    }

    async fn list_devices_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_ids: &[String],
    ) -> Result<HashMap<String, i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let map = self
            .device
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mut result = HashMap::new();
        for device_id in device_ids {
            if let Some(item) = map.get(&device_key(&ctx.tenant_id, project_id, device_id)) {
                result.insert(device_id.clone(), item.last_seen_at_ms);
            }
        }
        Ok(result)
    }
}

