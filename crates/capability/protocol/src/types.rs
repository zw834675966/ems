//! 协议相关类型定义

use serde::{Deserialize, Serialize};

/// 协议数据事件
///
/// 从协议层采集到的原始数据，将转换为 domain::RawEvent
#[derive(Debug, Clone)]
pub struct ProtocolEvent {
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
    /// 数据值（已解析）
    pub value: f64,
    /// 接收时间戳（毫秒）
    pub received_at_ms: i64,
}

/// Modbus 寄存器数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModbusDataType {
    /// 16位有符号整数
    Int16,
    /// 16位无符号整数
    Uint16,
    /// 32位有符号整数（2个寄存器）
    Int32,
    /// 32位无符号整数（2个寄存器）
    Uint32,
    /// 32位浮点数（2个寄存器）
    Float32,
    /// 64位浮点数（4个寄存器）
    Float64,
}

impl Default for ModbusDataType {
    fn default() -> Self {
        Self::Int16
    }
}

/// Modbus 功能码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModbusFunctionCode {
    /// 读线圈状态 (0x01)
    ReadCoils = 1,
    /// 读离散输入 (0x02)
    ReadDiscreteInputs = 2,
    /// 读保持寄存器 (0x03)
    ReadHoldingRegisters = 3,
    /// 读输入寄存器 (0x04)
    ReadInputRegisters = 4,
}

impl Default for ModbusFunctionCode {
    fn default() -> Self {
        Self::ReadHoldingRegisters
    }
}

/// 点位协议详情（Modbus）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusPointDetail {
    /// 功能码
    #[serde(default)]
    pub function_code: u8,
    /// 寄存器起始地址
    pub register_address: u16,
    /// 寄存器数量
    #[serde(default = "default_register_count")]
    pub register_count: u16,
    /// 数据类型
    #[serde(default)]
    pub data_type: ModbusDataType,
    /// 字节序（big_endian / little_endian）
    #[serde(default = "default_byte_order")]
    pub byte_order: String,
}

fn default_register_count() -> u16 {
    1
}

fn default_byte_order() -> String {
    "big_endian".to_string()
}

/// 设备地址配置（Modbus）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusDeviceAddress {
    /// 从站 ID (1-247)
    pub slave_id: u8,
}

/// 获取当前时间戳（毫秒）
pub fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
