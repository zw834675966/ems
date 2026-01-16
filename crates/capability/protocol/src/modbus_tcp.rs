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
    now_epoch_ms, ModbusDataType, ModbusDeviceAddress, ModbusPointDetail, ProtocolEvent,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio_modbus::prelude::*;
use tracing::{debug, info, warn};

/// Modbus TCP 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusTcpConfig {
    /// Modbus 服务器主机地址
    pub host: String,
    /// Modbus 服务器端口（默认 502）
    #[serde(default = "default_modbus_port")]
    pub port: u16,
    /// 轮询间隔（毫秒）
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    /// 连接超时（毫秒）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// 读取超时（毫秒）
    #[serde(default = "default_read_timeout")]
    pub read_timeout_ms: u64,
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

fn default_read_timeout() -> u64 {
    3000
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
    /// 从站 ID
    pub slave_id: u8,
    /// 寄存器地址
    pub register_address: u16,
    /// 寄存器数量
    pub register_count: u16,
    /// 功能码
    pub function_code: u8,
    /// 数据类型
    pub data_type: ModbusDataType,
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
            slave_id: device_addr.slave_id,
            register_address: point_detail.register_address,
            register_count: point_detail.register_count,
            function_code: point_detail.function_code,
            data_type: point_detail.data_type,
            scale,
            offset,
        };

        self.tasks.push(task);
        Ok(())
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

        // 连接 Modbus 服务器
        let mut ctx = tcp::connect(addr)
            .await
            .map_err(|e| ProtocolError::Connection(e.to_string()))?;

        info!("connected to modbus server at {}", addr);

        // 轮询循环
        let mut poll_interval = interval(Duration::from_millis(self.config.poll_interval_ms));

        loop {
            poll_interval.tick().await;

            for task in &self.tasks {
                match self.poll_task(&mut ctx, task).await {
                    Ok(value) => {
                        let event = ProtocolEvent {
                            tenant_id: task.tenant_id.clone(),
                            project_id: task.project_id.clone(),
                            gateway_id: task.gateway_id.clone(),
                            device_id: task.device_id.clone(),
                            source_id: task.source_id.clone(),
                            value,
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
                    Err(e) => {
                        warn!(
                            source_id = %task.source_id,
                            slave = task.slave_id,
                            register = task.register_address,
                            error = %e,
                            "failed to poll modbus register"
                        );
                    }
                }
            }
        }
    }

    /// 轮询单个任务
    async fn poll_task(
        &self,
        ctx: &mut tokio_modbus::client::Context,
        task: &PollTask,
    ) -> Result<f64, ProtocolError> {
        ctx.set_slave(Slave(task.slave_id));

        let registers = match task.function_code {
            3 => {
                // 读保持寄存器
                ctx.read_holding_registers(task.register_address, task.register_count)
                    .await
                    .map_err(|e| ProtocolError::Modbus(e.to_string()))?
                    .map_err(|e| ProtocolError::Modbus(format!("exception: {:?}", e)))?
            }
            4 => {
                // 读输入寄存器
                ctx.read_input_registers(task.register_address, task.register_count)
                    .await
                    .map_err(|e| ProtocolError::Modbus(e.to_string()))?
                    .map_err(|e| ProtocolError::Modbus(format!("exception: {:?}", e)))?
            }
            _ => {
                return Err(ProtocolError::ConfigParse(format!(
                    "unsupported function code: {}",
                    task.function_code
                )));
            }
        };

        debug!(
            slave = task.slave_id,
            register = task.register_address,
            count = task.register_count,
            values = ?registers,
            "read modbus registers"
        );

        // 解析数据
        let raw_value = self.parse_registers(&registers, task.data_type)?;

        // 应用缩放和偏移
        let scaled_value = match (task.scale, task.offset) {
            (Some(scale), Some(offset)) => raw_value * scale + offset,
            (Some(scale), None) => raw_value * scale,
            (None, Some(offset)) => raw_value + offset,
            (None, None) => raw_value,
        };

        Ok(scaled_value)
    }

    /// 解析寄存器数据为浮点值
    fn parse_registers(
        &self,
        registers: &[u16],
        data_type: ModbusDataType,
    ) -> Result<f64, ProtocolError> {
        if registers.is_empty() {
            return Err(ProtocolError::DataParse("empty registers".to_string()));
        }

        let value = match data_type {
            ModbusDataType::Int16 => registers[0] as i16 as f64,
            ModbusDataType::Uint16 => registers[0] as f64,
            ModbusDataType::Int32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::DataParse(
                        "need 2 registers for int32".to_string(),
                    ));
                }
                let high = registers[0] as u32;
                let low = registers[1] as u32;
                let val = (high << 16) | low;
                val as i32 as f64
            }
            ModbusDataType::Uint32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::DataParse(
                        "need 2 registers for uint32".to_string(),
                    ));
                }
                let high = registers[0] as u32;
                let low = registers[1] as u32;
                ((high << 16) | low) as f64
            }
            ModbusDataType::Float32 => {
                if registers.len() < 2 {
                    return Err(ProtocolError::DataParse(
                        "need 2 registers for float32".to_string(),
                    ));
                }
                let high = registers[0] as u32;
                let low = registers[1] as u32;
                let bits = (high << 16) | low;
                f32::from_bits(bits) as f64
            }
            ModbusDataType::Float64 => {
                if registers.len() < 4 {
                    return Err(ProtocolError::DataParse(
                        "need 4 registers for float64".to_string(),
                    ));
                }
                let r0 = registers[0] as u64;
                let r1 = registers[1] as u64;
                let r2 = registers[2] as u64;
                let r3 = registers[3] as u64;
                let bits = (r0 << 48) | (r1 << 32) | (r2 << 16) | r3;
                f64::from_bits(bits)
            }
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{"host": "192.168.1.100", "port": 502, "poll_interval_ms": 1000}"#;
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
            read_timeout_ms: 3000,
        });

        // 正数
        let registers = [100u16];
        let value = source
            .parse_registers(&registers, ModbusDataType::Int16)
            .unwrap();
        assert_eq!(value, 100.0);

        // 负数
        let registers = [(-100i16) as u16];
        let value = source
            .parse_registers(&registers, ModbusDataType::Int16)
            .unwrap();
        assert_eq!(value, -100.0);
    }

    #[test]
    fn test_parse_device_address() {
        let json = r#"{"slave_id": 1}"#;
        let addr: ModbusDeviceAddress = serde_json::from_str(json).unwrap();
        assert_eq!(addr.slave_id, 1);
    }

    #[test]
    fn test_parse_point_detail() {
        let json = r#"{"function_code": 3, "register_address": 100, "register_count": 1, "data_type": "int16"}"#;
        let detail: ModbusPointDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.function_code, 3);
        assert_eq!(detail.register_address, 100);
        assert_eq!(detail.register_count, 1);
        assert_eq!(detail.data_type, ModbusDataType::Int16);
    }
}
