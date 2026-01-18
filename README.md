# EMS 项目说明

> 能源管理系统（Energy Management System） - 基于构建的高性能多租户 SaaS 平台

## 快速开始

### 前置要求

- **Rust**: 1.70+ (推荐使用 rustup 安装)
- **Node.js**: 20.19+ 或 22.13+
- **pnpm**: 9+
- **PostgreSQL**: 16+ (可选 TimescaleDB 扩展)
- **Redis**: 7+
- **Mosquitto**: 2+ (MQTT Broker，可选)

### 一键启动（开发环境）

```bash
# 1. 初始化数据库和依赖
scripts/db-init.sh
scripts/health-check.sh

# 2. 启动后端 API
cargo run -p ems-api

# 3. （可选）前后端联调
EMS_WEB_ADMIN=on cargo run -p ems-api
```

### 快速验证

```bash
# 获取访问令牌
ACCESS_TOKEN=$(curl -sS -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["accessToken"])')

# 查询项目列表
curl -sS http://127.0.0.1:8080/projects \
  -H "Authorization: Bearer $ACCESS_TOKEN"

# 健康检查
curl -sS http://127.0.0.1:8080/livez
curl -sS http://127.0.0.1:8080/readyz
```

---

## 项目概览

### 技术架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         EMS 系统架构                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────────────┐ │
│  │  HTTP API    │────▶│   Axum      │────▶│   Request Handlers   │ │
│  │  (ems-api)   │     │   Router    │     │                      │ │
│  └──────────────┘     └──────────────┘     └──────────────────────┘ │
│         │                                        │                │
│         │                                        ▼                │
│         │                              ┌──────────────────────┐    │
│         │                              │  业务服务层          │    │
│         │                              │  - AuthService      │    │
│         │                              │  - CommandService  │    │
│         │                              │  - RbacService     │    │
│         │                              └──────────────────────┘    │
│         │                                        │                │
│         ▼                                        ▼                │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    存储层 (Storage Layer)                   │  │
│  ├──────────────────────────────────────────────────────────────┤  │
│  │  PostgreSQL+Timescale   │  Redis      │  MQTT Broker      │  │
│  │  - 元数据              │  - 实时数据 │  - 数据采集       │  │
│  │  - 时序数据            │  - 在线状态 │  - 控制指令       │  │
│  │  - 控制与审计         │             │  - 回执监听       │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 仓库结构

```
ems/
├── apps/ems-api/          # HTTP API 服务器（唯一运行时二进制）
│   ├── src/
│   │   ├── main.rs        # 主入口
│   │   ├── routes.rs      # 路由定义
│   │   ├── handlers/     # 请求处理器
│   │   ├── middleware/    # 中间件（认证、追踪）
│   │   └── ingest.rs     # 数据采集管道
│   └── Cargo.toml
├── crates/core/           # 核心领域层
│   ├── domain/            # 领域模型与业务规则
│   │   ├── lib.rs        # TenantContext
│   │   ├── data.rs       # RawEvent, PointValue
│   │   └── permissions.rs # 20 个权限常量
│   └── api-contract/     # API 契约
│       ├── lib.rs        # DTOs, ErrorCode, ApiResponse
│       └── error_codes/ # 5 个错误码
├── crates/capability/    # 能力模块
│   ├── auth/             # JWT 认证与 RBAC
│   ├── config/           # 配置管理
│   ├── storage/          # 存储抽象（PostgreSQL + Redis）
│   ├── ingest/           # 数据接入（MQTT）
│   ├── normalize/        # 数据标准化
│   ├── pipeline/         # 数据流水线
│   ├── control/          # 设备控制
│   ├── protocol/         # 协议支持（Modbus TCP）
│   └── telemetry/       # 可观测性（日志、指标）
├── migrations/           # 数据库迁移脚本
│   ├── 001_init.sql
│   ├── 002_seed.sql
│   ├── 003_assets.sql
│   ├── 004_timescale.sql
│   ├── 005_control.sql
│   ├── 006_rbac.sql
│   ├── 007_auth_sessions.sql
│   ├── 008_building_hierarchy_protocol.sql
│   ├── 009_add_point_protocol_detail.sql
│   └── 010_collection_strategies.sql
├── scripts/             # 工具脚本
│   ├── db-init.sh              # 数据库初始化
│   ├── health-check.sh          # 健康检查
│   ├── mqtt-simulate.sh        # MQTT 数据模拟
│   ├── control-receipt-simulate.sh  # 控制回执模拟
│   ├── device-emulator.sh      # 设备模拟器
│   ├── mvp-acceptance.sh      # MVP 验收脚本
│   ├── stability-check.sh      # 稳定性测试
│   └── rbac-acceptance.sh    # RBAC 回归测试
├── web/admin/            # 管理前端（pure-admin-thin）
│   └── ...
├── Cargo.toml           # Workspace 配置
├── 00_项目总览.md       # 项目文档
├── 03_系统架构与模块边界.md
└── 06_开发环境与工作流_WSL2.md
```

---

## 环境配置

### 必需环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `EMS_DATABASE_URL` | PostgreSQL 连接字符串 | `postgresql://ems:admin123@localhost:5432/ems` |
| `EMS_JWT_SECRET` | JWT 签名密钥 | `dev`（仅开发环境） |
| `EMS_JWT_ACCESS_TTL_SECONDS` | 访问令牌有效期（秒） | `3600` |
| `EMS_JWT_REFRESH_TTL_SECONDS` | 刷新令牌有效期（秒） | `7200` |

### 可选环境变量

#### HTTP 服务器
| 变量名 | 默认值 | 说明 |
|--------|---------|------|
| `EMS_HTTP_ADDR` | `127.0.0.1:8080` | HTTP 监听地址 |

#### Redis 配置
| 变量名 | 默认值 | 说明 |
|--------|---------|------|
| `EMS_REDIS_URL` | `redis://default:admin123@localhost:6379` | Redis 连接字符串 |
| `EMS_REDIS_LAST_VALUE_TTL_SECONDS` | 无（不设置 TTL） | 实时数据缓存过期秒数 |
| `EMS_REDIS_ONLINE_TTL_SECONDS` | `60` | 在线状态缓存过期秒数 |

#### MQTT 配置
| 变量名 | 默认值 | 说明 |
|--------|---------|------|
| `EMS_MQTT_HOST` | `127.0.0.1` | MQTT Broker 主机 |
| `EMS_MQTT_PORT` | `1883` | MQTT Broker 端口 |
| `EMS_MQTT_USERNAME` | - | MQTT 用户名 |
| `EMS_MQTT_PASSWORD` | - | MQTT 密码 |
| `EMS_MQTT_TOPIC_PREFIX` | `ems` | MQTT 主题根前缀 |
| `EMS_MQTT_DATA_TOPIC_PREFIX` | `{prefix}/data` | 数据采集主题前缀 |
| `EMS_MQTT_DATA_TOPIC_HAS_SOURCE_ID` | `false` | 采集主题是否包含 source_id |
| `EMS_MQTT_COMMAND_TOPIC_PREFIX` | `{prefix}/commands` | 控制指令主题前缀 |
| `EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET` | `false` | 命令主题是否包含 target |
| `EMS_MQTT_RECEIPT_TOPIC_PREFIX` | `{prefix}/receipts` | 回执主题前缀 |
| `EMS_MQTT_COMMAND_QOS` | `1` | 控制指令 QoS 级别 |
| `EMS_MQTT_RECEIPT_QOS` | `1` | 回执订阅 QoS 级别 |

#### 控制配置
| 变量名 | 默认值 | 说明 |
|--------|---------|------|
| `EMS_CONTROL_DISPATCH_MAX_RETRIES` | `2` | 控制指令最大重试次数 |
| `EMS_CONTROL_DISPATCH_BACKOFF_MS` | `200` | 重试退避时间（毫秒） |
| `EMS_CONTROL_RECEIPT_TIMEOUT_SECONDS` | `30` | 等待设备回执超时秒数 |

#### 功能开关
| 变量名 | 默认值 | 说明 |
|--------|---------|------|
| `EMS_INGEST` | `off` | 是否启用 MQTT 数据采集（`1`/`true`/`on` 启用） |
| `EMS_CONTROL` | `off` | 是否启用控制链路（`1`/`true`/`on` 启用） |
| `EMS_WEB_ADMIN` | `off` | 前端启动模式（`on`=前后端，`only`=仅前端，`off`=仅后端） |
| `EMS_REQUIRE_TIMESCALE` | `off` | 是否要求 TimescaleDB 扩展 |

---

## API 接口

### 健康检查与监控（无需认证）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 存活探针（等价于 `/livez`） |
| GET | `/livez` | 存活探针，返回 `{"ok": true}` |
| GET | `/readyz` | 就绪探针，检查 PostgreSQL 连接 |
| GET | `/metrics` | 指标快照（需要 Bearer token + `SYSTEM.METRICS.READ` 权限） |

**注**：`/metrics` 接口包含以下指标：
- **采集指标**：raw_events, normalized_values, write_success, write_failure
- **丢弃指标**：dropped_duplicate, dropped_invalid, dropped_stale, dropped_unmapped
- **延迟指标**：write_latency_ms_total/count, end_to_end_latency_ms_total/count
- **控制指标**：commands_issued, command_dispatch_success/failure, command_issue_latency_ms_total/count, receipts_processed

### 认证接口

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/login` | 用户登录，返回 access/refresh token |
| POST | `/refresh-token` | 使用 refresh token 刷新 access token |
| GET | `/get-async-routes` | 获取前端动态路由（需要 Bearer token） |

### RBAC 管理（需要权限）

| 方法 | 路径 | 权限 |
|------|------|------|
| GET | `/rbac/users` | `RBAC.USER_READ` |
| POST | `/rbac/users` | `RBAC.USER_WRITE` |
| PUT | `/rbac/users/:user_id` | `RBAC.USER_WRITE` |
| PUT | `/rbac/users/:user_id/roles` | `RBAC.USER_WRITE` |
| GET | `/rbac/roles` | `RBAC.ROLE_READ` |
| POST | `/rbac/roles` | `RBAC.ROLE_WRITE` |
| DELETE | `/rbac/roles/:role_code` | `RBAC.ROLE_WRITE` |
| PUT | `/rbac/roles/:role_code/permissions` | `RBAC.ROLE_WRITE` |
| GET | `/rbac/permissions` | `RBAC.ROLE_READ` |

### 资源管理

#### 项目（需要 `PROJECT.READ` / `PROJECT.WRITE` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects` | 列出当前租户的所有项目 |
| POST | `/projects` | 创建新项目 |
| GET | `/projects/:project_id` | 获取项目详情 |
| PUT | `/projects/:project_id` | 更新项目信息 |
| DELETE | `/projects/:project_id` | 删除项目（级联删除所有资源） |

#### 网关（需要 `ASSET.GATEWAY.READ` / `ASSET.GATEWAY.WRITE` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/gateways` | 列出项目的网关 |
| POST | `/projects/:project_id/gateways` | 创建网关 |
| GET | `/projects/:project_id/gateways/:gateway_id` | 获取网关详情 |
| PUT | `/projects/:project_id/gateways/:gateway_id` | 更新网关信息 |
| DELETE | `/projects/:project_id/gateways/:gateway_id` | 删除网关 |

**网关协议类型**：`mqtt`, `modbus_tcp`, `tcp_server`, `tcp_client`

#### 设备（需要 `ASSET.DEVICE_READ` / `ASSET.DEVICE_WRITE` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/devices` | 列出项目的设备 |
| POST | `/projects/:project_id/devices` | 创建设备 |
| GET | `/projects/:project_id/devices/:device_id` | 获取设备详情 |
| PUT | `/projects/:project_id/devices/:device_id` | 更新设备信息 |
| DELETE | `/projects/:project_id/devices/:device_id` | 删除设备（级联删除点位和映射） |

#### 点位（需要 `ASSET.POINT.READ` / `ASSET.POINT.WRITE` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/points` | 列出项目的点位 |
| POST | `/projects/:project_id/points` | 创建点位 |
| GET | `/projects/:project_id/points/:point_id` | 获取点位详情 |
| PUT | `/projects/:project_id/points/:point_id` | 更新点位信息 |
| DELETE | `/projects/:project_id/points/:point_id` | 删除点位（级联删除映射） |
| POST | `/projects/:project_id/points/:point_id/test` | 测试点位读取（`DATA.REALTIME_READ`） |

**点位数据类型**：`i64`, `f64`, `bool`, `string`

#### 点位映射（需要 `ASSET.POINT.READ` / `ASSET.POINT.WRITE` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/point-mappings` | 列出项目的点位映射 |
| POST | `/projects/:project_id/point-mappings` | 创建点位映射 |
| GET | `/projects/:project_id/point-mappings/:source_id` | 获取映射详情 |
| PUT | `/projects/:project_id/point-mappings/:source_id` | 更新映射信息 |
| DELETE | `/projects/:project_id/point-mappings/:source_id` | 删除映射 |

**映射源类型**：`mqtt`, `modbus`, `tcp`

### 数据查询

#### 实时数据（需要 `DATA.REALTIME_READ` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/realtime` | 查询实时点位值（缓存自 Redis） |

**查询参数**：
- `pointId`（可选）：指定点位 ID

#### 历史数据（需要 `DATA.MEASUREMENTS_READ` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/measurements` | 查询历史测量数据 |

**查询参数**：
- `pointId`（必填）：点位 ID
- `from`：起始时间戳（毫秒）
- `to`：结束时间戳（毫秒）
- `limit`：最大返回条数（1-5000，默认 1000）
- `cursorTsMs`：游标分页时间戳
- `order`：排序（`asc`/`desc`，默认 `asc`）
- `bucketMs`：聚合桶大小（毫秒）
- `agg`：聚合函数（`avg`/`min`/`max`/`sum`/`count`，默认 `avg`）

### 设备控制（需要 `CONTROL.COMMAND.READ` / `CONTROL.COMMAND.ISSUE` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/commands` | 列出控制指令 |
| POST | `/projects/:project_id/commands` | 发起控制指令 |
| GET | `/projects/:project_id/commands/:command_id/receipts` | 查询指令回执 |

**创建指令请求体**：
```json
{
  "target": "device:device-1",
  "payload": {"action": "set", "value": 42}
}
```

### 审计日志（需要 `CONTROL.COMMAND.READ` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/audit` | 查询审计日志 |

**查询参数**：
- `from`：起始时间戳（毫秒）
- `to`：结束时间戳（毫秒）
- `limit`：最大返回条数（默认 100）

### 采集策略（需要 `DATA.REALTIME_READ` / `ASSET.POINT.WRITE` 权限）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/projects/:project_id/collection-strategies` | 列出采集策略 |
| POST | `/projects/:project_id/collection-strategies` | 批量创建/更新策略 |
| DELETE | `/projects/:project_id/collection-strategies/:strategy_id` | 删除策略 |
| POST | `/projects/:project_id/collection-strategies/batch-enabled` | 批量更新策略启用状态 |

### 批量操作

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/points/batch` | 批量查询多个项目的点位 |

**查询参数**：
- `projectIds`：项目 ID 列表（逗号分隔或 JSON 数组）

---

## MQTT 协议

### 数据采集

#### 主题格式

**标准格式**（`EMS_MQTT_DATA_TOPIC_HAS_SOURCE_ID=off`）：
```
{EMS_MQTT_DATA_TOPIC_PREFIX}/{tenant_id}/{project_id}/{address}
```

**包含 source_id 格式**（`EMS_MQTT_DATA_TOPIC_HAS_SOURCE_ID=on`）：
```
{EMS_MQTT_DATA_TOPIC_PREFIX}/{tenant_id}/{project_id}/{source_id}/{address}
```

#### 示例

```bash
# 发布数据到 demo/topic
mosquitto_pub -h 127.0.0.1 -p 1883 -u ems -P admin123 \
  -t "ems/data/tenant-1/project-1/demo/topic" \
  -m "25.5"

# 使用脚本模拟
EMS_TENANT_ID=tenant-1 EMS_PROJECT_ID=project-1 EMS_POINT_ADDRESS=demo/topic \
EMS_MQTT_USERNAME=ems EMS_MQTT_PASSWORD=admin123 \
scripts/mqtt-simulate.sh
```

### 设备控制

#### 命令主题

**标准格式**（`EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET=off`）：
```
{EMS_MQTT_COMMAND_TOPIC_PREFIX}/{tenant_id}/{project_id}/{command_id}
```

**包含 target 格式**（`EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET=on`）：
```
{EMS_MQTT_COMMAND_TOPIC_PREFIX}/{tenant_id}/{project_id}/{target}/{command_id}
```

#### 命令 Payload（服务端发布）
```json
{
  "action": "set",
  "value": 42
}
```

#### 回执主题

```
{EMS_MQTT_RECEIPT_TOPIC_PREFIX}/{tenant_id}/{project_id}/{command_id}
```

#### 回执 Payload（设备侧发布）
```json
{
  "status": "success",
  "message": "applied",
  "tsMs": 1700000000000
}
```

**status 建议**：`accepted` / `success` / `failed` / `timeout`

### 控制闭环示例

#### 1. 服务端发起命令

```bash
# 获取 access token
ACCESS_TOKEN=$(curl -sS -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["accessToken"])')

# 创建命令
COMMAND_ID=$(curl -sS -X POST http://127.0.0.1:8080/projects/project-1/commands \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d '{"target":"demo-target","payload":{"action":"set","value":42}}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["commandId"])')

echo "Command ID: $COMMAND_ID"
```

#### 2. 设备侧发送回执

```bash
# 使用 mosquitto_pub
mosquitto_pub -h 127.0.0.1 -p 1883 -u ems -P admin123 \
  -t "ems/receipts/tenant-1/project-1/$COMMAND_ID" \
  -m '{"status":"success","message":"applied","tsMs":1700000000000}'

# 或使用脚本
EMS_COMMAND_ID="$COMMAND_ID" \
EMS_MQTT_USERNAME=ems EMS_MQTT_PASSWORD=admin123 \
scripts/control-receipt-simulate.sh
```

#### 3. 查询回执

```bash
curl -sS http://127.0.0.1:8080/projects/project-1/commands/$COMMAND_ID/receipts \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

#### 4. 查询审计日志

```bash
curl -sS "http://127.0.0.1:8080/projects/project-1/audit?limit=20" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

---

## 数据库结构

### 核心表

| 表名 | 说明 |
|------|------|
| `tenants` | 租户表 |
| `projects` | 项目表 |
| `users` | 用户表（认证用，包含密码哈希） |
| `roles` | 全局角色表（遗留） |
| `permissions` | 权限表 |
| `user_roles` | 用户角色关联表（遗留） |

### 租户级 RBAC 表

| 表名 | 说明 |
|------|------|
| `tenant_roles` | 租户级角色 |
| `tenant_user_roles` | 租户用户角色关联 |
| `tenant_role_permissions` | 租户角色权限关联 |

### 资产表

| 表名 | 说明 |
|------|------|
| `gateways` | 网关表（支持 `mqtt` / `modbus_tcp` / `tcp_server` / `tcp_client`） |
| `devices` | 设备表（支持关联到房间） |
| `points` | 点位表（支持 `i64` / `f64` / `bool` / `string` 数据类型） |
| `point_sources` | 点位源映射表（支持 MQTT / Modbus / TCP） |

### 建筑层级表

| 表名 | 说明 |
|------|------|
| `areas` | 区域表（4 级层级：Area → Building → Floor → Room） |
| `buildings` | 建筑表 |
| `floors` | 楼层表（支持负楼层） |
| `rooms` | 房间表（支持房间类型） |

### 时序表

| 表名 | 说明 |
|------|------|
| `measurement` | 测量数据表（TimescaleDB hypertable，可选） |
| `event` | 事件表（TimescaleDB hypertable，可选） |

### 控制表

| 表名 | 说明 |
|------|------|
| `commands` | 控制指令表 |
| `command_receipts` | 控制回执表 |
| `audit_logs` | 审计日志表 |

### 采集策略表

| 表名 | 说明 |
|------|------|
| `collection_strategies` | 采集策略表（每点唯一） |

---

## 权限系统

### 20 个权限常量

#### 项目权限
- `PROJECT.READ` - 读取项目
- `PROJECT.WRITE` - 写入项目

#### 资产权限
- `ASSET.GATEWAY.READ` - 读取网关
- `ASSET.GATEWAY.WRITE` - 写入网关
- `ASSET.DEVICE.READ` - 读取设备
- `ASSET.DEVICE.WRITE` - 写入设备
- `ASSET.POINT.READ` - 读取点位
- `ASSET.POINT.WRITE` - 写入点位

#### 数据权限
- `DATA.REALTIME.READ` - 读取实时数据
- `DATA.MEASUREMENTS.READ` - 读取历史数据

#### 控制权限
- `CONTROL.COMMAND.ISSUE` - 发起控制指令
- `CONTROL.COMMAND.READ` - 读取控制指令

#### 告警权限
- `ALARM.RULE.READ` - 读取告警规则
- `ALARM.RULE.WRITE` - 写入告警规则
- `ALARM.EVENT.READ` - 读取告警事件

#### RBAC 权限
- `RBAC.USER.READ` - 读取用户
- `RBAC.USER.WRITE` - 写入用户
- `RBAC.ROLE.READ` - 读取角色
- `RBAC.ROLE.WRITE` - 写入角色

#### 系统权限
- `SYSTEM.METRICS.READ` - 读取指标快照

---

## 工具脚本

### 数据库初始化

```bash
# 初始化所有迁移脚本（按顺序执行）
scripts/db-init.sh

# 接受环境变量
export EMS_DATABASE_URL="postgresql://user:pass@host:5432/db"
export EMS_REQUIRE_TIMESCALE=on  # 可选：要求 TimescaleDB
scripts/db-init.sh
```

### 健康检查

```bash
# 检查 PostgreSQL、Redis、MQTT 可用性
scripts/health-check.sh

# 接受环境变量
export EMS_DATABASE_URL="..."
export EMS_REDIS_URL="redis://default:pass@host:6379"
export EMS_MQTT_HOST="127.0.0.1"
export EMS_MQTT_PORT="1883"
export EMS_MQTT_USERNAME="ems"
export EMS_MQTT_PASSWORD="admin123"
scripts/health-check.sh
```

### MQTT 数据模拟

```bash
# 模拟 MQTT 数据上报
EMS_TENANT_ID=tenant-1 \
EMS_PROJECT_ID=project-1 \
EMS_POINT_ADDRESS=demo/topic \
EMS_PAYLOAD="25.5" \
EMS_MQTT_USERNAME=ems \
EMS_MQTT_PASSWORD=admin123 \
scripts/mqtt-simulate.sh

# 支持的环境变量
# - EMS_MQTT_HOST（默认 127.0.0.1）
# - EMS_MQTT_PORT（默认 1883）
# - EMS_MQTT_TOPIC_PREFIX（默认 ems）
# - EMS_MQTT_DATA_TOPIC_PREFIX（可选，默认 {prefix}/data）
# - EMS_SOURCE_ID（可选，启用时包含在主题中）
```

### 控制回执模拟

```bash
# 模拟设备侧回执
EMS_COMMAND_ID=command-123 \
EMS_MQTT_USERNAME=ems \
EMS_MQTT_PASSWORD=admin123 \
EMS_MQTT_TOPIC_PREFIX=ems \
scripts/control-receipt-simulate.sh
```

### 设备模拟器

```bash
# 启动设备模拟器（自动订阅命令并回执）
EMS_MQTT_USERNAME=ems \
EMS_MQTT_PASSWORD=admin123 \
EMS_MQTT_TOPIC_PREFIX=ems \
EMS_DEVICE_COMMAND_QOS=1 \
EMS_DEVICE_RECEIPT_QOS=1 \
scripts/device-emulator.sh
```

### MVP 验收

```bash
# 一键验收脚本（启动临时 API 实例，端口 18080）
scripts/mvp-acceptance.sh

# 接受环境变量
export EMS_HTTP_ADDR="127.0.0.1:18080"
export EMS_DATABASE_URL="..."
export EMS_JWT_SECRET="dev"
export EMS_REDIS_URL="..."
export EMS_MQTT_HOST="127.0.0.1"
export EMS_MQTT_PORT="1883"
export EMS_MQTT_USERNAME="ems"
export EMS_MQTT_PASSWORD="admin123"
scripts/mvp-acceptance.sh

# 验收流程：
# 1. 初始化数据库
# 2. 启动 API（启用采集和控制）
# 3. 登录获取 token
# 4. 创建项目/网关/设备/点位/映射
# 5. 模拟 MQTT 数据上报
# 6. 验证实时和历史数据查询
# 7. 发起控制指令
# 8. 验证回执和审计日志
# 9. 清理测试数据
```

### 稳定性测试

```bash
# 稳定性验证脚本（端口 18081）
scripts/stability-check.sh

# 测试内容：
# - 批量写入（200 条 MQTT 数据）
# - 异常输入测试
# - 租户隔离测试
# - 控制闭环测试
# - 指标验证
```

### RBAC 回归测试

```bash
# RBAC 管理面回归测试（端口 18082）
scripts/rbac-acceptance.sh

# 测试内容：
# - 用户创建/更新/删除
# - 角色创建/更新/删除
# - 权限绑定
# - 权限验证
```

---

## 前端开发

### 启动前端

```bash
# 仅前端（不启动后端 API）
cd web/admin
pnpm install
pnpm dev

# 前后端联调（Rust 启动 API + pnpm 启动前端）
EMS_WEB_ADMIN=on cargo run -p ems-api

# 仅启动前端（后端 API 已在运行）
EMS_WEB_ADMIN=only cargo run -p ems-api
```

### 前端配置

#### 开发环境（`.env.development`）

```env
VITE_API_BASE_URL=http://127.0.0.1:8080
VITE_ENABLE_MOCK=false  # 关闭 mock，使用真实 API
```

#### 生产环境（`.env.production`）

```env
VITE_API_BASE_URL=/api  # 使用相对路径
VITE_ENABLE_MOCK=false
```

### 前端技术栈

- **框架**：Vue 3 + TypeScript
- **UI 库**：Element Plus
- **路由**：Vue Router 4
- **状态管理**：Pinia 3
- **HTTP 客户端**：Axios
- **样式**：Tailwind CSS 4
- **构建工具**：Vite 7
- **包管理器**：pnpm

---

## 构建与部署

### 构建 API

```bash
# 开发构建
cargo build -p ems-api

# 生产构建
cargo build -p ems-api --release

# 运行生产版本
./target/release/ems-api
```

### 构建前端

```bash
cd web/admin

# 开发构建
pnpm build

# 生产构建
pnpm build:production

# 预览构建结果
pnpm preview
```

### Docker 部署

```bash
# 使用 Docker Compose 启动应用栈
docker compose --profile app up -d

# 查看日志
docker compose logs -f
```

**注意**：需要本机安装 Docker 和 Docker Compose

---

## 多租户隔离

### 三层隔离策略

#### 1. 应用层验证
- 所有请求必须提供 `TenantContext`（包含 `tenant_id`）
- `TenantContext` 从 JWT 中提取
- 所有 API 调用自动携带租户上下文

#### 2. 记录层验证
- 创建/更新操作检查记录的 `tenant_id` 是否与上下文匹配
- 删除操作检查资源是否属于当前租户

#### 3. 数据库层过滤
- 所有 SQL 查询包含 `tenant_id` 过滤条件
- 外键约束确保跨租户访问失败

### 租户上下文结构

```rust
pub struct TenantContext {
    pub tenant_id: String,      // 必填
    pub user_id: String,        // 当前用户
    pub roles: Vec<String>,     // 用户角色
    pub permissions: Vec<String>, // 用户权限
    pub project_scope: Option<String>, // 可选项目级别作用域
}
```

---

## 开发规范

### 依赖方向规则

- `domain` 不依赖任何其他 crate
- `api-contract` 不依赖 `storage` 或 `web` 层
- 业务模块只依赖 `domain` / `api-contract`（及必要的 trait）
- `ems-api` 仅负责装配与启动，其他 crate 禁止反向依赖 `ems-api`

### 编码规范

- **Handler 层**：禁止直接写 SQL，统一走 `storage` 层
- **数据访问**：所有方法必须显式接收 `TenantContext`
- **多租户隔离**：保证多租户隔离逻辑在链路内生效
- **日志追踪**：所有请求输出 `trace_id` / `request_id`
- **结构化日志**：使用 `tracing` 框架

### 测试

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p ems-storage

# 运行集成测试
cargo test --test '*'
```

---

## 故障排查

### 常见问题

#### 1. PostgreSQL 连接失败

```bash
# 检查 PostgreSQL 是否运行
systemctl status postgresql

# 检查连接字符串
export EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
scripts/health-check.sh
```

#### 2. Redis 连接失败

```bash
# 检查 Redis 是否运行
systemctl status redis-server

# 检查连接字符串
export EMS_REDIS_URL="redis://default:admin123@localhost:6379"
redis-cli -u "$EMS_REDIS_URL" ping
```

#### 3. MQTT 连接失败

```bash
# 检查 Mosquitto 是否运行
systemctl status mosquitto

# 测试 MQTT 连接
mosquitto_sub -h 127.0.0.1 -p 1883 -u ems -P admin123 \
  -t "test" -C 1
```

#### 4. JWT Token 无效

```bash
# 检查 JWT_SECRET 是否一致
export EMS_JWT_SECRET="dev"

# 重新登录获取新 token
curl -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'
```

### 日志调试

```bash
# 启用详细日志
export RUST_LOG=debug
cargo run -p ems-api

# 查看特定模块的日志
export RUST_LOG=ems_api=debug,sqlx=debug
cargo run -p ems-api
```

---

## WSL2 开发环境

### 系统信息

- **操作系统**：Ubuntu 24.04.3 LTS (WSL2)
- **内核**：6.6.87.2-microsoft-standard-WSL2
- **Node.js**：v24.12.0
- **npm**：11.6.2
- **Python**：3.12.3

### WSL2 服务自动启动

```bash
# 1. 启用 systemd（需要 sudo）
sudo tee /etc/wsl.conf >/dev/null <<'EOF'
[boot]
systemd=true
EOF

# 2. 重启 WSL
wsl --shutdown

# 3. 启用并启动服务
sudo systemctl enable --now postgresql redis-server mosquitto

# 4. 校验
systemctl is-enabled postgresql redis-server mosquitto
systemctl status postgresql redis-server mosquitto
```

### 默认账号

- **PostgreSQL**：`ems` / `admin123`（数据库：`ems`）
- **Redis**：`default` / `admin123`
- **MQTT (Mosquitto)**：`ems` / `admin123`
- **EMS API**：`admin` / `admin123`

---

## 贡献指南

### 提交代码

```bash
# 格式化代码
cargo fmt

# 运行 clippy
cargo clippy -- -D warnings

# 运行测试
cargo test

# 提交信息格式（推荐）
git commit -m "ems-api: add new endpoint for device control"
```

### 分支策略

- `main`：主分支，稳定版本
- `develop`：开发分支
- `feature/*`：功能分支
- `bugfix/*`：Bug 修复分支

---

## 许可证

MIT License

---

## 联系方式

如有问题或建议，请通过以下方式联系：

- GitHub Issues：[项目仓库](https://github.com/your-org/ems)
- 文档：查看 `/docs` 目录下的详细文档
