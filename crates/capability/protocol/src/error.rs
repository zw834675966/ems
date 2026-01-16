//! 协议错误类型定义

/// 协议通信错误
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// 连接错误
    #[error("connection error: {0}")]
    Connection(String),

    /// IO 错误
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Modbus 错误
    #[error("modbus error: {0}")]
    Modbus(String),

    /// 配置解析错误
    #[error("config parse error: {0}")]
    ConfigParse(String),

    /// 数据解析错误
    #[error("data parse error: {0}")]
    DataParse(String),

    /// 超时错误
    #[error("timeout: {0}")]
    Timeout(String),

    /// 通道关闭
    #[error("channel closed")]
    ChannelClosed,
}
