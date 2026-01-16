# ems-pipeline

提供 PointValue 写入管道的 MVP 实现（去重、质量校验、批写、重试、背压）。

## 示例
```rust
use ems_pipeline::{NoopWriter, Pipeline};
use domain::{PointValue, PointValueData};
use std::sync::Arc;

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let pipeline = Pipeline::new(Arc::new(NoopWriter::default()));
let value = PointValue {
    tenant_id: "t-1".to_string(),
    project_id: "p-1".to_string(),
    point_id: "point-1".to_string(),
    ts_ms: 0,
    value: PointValueData::F64(1.0),
    quality: None,
};
let _ = pipeline.handle(value).await?;
# Ok(())
# }
```

## 配置参数（MVP）
```rust
use ems_pipeline::PipelineConfig;

let config = PipelineConfig {
    batch_size: 100,
    max_buffer_size: 1000,
    max_retries: 3,
    dedup_cache_size: 10_000,
    max_age_ms: None,
};
```

## 行为说明
- 去重：同一 tenant/project/point 在相同 ts/value/quality 下重复值会被丢弃（reason=duplicate）。
- 质量：时间戳非法或 f64 非有限值会被丢弃（reason=invalid_ts/invalid_value）。
- 批写：达到 batch_size 后批量写入 measurement；last_value 逐条更新。
- 重试：写入失败时最多重试 max_retries 次。
- 背压：buffer 超过 max_buffer_size 时返回 backpressure 错误。

## 基于存储的写入器
```rust
use ems_pipeline::StoragePointValueWriter;
use ems_storage::{InMemoryMeasurementStore, InMemoryRealtimeStore};
use std::sync::Arc;

let writer = StoragePointValueWriter::new(
    Arc::new(InMemoryMeasurementStore::new()),
    Arc::new(InMemoryRealtimeStore::new()),
);
```
