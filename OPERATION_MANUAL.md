# EMS 项目操作指南书

> **Energy Management System (EMS)** - 能源管理系统  
> 版本: MVP (M0-M5)  
> 最后更新: 2026-01-16

---

## 目录

1. [项目概述](#1-项目概述)
2. [部署指南](#2-部署指南)
3. [运行指南](#3-运行指南)
4. [配置说明](#4-配置说明)
5. [工程书 (代码功能详解)](#5-工程书)
6. [项目框架](#6-项目框架)
7. [项目运行流程图](#7-项目运行流程图)
8. [附录](#8-附录)

---

## 1. 项目概述

### 1.1 项目目标

EMS 是一个面向能源管理领域的 SaaS 平台，支持：
- **设备数据采集**: 通过 MQTT 协议实时采集设备点位数据
- **时序存储**: 使用 TimescaleDB 存储历史测量数据
- **实时状态查询**: Redis 缓存最新点位值，支持毫秒级实时查询
- **反向控制**: 通过 MQTT 下发控制命令，接收设备回执
- **多租户隔离**: 从第一天起支持 Tenant + Project 隔离

### 1.2 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| **后端语言** | Rust | 高性能、内存安全 |
| **Web 框架** | Axum + Tokio | 异步 HTTP 服务器 |
| **数据库** | PostgreSQL + TimescaleDB | 关系型 + 时序扩展 |
| **缓存** | Redis | 实时数据 + 在线状态 |
| **消息队列** | MQTT (Mosquitto) | 设备数据采集与控制 |
| **前端框架** | Vue 3 + Vite + Element Plus | Pure Admin Thin 模板 |
| **认证** | JWT (HS256) | Access Token + Refresh Token |

### 1.3 核心术语

| 术语 | 定义 |
|------|------|
| **Tenant** | 租户，SaaS 组织边界 |
| **Project** | 项目，租户下的现场/园区/站点边界 |
| **Gateway** | 网关，采集边界设备 |
| **Device** | 设备，挂载在网关下的终端设备 |
| **Point** | 点位，设备上可采集或可控的变量 |
| **PointMapping** | 点位映射，协议地址到 Point 的配置 |
| **RawEvent** | 原始事件，采集源接收的未处理数据 |
| **PointValue** | 标准化后的点位值 |

---

## 2. 部署指南

### 2.1 系统要求

**操作系统:**
- Ubuntu 22.04+ (推荐)
- WSL2 (Windows 开发)

**硬件资源 (最小):**
- CPU: 2 核
- 内存: 4 GB
- 磁盘: 20 GB

### 2.2 依赖工具安装

#### 2.2.1 Rust 工具链

```bash
# 安装 Rust (stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 验证
rustc --version  # 应输出 rustc 1.xx.x
cargo --version
```

#### 2.2.2 Node.js + pnpm

```bash
# 安装 Node.js (v20+ 或 v22+)
# 方式一: 使用 nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 22
nvm use 22

# 安装 pnpm (v9+)
npm install -g pnpm

# 验证
node --version   # v22.x.x
pnpm --version   # 9.x.x
```

#### 2.2.3 PostgreSQL

```bash
# Ubuntu
sudo apt update
sudo apt install postgresql postgresql-contrib

# 启动服务
sudo systemctl start postgresql
sudo systemctl enable postgresql

# 创建数据库和用户
sudo -u postgres psql <<EOF
CREATE USER ems WITH PASSWORD 'admin123';
CREATE DATABASE ems OWNER ems;
GRANT ALL PRIVILEGES ON DATABASE ems TO ems;
EOF

# 验证连接
psql postgresql://ems:admin123@localhost:5432/ems -c "SELECT 1;"
```

#### 2.2.4 Redis

```bash
# Ubuntu
sudo apt install redis-server

# 配置 ACL (可选，安全加固)
# 编辑 /etc/redis/redis.conf 添加:
# user ems on >admin123 ~* +@all

# 启动服务
sudo systemctl start redis-server
sudo systemctl enable redis-server

# 验证
redis-cli ping  # 应返回 PONG
```

#### 2.2.5 MQTT Broker (Mosquitto)

```bash
# Ubuntu
sudo apt install mosquitto mosquitto-clients

# 配置用户认证
sudo mosquitto_passwd -c /etc/mosquitto/passwd ems
# 输入密码: admin123

# 编辑 /etc/mosquitto/conf.d/default.conf:
# listener 1883
# allow_anonymous false
# password_file /etc/mosquitto/passwd

# 重启服务
sudo systemctl restart mosquitto

# 验证
mosquitto_pub -h localhost -u ems -P admin123 -t test -m "hello"
```

### 2.3 数据库初始化

项目提供了一键初始化脚本，位于 `scripts/db-init.sh`：

```bash
# 在项目根目录执行
chmod +x scripts/*.sh
./scripts/db-init.sh
```

该脚本执行以下 Migration 文件：
1. `001_init.sql` - 创建基础表 (tenants, users, roles, permissions)
2. `003_assets.sql` - 创建资产表 (gateways, devices, points, point_sources)
3. `004_timescale.sql` - 创建时序表 (measurement, event)
4. `005_control.sql` - 创建控制表 (commands, command_receipts, audit_logs)
5. `006_rbac.sql` - 创建租户级 RBAC 表 (tenant_roles, tenant_user_roles, tenant_role_permissions)
6. `007_auth_sessions.sql` - refresh token rotation 支持 (users.refresh_jti)
7. `002_seed.sql` - 插入种子数据 (默认租户、用户、角色、权限)

### 2.4 健康检查

```bash
./scripts/health-check.sh
```

输出示例：
```
postgres: checking...
...accepting connections
redis: checking...
PONG
mqtt: ok
health check ok
```

---

## 3. 运行指南

### 3.1 环境变量配置

在项目根目录创建 `.env` 文件：

```env
# 必填
EMS_DATABASE_URL=postgresql://ems:admin123@localhost:5432/ems
EMS_JWT_SECRET=your-32-byte-random-secret-key-here
EMS_JWT_ACCESS_TTL_SECONDS=3600
EMS_JWT_REFRESH_TTL_SECONDS=2592000

# 可选 (有默认值)
EMS_HTTP_ADDR=127.0.0.1:8080
EMS_REDIS_URL=redis://default:admin123@localhost:6379
EMS_MQTT_HOST=127.0.0.1
EMS_MQTT_PORT=1883
EMS_MQTT_USERNAME=ems
EMS_MQTT_PASSWORD=admin123
```

### 3.2 开发模式

#### 方式一：后端带动前端 (推荐)

```bash
# 启动后端 + 自动启动前端
EMS_WEB_ADMIN=on cargo run -p ems-api
```

访问地址：
- 后端 API: `http://127.0.0.1:8080`
- 前端页面: `http://localhost:5173` (Vite 开发服务器)

#### 方式二：前后端分离

**终端 1 (后端):**
```bash
cargo run -p ems-api
```

**终端 2 (前端):**
```bash
cd web/admin
pnpm install  # 首次运行
pnpm dev
```

#### 方式三：启用采集与控制

```bash
EMS_INGEST=on EMS_CONTROL=on cargo run -p ems-api
```

### 3.3 生产构建

**后端构建:**
```bash
cargo build --release -p ems-api
# 产物: target/release/ems-api
```

**前端构建:**
```bash
cd web/admin
pnpm build
# 产物: dist/
```

### 3.4 验收测试

项目提供了端到端验收脚本：

```bash
./scripts/mvp-acceptance.sh
```

该脚本自动完成：
1. 初始化数据库
2. 启动 ems-api (端口 18080)
3. 创建项目/网关/设备/点位/映射
4. 模拟 MQTT 数据采集
5. 验证实时/历史查询
6. 验证控制命令/回执/审计
7. 验证 `/metrics` 指标快照（需鉴权）
8. 清理插入的数据（脚本退出时执行，避免污染数据库）

---

## 4. 配置说明

### 4.1 环境变量完整表

| 变量名 | 类型 | 默认值 | 必填 | 说明 |
|--------|------|--------|------|------|
| **基础配置** |
| `EMS_HTTP_ADDR` | string | `127.0.0.1:8080` | 否 | HTTP 监听地址 |
| `EMS_WEB_ADMIN` | enum | `off` | 否 | 前端启动模式: `off`/`on`/`only` |
| `RUST_LOG` | string | `info` | 否 | 日志级别: `error`/`warn`/`info`/`debug`/`trace` |
| **数据库** |
| `EMS_DATABASE_URL` | string | - | **是** | PostgreSQL 连接串 |
| `EMS_REQUIRE_TIMESCALE` | bool | `false` | 否 | 是否强依赖 timescaledb 扩展（开启时启动与 db-init 检查并 fail-fast） |
| `EMS_REDIS_URL` | string | `redis://default:admin123@localhost:6379` | 否 | Redis 连接串 |
| `EMS_REDIS_LAST_VALUE_TTL_SECONDS` | u64 | 无 | 否 | 实时值 TTL (秒)，0 或空表示永不过期 |
| `EMS_REDIS_ONLINE_TTL_SECONDS` | u64 | `60` | 否 | 设备在线状态 TTL (秒) |
| **认证** |
| `EMS_JWT_SECRET` | string | - | **是** | JWT 签名密钥 (建议 ≥32 字节) |
| `EMS_JWT_ACCESS_TTL_SECONDS` | u64 | - | **是** | Access Token 有效期 (秒) |
| `EMS_JWT_REFRESH_TTL_SECONDS` | u64 | - | **是** | Refresh Token 有效期 (秒) |
| **MQTT** |
| `EMS_MQTT_HOST` | string | `127.0.0.1` | 否 | MQTT Broker 地址 |
| `EMS_MQTT_PORT` | u16 | `1883` | 否 | MQTT 端口 |
| `EMS_MQTT_USERNAME` | string | 空 | 否 | MQTT 用户名 |
| `EMS_MQTT_PASSWORD` | string | 空 | 否 | MQTT 密码 |
| `EMS_MQTT_TOPIC_PREFIX` | string | `ems` | 否 | 系统级 Topic 前缀 |
| `EMS_MQTT_DATA_TOPIC_PREFIX` | string | `{prefix}/data` | 否 | 采集数据 Topic 前缀 |
| `EMS_MQTT_COMMAND_TOPIC_PREFIX` | string | `{prefix}/commands` | 否 | 控制下发 Topic 前缀 |
| `EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET` | bool | `false` | 否 | 控制命令 topic 是否包含 target（开启后主题形如 `{commandPrefix}/{tenant_id}/{project_id}/{target}/{command_id}`） |
| `EMS_MQTT_RECEIPT_TOPIC_PREFIX` | string | `{prefix}/receipts` | 否 | 设备回执订阅 Topic 前缀 |
| `EMS_MQTT_DATA_TOPIC_HAS_SOURCE_ID` | bool | `false` | 否 | 采集 Topic 是否包含 source_id |
| **采集与控制** |
| `EMS_INGEST` | bool | `false` | 否 | 启用采集模块 |
| `EMS_CONTROL` | bool | `false` | 否 | 启用控制模块 |
| `EMS_MQTT_COMMAND_QOS` | u8 | `1` | 否 | 控制下发 QoS（0/1/2） |
| `EMS_MQTT_RECEIPT_QOS` | u8 | `1` | 否 | 回执订阅 QoS（0/1/2） |
| `EMS_CONTROL_DISPATCH_MAX_RETRIES` | u64 | `2` | 否 | 命令下发最大重试次数 |
| `EMS_CONTROL_DISPATCH_BACKOFF_MS` | u64 | `200` | 否 | 命令下发重试间隔 (ms) |
| `EMS_CONTROL_RECEIPT_TIMEOUT_SECONDS` | u64 | `30` | 否 | 等待设备回执超时 (秒) |

### 4.2 MQTT Topic 结构

**采集数据 Topic:**
```
{EMS_MQTT_DATA_TOPIC_PREFIX}/{tenant_id}/{project_id}/{address}
# 示例: ems/data/tenant-1/project-1/demo/topic
```

**控制命令 Topic:**
```
{EMS_MQTT_COMMAND_TOPIC_PREFIX}/{tenant_id}/{project_id}/{command_id}
# 示例: ems/commands/tenant-1/project-1/cmd-123

# 若 EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET=true:
{EMS_MQTT_COMMAND_TOPIC_PREFIX}/{tenant_id}/{project_id}/{target}/{command_id}
# 示例: ems/commands/tenant-1/project-1/device/device-1/cmd-123
```

**设备回执 Topic:**
```
{EMS_MQTT_RECEIPT_TOPIC_PREFIX}/{tenant_id}/{project_id}/{command_id}
# 示例: ems/receipts/tenant-1/project-1/cmd-123
```

### 4.3 前端配置

前端环境变量位于 `web/admin/.env.development`：

```env
VITE_PORT = 5173
VITE_PUBLIC_PATH = /
VITE_ROUTER_HISTORY = "hash"
VITE_ENABLE_MOCK = false
VITE_API_BASE_URL = http://127.0.0.1:8080
```

---

## 5. 工程书

### 5.1 后端架构

#### 5.1.1 Workspace 结构

```
ems/
├── apps/
│   └── ems-api/              # HTTP API 二进制入口
│       └── src/
│           ├── main.rs       # 主函数，启动流程
│           ├── routes.rs     # 路由定义
│           ├── handlers/     # HTTP Handler 实现
│           ├── middleware/   # 认证中间件
│           ├── ingest.rs     # 采集链路装配
│           └── utils/        # 响应工具、验证
└── crates/
    ├── core/
    │   ├── domain/           # 领域模型
    │   └── api-contract/     # DTO 契约
    └── capability/
        ├── auth/             # 认证能力
        ├── config/           # 配置加载
        ├── control/          # 反向控制
        ├── ingest/           # 数据采集
        ├── normalize/        # 数据标准化
        ├── pipeline/         # 数据流水线
        ├── storage/          # 存储抽象
        └── telemetry/        # 遥测指标
```

#### 5.1.2 核心模块详解

##### `domain` (crates/core/domain)

**文件:** `lib.rs`, `data.rs`, `permissions.rs`

**功能:**
- 定义 `TenantContext`: 租户上下文，包含 `tenant_id`, `user_id`, `roles`, `permissions`, `project_scope`
- 定义 `RawEvent`: 原始采集事件
- 定义 `PointValue`, `PointValueData`: 标准化点位值
- 定义 `permissions` 常量: 所有权限码

```rust
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub project_scope: Option<String>,
}
```

##### `api-contract` (crates/core/api-contract)

**文件:** `lib.rs`

**功能:**
- 定义所有 API 请求/响应 DTO
- 定义 `ApiResponse<T>` 标准响应封装
- 定义 `error_codes` 常量

**关键 DTO:**
- `LoginRequest/LoginResponse`: 登录
- `CreateProjectRequest/ProjectDto`: 项目 CRUD
- `CreateDeviceRequest/DeviceDto`: 设备 CRUD
- `CreateCommandRequest/CommandDto`: 控制命令
- `MeasurementsQuery/MeasurementValueDto`: 历史查询
- `RealtimeQuery/RealtimeValueDto`: 实时查询

##### `auth` (crates/capability/auth)

**文件:** `lib.rs`, `jwt.rs`, `password.rs`

**功能:**
- `AuthService`: 登录验证、Token 校验、Token 刷新
- `JwtManager`: JWT 编码/解码 (HS256)
- `hash_password`, `verify_password_and_maybe_upgrade`: Argon2id 密码哈希

**登录流程:**
1. 查询用户 (`UserStore::find_by_username`)
2. 验证密码 (`verify_password_and_maybe_upgrade`)
3. 签发 Token (`JwtManager::issue_tokens`)

##### `config` (crates/capability/config)

**文件:** `lib.rs`

**功能:**
- `AppConfig::from_env()`: 从环境变量加载全部配置
- 支持类型安全解析、默认值、必填校验

##### `ingest` (crates/capability/ingest)

**文件:** `lib.rs`

**功能:**
- `MqttSource`: MQTT 采集源实现
- `RawEventHandler` Trait: 原始事件处理接口
- Topic 解析: `{prefix}/{tenant_id}/{project_id}/{address}`

**核心逻辑:**
```rust
impl Source for MqttSource {
    async fn run(&self, handler: Arc<dyn RawEventHandler>) -> Result<(), IngestError> {
        // 1. 订阅 Topic
        // 2. 循环 poll 消息
        // 3. 解析 tenant_id/project_id/address
        // 4. 构造 RawEvent 调用 handler.handle()
    }
}
```

##### `normalize` (crates/capability/normalize)

**文件:** `lib.rs`

**功能:**
- `Normalizer`: 将 RawEvent 转换为 PointValue
- `PointMappingProvider` Trait: 点位映射查询接口
- `StoragePointMappingProvider`: 基于 Storage 的实现

**转换流程:**
1. 根据 `address` 查询 `PointMapping`
2. 解析 payload 为 `f64`
3. 应用 `scale` 和 `offset`
4. 返回 `PointValue`

##### `pipeline` (crates/capability/pipeline)

**文件:** `lib.rs`

**功能:**
- `Pipeline`: 数据处理流水线 (批量、去重、重试)
- `PointValueWriter` Trait: 写入接口
- `StoragePointValueWriter`: 写入 Measurement + LastValue

**特性:**
- 批量写入 (`batch_size`)
- 背压控制 (`max_buffer_size`)
- 重复值过滤 (`dedup_cache_size`)
- 过期值丢弃 (`max_age_ms`)

##### `control` (crates/capability/control)

**文件:** `lib.rs`

**功能:**
- `CommandService`: 命令下发服务
- `MqttDispatcher`: MQTT 命令下发实现
- `spawn_receipt_listener`: MQTT 回执监听

**命令状态流转:**
```
issued → accepted → success/failed/timeout
```

##### `storage` (crates/capability/storage)

**文件:** `lib.rs`, `traits.rs`, `models.rs`, `postgres/`, `in_memory/`, `redis.rs`

**功能:**
- 定义所有存储 Trait: `UserStore`, `ProjectStore`, `DeviceStore`, `PointStore`, `PointMappingStore`, `MeasurementStore`, `RealtimeStore`, `CommandStore`, `AuditLogStore` 等
- PostgreSQL 实现: `PgUserStore`, `PgProjectStore`, ...
- InMemory 实现: 用于测试
- Redis 实现: `RedisRealtimeStore`, `RedisOnlineStore`

**设计原则:**
- 所有方法显式接收 `TenantContext`
- 多租户隔离: SQL 查询自动添加 `tenant_id` 过滤

##### `telemetry` (crates/capability/telemetry)

**文件:** `lib.rs`

**功能:**
- `init_tracing`: 初始化日志
- `TelemetryMetrics`: 采集指标 (原子计数器)
- 指标记录函数: `record_raw_event`, `record_write_success`, `record_command_issued`, ...

#### 5.1.3 API Handler 详解

位于 `apps/ems-api/src/handlers/`：

| 文件 | 端点 | 功能 |
|------|------|------|
| `auth.rs` | `/login`, `/refresh-token`, `/get-async-routes`, `/livez`, `/readyz`, `/health` | 认证、动态路由、探针 |
| `projects.rs` | `/projects` | 项目 CRUD |
| `gateways.rs` | `/projects/:id/gateways` | 网关 CRUD |
| `devices.rs` | `/projects/:id/devices` | 设备 CRUD |
| `points.rs` | `/projects/:id/points` | 点位 CRUD |
| `point_mappings.rs` | `/projects/:id/point-mappings` | 点位映射 CRUD |
| `realtime.rs` | `/projects/:id/realtime` | 实时查询 |
| `measurements.rs` | `/projects/:id/measurements` | 历史查询 (支持聚合) |
| `commands.rs` | `/projects/:id/commands` | 控制命令发送、查询 |
| `audit.rs` | `/projects/:id/audit` | 审计日志查询 |
| `rbac.rs` | `/rbac/users`, `/rbac/roles`, `/rbac/permissions` | RBAC 管理 |
| `metrics.rs` | `/metrics` | 遥测指标快照（需 Bearer token + `SYSTEM.METRICS.READ`） |

### 5.2 前端架构

#### 5.2.1 技术栈

- **框架:** Vue 3 + Composition API
- **构建:** Vite 7
- **UI 库:** Element Plus
- **状态管理:** Pinia
- **路由:** Vue Router 4 (Hash 模式)
- **样式:** TailwindCSS 4 + SCSS
- **模板:** Pure Admin Thin v6.2.0

#### 5.2.2 目录结构

```
web/admin/src/
├── api/            # API 请求封装
├── assets/         # 静态资源
├── components/     # 公共组件
├── config/         # 应用配置
├── directives/     # 自定义指令
├── layout/         # 布局组件 (侧边栏、导航栏、标签页)
├── plugins/        # 插件注册
├── router/         # 路由配置
├── store/          # Pinia 状态管理
├── style/          # 全局样式
├── utils/          # 工具函数
├── views/          # 页面视图
│   ├── ems/        # EMS 业务页面
│   │   ├── projects/
│   │   ├── gateways/
│   │   ├── devices/
│   │   ├── points/
│   │   ├── point-mappings/
│   │   ├── realtime/
│   │   ├── measurements/
│   │   ├── commands/
│   │   ├── audit/
│   │   └── rbac/
│   ├── login/
│   └── error/
├── App.vue
└── main.ts
```

#### 5.2.3 动态路由

前端路由由后端 `/get-async-routes` 接口动态返回，根据用户权限过滤。

### 5.3 数据库 Schema

#### 5.3.1 核心表

```sql
-- 租户
tenants (tenant_id PK, name, status, created_at)

-- 项目
projects (project_id PK, tenant_id FK, name, timezone, created_at)

-- 用户
users (user_id PK, tenant_id FK, username UNIQUE, password_hash, status, created_at)

-- 角色与权限 (全局)
roles (role_code PK, name)
permissions (permission_code PK, description)
user_roles (user_id, role_code) PK
role_permissions (role_code, permission_code) PK

-- 角色与权限 (租户级)
tenant_roles (tenant_id, role_code, name) PK(tenant_id, role_code)
tenant_user_roles (tenant_id, user_id, role_code) PK
tenant_role_permissions (tenant_id, role_code, permission_code) PK
```

#### 5.3.2 资产表

```sql
gateways (gateway_id PK, tenant_id, project_id, name, status, last_seen_at)
devices (device_id PK, tenant_id, project_id, gateway_id, name, model)
points (point_id PK, tenant_id, project_id, device_id, key, data_type, unit)
point_sources (source_id PK, tenant_id, project_id, point_id, source_type, address, scale, offset_value)
```

#### 5.3.3 时序表 (TimescaleDB Hypertable)

```sql
measurement (tenant_id, project_id, point_id, ts, value, quality)
-- 索引: (tenant_id, project_id, point_id, ts DESC)

event (tenant_id, project_id, ts, type, payload JSONB)
```

#### 5.3.4 控制与审计表

```sql
commands (command_id PK, tenant_id, project_id, target, payload JSONB, status, issued_by, issued_at)
command_receipts (receipt_id PK, tenant_id, project_id, command_id, ts, status, message)
audit_logs (audit_id PK, tenant_id, project_id, actor, action, resource, result, detail, ts)
```

---

## 6. 项目框架

```
ems/
├── apps/
│   └── ems-api/                   # HTTP API 主程序
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # 入口：配置、初始化、路由、启动
│           ├── routes.rs          # 路由映射
│           ├── ingest.rs          # 采集链路装配
│           ├── handlers/          # 业务 Handler
│           │   ├── mod.rs
│           │   ├── auth.rs
│           │   ├── projects.rs
│           │   ├── gateways.rs
│           │   ├── devices.rs
│           │   ├── points.rs
│           │   ├── point_mappings.rs
│           │   ├── realtime.rs
│           │   ├── measurements.rs
│           │   ├── commands.rs
│           │   ├── audit.rs
│           │   ├── rbac.rs
│           │   └── metrics.rs
│           ├── middleware/        # 中间件
│           │   ├── mod.rs
│           │   └── auth.rs
│           └── utils/             # 工具
│               ├── mod.rs
│               ├── response.rs
│               └── validation.rs
├── crates/
│   ├── core/
│   │   ├── domain/                # 领域模型
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs         # TenantContext
│   │   │       ├── data.rs        # RawEvent, PointValue, PointValueData
│   │   │       └── permissions.rs # 权限码常量
│   │   └── api-contract/          # API 契约
│   │       ├── Cargo.toml
│   │       └── src/
│   │           └── lib.rs         # 所有 DTO 定义
│   └── capability/
│       ├── auth/                  # 认证能力
│       │   └── src/
│       │       ├── lib.rs         # AuthService
│       │       ├── jwt.rs         # JwtManager
│       │       └── password.rs    # Argon2id
│       ├── config/                # 配置加载
│       │   └── src/lib.rs         # AppConfig
│       ├── control/               # 反向控制
│       │   └── src/lib.rs         # CommandService, MqttDispatcher
│       ├── ingest/                # 数据采集
│       │   └── src/lib.rs         # MqttSource
│       ├── normalize/             # 数据标准化
│       │   └── src/lib.rs         # Normalizer
│       ├── pipeline/              # 数据流水线
│       │   └── src/lib.rs         # Pipeline
│       ├── storage/               # 存储抽象
│       │   └── src/
│       │       ├── lib.rs         # 模块导出
│       │       ├── traits.rs      # 存储 Trait 定义
│       │       ├── models.rs      # 数据模型
│       │       ├── error.rs       # StorageError
│       │       ├── connection.rs  # PgPool
│       │       ├── validation.rs  # 租户验证
│       │       ├── redis.rs       # Redis 实现
│       │       ├── online.rs      # OnlineStore Trait
│       │       ├── postgres/      # Pg 实现
│       │       └── in_memory/     # 内存实现
│       └── telemetry/             # 遥测
│           └── src/lib.rs         # Metrics
├── web/
│   └── admin/                     # Vue3 前端
│       ├── package.json
│       ├── vite.config.ts
│       └── src/
│           ├── views/ems/         # EMS 业务页面
│           ├── api/               # API 封装
│           ├── router/            # 路由
│           ├── store/             # Pinia
│           └── layout/            # 布局
├── migrations/                    # 数据库迁移
│   ├── 001_init.sql
│   ├── 002_seed.sql
│   ├── 003_assets.sql
│   ├── 004_timescale.sql
│   ├── 005_control.sql
│   └── 006_rbac.sql
├── scripts/                       # 运维脚本
│   ├── db-init.sh                 # 数据库初始化
│   ├── health-check.sh            # 健康检查
│   ├── mvp-acceptance.sh          # 验收测试
│   ├── rbac-acceptance.sh         # RBAC 验收
│   ├── mqtt-simulate.sh           # MQTT 数据模拟
│   ├── control-receipt-simulate.sh # 回执模拟
│   ├── device-emulator.sh         # 设备模拟器
│   └── stability-check.sh         # 稳定性测试
├── docker/                        # Docker 配置
├── docker-compose.yml             # 基础设施编排
├── Cargo.toml                     # Workspace 配置
├── .env                           # 环境变量
└── *.md                           # 文档
```

---

## 7. 项目运行流程图

### 7.1 系统启动流程

```mermaid
flowchart TD
    A[main.rs 启动] --> B[dotenvy::dotenv 加载 .env]
    B --> C[AppConfig::from_env 读取配置]
    C --> D[init_tracing 初始化日志]
    D --> E{EMS_WEB_ADMIN?}
    E -->|on/only| F[spawn pnpm dev]
    E -->|off| G[跳过前端]
    F --> G
    G --> H[connect_pool 连接 Postgres]
    H --> I[初始化所有 Store]
    I --> J{EMS_CONTROL=on?}
    J -->|Yes| K[MqttDispatcher.connect]
    K --> L[spawn_receipt_listener]
    J -->|No| M[NoopDispatcher]
    L --> N{EMS_INGEST=on?}
    M --> N
    N -->|Yes| O[spawn_ingest MQTT 订阅]
    N -->|No| P[NoopSource]
    O --> Q[构建 AppState]
    P --> Q
    Q --> R[create_api_router 路由]
    R --> S[添加 middleware]
    S --> T[TcpListener::bind]
    T --> U[axum::serve 启动 HTTP]
```

### 7.2 数据采集流程

```mermaid
sequenceDiagram
    participant Device as 设备
    participant MQTT as MQTT Broker
    participant Ingest as ems-ingest
    participant Normalize as ems-normalize
    participant Pipeline as ems-pipeline
    participant Pg as PostgreSQL
    participant Redis as Redis

    Device->>MQTT: PUBLISH ems/data/tenant/project/address
    MQTT->>Ingest: 消息推送
    Ingest->>Ingest: 解析 Topic 提取 tenant_id, project_id, address
    Ingest->>Ingest: 构造 RawEvent
    Ingest->>Normalize: handler.handle(RawEvent)
    Normalize->>Normalize: 查询 PointMapping (address → point_id)
    Normalize->>Normalize: 解析 payload, 应用 scale/offset
    Normalize-->>Ingest: PointValue
    Ingest->>Pipeline: pipeline.handle(PointValue)
    Pipeline->>Pipeline: 验证 (ts, value)
    Pipeline->>Pipeline: 去重检查
    Pipeline->>Pipeline: 加入 buffer
    alt buffer 满
        Pipeline->>Pg: write_measurements (批量 INSERT)
        Pipeline->>Redis: upsert_last_value (SET)
    end
    Pipeline-->>Ingest: WriteResult
```

### 7.3 控制命令流程

```mermaid
sequenceDiagram
    participant User as 用户/前端
    participant API as ems-api
    participant CommandService as CommandService
    participant Pg as PostgreSQL
    participant MQTT as MQTT Broker
    participant Device as 设备
    participant ReceiptListener as ReceiptListener

    User->>API: POST /projects/:id/commands
    API->>API: 验证 JWT, 权限
    API->>CommandService: issue_command(ctx, request)
    CommandService->>Pg: create_command (status=issued)
    CommandService->>MQTT: MqttDispatcher.dispatch (PUBLISH)
    MQTT->>Device: 命令推送
    CommandService->>Pg: update_command_status (status=accepted)
    CommandService->>Pg: create_audit_log (CONTROL.COMMAND.ISSUE)
    CommandService-->>API: CommandRecord
    API-->>User: 200 OK {commandId, status}

    Note over Device,MQTT: 设备执行命令...

    Device->>MQTT: PUBLISH ems/receipts/tenant/project/command_id
    MQTT->>ReceiptListener: 回执推送
    ReceiptListener->>Pg: create_receipt
    ReceiptListener->>Pg: update_command_status (status=success)
    ReceiptListener->>Pg: create_audit_log (CONTROL.COMMAND.RECEIPT)
```

### 7.4 认证流程

```mermaid
sequenceDiagram
    participant Client as 前端
    participant API as ems-api
    participant Auth as AuthService
    participant Jwt as JwtManager
    participant Pg as PostgreSQL

    Client->>API: POST /login {username, password}
    API->>Auth: login(username, password)
    Auth->>Pg: find_by_username
    Pg-->>Auth: UserRecord
    Auth->>Auth: verify_password (Argon2id)
    Auth->>Jwt: issue_tokens(TenantContext)
    Jwt-->>Auth: access_token, refresh_token, expires
    Auth-->>API: (UserRecord, AuthTokens)
    API-->>Client: 200 OK {accessToken, refreshToken, expires, ...}

    Note over Client,API: 后续请求携带 Authorization: Bearer <accessToken>

    Client->>API: GET /projects (Authorization: Bearer ...)
    API->>Auth: verify_access_token(token)
    Auth->>Jwt: decode_access(token)
    Jwt-->>Auth: TenantContext
    API->>API: 执行业务逻辑
    API-->>Client: 200 OK {data}
```

---

## 8. 附录

### 8.1 默认账户

| 用户名 | 初始密码 | 租户 | 角色 |
|--------|------|------|------|
| admin | admin123 | tenant-1 | admin |
| admin2 | admin123 | tenant-2 | admin |
说明：数据库仅存储口令哈希（Argon2id），不存储明文；生产环境请修改默认密码并妥善管理。

### 8.2 API 快速参考

```bash
# 登录
curl -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'

# 设置 Token
ACCESS_TOKEN="..."
AUTH="Authorization: Bearer $ACCESS_TOKEN"

# 列出项目
curl http://127.0.0.1:8080/projects -H "$AUTH"

# 创建项目
curl -X POST http://127.0.0.1:8080/projects \
  -H "Content-Type: application/json" -H "$AUTH" \
  -d '{"name":"my-project","timezone":"Asia/Shanghai"}'

# 查询实时数据
curl "http://127.0.0.1:8080/projects/{project_id}/realtime?pointId={point_id}" -H "$AUTH"

# 查询历史数据 (带聚合)
curl "http://127.0.0.1:8080/projects/{project_id}/measurements?pointId={point_id}&bucketMs=60000&agg=avg&limit=100" -H "$AUTH"

# 发送控制命令
curl -X POST "http://127.0.0.1:8080/projects/{project_id}/commands" \
  -H "Content-Type: application/json" -H "$AUTH" \
  -d '{"target":"device:xxx","payload":{"action":"set","value":42}}'
```

### 8.3 日志级别建议

| 环境 | RUST_LOG |
|------|----------|
| 开发 | `debug,sqlx=warn` |
| 测试 | `info` |
| 生产 | `warn,ems=info` |

### 8.4 常见问题

**Q: 数据库连接失败?**
- 检查 PostgreSQL 服务: `sudo systemctl status postgresql`
- 检查连接串: `psql "$EMS_DATABASE_URL" -c "SELECT 1;"`

**Q: MQTT 采集不生效?**
- 确认 `EMS_INGEST=on`
- 检查 Topic 格式: `{prefix}/data/{tenant_id}/{project_id}/{address}`
- 检查 PointMapping 配置

**Q: 前端登录失败?**
- 检查后端 `/livez` 与 `/readyz` 是否正常（`/health` 等价于 `/livez`）
- 检查 CORS 配置 (当前后端默认允许)

---

> **文档结束**  
> 如有问题，请参考项目内其他 MD 文档或源代码注释。
