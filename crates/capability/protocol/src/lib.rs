//! # 协议通信能力模块
//!
//! 提供多协议数据采集能力，支持：
//! - **Modbus TCP**：读取从设备寄存器数据
//! - **TCP Server**：监听端口接收设备上报数据
//! - **TCP Client**：主动连接设备获取数据
//!
//! ## 架构设计
//!
//! ```text
//! Gateway 配置 (protocol_type + protocol_config)
//!       │
//!       ▼
//! ProtocolManager
//!       │
//!       ├── ModbusTcpSource
//!       ├── TcpServerSource
//!       └── TcpClientSource
//!       │
//!       ▼
//! RawEventHandler (与 ingest 共用)
//!       │
//!       ▼
//! Pipeline → Storage
//! ```
//!
//! ## 配置格式
//!
//! ### Modbus TCP
//! ```json
//! // gateway.protocol_config
//! { "host": "192.168.1.100", "port": 502, "poll_interval_ms": 1000 }
//!
//! // device.address_config
//! { "slave_id": 1 }
//!
//! // point_mapping.protocol_detail
//! { "function_code": 3, "register_address": 100, "register_count": 1, "data_type": "int16" }
//! ```
//!
//! ### TCP Server
//! ```json
//! // gateway.protocol_config
//! { "listen_port": 9000, "frame_delimiter": "\n" }
//! ```

mod error;
mod modbus_tcp;
mod tcp_client;
mod tcp_server;
mod types;

pub use error::ProtocolError;
pub use modbus_tcp::{ModbusTcpConfig, ModbusTcpSource};
pub use tcp_client::{TcpClientConfig, TcpClientSource};
pub use tcp_server::{TcpServerConfig, TcpServerSource};
pub use types::*;
