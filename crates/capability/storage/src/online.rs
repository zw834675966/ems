//! Online 状态存储接口与实现。

use crate::error::StorageError;
use domain::TenantContext;

#[derive(Debug, Clone)]
pub struct OnlineRecord {
    pub tenant_id: String,
    pub project_id: String,
    pub resource_id: String,
    pub last_seen_at_ms: i64,
}

#[async_trait::async_trait]
pub trait OnlineStore: Send + Sync {
    async fn touch_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
        ts_ms: i64,
    ) -> Result<(), StorageError>;

    async fn touch_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
        ts_ms: i64,
    ) -> Result<(), StorageError>;

    async fn get_gateway_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<Option<i64>, StorageError>;

    async fn get_device_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<i64>, StorageError>;

    async fn list_gateways_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_ids: &[String],
    ) -> Result<std::collections::HashMap<String, i64>, StorageError>;

    async fn list_devices_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_ids: &[String],
    ) -> Result<std::collections::HashMap<String, i64>, StorageError>;
}

