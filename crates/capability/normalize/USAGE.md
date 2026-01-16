# ems-normalize

提供 RawEvent -> PointValue 的最小规范化能力。

## 示例
```rust
use ems_normalize::{Normalizer, PointMapping, PointMappingProvider};
use domain::RawEvent;
use std::sync::Arc;

struct Provider;

#[async_trait::async_trait]
impl PointMappingProvider for Provider {
    async fn find_mapping(
        &self,
        _tenant_id: &str,
        _project_id: &str,
        _source_id: &str,
        _address: &str,
    ) -> Result<Option<PointMapping>, ems_normalize::NormalizeError> {
        Ok(Some(PointMapping {
            point_id: "point-1".to_string(),
            scale: Some(1.0),
            offset: Some(0.0),
        }))
    }
}

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let normalizer = Normalizer::new(Arc::new(Provider));
let event = RawEvent {
    tenant_id: "t-1".to_string(),
    project_id: "p-1".to_string(),
    source_id: "s-1".to_string(),
    address: "topic".to_string(),
    payload: b"12.3".to_vec(),
    received_at_ms: 0,
};
let value = normalizer.normalize(event).await?;
# Ok(())
# }
```

## 基于 storage 的 Provider
```rust
use ems_normalize::StoragePointMappingProvider;
use ems_storage::InMemoryPointMappingStore;
use std::sync::Arc;

let store = Arc::new(InMemoryPointMappingStore::new());
let provider = StoragePointMappingProvider::new(store);
```
