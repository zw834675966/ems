#![allow(dead_code)]
//! 采集链路装配模块
//!
//! 该模块负责将数据采集的各个组件（数据源、规整器、处理流水线、存储层）组装在一起，
//! 构建完整的数据处理链路。它定义了如何从数据源（如 MQTT）接收原始数据，
//! 经过标准化处理后，通过流水线写入存储，并同步更新设备的在线状态。

use ems_config::AppConfig;
use ems_ingest::{IngestError, MqttSource, MqttSourceConfig, NoopSource, RawEventHandler, Source};
use ems_normalize::{Normalizer, StoragePointMappingProvider};
use ems_pipeline::{Pipeline, PipelineError, StoragePointValueWriter};
use ems_protocol::{
    ModbusTcpConfig, ModbusTcpSource, ProtocolEvent, TcpDeviceAddress, TcpPointDetail,
    TcpPointMapping, TcpServerConfig, TcpServerSource,
};
use ems_storage::{
    DeviceStore, IngestWalStore, MeasurementStore, OnlineStore, PointMappingStore, PointStore,
    RealtimeStore,
};
use ems_telemetry::{
    record_backpressure, record_dropped_duplicate, record_dropped_invalid, record_dropped_stale,
    record_dropped_unmapped, record_end_to_end_latency_ms, record_normalized_value,
    record_raw_event, record_write_failure, record_write_latency_ms, record_write_success,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

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
    /// 采集策略存储，用于查询采集策略配置（预留供后续使用）
    #[allow(dead_code)]
    collection_strategy_store: Arc<dyn ems_storage::CollectionStrategyStore>,
    /// WAL 存储，用于原始事件持久化（防止崩溃丢数据）
    wal_store: Arc<dyn IngestWalStore>,
}

impl PipelineHandler {
    /// 处理已规整好或采集层直接解析出的点位值
    pub async fn handle_point_value(&self, value: domain::PointValue) -> Result<(), IngestError> {
        // 2. 流水线处理：负责过滤、去重并最终写入存储
        let write_started_at = Instant::now();
        let point_id = value.point_id.clone();
        let tenant_id = value.tenant_id.clone();
        let project_id = value.project_id.clone();
        let ts_ms = value.ts_ms;
        let value_str = point_value_to_string(&value.value);
        let quality = value.quality.clone();

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
                    quality = ?quality,
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

        // 0. WAL 持久化：先写入 WAL 再处理，防止崩溃丢数据
        let wal_id = match self.wal_store.push_event(&event).await {
            Ok(id) => id,
            Err(err) => {
                warn!(target: "ems.ingest", error = %err, "wal_push_failed");
                // WAL 写入失败时，继续处理但不保证不丢数据
                String::new()
            }
        };

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
        self.handle_point_value(value).await?;

        // 4. WAL 确认：处理成功后从 WAL 中移除
        if !wal_id.is_empty()
            && let Err(err) = self.wal_store.ack_event(&wal_id).await
        {
            warn!(target: "ems.ingest", wal_id = %wal_id, error = %err, "wal_ack_failed");
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
/// - `collection_strategy_store`: 采集策略存储
/// - `wal_store`: WAL 存储（原始事件持久化）
#[allow(clippy::too_many_arguments)]
pub fn spawn_ingest(
    config: &AppConfig,
    gateway_store: Arc<dyn ems_storage::GatewayStore>,
    point_mapping_store: Arc<dyn PointMappingStore>,
    point_store: Arc<dyn PointStore>,
    device_store: Arc<dyn DeviceStore>,
    measurement_store: Arc<dyn MeasurementStore>,
    realtime_store: Arc<dyn RealtimeStore>,
    online_store: Arc<dyn OnlineStore>,
    collection_strategy_store: Arc<dyn ems_storage::CollectionStrategyStore>,
    wal_store: Arc<dyn IngestWalStore>,
) -> tokio::task::JoinHandle<()> {
    // 初始化规整化服务
    let provider = StoragePointMappingProvider::new(point_mapping_store.clone());
    let normalizer = Normalizer::new(Arc::new(provider));

    // 初始化流水线写入器
    let writer = StoragePointValueWriter::new(measurement_store, realtime_store);
    let pipeline = Pipeline::new(Arc::new(writer));

    // 创建全局唯一的流水线处理器
    let gateway_store_for_protocols = gateway_store.clone();
    let point_store_for_protocols = point_store.clone();
    let device_store_for_protocols = device_store.clone();
    let point_mapping_store_for_protocols = point_mapping_store.clone();
    let online_store_for_protocols = online_store.clone();
    let handler = Arc::new(PipelineHandler {
        normalizer,
        pipeline,
        point_store,
        device_store: device_store.clone(),
        online_store: online_store.clone(),
        collection_strategy_store,
        wal_store,
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

    // 2.    // 3. 产生 MQTT 采集任务
    let source: Arc<dyn Source> = if config.ingest_enabled {
        let mqtt_config = MqttSourceConfig {
            host: config.mqtt_host.clone(),
            port: config.mqtt_port,
            username: config.mqtt_username.clone(),
            password: config.mqtt_password.clone(),
            topic_prefix: config.mqtt_topic_prefix.clone(),
            has_source_id: true,
            use_shared_subscription: config.mqtt_use_shared_subscription,
            shared_group: config.mqtt_shared_group.clone(),
            use_tls: config.mqtt_use_tls,
            ca_cert_path: config.mqtt_ca_cert_path.clone(),
            client_cert_path: config.mqtt_client_cert_path.clone(),
            client_key_path: config.mqtt_client_key_path.clone(),
        };
        Arc::new(MqttSource::new(mqtt_config))
    } else {
        Arc::new(NoopSource)
    };

    let handler_move = handler.clone();
    tokio::spawn(async move {
        if let Err(err) = source.run(handler_move).await {
            warn!("ingest stopped: {}", err);
        }
    });

    // 4. 启动 Modbus 多网关管理
    if config.ingest_enabled {
        let manager = ModbusProtocolManager::new(
            handler.clone(),
            gateway_store_for_protocols.clone(),
            device_store_for_protocols.clone(),
            point_mapping_store_for_protocols.clone(),
            online_store_for_protocols.clone(),
        );
        tokio::spawn(async move {
            manager.run().await;
        });
    }

    // 5. 启动 TCP Server 多网关管理
    if config.ingest_enabled {
        let manager = TcpServerProtocolManager::new(
            handler.clone(),
            gateway_store_for_protocols,
            device_store_for_protocols,
            point_store_for_protocols,
            point_mapping_store_for_protocols,
            online_store_for_protocols,
        );
        tokio::spawn(async move {
            manager.run().await;
        });
    }

    tokio::spawn(async move {
        // 占位 JoinHandle
    })
}

/// Modbus 协议管理器，负责动态加载网关并管理采集任务
struct ModbusProtocolManager {
    handler: Arc<PipelineHandler>,
    gateway_store: Arc<dyn ems_storage::GatewayStore>,
    device_store: Arc<dyn ems_storage::DeviceStore>,
    point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
    online_store: Arc<dyn OnlineStore>,
    /// 已启动的网关任务 (gateway_id -> AbortHandle)
    tasks: Arc<Mutex<std::collections::HashMap<String, tokio::task::AbortHandle>>>,
}

impl ModbusProtocolManager {
    fn new(
        handler: Arc<PipelineHandler>,
        gateway_store: Arc<dyn ems_storage::GatewayStore>,
        device_store: Arc<dyn ems_storage::DeviceStore>,
        point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
        online_store: Arc<dyn OnlineStore>,
    ) -> Self {
        Self {
            handler,
            gateway_store,
            device_store,
            point_mapping_store,
            online_store,
            tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    async fn run(&self) {
        info!("modbus protocol manager started");
        let mut interval = tokio::time::interval(Duration::from_secs(30)); // 每 30 秒扫描一次网关配置变更
        loop {
            interval.tick().await;
            if let Err(e) = self.sync_gateways().await {
                error!("failed to sync modbus gateways: {}", e);
            }
        }
    }

    async fn sync_gateways(&self) -> Result<(), ems_storage::StorageError> {
        // 1. 获取所有 modbus_tcp 类型的网关（后台扫描）
        let ctx = domain::TenantContext::new(
            "".to_string(), // 全量扫描租户 ID 可为空或特定系统租户
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            None,
        );
        let gateways = self
            .gateway_store
            .list_gateways_by_protocol_type(&ctx, "modbus_tcp")
            .await?;

        let mut current_tasks = self.tasks.lock().await;
        let mut active_gateway_ids = std::collections::HashSet::new();

        for gw in gateways {
            if gw.protocol_type != "modbus_tcp" {
                continue;
            }
            active_gateway_ids.insert(gw.gateway_id.clone());

            let gw_id = gw.gateway_id.clone();
            current_tasks.entry(gw_id).or_insert_with(|| {
                // 启动新任务
                self.spawn_gateway_task(gw)
            });
        }

        // 清理已删除或类型变更的网关
        current_tasks.retain(|id, handle| {
            if !active_gateway_ids.contains(id) {
                handle.abort();
                false
            } else {
                true
            }
        });

        Ok(())
    }

    fn spawn_gateway_task(
        &self,
        gateway: ems_storage::models::GatewayRecord,
    ) -> tokio::task::AbortHandle {
        let manager = Arc::new(self.clone_lite());
        let join_handle = tokio::spawn(async move {
            loop {
                let res = manager.run_gateway_loop(&gateway).await;
                if let Err(e) = res {
                    warn!(gateway_id = %gateway.gateway_id, error = %e, "modbus gateway loop crashed, restarting...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });
        join_handle.abort_handle()
    }

    fn clone_lite(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            gateway_store: self.gateway_store.clone(),
            device_store: self.device_store.clone(),
            point_mapping_store: self.point_mapping_store.clone(),
            online_store: self.online_store.clone(),
            tasks: self.tasks.clone(),
        }
    }

    async fn run_gateway_loop(
        &self,
        gateway: &ems_storage::models::GatewayRecord,
    ) -> Result<(), IngestError> {
        let config_str = gateway.protocol_config.as_deref().unwrap_or("{}");
        let config: ModbusTcpConfig = serde_json::from_str(config_str).map_err(|_| {
            IngestError::Source(format!("invalid modbus_tcp config: {}", config_str))
        })?;

        // 构建任务列表
        let ctx = domain::TenantContext::new(
            gateway.tenant_id.clone(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            Some(gateway.project_id.clone()),
        );

        let devices = self
            .device_store
            .list_devices(&ctx, &gateway.project_id)
            .await
            .map_err(|e| IngestError::Source(e.to_string()))?;
        let mut source = ModbusTcpSource::new(config.clone());

        for dev in devices {
            if dev.gateway_id != gateway.gateway_id {
                continue;
            }
            let addr_config = dev.address_config.as_deref().unwrap_or("{}");

            let mappings = self
                .point_mapping_store
                .list_point_mappings(&ctx, &gateway.project_id)
                .await
                .map_err(|e| IngestError::Source(e.to_string()))?;

            for mapping in mappings {
                if mapping.source_type != "modbus" {
                    continue;
                }
                // 这里需要根据 mapping.point_id 确认是否属于该设备
                // 简单起见，如果 point_sources 表里没存设备 ID，得去点位表查
                // 这是一个 N+1 隐患，但在采集初始化阶段可以接受
                // 或者我们可以先 list_points
                let detail = mapping.protocol_detail.as_deref().unwrap_or("{}");

                let _ = source.add_task_from_config(
                    &gateway.tenant_id,
                    &gateway.project_id,
                    &gateway.gateway_id,
                    &dev.device_id,
                    &mapping.source_id,
                    addr_config,
                    detail,
                    mapping.scale,
                    mapping.offset,
                );
            }
        }

        // 运行协议采集
        let proxy_handler = Arc::new(ModbusToIngestHandler {
            handler: self.handler.clone(),
            point_mapping_store: self.point_mapping_store.clone(),
            project_id: gateway.project_id.clone(),
        });

        // 这里的 run 需要增加错误报告逻辑
        // 因为 ems-protocol 的 run 是死循环，我们可能需要修改它或者在循环外层处理连接错误
        loop {
            match source.run(proxy_handler.clone()).await {
                Ok(_) => break, // 正常退出
                Err(e) => {
                    let err_msg = e.to_string();
                    warn!(gateway_id = %gateway.gateway_id, error = %err_msg, "modbus poll error");
                    // 报告给 OnlineStore
                    let _ = self
                        .online_store
                        .report_resource_error(
                            &ctx,
                            &gateway.project_id,
                            &gateway.gateway_id,
                            &err_msg,
                        )
                        .await;
                    // 指数退避重连
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        Ok(())
    }
}

/// TCP Server 协议管理器，负责动态加载网关并管理监听任务
struct TcpServerProtocolManager {
    handler: Arc<PipelineHandler>,
    gateway_store: Arc<dyn ems_storage::GatewayStore>,
    device_store: Arc<dyn ems_storage::DeviceStore>,
    point_store: Arc<dyn ems_storage::PointStore>,
    point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
    online_store: Arc<dyn OnlineStore>,
    tasks: Arc<Mutex<std::collections::HashMap<String, tokio::task::AbortHandle>>>,
}

impl TcpServerProtocolManager {
    fn new(
        handler: Arc<PipelineHandler>,
        gateway_store: Arc<dyn ems_storage::GatewayStore>,
        device_store: Arc<dyn ems_storage::DeviceStore>,
        point_store: Arc<dyn ems_storage::PointStore>,
        point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
        online_store: Arc<dyn OnlineStore>,
    ) -> Self {
        Self {
            handler,
            gateway_store,
            device_store,
            point_store,
            point_mapping_store,
            online_store,
            tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    async fn run(&self) {
        info!("tcp server protocol manager started");
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = self.sync_gateways().await {
                error!("failed to sync tcp_server gateways: {}", e);
            }
        }
    }

    async fn sync_gateways(&self) -> Result<(), ems_storage::StorageError> {
        let ctx = domain::TenantContext::new(
            "".to_string(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            None,
        );
        let gateways = self
            .gateway_store
            .list_gateways_by_protocol_type(&ctx, "tcp_server")
            .await?;

        let mut current_tasks = self.tasks.lock().await;
        let mut active_gateway_ids = std::collections::HashSet::new();
        for gw in gateways {
            active_gateway_ids.insert(gw.gateway_id.clone());
            let gw_id = gw.gateway_id.clone();
            current_tasks.entry(gw_id).or_insert_with(|| self.spawn_gateway_task(gw));
        }

        current_tasks.retain(|id, handle| {
            if !active_gateway_ids.contains(id) {
                handle.abort();
                false
            } else {
                true
            }
        });

        Ok(())
    }

    fn spawn_gateway_task(
        &self,
        gateway: ems_storage::models::GatewayRecord,
    ) -> tokio::task::AbortHandle {
        let manager = Arc::new(self.clone_lite());
        let join_handle = tokio::spawn(async move {
            let res = manager.run_gateway_loop(&gateway).await;
            if let Err(e) = res {
                warn!(
                    gateway_id = %gateway.gateway_id,
                    error = %e,
                    "tcp_server gateway loop stopped"
                );
            }
        });
        join_handle.abort_handle()
    }

    fn clone_lite(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            gateway_store: self.gateway_store.clone(),
            device_store: self.device_store.clone(),
            point_store: self.point_store.clone(),
            point_mapping_store: self.point_mapping_store.clone(),
            online_store: self.online_store.clone(),
            tasks: self.tasks.clone(),
        }
    }

    async fn run_gateway_loop(
        &self,
        gateway: &ems_storage::models::GatewayRecord,
    ) -> Result<(), IngestError> {
        let config_str = gateway.protocol_config.as_deref().unwrap_or("{}");
        let config: TcpServerConfig = serde_json::from_str(config_str).map_err(|_| {
            IngestError::Source(format!("invalid tcp_server config: {}", config_str))
        })?;

        let source = TcpServerSource::new(config);

        // 构建映射表：device.address_config(devId) + point_sources.protocol_detail(tag/valueType/endian)
        let ctx = domain::TenantContext::new(
            gateway.tenant_id.clone(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            Some(gateway.project_id.clone()),
        );

        let devices = self
            .device_store
            .list_devices(&ctx, &gateway.project_id)
            .await
            .map_err(|e| IngestError::Source(e.to_string()))?;
        let mut device_addr: std::collections::HashMap<String, TcpDeviceAddress> =
            std::collections::HashMap::new();
        for dev in &devices {
            if dev.gateway_id != gateway.gateway_id {
                continue;
            }
            let addr_json = dev.address_config.as_deref().unwrap_or("{}");
            let addr: TcpDeviceAddress = serde_json::from_str(addr_json).map_err(|_| {
                IngestError::Source(format!("invalid tcp device address_config: {}", addr_json))
            })?;
            device_addr.insert(dev.device_id.clone(), addr);
        }

        let points = self
            .point_store
            .list_points(&ctx, &gateway.project_id)
            .await
            .map_err(|e| IngestError::Source(e.to_string()))?;
        let mut point_to_device: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for p in points {
            point_to_device.insert(p.point_id, p.device_id);
        }

        let mappings = self
            .point_mapping_store
            .list_point_mappings(&ctx, &gateway.project_id)
            .await
            .map_err(|e| IngestError::Source(e.to_string()))?;

        for mapping in mappings {
            if mapping.source_type != "tcp" {
                continue;
            }
            let Some(device_id) = point_to_device.get(&mapping.point_id).cloned() else {
                continue;
            };
            let Some(addr) = device_addr.get(&device_id).cloned() else {
                continue;
            };

            let detail_json = mapping.protocol_detail.as_deref().unwrap_or("{}");
            let detail: TcpPointDetail = serde_json::from_str(detail_json).map_err(|_| {
                IngestError::Source(format!(
                    "invalid tcp point protocol_detail: {}",
                    detail_json
                ))
            })?;

            source
                .register_point_mapping(
                    addr.dev_id,
                    detail.tag,
                    TcpPointMapping {
                        tenant_id: gateway.tenant_id.clone(),
                        project_id: gateway.project_id.clone(),
                        gateway_id: gateway.gateway_id.clone(),
                        device_id,
                        source_id: mapping.source_id.clone(),
                        value_type: detail.value_type,
                        endian: detail.endian,
                        scale: mapping.scale,
                        offset: mapping.offset,
                    },
                )
                .await;
        }

        let proxy_handler = Arc::new(TcpToIngestHandler {
            handler: self.handler.clone(),
            point_mapping_store: self.point_mapping_store.clone(),
            project_id: gateway.project_id.clone(),
        });

        // TCP server 会一直监听，只有 bind 等错误才会返回 Err
        if let Err(err) = source.run(proxy_handler).await {
            let err_msg = err.to_string();
            warn!(gateway_id = %gateway.gateway_id, error = %err_msg, "tcp_server run failed");
            let _ = self
                .online_store
                .report_resource_error(&ctx, &gateway.project_id, &gateway.gateway_id, &err_msg)
                .await;
            return Err(IngestError::Source(err_msg));
        }

        Ok(())
    }
}

struct TcpToIngestHandler {
    handler: Arc<PipelineHandler>,
    point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
    project_id: String,
}

#[async_trait::async_trait]
impl ems_protocol::ProtocolEventHandler for TcpToIngestHandler {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ems_protocol::ProtocolError> {
        let ctx = domain::TenantContext::new(
            event.tenant_id.clone(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            Some(self.project_id.clone()),
        );

        let mapping = self
            .point_mapping_store
            .find_point_mapping(&ctx, &self.project_id, &event.source_id)
            .await
            .map_err(|e| ems_protocol::ProtocolError::DataParse(e.to_string()))?;

        let Some(mapping) = mapping else {
            return Ok(());
        };

        let value = domain::PointValue {
            tenant_id: event.tenant_id,
            project_id: event.project_id,
            point_id: mapping.point_id,
            ts_ms: event.received_at_ms,
            value: domain::PointValueData::F64(event.value),
            quality: None,
        };
        let _ = self.handler.handle_point_value(value).await;
        Ok(())
    }
}

struct ModbusToIngestHandler {
    handler: Arc<PipelineHandler>,
    point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
    project_id: String,
}

#[async_trait::async_trait]
impl ems_protocol::ProtocolEventHandler for ModbusToIngestHandler {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ems_protocol::ProtocolError> {
        let ctx = domain::TenantContext::new(
            event.tenant_id.clone(),
            "system".to_string(),
            Vec::new(),
            Vec::new(),
            Some(self.project_id.clone()),
        );

        // 1. 获取映射以解析出 point_id
        let mapping = self
            .point_mapping_store
            .find_point_mapping(&ctx, &self.project_id, &event.source_id)
            .await
            .map_err(|e| ems_protocol::ProtocolError::DataParse(e.to_string()))?;

        let Some(mapping) = mapping else {
            return Ok(());
        };

        let value = domain::PointValue {
            tenant_id: event.tenant_id,
            project_id: event.project_id,
            point_id: mapping.point_id,
            ts_ms: event.received_at_ms,
            value: domain::PointValueData::F64(event.value),
            quality: None,
        };

        // 直接进入流水线处理器
        let _ = self.handler.handle_point_value(value).await;

        Ok(())
    }
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
