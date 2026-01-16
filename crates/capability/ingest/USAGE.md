# ems-ingest

占位的采集能力模块，用于定义 RawEvent 处理器与 Source 抽象。

## 示例
```rust
use ems_ingest::{NoopSource, RawEventHandler, Source};
use domain::RawEvent;
use std::sync::Arc;

struct Handler;

#[async_trait::async_trait]
impl RawEventHandler for Handler {
    async fn handle(&self, _event: RawEvent) -> Result<(), ems_ingest::IngestError> {
        Ok(())
    }
}

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let source = NoopSource::default();
source.run(Arc::new(Handler)).await?;
# Ok(())
# }
```
