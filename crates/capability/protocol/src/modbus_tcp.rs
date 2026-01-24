//! Modbus TCP 客户端实现
//!
//! 连接 Modbus 从设备，周期性轮询寄存器数据。
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! let config = ModbusTcpConfig {
//!     host: "192.168.1.100".to_string(),
//!     port: 502,
//!     poll_interval_ms: 1000,
//! };
//! let source = ModbusTcpSource::new(config);
//! source.run(handler).await?;
//! ```

use crate::error::ProtocolError;
use crate::types::{
    now_epoch_ms, Endian, ModbusDataType, ModbusDeviceAddress, ModbusPointDetail, ModbusWordOrder,
    ProtocolEvent,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use tokio_modbus::prelude::*;
use tracing::{debug, info, warn};

/// Modbus TCP 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModbusTcpConfig {
    /// Modbus 服务器主机地址
    pub host: String,
    /// Modbus 服务器端口（默认 502）
    #[serde(default = "default_modbus_port")]
    pub port: u16,
    /// 轮询间隔（毫秒）
    #[serde(default = "default_poll_interval", alias = "poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// 连接超时（毫秒）
    #[serde(default = "default_connect_timeout", alias = "connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    /// 单次请求超时（毫秒）
    #[serde(default = "default_request_timeout", alias = "read_timeout_ms")]
    pub request_timeout_ms: u64,
    /// 超时重试次数（默认 0）
    #[serde(default)]
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    #[serde(default)]
    pub retry_interval_ms: u64,
    /// 重连退避（毫秒）
    #[serde(default = "default_reconnect_backoff")]
    pub reconnect_backoff_ms: u64,
}

fn default_modbus_port() -> u16 {
    502
}

fn default_poll_interval() -> u64 {
    1000
}

fn default_connect_timeout() -> u64 {
    5000
}

fn default_request_timeout() -> u64 {
    3000
}

fn default_reconnect_backoff() -> u64 {
    5000
}

/// 轮询任务配置
#[derive(Debug, Clone)]
pub struct PollTask {
    /// 租户 ID
    pub tenant_id: String,
    /// 项目 ID
    pub project_id: String,
    /// 网关 ID
    pub gateway_id: String,
    /// 设备 ID
    pub device_id: String,
    /// 点位映射 ID
    pub source_id: String,
    /// 从站 ID（Unit ID）
    pub unit_id: u8,
    /// 寄存器地址
    pub register_address: u16,
    /// 寄存器数量
    pub register_count: u16,
    /// 功能码
    pub function_code: u8,
    /// 数据类型
    pub data_type: ModbusDataType,
    /// 16-bit 字节序
    pub endian: Endian,
    /// 32-bit 字序/字节序组合（Int32/Uint32/Float32 必填）
    pub word_order: Option<ModbusWordOrder>,
    /// 缩放系数
    pub scale: Option<f64>,
    /// 偏移量
    pub offset: Option<f64>,
}

/// 协议事件处理器
#[async_trait]
pub trait ProtocolEventHandler: Send + Sync {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ProtocolError>;
}

/// Modbus TCP 采集源
pub struct ModbusTcpSource {
    config: ModbusTcpConfig,
    tasks: Vec<PollTask>,
}

impl ModbusTcpSource {
    /// 创建新的 Modbus TCP 源
    pub fn new(config: ModbusTcpConfig) -> Self {
        Self {
            config,
            tasks: Vec::new(),
        }
    }

    /// 从 JSON 配置字符串解析
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let config: ModbusTcpConfig =
            serde_json::from_str(json).map_err(|e| ProtocolError::ConfigParse(e.to_string()))?;
        Ok(Self::new(config))
    }

    /// 添加轮询任务
    pub fn add_task(&mut self, task: PollTask) {
        self.tasks.push(task);
    }

    /// 从设备和点位配置添加任务
    #[allow(clippy::too_many_arguments)]
    pub fn add_task_from_config(
        &mut self,
        tenant_id: &str,
        project_id: &str,
        gateway_id: &str,
        device_id: &str,
        source_id: &str,
        device_address_json: &str,
        point_detail_json: &str,
        scale: Option<f64>,
        offset: Option<f64>,
    ) -> Result<(), ProtocolError> {
        let device_addr: ModbusDeviceAddress = serde_json::from_str(device_address_json)
            .map_err(|e| ProtocolError::ConfigParse(format!("device address: {}", e)))?;
        let point_detail: ModbusPointDetail = serde_json::from_str(point_detail_json)
            .map_err(|e| ProtocolError::ConfigParse(format!("point detail: {}", e)))?;

        let task = PollTask {
            tenant_id: tenant_id.to_string(),
            project_id: project_id.to_string(),
            gateway_id: gateway_id.to_string(),
            device_id: device_id.to_string(),
            source_id: source_id.to_string(),
            unit_id: device_addr.unit_id,
            register_address: point_detail.register_address,
            register_count: point_detail.register_count,
            function_code: point_detail.function_code,
            data_type: point_detail.data_type,
            endian: point_detail.endian,
            word_order: point_detail.word_order,
            scale,
            offset,
        };

        self.tasks.push(task);
        Ok(())
    }

    /// 执行一次性采集测试
    pub async fn test_once(&self) -> Result<Vec<ProtocolEvent>, ProtocolError> {
        if self.tasks.is_empty() {
            return Ok(Vec::new());
        }
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| ProtocolError::ConfigParse(format!("invalid address: {}", e)))?;

        let mut ctx = self.connect(addr).await?;

        let handler = Arc::new(TestEventHandler {
            events: Mutex::new(Vec::new()),
        });

        self.poll_tick(
            &mut ctx,
            &(handler.clone() as Arc<dyn ProtocolEventHandler>),
        )
        .await?;

        let events = handler.events.lock().await.clone();
        Ok(events)
    }

    /// 运行采集循环
    pub async fn run(&self, handler: Arc<dyn ProtocolEventHandler>) -> Result<(), ProtocolError> {
        if self.tasks.is_empty() {
            warn!("no poll tasks configured for modbus source");
            return Ok(());
        }

        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| ProtocolError::ConfigParse(format!("invalid address: {}", e)))?;

        info!(
            "connecting to modbus server at {} with {} tasks",
            addr,
            self.tasks.len()
        );

        // 轮询循环：连接断开时返回 Err，由上层（管理器）负责重启与告警。
        let mut poll_interval = interval(Duration::from_millis(self.config.poll_interval_ms));
        let mut ctx = self.connect(addr).await?;
        info!("connected to modbus server at {}", addr);

        loop {
            poll_interval.tick().await;

            match self.poll_tick(&mut ctx, &handler).await {
                Ok(()) => {}
                Err(err) => {
                    warn!(error = %err, "modbus poll tick failed");
                    return Err(err);
                }
            }
        }
    }

    async fn connect(
        &self,
        addr: SocketAddr,
    ) -> Result<tokio_modbus::client::Context, ProtocolError> {
        tokio::time::timeout(
            Duration::from_millis(self.config.connect_timeout_ms),
            tcp::connect(addr),
        )
        .await
        .map_err(|_| ProtocolError::Timeout("connect timeout".to_string()))?
        .map_err(|e| ProtocolError::Connection(e.to_string()))
    }

    async fn poll_tick(
        &self,
        ctx: &mut tokio_modbus::client::Context,
        handler: &Arc<dyn ProtocolEventHandler>,
    ) -> Result<(), ProtocolError> {
        // 以 (unit_id, function_code) 分组做批读，减少请求次数。
        let mut groups: std::collections::HashMap<(u8, u8), Vec<&PollTask>> =
            std::collections::HashMap::new();
        for task in &self.tasks {
            groups
                .entry((task.unit_id, task.function_code))
                .or_default()
                .push(task);
        }

        for ((unit_id, function_code), mut tasks) in groups {
            tasks.sort_by_key(|t| t.register_address);
            self.poll_group(ctx, unit_id, function_code, &tasks, handler)
                .await?;
        }

        Ok(())
    }

    async fn poll_group(
        &self,
        ctx: &mut tokio_modbus::client::Context,
        unit_id: u8,
        function_code: u8,
        tasks: &[&PollTask],
        handler: &Arc<dyn ProtocolEventHandler>,
    ) -> Result<(), ProtocolError> {
        if tasks.is_empty() {
            return Ok(());
        }

        ctx.set_slave(Slave(unit_id));

        #[derive(Debug)]
        struct Segment<'a> {
            start: u16,
            count: u16,
            tasks: Vec<&'a PollTask>,
        }

        let mut segments: Vec<Segment<'_>> = Vec::new();
        for task in tasks {
            let task_end = task
                .register_address
                .saturating_add(task.register_count.saturating_sub(1));
            if let Some(seg) = segments.last_mut() {
                let seg_end = seg.start.saturating_add(seg.count.saturating_sub(1));
                if task.register_address <= seg_end.saturating_add(1) {
                    // contiguous: merge
                    let new_end = seg_end.max(task_end);
                    seg.count = new_end.saturating_sub(seg.start).saturating_add(1);
                    seg.tasks.push(task);
                    continue;
                }
            }
            segments.push(Segment {
                start: task.register_address,
                count: task.register_count,
                tasks: vec![task],
            });
        }

        for seg in segments {
            match function_code {
                1 | 2 => {
                    let bits = self
                        .read_bits_with_retries(ctx, function_code, seg.start, seg.count)
                        .await?;
                    for task in seg.tasks {
                        let offset = task.register_address.saturating_sub(seg.start) as usize;
                        let value = bits.get(offset).copied().ok_or_else(|| {
                            ProtocolError::DataParse("bit index out of range".to_string())
                        })?;
                        let event = ProtocolEvent {
                            tenant_id: task.tenant_id.clone(),
                            project_id: task.project_id.clone(),
                            gateway_id: task.gateway_id.clone(),
                            device_id: task.device_id.clone(),
                            source_id: task.source_id.clone(),
                            value: if value { 1.0 } else { 0.0 },
                            received_at_ms: now_epoch_ms(),
                        };
                        if let Err(e) = handler.handle(event).await {
                            warn!(
                                source_id = %task.source_id,
                                error = %e,
                                "failed to handle protocol event"
                            );
                        }
                    }
                }
                3 | 4 => {
                    let registers = self
                        .read_registers_with_retries(ctx, function_code, seg.start, seg.count)
                        .await?;
                    debug!(
                        unit_id = unit_id,
                        function_code = function_code,
                        register = seg.start,
                        count = seg.count,
                        values = ?registers,
                        "read modbus registers"
                    );

                    for task in seg.tasks {
                        let offset = task.register_address.saturating_sub(seg.start) as usize;
                        let count = task.register_count as usize;
                        let slice = registers
                            .get(offset..offset.saturating_add(count))
                            .ok_or_else(|| {
                                ProtocolError::DataParse("register slice out of range".to_string())
                            })?;

                        let raw_value = self.parse_registers(
                            slice,
                            task.data_type,
                            task.endian,
                            task.word_order,
                        )?;

                        let scaled_value = match (task.scale, task.offset) {
                            (Some(scale), Some(offset)) => raw_value * scale + offset,
                            (Some(scale), None) => raw_value * scale,
                            (None, Some(offset)) => raw_value + offset,
                            (None, None) => raw_value,
                        };

                        let event = ProtocolEvent {
                            tenant_id: task.tenant_id.clone(),
                            project_id: task.project_id.clone(),
                            gateway_id: task.gateway_id.clone(),
                            device_id: task.device_id.clone(),
                            source_id: task.source_id.clone(),
                            value: scaled_value,
                            received_at_ms: now_epoch_ms(),
                        };
                        if let Err(e) = handler.handle(event).await {
                            warn!(
                                source_id = %task.source_id,
                                error = %e,
                                "failed to handle protocol event"
                            );
                        }
                    }
                }
                _ => {
                    return Err(ProtocolError::ConfigParse(format!(
                        "unsupported function code: {}",
                        function_code
                    )));
                }
            }
        }

        Ok(())
    }

    async fn read_registers_with_retries(
        &self,
        ctx: &mut tokio_modbus::client::Context,
        function_code: u8,
        start: u16,
        count: u16,
    ) -> Result<Vec<u16>, ProtocolError> {
        let mut attempt = 0u32;
        loop {
            let res = tokio::time::timeout(
                Duration::from_millis(self.config.request_timeout_ms),
                async {
                    match function_code {
                        3 => ctx.read_holding_registers(start, count).await,
                        4 => ctx.read_input_registers(start, count).await,
                        _ => unreachable!("checked by caller"),
                    }
                },
            )
            .await;

            match res {
                Err(_) => {
                    attempt += 1;
                    if attempt > self.config.max_retries {
                        return Err(ProtocolError::Timeout("modbus request timeout".to_string()));
                    }
                }
                Ok(Err(e)) => {
                    attempt += 1;
                    if attempt > self.config.max_retries {
                        return Err(ProtocolError::Connection(e.to_string()));
                    }
                }
                Ok(Ok(Err(exc))) => {
                    return Err(ProtocolError::Modbus(format!("exception: {:?}", exc)));
                }
                Ok(Ok(Ok(values))) => return Ok(values),
            }

            if self.config.retry_interval_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.retry_interval_ms)).await;
            }
        }
    }

    async fn read_bits_with_retries(
        &self,
        ctx: &mut tokio_modbus::client::Context,
        function_code: u8,
        start: u16,
        count: u16,
    ) -> Result<Vec<bool>, ProtocolError> {
        let mut attempt = 0u32;
        loop {
            let res = tokio::time::timeout(
                Duration::from_millis(self.config.request_timeout_ms),
                async {
                    match function_code {
                        1 => ctx.read_coils(start, count).await,
                        2 => ctx.read_discrete_inputs(start, count).await,
                        _ => unreachable!("checked by caller"),
                    }
                },
            )
            .await;

            match res {
                Err(_) => {
                    attempt += 1;
                    if attempt > self.config.max_retries {
                        return Err(ProtocolError::Timeout("modbus request timeout".to_string()));
                    }
                }
                Ok(Err(e)) => {
                    attempt += 1;
                    if attempt > self.config.max_retries {
                        return Err(ProtocolError::Connection(e.to_string()));
                    }
                }
                Ok(Ok(Err(exc))) => {
                    return Err(ProtocolError::Modbus(format!("exception: {:?}", exc)));
                }
                Ok(Ok(Ok(values))) => return Ok(values),
            }

            if self.config.retry_interval_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.retry_interval_ms)).await;
            }
        }
    }

    /// 解析寄存器数据为浮点值
    fn parse_registers(
        &self,
        registers: &[u16],
        data_type: ModbusDataType,
        endian: Endian,
        word_order: Option<ModbusWordOrder>,
    ) -> Result<f64, ProtocolError> {
        if registers.is_empty() {
            return Err(ProtocolError::DataParse("empty registers".to_string()));
        }

        let value = match data_type {
            ModbusDataType::Bool => {
                let v = self.u16_with_endian(registers[0], endian);
                if v == 0 {
                    0.0
                } else {
                    1.0
                }
            }
            ModbusDataType::Int16 => self.u16_with_endian(registers[0], endian) as i16 as f64,
            ModbusDataType::Uint16 => self.u16_with_endian(registers[0], endian) as f64,
            ModbusDataType::Int32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::DataParse(
                        "need 2 registers for int32".to_string(),
                    ));
                }
                let bytes = self.u32_bytes(registers, endian, word_order)?;
                i32::from_be_bytes(bytes) as f64
            }
            ModbusDataType::Uint32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::DataParse(
                        "need 2 registers for uint32".to_string(),
                    ));
                }
                let bytes = self.u32_bytes(registers, endian, word_order)?;
                u32::from_be_bytes(bytes) as f64
            }
            ModbusDataType::Float32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::DataParse(
                        "need 2 registers for float32".to_string(),
                    ));
                }
                let bytes = self.u32_bytes(registers, endian, word_order)?;
                f32::from_bits(u32::from_be_bytes(bytes)) as f64
            }
            ModbusDataType::Float64 => {
                if registers.len() < 4 {
                    return Err(ProtocolError::DataParse(
                        "need 4 registers for float64".to_string(),
                    ));
                }
                let mut bytes = [0u8; 8];
                for (idx, reg) in registers[..4].iter().enumerate() {
                    let word = self.u16_bytes(*reg, endian);
                    bytes[idx * 2] = word[0];
                    bytes[idx * 2 + 1] = word[1];
                }
                f64::from_bits(u64::from_be_bytes(bytes))
            }
        };

        Ok(value)
    }

    fn u16_with_endian(&self, value: u16, endian: Endian) -> u16 {
        match endian {
            Endian::Big => value,
            Endian::Little => value.swap_bytes(),
        }
    }

    fn u16_bytes(&self, value: u16, endian: Endian) -> [u8; 2] {
        self.u16_with_endian(value, endian).to_be_bytes()
    }

    fn u32_bytes(
        &self,
        registers: &[u16],
        endian: Endian,
        word_order: Option<ModbusWordOrder>,
    ) -> Result<[u8; 4], ProtocolError> {
        if word_order.is_none() {
            return Err(ProtocolError::ConfigParse(
                "wordOrder is required for 32-bit values".to_string(),
            ));
        }
        let mut base = [0u8; 4];
        let r0 = self.u16_bytes(registers[0], endian);
        let r1 = self.u16_bytes(registers[1], endian);
        base[0] = r0[0];
        base[1] = r0[1];
        base[2] = r1[0];
        base[3] = r1[1];
        Ok(apply_word_order(base, word_order.unwrap()))
    }
}

fn apply_word_order(bytes: [u8; 4], order: ModbusWordOrder) -> [u8; 4] {
    match order {
        ModbusWordOrder::ABCD => bytes,
        ModbusWordOrder::CDAB => [bytes[2], bytes[3], bytes[0], bytes[1]],
        ModbusWordOrder::BADC => [bytes[1], bytes[0], bytes[3], bytes[2]],
        ModbusWordOrder::DCBA => [bytes[3], bytes[2], bytes[1], bytes[0]],
    }
}

struct TestEventHandler {
    events: Mutex<Vec<ProtocolEvent>>,
}

#[async_trait]
impl ProtocolEventHandler for TestEventHandler {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ProtocolError> {
        self.events.lock().await.push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{"host": "192.168.1.100", "port": 502, "pollIntervalMs": 1000}"#;
        let source = ModbusTcpSource::from_json(json).unwrap();
        assert_eq!(source.config.host, "192.168.1.100");
        assert_eq!(source.config.port, 502);
        assert_eq!(source.config.poll_interval_ms, 1000);
    }

    #[test]
    fn test_parse_registers_int16() {
        let source = ModbusTcpSource::new(ModbusTcpConfig {
            host: "localhost".to_string(),
            port: 502,
            poll_interval_ms: 1000,
            connect_timeout_ms: 5000,
            request_timeout_ms: 3000,
            max_retries: 0,
            retry_interval_ms: 0,
            reconnect_backoff_ms: 5000,
        });

        // 正数
        let registers = [100u16];
        let value = source
            .parse_registers(&registers, ModbusDataType::Int16, Endian::Big, None)
            .unwrap();
        assert_eq!(value, 100.0);

        // 负数
        let registers = [(-100i16) as u16];
        let value = source
            .parse_registers(&registers, ModbusDataType::Int16, Endian::Big, None)
            .unwrap();
        assert_eq!(value, -100.0);
    }

    #[test]
    fn test_parse_device_address() {
        let json = r#"{"unitId": 1}"#;
        let addr: ModbusDeviceAddress = serde_json::from_str(json).unwrap();
        assert_eq!(addr.unit_id, 1);
    }

    #[test]
    fn test_parse_point_detail() {
        let json = r#"{"functionCode": 3, "registerAddress": 100, "registerCount": 1, "dataType": "int16"}"#;
        let detail: ModbusPointDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.function_code, 3);
        assert_eq!(detail.register_address, 100);
        assert_eq!(detail.register_count, 1);
        assert_eq!(detail.data_type, ModbusDataType::Int16);
    }
}
