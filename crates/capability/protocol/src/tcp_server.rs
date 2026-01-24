//! TCP 服务器实现
//!
//! 按 `docs/protocols/TCP.md` 的二进制协议定义：
//! - `SOF(0xAA55) + VER + FRAME_LEN + DEV_COUNT + DEV_BLOCK... + CRC16(Modbus)`
//! - `DEV_BLOCK = DEV_ID(u16) + DEV_TYPE(u8) + DEV_LEN(u16) + DEV_PAYLOAD`
//! - `DEV_PAYLOAD` 采用 TLV：`TAG(u8) + LEN(u8) + VALUE(N)`
//!
//! 解析后根据 (dev_id, tag) 映射到内部 `source_id`，并输出 `ProtocolEvent`。

use crate::error::ProtocolError;
use crate::modbus_tcp::ProtocolEventHandler;
use crate::types::{now_epoch_ms, Endian, ProtocolEvent, TcpValueType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// TCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpServerConfig {
    /// 监听端口
    #[serde(alias = "listen_port")]
    pub listen_port: u16,
    /// 最大连接数
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// 连接超时（秒）
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
}

fn default_max_connections() -> usize {
    100
}

fn default_connection_timeout() -> u64 {
    300
}

/// TLV 点位映射信息：以 (dev_id, tag) 映射到内部 source_id。
#[derive(Debug, Clone)]
pub struct TcpPointMapping {
    pub tenant_id: String,
    pub project_id: String,
    pub gateway_id: String,
    pub device_id: String,
    pub source_id: String,
    pub value_type: TcpValueType,
    pub endian: Endian,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
}

/// TCP 服务器采集源
pub struct TcpServerSource {
    config: TcpServerConfig,
    point_mappings: Arc<RwLock<HashMap<(u16, u8), TcpPointMapping>>>,
}

impl TcpServerSource {
    /// 创建新的 TCP 服务器源
    pub fn new(config: TcpServerConfig) -> Self {
        Self {
            config,
            point_mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 从 JSON 配置字符串解析
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let config: TcpServerConfig =
            serde_json::from_str(json).map_err(|e| ProtocolError::ConfigParse(e.to_string()))?;
        Ok(Self::new(config))
    }

    /// 注册 (dev_id, tag) 映射
    pub async fn register_point_mapping(&self, dev_id: u16, tag: u8, mapping: TcpPointMapping) {
        let mut mappings = self.point_mappings.write().await;
        mappings.insert((dev_id, tag), mapping);
    }

    /// 运行服务器
    pub async fn run(&self, handler: Arc<dyn ProtocolEventHandler>) -> Result<(), ProtocolError> {
        let addr = format!("0.0.0.0:{}", self.config.listen_port);
        let listener = TcpListener::bind(&addr).await?;

        info!("tcp server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("new connection from {}", peer_addr);

                    let handler = Arc::clone(&handler);
                    let mappings = Arc::clone(&self.point_mappings);

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            peer_addr.to_string(),
                            handler,
                            mappings,
                        )
                        .await
                        {
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

    async fn handle_connection(
        stream: TcpStream,
        peer_id: String,
        handler: Arc<dyn ProtocolEventHandler>,
        mappings: Arc<RwLock<HashMap<(u16, u8), TcpPointMapping>>>,
    ) -> Result<(), ProtocolError> {
        let mut stream = stream;
        let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
        let mut tmp = [0u8; 4096];

        loop {
            let n = stream.read(&mut tmp).await?;
            if n == 0 {
                info!("connection closed by {}", peer_id);
                break;
            }
            buf.extend_from_slice(&tmp[..n]);

            loop {
                let Some(frame) = try_take_frame(&mut buf)? else {
                    break;
                };
                debug!(
                    peer = %peer_id,
                    ver = frame.version,
                    dev_count = frame.dev_blocks.len(),
                    "tcp frame received"
                );

                for dev in frame.dev_blocks {
                    for tlv in dev.tlvs {
                        let mapping = {
                            let map = mappings.read().await;
                            map.get(&(dev.dev_id, tlv.tag)).cloned()
                        };
                        let Some(mapping) = mapping else {
                            continue;
                        };

                        let Some(raw) =
                            decode_tlv_value(&tlv.value, mapping.value_type, mapping.endian)
                        else {
                            continue;
                        };

                        let value = match (mapping.scale, mapping.offset) {
                            (Some(scale), Some(offset)) => raw * scale + offset,
                            (Some(scale), None) => raw * scale,
                            (None, Some(offset)) => raw + offset,
                            (None, None) => raw,
                        };

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
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct TcpFrame {
    version: u8,
    dev_blocks: Vec<DevBlock>,
}

#[derive(Debug)]
struct DevBlock {
    dev_id: u16,
    #[allow(dead_code)]
    dev_type: u8,
    tlvs: Vec<Tlv>,
}

#[derive(Debug)]
struct Tlv {
    tag: u8,
    value: Vec<u8>,
}

fn decode_tlv_value(value: &[u8], value_type: TcpValueType, endian: Endian) -> Option<f64> {
    match value_type {
        TcpValueType::UInt8 => value.first().copied().map(|v| v as f64),
        TcpValueType::UInt16 => {
            if value.len() != 2 {
                return None;
            }
            let bytes: [u8; 2] = [value[0], value[1]];
            let v = match endian {
                Endian::Big => u16::from_be_bytes(bytes),
                Endian::Little => u16::from_le_bytes(bytes),
            };
            Some(v as f64)
        }
        TcpValueType::UInt32 => {
            if value.len() != 4 {
                return None;
            }
            let bytes: [u8; 4] = [value[0], value[1], value[2], value[3]];
            let v = match endian {
                Endian::Big => u32::from_be_bytes(bytes),
                Endian::Little => u32::from_le_bytes(bytes),
            };
            Some(v as f64)
        }
        TcpValueType::Int32 => {
            if value.len() != 4 {
                return None;
            }
            let bytes: [u8; 4] = [value[0], value[1], value[2], value[3]];
            let v = match endian {
                Endian::Big => i32::from_be_bytes(bytes),
                Endian::Little => i32::from_le_bytes(bytes),
            };
            Some(v as f64)
        }
    }
}

fn try_take_frame(buf: &mut Vec<u8>) -> Result<Option<TcpFrame>, ProtocolError> {
    const SOF0: u8 = 0xAA;
    const SOF1: u8 = 0x55;

    let mut sof_pos = None;
    for i in 0..buf.len().saturating_sub(1) {
        if buf[i] == SOF0 && buf[i + 1] == SOF1 {
            sof_pos = Some(i);
            break;
        }
    }
    let Some(pos) = sof_pos else {
        if buf.len() > 1 {
            buf.drain(..buf.len() - 1);
        }
        return Ok(None);
    };
    if pos > 0 {
        buf.drain(..pos);
    }

    if buf.len() < 2 + 1 + 2 {
        return Ok(None);
    }
    let version = buf[2];
    let frame_len = u16::from_be_bytes([buf[3], buf[4]]) as usize;
    if frame_len < 4 {
        buf.drain(..2);
        return Ok(None);
    }
    let total_len = 2 + frame_len + 2;
    if buf.len() < total_len {
        return Ok(None);
    }

    let payload_start = 2;
    let payload_end = 2 + frame_len;
    let payload = &buf[payload_start..payload_end];
    let crc_bytes = [buf[payload_end], buf[payload_end + 1]];
    let crc_recv_be = u16::from_be_bytes(crc_bytes);
    let crc_recv_le = u16::from_le_bytes(crc_bytes);
    let crc_calc = crc16_modbus(payload);
    if crc_calc != crc_recv_be && crc_calc != crc_recv_le {
        warn!(
            crc_calc = crc_calc,
            crc_recv_be = crc_recv_be,
            crc_recv_le = crc_recv_le,
            "tcp frame crc mismatch"
        );
        buf.drain(..1);
        return Ok(None);
    }

    if payload.len() < 1 + 2 + 1 {
        buf.drain(..total_len);
        return Ok(None);
    }
    let dev_count = payload[3] as usize;
    let mut cursor = 4usize;
    let mut dev_blocks = Vec::with_capacity(dev_count);
    for _ in 0..dev_count {
        if cursor + 2 + 1 + 2 > payload.len() {
            buf.drain(..total_len);
            return Err(ProtocolError::DataParse(
                "tcp dev block truncated".to_string(),
            ));
        }
        let dev_id = u16::from_be_bytes([payload[cursor], payload[cursor + 1]]);
        cursor += 2;
        let dev_type = payload[cursor];
        cursor += 1;
        let dev_len = u16::from_be_bytes([payload[cursor], payload[cursor + 1]]) as usize;
        cursor += 2;
        if cursor + dev_len > payload.len() {
            buf.drain(..total_len);
            return Err(ProtocolError::DataParse(
                "tcp dev payload truncated".to_string(),
            ));
        }
        let dev_payload = &payload[cursor..cursor + dev_len];
        cursor += dev_len;

        let mut tlvs = Vec::new();
        let mut p = 0usize;
        while p + 2 <= dev_payload.len() {
            let tag = dev_payload[p];
            let len = dev_payload[p + 1] as usize;
            p += 2;
            if p + len > dev_payload.len() {
                break;
            }
            tlvs.push(Tlv {
                tag,
                value: dev_payload[p..p + len].to_vec(),
            });
            p += len;
        }

        dev_blocks.push(DevBlock {
            dev_id,
            dev_type,
            tlvs,
        });
    }

    buf.drain(..total_len);
    Ok(Some(TcpFrame {
        version,
        dev_blocks,
    }))
}

fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc >>= 1;
                crc ^= 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{"listenPort": 9000}"#;
        let source = TcpServerSource::from_json(json).unwrap();
        assert_eq!(source.config.listen_port, 9000);
    }

    #[test]
    fn test_crc16_modbus_known_vector() {
        // CRC16-Modbus("123456789") == 0x4B37
        assert_eq!(crc16_modbus(b"123456789"), 0x4B37);
    }

    #[test]
    fn test_take_frame_minimal() {
        // 构造一个只包含 1 个设备、1 个 TLV 的帧：
        // SOF AA55
        // VER 01
        // LEN 000D (payload bytes from VER..before CRC, include VER+LEN+DEV_COUNT+DEV_BLOCK)
        // DEV_COUNT 01
        // DEV_ID 0001
        // DEV_TYPE 01
        // DEV_LEN 0004
        // TLV: TAG 01 LEN 02 VALUE 0x0E 0xD6
        let payload: Vec<u8> = vec![
            0x01, // VER
            0x00, 0x0D, // LEN
            0x01, // DEV_COUNT
            0x00, 0x01, // DEV_ID
            0x01, // DEV_TYPE
            0x00, 0x04, // DEV_LEN
            0x01, 0x02, 0x0E, 0xD6, // TLV
        ];
        let crc = crc16_modbus(&payload);
        let mut frame = vec![0xAA, 0x55];
        frame.extend_from_slice(&payload);
        frame.extend_from_slice(&crc.to_be_bytes());

        let mut buf = frame.clone();
        let parsed = try_take_frame(&mut buf).unwrap().unwrap();
        assert_eq!(parsed.version, 0x01);
        assert_eq!(parsed.dev_blocks.len(), 1);
        assert_eq!(parsed.dev_blocks[0].dev_id, 1);
        assert_eq!(parsed.dev_blocks[0].dev_type, 1);
        assert_eq!(parsed.dev_blocks[0].tlvs.len(), 1);
        assert_eq!(parsed.dev_blocks[0].tlvs[0].tag, 1);
        assert_eq!(parsed.dev_blocks[0].tlvs[0].value, vec![0x0E, 0xD6]);
        assert!(buf.is_empty());
    }
}
