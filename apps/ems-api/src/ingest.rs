//! 采集链路装配模块
//!
//! 该模块负责将数据采集的各个组件（数据源、规整器、处理流水线、存储层）组装在一起，
//! 构建完整的数据处理链路。它定义了如何从数据源（如 MQTT）接收原始数据，
//! 经过标准化处理后，通过流水线写入存储，并同步更新设备的在线状态。

use ems_config::AppConfig;
use ems_ingest::{IngestError, MqttSource, MqttSourceConfig, NoopSource, RawEventHandler, Source};
use ems_normalize::{Normalizer, StoragePointMappingProvider};
use ems_pipeline::{Pipeline, PipelineError, StoragePointValueWriter};
use ems_storage::{
    DeviceStore, MeasurementStore, OnlineStore, PointMappingStore, PointStore, RealtimeStore,
};
use ems_telemetry::{
    record_backpressure, record_dropped_duplicate, record_dropped_invalid, record_dropped_stale,
    record_dropped_unmapped, record_end_to_end_latency_ms, record_normalized_value,
    record_raw_event, record_write_failure, record_write_latency_ms, record_write_success,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// 流水线处理器
///
/// 实现了 `RawEventHandler` 接口，负责处理从采集源接收到的原始事件。
/// 它连接了规整化（Normalizer）和数据流水线（Pipeline）两个核心环节。
struct PipelineHandler {
    /// 规整化器，将原始报文根据映射规则转换为标准化点位值
    normalizer: Normalizer,
    /// 数据流水线，负责点位值的后续处理和持久化写入
    pipeline: Pipeline,
    /// 点位存储，用于查询点位详情
    point_store: Arc<dyn PointStore>,
    /// 设备存储，用于查询设备详情
    device_store: Arc<dyn DeviceStore>,
    /// 在线状态存储，用于记录设备和网关的活跃状态
    online_store: Arc<dyn OnlineStore>,
}

#[async_trait::async_trait]
impl RawEventHandler for PipelineHandler {
    /// 处理接收到的原始采集事件
    async fn handle(&self, event: domain::RawEvent) -> Result<(), IngestError> {
        // 记录原始事件指标
        record_raw_event();
        info!(
            target: "ems.ingest",
            tenant_id = %event.tenant_id,
            project_id = %event.project_id,
            source_id = %event.source_id,
            address = %event.address,
            payload_size = event.payload.len(),
            received_at_ms = event.received_at_ms,
            "raw_event_received"
        );

        // 1. 规整化：将原始报文转换为标准化点位值
        let value = self.normalizer.normalize(event).await.map_err(|err| {
            record_dropped_invalid();
            warn!(target: "ems.ingest", error = %err, "normalize_failed");
            IngestError::Handler(err.to_string())
        });

        // 如果规整化过程中出错，且错误已被记录，则返回 Ok 继续处理后续事件
        let value = match value {
            Ok(value) => value,
            Err(_) => return Ok(()),
        };

        // 如果没有找到对应的映射规则，则跳过该事件
        let Some(value) = value else {
            record_dropped_unmapped();
            info!(target: "ems.ingest", "normalize_skipped");
            return Ok(());
        };

        // 记录规整化成功的点位值指标
        record_normalized_value();
        let point_id = value.point_id.clone();
        let tenant_id = value.tenant_id.clone();
        let project_id = value.project_id.clone();
        let ts_ms = value.ts_ms;
        let value_str = point_value_to_string(&value.value);
        let quality = value.quality.clone();

        info!(
            target: "ems.ingest",
            tenant_id = %tenant_id,
            project_id = %project_id,
            point_id = %point_id,
            ts_ms = ts_ms,
            value = %value_str,
            quality = ?quality,
            "point_value_normalized"
        );

        // 2. 流水线处理：负责过滤、去重并最终写入存储
        let write_started_at = Instant::now();
        match self.pipeline.handle(value).await {
            Ok(result) => {
                // 3. 更新在线状态：根据成功处理的点位，更新设备和网关的最后活跃时间
                let ctx = domain::TenantContext::new(
                    tenant_id.clone(),
                    "system".to_string(),
                    Vec::new(),
                    Vec::new(),
                    Some(project_id.clone()),
                );
                let _ = touch_online_from_point(
                    &ctx,
                    &project_id,
                    &point_id,
                    ts_ms,
                    self.point_store.clone(),
                    self.device_store.clone(),
                    self.online_store.clone(),
                )
                .await;

                // 物理写入成功后记录各类指标
                if result.written {
                    record_write_success();
                    record_write_latency_ms(write_started_at.elapsed().as_millis() as u64);
                    if let Some(latency_ms) = end_to_end_latency_ms(ts_ms) {
                        record_end_to_end_latency_ms(latency_ms);
                    }
                } else if let Some(reason) = result.reason.as_deref() {
                    // 如果数据被丢弃，记录原因（通过指标统计）
                    match reason {
                        "duplicate" => record_dropped_duplicate(),
                        "invalid_ts" | "invalid_value" => record_dropped_invalid(),
                        "stale" => record_dropped_stale(),
                        _ => {}
                    }
                }
                info!(
                    target: "ems.ingest",
                    tenant_id = %tenant_id,
                    project_id = %project_id,
                    point_id = %point_id,
                    ts_ms = ts_ms,
                    value = %value_str,
                    written = result.written,
                    reason = ?result.reason,
                    "pipeline_write_result"
                );
            }
            Err(err) => {
                // 写入流水线过程中发生不可恢复的错误
                record_write_failure();
                if matches!(err, PipelineError::Backpressure(_)) {
                    record_backpressure();
                }
                warn!(
                    target: "ems.ingest",
                    tenant_id = %tenant_id,
                    project_id = %project_id,
                    point_id = %point_id,
                    ts_ms = ts_ms,
                    value = %value_str,
                    error = %err,
                    "pipeline_write_failed"
                );
                return Err(IngestError::Handler(err.to_string()));
            }
        }
        Ok(())
    }
}

/// 将点位数据值转换为字符串，用于日志记录
fn point_value_to_string(value: &domain::PointValueData) -> String {
    match value {
        domain::PointValueData::I64(v) => v.to_string(),
        domain::PointValueData::F64(v) => v.to_string(),
        domain::PointValueData::Bool(v) => v.to_string(),
        domain::PointValueData::String(v) => v.clone(),
    }
}

/// 计算端到端延迟（从点位时间戳到当前系统时间）
fn end_to_end_latency_ms(ts_ms: i64) -> Option<u64> {
    if ts_ms <= 0 {
        return None;
    }
    let now_ms = now_epoch_ms();
    let delta = now_ms.saturating_sub(ts_ms);
    u64::try_from(delta).ok()
}

/// 获取当前 Unix 时间戳（微秒）
fn now_epoch_ms() -> i64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as i64
}

/// 启动采集任务
///
/// 该函数负责初始化规整器、流水线、数据源，并启动后台任务。
///
/// # 参数
/// - `config`: 应用程序统一配置
/// - `point_mapping_store`: 点位映射规则存储
/// - `point_store`: 点位元数据存储
/// - `device_store`: 设备元数据存储
/// - `measurement_store`: 历史时序数据存储
/// - `realtime_store`: 实时点位值存储
/// - `online_store`: 在线状态存储
pub fn spawn_ingest(
    config: &AppConfig,
    point_mapping_store: Arc<dyn PointMappingStore>,
    point_store: Arc<dyn PointStore>,
    device_store: Arc<dyn DeviceStore>,
    measurement_store: Arc<dyn MeasurementStore>,
    realtime_store: Arc<dyn RealtimeStore>,
    online_store: Arc<dyn OnlineStore>,
) -> tokio::task::JoinHandle<()> {
    // 初始化规整化服务
    let provider = StoragePointMappingProvider::new(point_mapping_store);
    let normalizer = Normalizer::new(Arc::new(provider));

    // 初始化流水线写入器
    let writer = StoragePointValueWriter::new(measurement_store, realtime_store);
    let pipeline = Pipeline::new(Arc::new(writer));

    // 创建全局唯一的流水线处理器
    let handler = Arc::new(PipelineHandler {
        normalizer,
        pipeline,
        point_store,
        device_store,
        online_store,
    });

    // 1. 如果启用了采集，启动流水线定时刷盘任务
    if config.ingest_enabled {
        let pipeline = handler.pipeline.clone();
        tokio::spawn(async move {
            loop {
                // 每秒触发一次刷新，确保缓冲的数据能够及时写入
                tokio::time::sleep(Duration::from_secs(1)).await;
                match pipeline.flush().await {
                    Ok(pairs) => {
                        if pairs.is_empty() {
                            continue;
                        }
                        info!(target: "ems.ingest", flushed = pairs.len(), "pipeline_flushed");
                        for (value, result) in pairs {
                            let point_id = value.point_id.clone();
                            let tenant_id = value.tenant_id.clone();
                            let project_id = value.project_id.clone();
                            let ts_ms = value.ts_ms;
                            let value_str = point_value_to_string(&value.value);

                            // 记录批量写入成功的延迟指标
                            if result.written {
                                record_write_success();
                                if let Some(latency_ms) = end_to_end_latency_ms(ts_ms) {
                                    record_end_to_end_latency_ms(latency_ms);
                                }
                            }
                            info!(
                                target: "ems.ingest",
                                tenant_id = %tenant_id,
                                project_id = %project_id,
                                point_id = %point_id,
                                ts_ms = ts_ms,
                                value = %value_str,
                                written = result.written,
                                reason = ?result.reason,
                                "pipeline_flush_write_result"
                            );
                        }
                    }
                    Err(err) => {
                        record_write_failure();
                        warn!(target: "ems.ingest", error = %err, "pipeline_flush_failed");
                    }
                }
            }
        });
    }

    // 2. 选择采集源：根据配置启用 MQTT 采集或空操作源
    let source: Arc<dyn Source> = if config.ingest_enabled {
        let mqtt_config = MqttSourceConfig {
            host: config.mqtt_host.clone(),
            port: config.mqtt_port,
            username: config.mqtt_username.clone(),
            password: config.mqtt_password.clone(),
            topic_prefix: config.mqtt_data_topic_prefix.clone(),
            has_source_id: config.mqtt_data_topic_has_source_id,
        };
        info!(
            "ingest source: mqtt {}:{} prefix={}",
            mqtt_config.host, mqtt_config.port, mqtt_config.topic_prefix
        );
        Arc::new(MqttSource::new(mqtt_config))
    } else {
        info!("ingest source: noop (EMS_INGEST=off)");
        Arc::new(NoopSource::default())
    };

    // 3. 运行采集源任务
    tokio::spawn(async move {
        if let Err(err) = source.run(handler).await {
            warn!("ingest stopped: {}", err);
        }
    })
}

/// 更新设备和网关的在线状态
///
/// 根据上报点位所属的设备信息，向 在线状态存储 发送一个 "活跃" 信号。
async fn touch_online_from_point(
    ctx: &domain::TenantContext,
    project_id: &str,
    point_id: &str,
    ts_ms: i64,
    point_store: Arc<dyn PointStore>,
    device_store: Arc<dyn DeviceStore>,
    online_store: Arc<dyn OnlineStore>,
) -> Result<(), ems_storage::StorageError> {
    // 查找点位，获取所属设备 ID
    let point = point_store.find_point(ctx, project_id, point_id).await?;
    let Some(point) = point else {
        return Ok(());
    };
    // 查找设备，获取所属网关 ID
    let device = device_store
        .find_device(ctx, project_id, &point.device_id)
        .await?;
    let Some(device) = device else {
        return Ok(());
    };
    // 更新设备心跳
    online_store
        .touch_device(ctx, project_id, &device.device_id, ts_ms)
        .await?;
    // 更新网关心跳
    online_store
        .touch_gateway(ctx, project_id, &device.gateway_id, ts_ms)
        .await?;
    Ok(())
}
