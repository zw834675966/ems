//! TCP 客户端实现
//!
//! 主动连接设备获取数据。
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! let config = TcpClientConfig {
//!     host: "192.168.1.100".to_string(),
//!     port: 8080,
//!     poll_interval_ms: 1000,
//! };
//! let source = TcpClientSource::new(config);
//! source.run(handler).await?;
//! ```

use crate::error::ProtocolError;
use crate::modbus_tcp::ProtocolEventHandler;
use crate::types::{ProtocolEvent, now_epoch_ms};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// TCP 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpClientConfig {
    /// 服务器主机地址
    pub host: String,
    /// 服务器端口
    pub port: u16,
    /// 轮询间隔（毫秒）
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    /// 连接超时（毫秒）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// 请求命令（可选，用于主动请求数据）
    pub request_command: Option<String>,
    /// 帧分隔符
    #[serde(default = "default_delimiter")]
    pub frame_delimiter: String,
    /// 自动重连
    #[serde(default = "default_auto_reconnect")]
    pub auto_reconnect: bool,
    /// 重连间隔（毫秒）
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval_ms: u64,
}

fn default_poll_interval() -> u64 {
    1000
}

fn default_connect_timeout() -> u64 {
    5000
}

fn default_delimiter() -> String {
    "\n".to_string()
}

fn default_auto_reconnect() -> bool {
    true
}

fn default_reconnect_interval() -> u64 {
    5000
}

/// 轮询任务配置
#[derive(Debug, Clone)]
pub struct TcpPollTask {
    pub tenant_id: String,
    pub project_id: String,
    pub gateway_id: String,
    pub device_id: String,
    pub source_id: String,
    /// 请求命令（每个点位可能有不同的请求）
    pub request_command: Option<String>,
    /// 缩放系数
    pub scale: Option<f64>,
    /// 偏移量
    pub offset: Option<f64>,
}

/// TCP 客户端采集源
pub struct TcpClientSource {
    config: TcpClientConfig,
    tasks: Vec<TcpPollTask>,
}

impl TcpClientSource {
    /// 创建新的 TCP 客户端源
    pub fn new(config: TcpClientConfig) -> Self {
        Self {
            config,
            tasks: Vec::new(),
        }
    }

    /// 从 JSON 配置字符串解析
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let config: TcpClientConfig = serde_json::from_str(json)
            .map_err(|e| ProtocolError::ConfigParse(e.to_string()))?;
        Ok(Self::new(config))
    }

    /// 添加轮询任务
    pub fn add_task(&mut self, task: TcpPollTask) {
        self.tasks.push(task);
    }

    /// 运行采集循环
    pub async fn run(
        &self,
        handler: Arc<dyn ProtocolEventHandler>,
    ) -> Result<(), ProtocolError> {
        if self.tasks.is_empty() {
            warn!("no poll tasks configured for tcp client source");
            return Ok(());
        }

        let addr = format!("{}:{}", self.config.host, self.config.port);
        
        loop {
            info!("connecting to tcp server at {}", addr);

            match TcpStream::connect(&addr).await {
                Ok(stream) => {
                    info!("connected to tcp server at {}", addr);
                    
                    if let Err(e) = self.poll_loop(stream, &handler).await {
                        error!("poll loop error: {}", e);
                    }
                }
                Err(e) => {
                    error!("failed to connect to {}: {}", addr, e);
                }
            }

            if !self.config.auto_reconnect {
                break;
            }

            warn!(
                "reconnecting in {}ms...",
                self.config.reconnect_interval_ms
            );
            tokio::time::sleep(Duration::from_millis(self.config.reconnect_interval_ms)).await;
        }

        Ok(())
    }

    /// 轮询循环
    async fn poll_loop(
        &self,
        stream: TcpStream,
        handler: &Arc<dyn ProtocolEventHandler>,
    ) -> Result<(), ProtocolError> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut poll_interval = interval(Duration::from_millis(self.config.poll_interval_ms));

        loop {
            poll_interval.tick().await;

            for task in &self.tasks {
                // 发送请求命令
                let command = task
                    .request_command
                    .as_ref()
                    .or(self.config.request_command.as_ref());

                if let Some(cmd) = command {
                    let cmd_with_delimiter = if cmd.ends_with('\n') {
                        cmd.clone()
                    } else {
                        format!("{}\n", cmd)
                    };

                    writer
                        .write_all(cmd_with_delimiter.as_bytes())
                        .await
                        .map_err(|e| ProtocolError::Io(e))?;
                    writer.flush().await.map_err(|e| ProtocolError::Io(e))?;

                    debug!(command = %cmd, "sent request command");
                }

                // 读取响应
                let mut response = String::new();
                match tokio::time::timeout(
                    Duration::from_millis(self.config.connect_timeout_ms),
                    reader.read_line(&mut response),
                )
                .await
                {
                    Ok(Ok(0)) => {
                        return Err(ProtocolError::Connection("connection closed".to_string()));
                    }
                    Ok(Ok(_)) => {
                        let data = response.trim();
                        debug!(response = %data, "received response");

                        if let Some(value) = self.parse_response(data) {
                            // 应用缩放和偏移
                            let scaled_value = match (task.scale, task.offset) {
                                (Some(scale), Some(offset)) => value * scale + offset,
                                (Some(scale), None) => value * scale,
                                (None, Some(offset)) => value + offset,
                                (None, None) => value,
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
                                    "failed to handle event",
                                );
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        return Err(ProtocolError::Io(e));
                    }
                    Err(_) => {
                        warn!(source_id = %task.source_id, "read timeout");
                    }
                }
            }
        }
    }

    /// 解析响应数据
    fn parse_response(&self, data: &str) -> Option<f64> {
        // 格式1：纯数值
        if let Ok(value) = data.parse::<f64>() {
            return Some(value);
        }

        // 格式2：JSON {"value": 123.45}
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(value) = json.get("value").and_then(|v| v.as_f64()) {
                return Some(value);
            }
        }

        // 格式3：key=value
        for part in data.split(['=', ':', ',']) {
            if let Ok(value) = part.trim().parse::<f64>() {
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
        let json = r#"{
            "host": "192.168.1.100",
            "port": 8080,
            "poll_interval_ms": 2000,
            "request_command": "READ"
        }"#;
        let source = TcpClientSource::from_json(json).unwrap();
        assert_eq!(source.config.host, "192.168.1.100");
        assert_eq!(source.config.port, 8080);
        assert_eq!(source.config.poll_interval_ms, 2000);
        assert_eq!(source.config.request_command, Some("READ".to_string()));
    }

    #[test]
    fn test_parse_response() {
        let source = TcpClientSource::new(TcpClientConfig {
            host: "localhost".to_string(),
            port: 8080,
            poll_interval_ms: 1000,
            connect_timeout_ms: 5000,
            request_command: None,
            frame_delimiter: "\n".to_string(),
            auto_reconnect: true,
            reconnect_interval_ms: 5000,
        });

        // 纯数值
        assert_eq!(source.parse_response("123.45"), Some(123.45));

        // JSON 格式
        assert_eq!(source.parse_response(r#"{"value": 42.0}"#), Some(42.0));

        // key=value 格式
        assert_eq!(source.parse_response("temp=25.5"), Some(25.5));
    }
}
