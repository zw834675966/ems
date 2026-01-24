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

/// Modbus 32-bit 字序/字节序组合（常见四种）。
///
/// 以 2 个寄存器（4 字节）为例，原始顺序为 `A B C D`：
/// - `ABCD`：不变（高字在前，高字节在前）
/// - `CDAB`：交换 16-bit word
/// - `BADC`：交换每个 word 内字节
/// - `DCBA`：全部反转
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModbusWordOrder {
    ABCD,
    CDAB,
    BADC,
    DCBA,
}

/// Modbus 寄存器数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModbusDataType {
    /// 线圈/离散输入（bool）
    Bool,
    /// 16位有符号整数
    #[default]
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

/// Modbus 功能码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ModbusFunctionCode {
    /// 读线圈状态 (0x01)
    ReadCoils = 1,
    /// 读离散输入 (0x02)
    ReadDiscreteInputs = 2,
    /// 读保持寄存器 (0x03)
    #[default]
    ReadHoldingRegisters = 3,
    /// 读输入寄存器 (0x04)
    ReadInputRegisters = 4,
}

/// 字节序
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Endian {
    #[default]
    #[serde(
        alias = "big",
        alias = "big_endian",
        alias = "bigEndian",
        alias = "BE",
        alias = "be"
    )]
    Big,
    #[serde(
        alias = "little",
        alias = "little_endian",
        alias = "littleEndian",
        alias = "LE",
        alias = "le"
    )]
    Little,
}

/// 点位协议详情（Modbus）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModbusPointDetail {
    /// 功能码
    #[serde(default, alias = "function_code")]
    pub function_code: u8,
    /// 寄存器起始地址
    #[serde(alias = "address")]
    pub register_address: u16,
    /// 寄存器数量
    #[serde(
        default = "default_register_count",
        alias = "quantity",
        alias = "register_count"
    )]
    pub register_count: u16,
    /// 数据类型
    #[serde(default, alias = "data_type")]
    pub data_type: ModbusDataType,
    /// 字节序（big_endian / little_endian）
    #[serde(default, alias = "byte_order", alias = "byteOrder")]
    pub endian: Endian,
    /// 32-bit 字序/字节序组合（Int32/Uint32/Float32 必填）
    pub word_order: Option<ModbusWordOrder>,
}

fn default_register_count() -> u16 {
    1
}

/// 设备地址配置（Modbus）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModbusDeviceAddress {
    /// 从站 ID (1-247)
    #[serde(alias = "slave_id")]
    pub unit_id: u8,
}

/// TCP 协议点位详情（TLV）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpPointDetail {
    pub tag: u8,
    #[serde(default)]
    pub endian: Endian,
    pub value_type: TcpValueType,
}

/// TCP TLV 值类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TcpValueType {
    #[serde(alias = "u8", alias = "uint8", alias = "UInt8", alias = "UINT8")]
    UInt8,
    #[serde(alias = "u16", alias = "uint16", alias = "UInt16", alias = "UINT16")]
    UInt16,
    #[serde(alias = "u32", alias = "uint32", alias = "UInt32", alias = "UINT32")]
    UInt32,
    #[serde(alias = "i32", alias = "Int32", alias = "INT32")]
    Int32,
}

/// TCP 设备地址配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpDeviceAddress {
    pub dev_id: u16,
    pub dev_type: Option<u8>,
}

/// 获取当前时间戳（毫秒）
pub fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
