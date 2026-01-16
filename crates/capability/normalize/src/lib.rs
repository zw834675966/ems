use async_trait::async_trait;
use domain::{PointValue, PointValueData, RawEvent, TenantContext};
use ems_storage::PointMappingStore;
use std::sync::Arc;

/// 点位映射信息。
#[derive(Debug, Clone)]
pub struct PointMapping {
    pub point_id: String,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
}

/// 规范化错误。
#[derive(Debug, thiserror::Error)]
pub enum NormalizeError {
    #[error("mapping provider error: {0}")]
    MappingProvider(String),
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
}

/// 点位映射提供者抽象。
#[async_trait]
pub trait PointMappingProvider: Send + Sync {
    async fn find_mapping(
        &self,
        tenant_id: &str,
        project_id: &str,
        source_id: &str,
        address: &str,
    ) -> Result<Option<PointMapping>, NormalizeError>;
}

/// RawEvent -> PointValue 的最小规范化实现。
#[derive(Clone)]
pub struct Normalizer {
    provider: Arc<dyn PointMappingProvider>,
}

impl Normalizer {
    pub fn new(provider: Arc<dyn PointMappingProvider>) -> Self {
        Self { provider }
    }

    pub async fn normalize(&self, event: RawEvent) -> Result<Option<PointValue>, NormalizeError> {
        let mapping = self
            .provider
            .find_mapping(
                &event.tenant_id,
                &event.project_id,
                &event.source_id,
                &event.address,
            )
            .await?;
        let mapping = match mapping {
            Some(mapping) => mapping,
            None => return Ok(None),
        };

        let payload_str = std::str::from_utf8(&event.payload)
            .map_err(|err| NormalizeError::InvalidPayload(err.to_string()))?;
        let mut value = payload_str
            .trim()
            .parse::<f64>()
            .map_err(|err| NormalizeError::InvalidPayload(err.to_string()))?;

        if let Some(scale) = mapping.scale {
            value *= scale;
        }
        if let Some(offset) = mapping.offset {
            value += offset;
        }

        Ok(Some(PointValue {
            tenant_id: event.tenant_id,
            project_id: event.project_id,
            point_id: mapping.point_id,
            ts_ms: event.received_at_ms,
            value: PointValueData::F64(value),
            quality: None,
        }))
    }
}

/// 基于 storage 的点位映射提供者。
#[derive(Clone)]
pub struct StoragePointMappingProvider {
    store: Arc<dyn PointMappingStore>,
}

impl StoragePointMappingProvider {
    pub fn new(store: Arc<dyn PointMappingStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl PointMappingProvider for StoragePointMappingProvider {
    async fn find_mapping(
        &self,
        tenant_id: &str,
        project_id: &str,
        source_id: &str,
        address: &str,
    ) -> Result<Option<PointMapping>, NormalizeError> {
        let ctx = TenantContext::new(
            tenant_id.to_string(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            Some(project_id.to_string()),
        );

        if !source_id.is_empty() {
            let record = self
                .store
                .find_point_mapping(&ctx, project_id, source_id)
                .await
                .map_err(|err| NormalizeError::MappingProvider(err.to_string()))?;
            if let Some(record) = record {
                if record.address == address {
                    return Ok(Some(PointMapping {
                        point_id: record.point_id,
                        scale: record.scale,
                        offset: record.offset,
                    }));
                }
            }
        }

        if !address.is_empty() {
            let mappings = self
                .store
                .list_point_mappings(&ctx, project_id)
                .await
                .map_err(|err| NormalizeError::MappingProvider(err.to_string()))?;
            if let Some(record) = mappings.into_iter().find(|item| item.address == address) {
                return Ok(Some(PointMapping {
                    point_id: record.point_id,
                    scale: record.scale,
                    offset: record.offset,
                }));
            }
        }

        Ok(None)
    }
}
