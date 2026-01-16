use async_trait::async_trait;
use domain::{PointValue, PointValueData, TenantContext};
use ems_storage::{MeasurementStore, RealtimeStore};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

/// 写入结果（最小占位）。
#[derive(Debug, Clone)]
pub struct WriteResult {
    pub point_id: String,
    pub written: bool,
    pub reason: Option<String>,
}

/// Pipeline 处理错误。
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("writer error: {0}")]
    Writer(String),
    #[error("backpressure: {0}")]
    Backpressure(String),
}

/// Pipeline 参数（MVP）。
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub batch_size: usize,
    pub max_buffer_size: usize,
    pub max_retries: usize,
    pub dedup_cache_size: usize,
    pub max_age_ms: Option<i64>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_buffer_size: 1000,
            max_retries: 3,
            dedup_cache_size: 10_000,
            max_age_ms: None,
        }
    }
}

impl PipelineConfig {
    fn sanitized(mut self) -> Self {
        if self.batch_size == 0 {
            self.batch_size = 1;
        }
        if self.max_buffer_size < self.batch_size {
            self.max_buffer_size = self.batch_size;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValueSignature {
    ts_ms: i64,
    value: String,
    quality: Option<String>,
}

struct DedupState {
    map: HashMap<String, (ValueSignature, u64)>,
    order: VecDeque<(String, u64)>,
    counter: u64,
    capacity: usize,
}

impl DedupState {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            counter: 0,
            capacity,
        }
    }

    fn is_duplicate(&mut self, key: String, signature: ValueSignature) -> bool {
        if self.capacity == 0 {
            return false;
        }
        if let Some((existing, _)) = self.map.get(&key) {
            if existing == &signature {
                return true;
            }
        }
        self.counter = self.counter.saturating_add(1);
        let token = self.counter;
        self.map.insert(key.clone(), (signature, token));
        self.order.push_back((key, token));
        while self.map.len() > self.capacity {
            if let Some((evict_key, evict_token)) = self.order.pop_front() {
                let should_remove = self
                    .map
                    .get(&evict_key)
                    .map(|(_, token)| *token == evict_token)
                    .unwrap_or(false);
                if should_remove {
                    self.map.remove(&evict_key);
                }
            } else {
                break;
            }
        }
        false
    }
}

/// 点位值写入器抽象。
#[async_trait]
pub trait PointValueWriter: Send + Sync {
    async fn write(&self, value: PointValue) -> Result<WriteResult, PipelineError>;

    async fn write_batch(&self, values: &[PointValue]) -> Result<Vec<WriteResult>, PipelineError> {
        let mut results = Vec::with_capacity(values.len());
        for value in values {
            results.push(self.write(value.clone()).await?);
        }
        Ok(results)
    }
}

struct PipelineState {
    buffer: Vec<PointValue>,
    dedup: DedupState,
}

struct PipelineInner {
    writer: Arc<dyn PointValueWriter>,
    config: PipelineConfig,
    state: Mutex<PipelineState>,
}

/// Pipeline 入口（MVP）。
#[derive(Clone)]
pub struct Pipeline {
    inner: Arc<PipelineInner>,
}

impl Pipeline {
    pub fn new(writer: Arc<dyn PointValueWriter>) -> Self {
        Self::with_config(writer, PipelineConfig::default())
    }

    pub fn with_config(writer: Arc<dyn PointValueWriter>, config: PipelineConfig) -> Self {
        let config = config.sanitized();
        let inner = PipelineInner {
            writer,
            config: config.clone(),
            state: Mutex::new(PipelineState {
                buffer: Vec::new(),
                dedup: DedupState::new(config.dedup_cache_size),
            }),
        };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub async fn handle(&self, value: PointValue) -> Result<WriteResult, PipelineError> {
        let point_id = value.point_id.clone();

        if let Some(reason) = validate_value(&value, self.inner.config.max_age_ms) {
            return Ok(WriteResult {
                point_id,
                written: false,
                reason: Some(reason),
            });
        }

        let mut state = self.inner.state.lock().await;
        if state.buffer.len() >= self.inner.config.max_buffer_size {
            return Err(PipelineError::Backpressure("buffer full".to_string()));
        }
        if state
            .dedup
            .is_duplicate(dedup_key(&value), signature_from_value(&value))
        {
            return Ok(WriteResult {
                point_id,
                written: false,
                reason: Some("duplicate".to_string()),
            });
        }
        state.buffer.push(value);
        let index = state.buffer.len().saturating_sub(1);
        if state.buffer.len() < self.inner.config.batch_size {
            return Ok(WriteResult {
                point_id,
                written: false,
                reason: Some("queued".to_string()),
            });
        }
        let mut batch = Vec::new();
        std::mem::swap(&mut state.buffer, &mut batch);
        drop(state);

        match self.write_batch_with_retry(&batch).await {
            Ok(results) => Ok(results.get(index).cloned().unwrap_or(WriteResult {
                point_id,
                written: true,
                reason: None,
            })),
            Err(err) => {
                self.requeue(batch).await?;
                Err(err)
            }
        }
    }

    pub async fn flush(&self) -> Result<Vec<(PointValue, WriteResult)>, PipelineError> {
        let mut state = self.inner.state.lock().await;
        if state.buffer.is_empty() {
            return Ok(Vec::new());
        }
        let mut batch = Vec::new();
        std::mem::swap(&mut state.buffer, &mut batch);
        drop(state);

        match self.write_batch_with_retry(&batch).await {
            Ok(results) => Ok(batch
                .into_iter()
                .zip(results.into_iter())
                .collect::<Vec<_>>()),
            Err(err) => {
                self.requeue(batch).await?;
                Err(err)
            }
        }
    }

    async fn write_batch_with_retry(
        &self,
        values: &[PointValue],
    ) -> Result<Vec<WriteResult>, PipelineError> {
        let mut attempt = 0;
        loop {
            match self.inner.writer.write_batch(values).await {
                Ok(results) => return Ok(results),
                Err(err) => {
                    attempt += 1;
                    if attempt > self.inner.config.max_retries {
                        return Err(err);
                    }
                }
            }
        }
    }

    async fn requeue(&self, mut values: Vec<PointValue>) -> Result<(), PipelineError> {
        if values.is_empty() {
            return Ok(());
        }
        let mut state = self.inner.state.lock().await;
        if state.buffer.len() + values.len() > self.inner.config.max_buffer_size {
            return Err(PipelineError::Backpressure(
                "buffer overflow after retry".to_string(),
            ));
        }
        state.buffer.append(&mut values);
        Ok(())
    }
}

fn dedup_key(value: &PointValue) -> String {
    format!(
        "tenant:{}:project:{}:point:{}",
        value.tenant_id, value.project_id, value.point_id
    )
}

fn signature_from_value(value: &PointValue) -> ValueSignature {
    let value_key = match &value.value {
        PointValueData::I64(v) => format!("i:{}", v),
        PointValueData::F64(v) => format!("f:{:x}", v.to_bits()),
        PointValueData::Bool(v) => format!("b:{}", v),
        PointValueData::String(v) => format!("s:{}", v),
    };
    ValueSignature {
        ts_ms: value.ts_ms,
        value: value_key,
        quality: value.quality.clone(),
    }
}

fn validate_value(value: &PointValue, max_age_ms: Option<i64>) -> Option<String> {
    if value.ts_ms <= 0 {
        return Some("invalid_ts".to_string());
    }
    if let PointValueData::F64(v) = &value.value {
        if !v.is_finite() {
            return Some("invalid_value".to_string());
        }
    }
    if let Some(max_age) = max_age_ms {
        let now = now_epoch_ms();
        if now.saturating_sub(value.ts_ms) > max_age {
            return Some("stale".to_string());
        }
    }
    None
}

fn now_epoch_ms() -> i64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as i64
}

/// 空写入器（用于接线与测试）。
#[derive(Debug, Default)]
pub struct NoopWriter;

#[async_trait]
impl PointValueWriter for NoopWriter {
    async fn write(&self, value: PointValue) -> Result<WriteResult, PipelineError> {
        let point_id = value.point_id.clone();
        Ok(WriteResult {
            point_id,
            written: false,
            reason: Some("noop".to_string()),
        })
    }
}

/// 基于存储层的写入器（measurement + last_value）。
#[derive(Clone)]
pub struct StoragePointValueWriter {
    measurement_store: Arc<dyn MeasurementStore>,
    realtime_store: Arc<dyn RealtimeStore>,
}

impl StoragePointValueWriter {
    pub fn new(
        measurement_store: Arc<dyn MeasurementStore>,
        realtime_store: Arc<dyn RealtimeStore>,
    ) -> Self {
        Self {
            measurement_store,
            realtime_store,
        }
    }
}

#[async_trait]
impl PointValueWriter for StoragePointValueWriter {
    async fn write(&self, value: PointValue) -> Result<WriteResult, PipelineError> {
        let ctx = TenantContext::new(
            value.tenant_id.clone(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            Some(value.project_id.clone()),
        );
        self.measurement_store
            .write_measurement(&ctx, &value)
            .await
            .map_err(|err| PipelineError::Writer(err.to_string()))?;
        self.realtime_store
            .upsert_last_value(&ctx, &value)
            .await
            .map_err(|err| PipelineError::Writer(err.to_string()))?;
        Ok(WriteResult {
            point_id: value.point_id,
            written: true,
            reason: None,
        })
    }

    async fn write_batch(&self, values: &[PointValue]) -> Result<Vec<WriteResult>, PipelineError> {
        if values.is_empty() {
            return Ok(Vec::new());
        }
        let ctx = TenantContext::new(
            values[0].tenant_id.clone(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            Some(values[0].project_id.clone()),
        );
        self.measurement_store
            .write_measurements(&ctx, values)
            .await
            .map_err(|err| PipelineError::Writer(err.to_string()))?;
        for value in values {
            self.realtime_store
                .upsert_last_value(&ctx, value)
                .await
                .map_err(|err| PipelineError::Writer(err.to_string()))?;
        }
        Ok(values
            .iter()
            .map(|value| WriteResult {
                point_id: value.point_id.clone(),
                written: true,
                reason: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Default)]
    struct CountingWriter {
        batches: Arc<Mutex<Vec<usize>>>,
    }

    #[derive(Default)]
    struct FailingWriter;

    #[async_trait]
    impl PointValueWriter for CountingWriter {
        async fn write(&self, value: PointValue) -> Result<WriteResult, PipelineError> {
            Ok(WriteResult {
                point_id: value.point_id,
                written: true,
                reason: None,
            })
        }

        async fn write_batch(
            &self,
            values: &[PointValue],
        ) -> Result<Vec<WriteResult>, PipelineError> {
            let mut batches = self.batches.lock().await;
            batches.push(values.len());
            Ok(values
                .iter()
                .map(|value| WriteResult {
                    point_id: value.point_id.clone(),
                    written: true,
                    reason: None,
                })
                .collect())
        }
    }

    #[async_trait]
    impl PointValueWriter for FailingWriter {
        async fn write(&self, _value: PointValue) -> Result<WriteResult, PipelineError> {
            Err(PipelineError::Writer("forced failure".to_string()))
        }

        async fn write_batch(
            &self,
            _values: &[PointValue],
        ) -> Result<Vec<WriteResult>, PipelineError> {
            Err(PipelineError::Writer("forced failure".to_string()))
        }
    }

    fn sample_value(ts_ms: i64, value: PointValueData) -> PointValue {
        PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms,
            value,
            quality: None,
        }
    }

    #[tokio::test]
    async fn pipeline_batches_values() {
        let writer = Arc::new(CountingWriter::default());
        let pipeline = Pipeline::with_config(
            writer.clone(),
            PipelineConfig {
                batch_size: 2,
                max_buffer_size: 10,
                max_retries: 1,
                dedup_cache_size: 0,
                max_age_ms: None,
            },
        );
        let _ = pipeline
            .handle(sample_value(1, PointValueData::I64(1)))
            .await
            .expect("queued");
        let _ = pipeline
            .handle(sample_value(2, PointValueData::I64(2)))
            .await
            .expect("written");
        let batches = writer.batches.lock().await;
        assert_eq!(batches.as_slice(), &[2]);
    }

    #[tokio::test]
    async fn pipeline_dedup_skips_duplicate() {
        let writer = Arc::new(CountingWriter::default());
        let pipeline = Pipeline::with_config(
            writer.clone(),
            PipelineConfig {
                batch_size: 1,
                max_buffer_size: 10,
                max_retries: 1,
                dedup_cache_size: 10,
                max_age_ms: None,
            },
        );
        let first = pipeline
            .handle(sample_value(1, PointValueData::I64(1)))
            .await
            .expect("written");
        let second = pipeline
            .handle(sample_value(1, PointValueData::I64(1)))
            .await
            .expect("duplicate");
        assert!(first.written);
        assert_eq!(second.reason.as_deref(), Some("duplicate"));
    }

    #[tokio::test]
    async fn pipeline_backpressure_rejects_when_full() {
        let writer = Arc::new(FailingWriter::default());
        let pipeline = Pipeline::with_config(
            writer,
            PipelineConfig {
                batch_size: 1,
                max_buffer_size: 1,
                max_retries: 1,
                dedup_cache_size: 0,
                max_age_ms: None,
            },
        );
        let _ = pipeline
            .handle(sample_value(1, PointValueData::I64(1)))
            .await
            .expect_err("write failure");
        let err = pipeline
            .handle(sample_value(2, PointValueData::I64(2)))
            .await
            .expect_err("backpressure");
        assert_eq!(err.to_string(), "backpressure: buffer full");
    }
}
