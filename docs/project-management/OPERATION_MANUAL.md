# EMS 项目操作指南书

> **Energy Management System (EMS)** - 能源管理系统  
> 版本: MVP (M0-M5)  
> 最后更新: 2026-01-24

---

## 目录

0. [快速开始（开发/本地）](#0-快速开始开发本地)
1. [项目概述](#1-项目概述)
2. [部署指南（依赖与初始化）](#2-部署指南)
3. [运行指南（开发/生产）](#3-运行指南)
4. [配置说明（后端/前端）](#4-配置说明)
5. [工程参考（架构/流程/目录）](#5-工程参考另见)
8. [附录（账号/接口/FAQ）](#8-附录)

---

## 0. 快速开始（开发/本地）

> 目标：后端 + 前端联调跑起来（动态菜单来自后端 `/get-async-routes`）。

```bash
# 1) 配置运行期必填环境变量（示例：本机 Postgres/Redis）
export EMS_DATABASE_URL="postgresql://ems:<PASSWORD>@localhost:5432/ems"
export EMS_REDIS_URL="redis://:<PASSWORD>@localhost:6379"
export EMS_JWT_SECRET="dev"
export EMS_JWT_ACCESS_TTL_SECONDS="3600"
export EMS_JWT_REFRESH_TTL_SECONDS="7200"

# 2) 初始化数据库（可选：为初始管理员指定固定口令；不设则自动生成随机口令并输出到 stderr）
export EMS_SEED_ADMIN_PASSWORD="<SET_A_STRONG_PASSWORD_OR_LEAVE_UNSET_TO_GENERATE>"
export EMS_SEED_ADMIN2_PASSWORD="<SET_A_STRONG_PASSWORD_OR_LEAVE_UNSET_TO_GENERATE>"
./scripts/db-init.sh

# 3) 启动后端并带起前端（需要 Node.js + pnpm）
EMS_WEB_ADMIN=on cargo run -p ems-api
```

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
CREATE USER ems WITH PASSWORD '<CHANGE_ME>';
CREATE DATABASE ems OWNER ems;
GRANT ALL PRIVILEGES ON DATABASE ems TO ems;
EOF

# 验证连接
psql postgresql://ems:<PASSWORD>@localhost:5432/ems -c "SELECT 1;"
```

#### 2.2.4 Redis

```bash
# Ubuntu
sudo apt install redis-server

# 配置 ACL (可选，安全加固)
# 编辑 /etc/redis/redis.conf 添加:
# user ems on ><CHANGE_ME> ~* +@all

# 启动服务
sudo systemctl start redis-server
sudo systemctl enable redis-server

# 验证
redis-cli ping  # 应返回 PONG
```

说明：
- EMS 后端运行期要求提供 `EMS_REDIS_URL`（不再内置弱口令默认值）。
- 若你使用 Redis ACL，请按实际用户名/密码拼出 URL，例如：`redis://ems:<PASSWORD>@localhost:6379`；
  若使用 `requirepass`（默认 user），例如：`redis://:<PASSWORD>@localhost:6379`。

#### 2.2.5 MQTT Broker (Mosquitto)

```bash
# Ubuntu
sudo apt install mosquitto mosquitto-clients

# 配置用户认证
sudo mosquitto_passwd -c /etc/mosquitto/passwd ems
# 输入密码: <CHANGE_ME>

# 编辑 /etc/mosquitto/conf.d/default.conf:
# listener 1883
# allow_anonymous false
# password_file /etc/mosquitto/passwd

# 重启服务
sudo systemctl restart mosquitto

# 验证
mosquitto_pub -h localhost -u ems -P <PASSWORD> -t test -m "hello"
```

说明：
- MQTT 账号密码属于运行期配置（`EMS_MQTT_USERNAME`/`EMS_MQTT_PASSWORD`），不要在生产环境复用弱口令。

### 2.3 数据库初始化

项目提供了一键初始化脚本，位于 `scripts/db-init.sh`：

```bash
# 在项目根目录执行
chmod +x scripts/*.sh

# 必填：数据库连接串
export EMS_DATABASE_URL="postgresql://ems:<PASSWORD>@localhost:5432/ems"

# 可选：为初始管理员指定口令（不设置则自动生成随机口令并打印到 stderr，请妥善保存）
export EMS_SEED_ADMIN_PASSWORD="<SET_A_STRONG_PASSWORD_OR_LEAVE_UNSET_TO_GENERATE>"
export EMS_SEED_ADMIN2_PASSWORD="<SET_A_STRONG_PASSWORD_OR_LEAVE_UNSET_TO_GENERATE>"

./scripts/db-init.sh
```

该脚本执行以下 Migration 文件：
1. `001_init.sql` - 创建基础表 (tenants, users, roles, permissions)
2. `003_assets.sql` - 创建资产表 (gateways, devices, points, point_sources)
3. `004_timescale.sql` - 创建时序表 (measurement, event)
4. `005_control.sql` - 创建控制表 (commands, command_receipts, audit_logs)
5. `006_rbac.sql` - 创建租户级 RBAC 表 (tenant_roles, tenant_user_roles, tenant_role_permissions)
6. `007_auth_sessions.sql` - refresh token rotation 支持 (users.refresh_jti)
7. `002_seed.sql` - 插入种子数据 (租户、用户、角色、权限；用户口令以 Argon2id 哈希形式写入)

说明：
- `db-init.sh` 会调用 `ems-api hash-password <password>` 生成 Argon2id 哈希；首次运行建议先 `cargo build -p ems-api`。
- 在 WSL/CI 中如遇到脚本卡在 hash 步骤，可使用 `bash -x scripts/db-init.sh` 定位，并确保使用最新的 `target/debug/ems-api`（不要误用旧的 `target/release/ems-api`）。
- 如需绕过脚本内哈希计算，可直接提供：`EMS_SEED_ADMIN_PASSWORD_HASH` / `EMS_SEED_ADMIN2_PASSWORD_HASH`。

### 2.4 健康检查

```bash
export EMS_DATABASE_URL="postgresql://ems:<PASSWORD>@localhost:5432/ems"
export EMS_REDIS_URL="redis://:<PASSWORD>@localhost:6379"

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
EMS_DATABASE_URL=postgresql://ems:<PASSWORD>@localhost:5432/ems
EMS_REDIS_URL=redis://:<PASSWORD>@localhost:6379
EMS_JWT_SECRET=your-32-byte-random-secret-key-here
EMS_JWT_ACCESS_TTL_SECONDS=3600
EMS_JWT_REFRESH_TTL_SECONDS=2592000

# 可选
EMS_HTTP_ADDR=127.0.0.1:8080
EMS_MQTT_HOST=127.0.0.1
EMS_MQTT_PORT=1883
EMS_MQTT_USERNAME=ems
EMS_MQTT_PASSWORD=<PASSWORD>

# 可选：数据库初始化 seed 口令（仅供 scripts/db-init.sh 使用）
EMS_SEED_ADMIN_PASSWORD=<SET_FOR_DEV_OR_LEAVE_EMPTY>
EMS_SEED_ADMIN2_PASSWORD=<SET_FOR_DEV_OR_LEAVE_EMPTY>
```

说明：
- 运行期必填：`EMS_DATABASE_URL`、`EMS_REDIS_URL`、`EMS_JWT_SECRET`、`EMS_JWT_ACCESS_TTL_SECONDS`、`EMS_JWT_REFRESH_TTL_SECONDS`。
- `EMS_SEED_*` 仅供 `scripts/db-init.sh` 使用，不影响已初始化数据库的登录口令。

### 3.2 开发模式

#### 方式一：后端带动前端 (推荐)

```bash
# 启动后端 + 自动启动前端
EMS_WEB_ADMIN=on cargo run -p ems-api
```

访问地址：
- 后端 API: `http://127.0.0.1:8080`
- 前端页面: `http://localhost:<VITE_PORT>`（默认见 `web/admin/.env.development`，当前项目为 `8848`）

说明：
- 前端菜单/路由来自后端 `GET /get-async-routes`，按 `TenantContext.permissions` 过滤。
- 若前端启动正常但菜单为空，请先确认后端可访问、登录成功且 `/get-async-routes` 返回非空。

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
VITE_API_BASE="https://api.example.com" pnpm build
# 产物: dist/
```

说明：
- `VITE_API_BASE` 生产构建必须提供（缺失会直接报错），避免部署时默默回退到 `localhost`。
- 如启用 MQTT TLS（`EMS_MQTT_USE_TLS=on`），请确保运行环境包含系统根证书（例如安装 `ca-certificates`），否则 TLS 初始化会失败并退出（不会 panic）。

### 3.4 验收测试

项目提供了端到端验收脚本：

```bash
export EMS_DATABASE_URL="postgresql://ems:<PASSWORD>@localhost:5432/ems"
export EMS_REDIS_URL="redis://:<PASSWORD>@localhost:6379"

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
| `EMS_LOG_FORMAT` | enum | `text` | 否 | 日志格式：`text`/`json` |
| **数据库** |
| `EMS_DATABASE_URL` | string | - | **是** | PostgreSQL 连接串 |
| `EMS_REQUIRE_TIMESCALE` | bool | `false` | 否 | 是否强依赖 timescaledb 扩展（开启时启动与 db-init 检查并 fail-fast） |
| `EMS_REDIS_URL` | string | - | **是** | Redis 连接串 |
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
| `EMS_MQTT_USE_SHARED_SUBSCRIPTION` | bool | `false` | 否 | 是否启用共享订阅（用于多实例采集扩容） |
| `EMS_MQTT_SHARED_GROUP` | string | `ems-ingest` | 否 | 共享订阅组名 |
| `EMS_MQTT_USE_TLS` | bool | `false` | 否 | 是否启用 MQTT TLS |
| `EMS_MQTT_CA_CERT_PATH` | string | 空 | 否 | MQTT CA 证书路径 |
| `EMS_MQTT_CLIENT_CERT_PATH` | string | 空 | 否 | MQTT 客户端证书路径（mTLS） |
| `EMS_MQTT_CLIENT_KEY_PATH` | string | 空 | 否 | MQTT 客户端私钥路径（mTLS） |
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
| **数据库初始化（仅脚本使用）** |
| `EMS_SEED_ADMIN_PASSWORD` | string | 空 | 否 | `scripts/db-init.sh` 用：tenant-1 的初始 `admin` 口令 |
| `EMS_SEED_ADMIN2_PASSWORD` | string | 空 | 否 | `scripts/db-init.sh` 用：tenant-2 的初始 `admin2` 口令 |
| `EMS_SEED_ADMIN_PASSWORD_HASH` | string | 空 | 否 | `scripts/db-init.sh` 用：直接提供 Argon2id 哈希（跳过 hash-password） |
| `EMS_SEED_ADMIN2_PASSWORD_HASH` | string | 空 | 否 | `scripts/db-init.sh` 用：直接提供 Argon2id 哈希（跳过 hash-password） |

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
VITE_PORT = 8848
VITE_PUBLIC_PATH = /
VITE_ROUTER_HISTORY = "hash"
VITE_ENABLE_MOCK = false
VITE_API_BASE = http://127.0.0.1:8080
```

说明：`VITE_API_BASE` 在构建阶段必须存在（生产构建缺失会直接失败），避免部署时默默回退到 `localhost`。

---

## 5. 工程参考（另见）

本手册仅保留部署/运行/配置与常见问题；工程细节与流程图请移步：

- `docs/project-management/ENGINEERING_REFERENCE.md`
- `ARCHITECTURE.md`

## 8. 附录

### 8.1 默认账户

| 用户名 | 初始密码 | 租户 | 角色 |
|--------|------|------|------|
| admin | （由 `scripts/db-init.sh` 生成或由环境变量指定） | tenant-1 | admin |
| admin2 | （由 `scripts/db-init.sh` 生成或由环境变量指定） | tenant-2 | admin |
说明：
- 数据库仅存储口令哈希（Argon2id），不存储明文。
- 开发环境如需固定口令，可在执行 `scripts/db-init.sh` 前设置 `EMS_SEED_ADMIN_PASSWORD` / `EMS_SEED_ADMIN2_PASSWORD`。

### 8.2 API 快速参考

```bash
# 登录
curl -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"<PASSWORD_FROM_DB_INIT>"}'

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
