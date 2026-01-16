//! TCP 服务器实现
//!
//! 监听 TCP 端口，接收设备上报的数据。
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! let config = TcpServerConfig {
//!     listen_port: 9000,
//!     frame_delimiter: "\n".to_string(),
//! };
//! let source = TcpServerSource::new(config);
//! source.run(handler).await?;
//! ```

use crate::error::ProtocolError;
use crate::modbus_tcp::ProtocolEventHandler;
use crate::types::{ProtocolEvent, now_epoch_ms};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// TCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpServerConfig {
    /// 监听端口
    pub listen_port: u16,
    /// 帧分隔符（如 "\n" 或固定长度）
    #[serde(default = "default_delimiter")]
    pub frame_delimiter: String,
    /// 最大连接数
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// 连接超时（秒）
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
}

fn default_delimiter() -> String {
    "\n".to_string()
}

fn default_max_connections() -> usize {
    100
}

fn default_connection_timeout() -> u64 {
    300
}

/// 设备映射信息
#[derive(Debug, Clone)]
pub struct DeviceMapping {
    pub tenant_id: String,
    pub project_id: String,
    pub gateway_id: String,
    pub device_id: String,
    pub source_id: String,
}

/// TCP 服务器采集源
pub struct TcpServerSource {
    config: TcpServerConfig,
    /// 设备映射表：通过某种标识（如设备 ID 或连接 IP）映射到设备信息
    device_mappings: Arc<RwLock<HashMap<String, DeviceMapping>>>,
}

impl TcpServerSource {
    /// 创建新的 TCP 服务器源
    pub fn new(config: TcpServerConfig) -> Self {
        Self {
            config,
            device_mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 从 JSON 配置字符串解析
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let config: TcpServerConfig = serde_json::from_str(json)
            .map_err(|e| ProtocolError::ConfigParse(e.to_string()))?;
        Ok(Self::new(config))
    }

    /// 注册设备映射
    pub async fn register_device(&self, identifier: String, mapping: DeviceMapping) {
        let mut mappings = self.device_mappings.write().await;
        mappings.insert(identifier, mapping);
    }

    /// 运行服务器
    pub async fn run(
        &self,
        handler: Arc<dyn ProtocolEventHandler>,
    ) -> Result<(), ProtocolError> {
        let addr = format!("0.0.0.0:{}", self.config.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("tcp server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("new connection from {}", peer_addr);
                    
                    let handler = Arc::clone(&handler);
                    let mappings = Arc::clone(&self.device_mappings);
                    let delimiter = self.config.frame_delimiter.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            peer_addr.to_string(),
                            handler,
                            mappings,
                            delimiter,
                        ).await {
                            warn!("connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("failed to accept connection: {}", e);
                }
            }
        }
    }

    /// 处理单个连接
    async fn handle_connection(
        stream: TcpStream,
        peer_id: String,
        handler: Arc<dyn ProtocolEventHandler>,
        mappings: Arc<RwLock<HashMap<String, DeviceMapping>>>,
        delimiter: String,
    ) -> Result<(), ProtocolError> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        loop {
            line.clear();
            
            let bytes_read = if delimiter == "\n" {
                reader.read_line(&mut line).await?
            } else {
                // 固定长度或其他分隔符处理
                let mut buf = vec![0u8; 1024];
                let n = reader.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                line = String::from_utf8_lossy(&buf[..n]).to_string();
                n
            };

            if bytes_read == 0 {
                info!("connection closed by {}", peer_id);
                break;
            }

            let data = line.trim();
            if data.is_empty() {
                continue;
            }

            debug!(peer = %peer_id, data = %data, "received tcp data");

            // 尝试解析数据并查找设备映射
            // 格式示例：device_id:value 或 JSON 格式
            if let Some(mapping) = Self::find_mapping(&mappings, &peer_id, data).await {
                if let Some(value) = Self::parse_value(data) {
                    let event = ProtocolEvent {
                        tenant_id: mapping.tenant_id,
                        project_id: mapping.project_id,
                        gateway_id: mapping.gateway_id,
                        device_id: mapping.device_id,
                        source_id: mapping.source_id,
                        value,
                        received_at_ms: now_epoch_ms(),
                    };

                    if let Err(e) = handler.handle(event).await {
                        warn!(peer = %peer_id, error = %e, "failed to handle event");
                    }
                }
            } else {
                debug!(peer = %peer_id, "no mapping found for data");
            }
        }

        Ok(())
    }

    /// 查找设备映射
    async fn find_mapping(
        mappings: &Arc<RwLock<HashMap<String, DeviceMapping>>>,
        peer_id: &str,
        data: &str,
    ) -> Option<DeviceMapping> {
        let mappings = mappings.read().await;
        
        // 首先尝试通过 peer_id（IP:port）查找
        if let Some(mapping) = mappings.get(peer_id) {
            return Some(mapping.clone());
        }

        // 尝试从数据中提取设备标识
        // 格式：device_id:value
        if let Some(idx) = data.find(':') {
            let device_id = &data[..idx];
            if let Some(mapping) = mappings.get(device_id) {
                return Some(mapping.clone());
            }
        }

        None
    }

    /// 解析数据值
    fn parse_value(data: &str) -> Option<f64> {
        // 格式1：纯数值
        if let Ok(value) = data.parse::<f64>() {
            return Some(value);
        }

        // 格式2：device_id:value
        if let Some(idx) = data.find(':') {
            if let Ok(value) = data[idx + 1..].trim().parse::<f64>() {
                return Some(value);
            }
        }

        // 格式3：JSON {"value": 123.45}
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(value) = json.get("value").and_then(|v| v.as_f64()) {
                return Some(value);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{"listen_port": 9000, "frame_delimiter": "\n"}"#;
        let source = TcpServerSource::from_json(json).unwrap();
        assert_eq!(source.config.listen_port, 9000);
        assert_eq!(source.config.frame_delimiter, "\n");
    }

    #[test]
    fn test_parse_value() {
        // 纯数值
        assert_eq!(TcpServerSource::parse_value("123.45"), Some(123.45));
        
        // device_id:value 格式
        assert_eq!(TcpServerSource::parse_value("dev1:99.5"), Some(99.5));
        
        // JSON 格式
        assert_eq!(TcpServerSource::parse_value(r#"{"value": 42.0}"#), Some(42.0));
        
        // 无效格式
        assert_eq!(TcpServerSource::parse_value("invalid"), None);
    }
}
