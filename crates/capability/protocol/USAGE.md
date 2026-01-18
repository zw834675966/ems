# protocol 使用方法

## 模块职责
提供多协议数据采集能力，支持 Modbus TCP、TCP Server、TCP Client 三种协议。

## 协议类型

### 1. Modbus TCP
通过 Modbus TCP 协议从设备寄存器读取数据。

**特性**：
- 支持标准 Modbus 功能码（0x01-0x04）
- 支持多种数据类型（Int16/Uint16/Int32/Uint32/Float32/Float64）
- 可配置字节序（大端/小端）
- 定时轮询采集

**配置示例**：

```json
// 网关配置 (gateway.protocol_config)
{
  "host": "192.168.1.100",
  "port": 502,
  "poll_interval_ms": 1000
}

// 设备地址配置 (device.address_config)
{
  "slave_id": 1
}

// 点位协议详情 (point_mapping.protocol_detail)
{
  "function_code": 3,
  "register_address": 100,
  "register_count": 1,
  "data_type": "int16",
  "byte_order": "big_endian"
}
```

**数据类型说明**：

| 数据类型 | 说明 | 寄存器数量 |
|---------|------|------------|
| `int16` | 16 位有符号整数 | 1 |
| `uint16` | 16 位无符号整数 | 1 |
| `int32` | 32 位有符号整数 | 2 |
| `uint32` | 32 位无符号整数 | 2 |
| `float32` | 32 位浮点数 | 2 |
| `float64` | 64 位浮点数 | 4 |

**功能码说明**：

| 功能码 | 名称 | 说明 |
|-------|------|------|
| `1` | ReadCoils | 读线圈状态 (0x01) |
| `2` | ReadDiscreteInputs | 读离散输入 (0x02) |
| `3` | ReadHoldingRegisters | 读保持寄存器 (0x03，默认) |
| `4` | ReadInputRegisters | 读输入寄存器 (0x04) |

### 2. TCP Server
监听 TCP 端口，接收设备主动上报的数据。

**特性**：
- 支持自定义帧分隔符
- 适合设备主动推送数据的场景

**配置示例**：

```json
// 网关配置 (gateway.protocol_config)
{
  "listen_port": 9000,
  "frame_delimiter": "\n"
}
```

### 3. TCP Client
主动连接到设备的 TCP 端口获取数据。

**特性**：
- 支持主动连接设备
- 支持自定义帧解析

**配置示例**：

```json
// 网关配置 (gateway.protocol_config)
{
  "host": "192.168.1.200",
  "port": 9001,
  "reconnect_interval_ms": 5000
}
```

## 数据流程

```
Gateway 配置 (protocol_type + protocol_config)
       │
       ▼
ProtocolManager
       │
       ├── ModbusTcpSource (Modbus TCP)
       ├── TcpServerSource (TCP Server)
       └── TcpClientSource (TCP Client)
       │
       ▼
RawEvent (转发给 ingest 模块)
       │
       ▼
Normalizer → Pipeline → Storage
```

## 架构设计

### 模块结构

```
protocol/
├── lib.rs           # 模块导出和文档
├── types.rs         # 协议类型定义
├── modbus_tcp.rs    # Modbus TCP 实现
├── tcp_server.rs    # TCP Server 实现
├── tcp_client.rs    # TCP Client 实现
└── error.rs        # 协议错误定义
```

### 核心类型

#### ProtocolEvent
从协议层采集到的原始数据事件：

```rust
pub struct ProtocolEvent {
    pub tenant_id: String,     // 租户 ID
    pub project_id: String,    // 项目 ID
    pub gateway_id: String,    // 网关 ID
    pub device_id: String,     // 设备 ID
    pub source_id: String,     // 点位映射 ID
    pub value: f64,           // 数据值（已解析）
    pub received_at_ms: i64,  // 接收时间戳（毫秒）
}
```

#### ModbusDataType
Modbus 寄存器数据类型：

```rust
pub enum ModbusDataType {
    Int16,    // 16 位有符号整数
    Uint16,   // 16 位无符号整数
    Int32,    // 32 位有符号整数（2 个寄存器）
    Uint32,   // 32 位无符号整数（2 个寄存器）
    Float32,  // 32 位浮点数（2 个寄存器）
    Float64,  // 64 位浮点数（4 个寄存器）
}
```

#### ModbusFunctionCode
Modbus 功能码：

```rust
pub enum ModbusFunctionCode {
    ReadCoils = 1,           // 读线圈状态 (0x01)
    ReadDiscreteInputs = 2,   // 读离散输入 (0x02)
    ReadHoldingRegisters = 3,  // 读保持寄存器 (0x03，默认)
    ReadInputRegisters = 4,    // 读输入寄存器 (0x04)
}
```

#### ModbusPointDetail
点位协议详情（Modbus）：

```rust
pub struct ModbusPointDetail {
    pub function_code: u8,          // 功能码（默认 3）
    pub register_address: u16,      // 寄存器起始地址
    pub register_count: u16,       // 寄存器数量（默认 1）
    pub data_type: ModbusDataType,  // 数据类型（默认 int16）
    pub byte_order: String,        // 字节序（默认 big_endian）
}
```

#### ModbusDeviceAddress
设备地址配置（Modbus）：

```rust
pub struct ModbusDeviceAddress {
    pub slave_id: u8,  // 从站 ID（1-247）
}
```

## 最小示例

### 使用 Modbus TCP

```rust
use ems_protocol::{ModbusTcpSource, ModbusTcpConfig, ProtocolEvent, RawEventHandler};
use domain::TenantContext;
use std::sync::Arc;

struct Handler;

#[async_trait::async_trait]
impl RawEventHandler for Handler {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ems_protocol::ProtocolError> {
        // 处理协议事件，转发给 ingest
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ModbusTcpConfig {
        host: "192.168.1.100".to_string(),
        port: 502,
        poll_interval_ms: 1000,
    };

    let source = ModbusTcpSource::new("gateway-1", config)?;
    source.run(Arc::new(Handler)).await?;
    Ok(())
}
```

### 使用 TCP Server

```rust
use ems_protocol::{TcpServerSource, TcpServerConfig, ProtocolEvent, RawEventHandler};
use std::sync::Arc;

struct Handler;

#[async_trait::async_trait]
impl RawEventHandler for Handler {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ems_protocol::ProtocolError> {
        // 处理接收到的数据
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TcpServerConfig {
        listen_port: 9000,
        frame_delimiter: "\n".to_string(),
    };

    let source = TcpServerSource::new("gateway-1", config)?;
    source.run(Arc::new(Handler)).await?;
    Ok(())
}
```

### 使用 TCP Client

```rust
use ems_protocol::{TcpClientSource, TcpClientConfig, ProtocolEvent, RawEventHandler};
use std::sync::Arc;

struct Handler;

#[async_trait::async_trait]
impl RawEventHandler for Handler {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ems_protocol::ProtocolError> {
        // 处理接收到的数据
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TcpClientConfig {
        host: "192.168.1.200".to_string(),
        port: 9001,
        reconnect_interval_ms: 5000,
    };

    let source = TcpClientSource::new("gateway-1", config)?;
    source.run(Arc::new(Handler)).await?;
    Ok(())
}
```

## 配置存储

### 数据库表

所有配置存储在以下数据库表中：

1. **gateways** 表：
   - `protocol_type`: 协议类型（`modbus_tcp` / `tcp_server` / `tcp_client`）
   - `protocol_config`: JSON 配置字符串

2. **devices** 表：
   - `address_config`: JSON 地址配置字符串

3. **point_mappings** 表：
   - `protocol_detail`: JSON 协议详情字符串

### 配置格式示例

#### 完整配置示例（网关 + 设备 + 点位）

```sql
-- 1. 创建 Modbus TCP 网关
INSERT INTO gateways (gateway_id, tenant_id, project_id, name, status, protocol_type, protocol_config)
VALUES (
    'gw-modbus-1',
    'tenant-1',
    'project-1',
    'Modbus Gateway',
    'online',
    'modbus_tcp',
    '{"host": "192.168.1.100", "port": 502, "poll_interval_ms": 1000}'
);

-- 2. 创建设备
INSERT INTO devices (device_id, tenant_id, project_id, gateway_id, name, address_config)
VALUES (
    'device-modbus-1',
    'tenant-1',
    'project-1',
    'gw-modbus-1',
    'Modbus Device',
    '{"slave_id": 1}'
);

-- 3. 创建点位
INSERT INTO points (point_id, tenant_id, project_id, device_id, key, data_type, unit)
VALUES (
    'point-modbus-1',
    'tenant-1',
    'project-1',
    'device-modbus-1',
    'temperature',
    'float',
    'C'
);

-- 4. 创建点位映射
INSERT INTO point_sources (source_id, tenant_id, project_id, point_id, source_type, protocol_detail)
VALUES (
    'source-modbus-1',
    'tenant-1',
    'project-1',
    'point-modbus-1',
    'modbus',
    '{
        "function_code": 3,
        "register_address": 100,
        "register_count": 1,
        "data_type": "float32",
        "byte_order": "big_endian"
    }'
);
```

## 错误处理

### ProtocolError

```rust
pub enum ProtocolError {
    // 连接错误
    ConnectionError(String),

    // 解析错误
    ParseError(String),

    // 配置错误
    ConfigError(String),

    // 超时错误
    TimeoutError,

    // IO 错误
    IoError(std::io::Error),
}
```

## 性能考虑

### Modbus TCP
- **轮询间隔**：默认 1000ms，可根据设备性能调整
- **批量读取**：支持连续读取多个寄存器，减少网络开销
- **超时设置**：建议设置合理的连接和读取超时

### TCP Server/Client
- **连接池**：建议实现连接池管理
- **重连机制**：支持自动重连，可配置重连间隔
- **缓冲区大小**：根据设备数据量配置合适的缓冲区大小

## 调试技巧

### 启用详细日志

```bash
export RUST_LOG=ems_protocol=debug
cargo run -p ems-api
```

### 测试协议连接

```bash
# 测试 Modbus TCP 连接
modpoll -m tcp -a 192.168.1.100 -p 502 -r 1 -c 3 -s 1

# 测试 TCP Server 连接
telnet 192.168.1.100 9000

# 测试 TCP Client 连接
telnet 192.168.1.200 9001
```

### 监控协议数据流

```rust
use tracing::{info, instrument};

#[async_trait::async_trait]
impl RawEventHandler for Handler {
    #[instrument(skip(self))]
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ems_protocol::ProtocolError> {
        info!(
            tenant_id = %event.tenant_id,
            project_id = %event.project_id,
            gateway_id = %event.gateway_id,
            device_id = %event.device_id,
            value = %event.value,
            "Received protocol event"
        );
        Ok(())
    }
}
```

## 安全考虑

### Modbus TCP
- **从站 ID**：确保从站 ID 在有效范围内（1-247）
- **网络隔离**：Modbus 设备应在隔离的子网中
- **认证**：Modbus 协议本身不支持认证，建议在网络层实施访问控制

### TCP Server/Client
- **TLS 加密**：生产环境建议使用 TLS 加密通信
- **认证机制**：实现设备认证机制（如设备证书）
- **速率限制**：限制单个设备的连接速率，防止滥用

## 依赖

```toml
[dependencies]
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
```

## 验证命令

```bash
# 检查代码
cargo check -p ems-protocol

# 运行测试
cargo test -p ems-protocol

# 格式化代码
cargo fmt -p ems-protocol

# 运行 clippy
cargo clippy -p ems-protocol
```
