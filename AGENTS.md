# EMS 代码库 Agent 指南

## 概述

本指南为 AI 编码代理（如 Sisyphus/Cursor/Copilot）提供 EMS 项目的工作规范和最佳实践。项目采用 Rust + Axum 技术栈，构建高性能多租户 SaaS 能源管理系统。

## 项目结构

```
ems/
├── apps/ems-api/          # HTTP API 服务器（唯一运行时二进制）
├── crates/
│   ├── core/
│   │   ├── domain/          # 领域模型与业务规则（TenantContext、PointValue 等）
│   │   └── api-contract/   # API 契约（DTO、ErrorCode、ApiResponse）
│   └── capability/
│       ├── auth/           # JWT 认证与 RBAC
│       ├── config/         # 配置加载（环境变量）
│       ├── storage/        # 存储层（PostgreSQL + Redis）
│       ├── ingest/          # 数据采集（MQTT 接入）
│       ├── normalize/       # 数据标准化（单位转换、数据清洗）
│       ├── pipeline/        # 数据流水线（去重、质量校验、批写、重试、背压）
│       ├── control/         # 设备控制（命令下发、回执监听）
│       ├── protocol/        # 协议支持（Modbus TCP、TCP Server、TCP Client）
│       └── telemetry/       # 可观测性（日志、追踪、指标）
├── migrations/               # 数据库迁移脚本
├── scripts/                 # 工具脚本（初始化、健康检查、验收测试）
└── web/admin/              # pure-admin-thin 管理前端
```

## 构建与测试命令

### 基本命令

```bash
# 构建项目
cargo build

# 运行 API 服务器
cargo run -p ems-api

# 运行所有测试
cargo test

# 运行单个 crate 的测试
cargo test -p ems-api
cargo test -p ems-storage
cargo test -p ems-auth
cargo test -p ems-control
```

### 运行单个测试（推荐方式）

```bash
# 方式 1：运行特定测试函数
cargo test -p ems-api realtime_returns_values

# 方式 2：运行测试模块
cargo test -p ems-api tests

# 方式 3：运行集成测试（测试包含多个步骤）
cargo test -p ems-api --test '*'

# 方式 4：按测试名称过滤（支持通配符）
cargo test -p ems-api test_name

# 方式 5：显示测试输出详细信息
cargo test -p ems-api -- --nocapture
```

### 开发环境命令

```bash
# 初始化数据库和依赖
scripts/db-init.sh
scripts/health-check.sh

# 运行验收测试（启动临时 API 实例）
scripts/mvp-acceptance.sh

# 运行稳定性测试
scripts/stability-check.sh

# 运行 RBAC 管理面回归测试
scripts/rbac-acceptance.sh
```

### 检查与格式化

```bash
# 检查代码编译
cargo check

# 格式化代码
cargo fmt

# 运行 Clippy linter
cargo clippy

# 运行 Clippy 并显示所有警告
cargo clippy -- -W clippy::all

# 修复 Clippy 警告（自动修复）
cargo clippy --fix
```

### 环境变量配置

```bash
# 必需环境变量
export EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
export EMS_JWT_SECRET="dev"
export EMS_JWT_ACCESS_TTL_SECONDS="3600"
export EMS_JWT_REFRESH_TTL_SECONDS="7200"

# 可选环境变量
export EMS_HTTP_ADDR="127.0.0.1:8080"
export EMS_REDIS_URL="redis://default:admin123@localhost:6379"
export EMS_MQTT_HOST="127.0.0.1"
export EMS_MQTT_PORT="1883"
export EMS_MQTT_USERNAME="ems"
export EMS_MQTT_PASSWORD="admin123"

# 功能开关
export EMS_INGEST="off"              # 是否启用 MQTT 数据采集
export EMS_CONTROL="off"            # 是否启用控制链路
export EMS_WEB_ADMIN="off"         # 前端启动模式（off/on/only）
export EMS_REQUIRE_TIMESCALE="off"  # 是否要求 TimescaleDB

# 带环境变量运行
cargo run -p ems-api
```

## 编码风格与规范

### 导入顺序（Imports）

```rust
// 标准导入顺序：
// 1. 标准库 std/core/alloc
// 2. 外部依赖（workspace 或第三方）
// 3. 本地模块（crate::）
// 4. 模块声明（use crate::module::）

use std::sync::Arc;           // 标准库
use axum::{Router, Json};        // 外部依赖
use crate::AppState;          // 本地模块
use crate::handlers;         // 本地模块（子路径）
```

### 类型命名规范

#### 结构体（Struct）

```rust
// 可见性：public 结构体使用 PascalCase
#[derive(Debug, Clone)]
pub struct PointValue {
    pub tenant_id: String,           // snake_case 字段名
    pub project_id: String,
    pub point_id: String,
}

// 序列化：API DTO 使用 camelCase
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    pub data: Option<T>,
    pub error: Option<ApiError>,
}
```

#### 枚举（Enum）

```rust
// 可见性：public 枚举使用 PascalCase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModbusDataType {
    Int16,    // PascalCase 变体
    Uint16,
    #[default]
    Int32,
}

// 序列化：枚举值使用 PascalCase 或 snake_case
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModbusFunctionCode {
    ReadCoils = 1,
    ReadHoldingRegisters = 3,
}
```

#### 函数命名

```rust
// 使用 snake_case
pub async fn list_projects(ctx: &TenantContext) -> Result<Vec<ProjectRecord>>

// 转换函数：from/to_<type>
pub fn from_request(req: &CreateRequest) -> ProjectRecord {
    ProjectRecord { ... }
}

// 验证函数：normalize_<field>
pub fn normalize_required(value: String, field: &str) -> Result<String, Response>
pub fn normalize_optional(value: Option<String>, field: &str) -> Result<Option<String>, Response>
```

#### Trait 方法命名

```rust
// CRUD 操作：list/find/create/update/delete
pub trait DeviceStore: Send + Sync {
    async fn list_devices(&self, ctx: &TenantContext, project_id: &str)
        -> Result<Vec<DeviceRecord>, StorageError>;
    async fn find_device(&self, ctx: &TenantContext, project_id: &str, device_id: &str)
        -> Result<Option<DeviceRecord>, StorageError>;
    async fn create_device(&self, ctx: &TenantContext, record: DeviceRecord)
        -> Result<DeviceRecord>, StorageError>;
    async fn update_device(&self, ctx: &TenantContext, project_id:str, device_id: &str, update: DeviceUpdate)
        -> Result<Option<DeviceRecord>, StorageError>;
    async fn delete_device(&self, ctx: &TenantContext, project_id: &str, device_id: &str)
        -> Result<bool, StorageError>;
}
```

### 错误处理规范

```rust
// 使用 thiserror 定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("token expired")]
    TokenExpired,
    #[error("internal error: {0}")]
    Internal(String),
}

// 使用 Result<T, Error> 模式
pub async fn get_project(
    &self,
    ctx: &TenantContext,
    project_id: &str,
) -> Result<Option<ProjectRecord>, StorageError> {
    let project = self.db_query(...).await?;
    Ok(project)
}

// 错误转换：From trait 实现错误转换
impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

// HTTP 响应错误转换
use crate::utils::response::storage_error;
Err(err) => return storage_error(err)
```

### 模块文档规范

```rust
//! 模块级文档（//!）
//!
//! # 标题
//!
//! ## 描述
//!
//! 简要描述模块职责和使用场景。
//!
//! ## 架构设计
//!
//! ```text
//! 架构图或流程图
//! ```
//!
//! ## 核心功能
//!
//! 1. 功能 A：描述
//! 2. 功能 B：描述
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use module_name::StructName;
//!
//! let instance = StructName::new();
//! instance.method().await?;
//! ```

// 结构体文档（///）
/// 结构体说明
///
/// ## 字段说明
///
/// - `field_a`：字段 a 的说明
/// - `field_b`：字段 b 的说明
///
/// ## 设计原则
///
/// - 原则 A：说明
/// - 原则 B：说明
#[derive(Debug, Clone)]
pub struct AppState {
    pub auth: Arc<AuthService>,
    pub db_pool: Option<sqlx::PgPool>,
}

// 函数文档（///）
/// 函数说明
///
/// ## 参数
///
/// - `param1`：参数 1 的说明
/// - `param2`：参数 2 的说明
///
/// ## 返回值
///
/// - `Ok(T)`：成功时的返回值说明
/// - `Err(E)`：失败时的错误类型说明
///
/// ## 示例
///
/// ```rust,ignore
/// let result = function_name(arg1, arg2).await?;
/// ```
///
/// ## 错误情况
///
/// - `Error1`：错误 1 的触发条件
/// - `Error2`：错误 2 的触发条件
pub async fn function_name(
    &self,
    param1: String,
    param2: i32,
) -> Result<SomeType, StorageError> {
    // 实现
}
```

### 测试编写规范

```rust
// 集成测试使用 #[cfg(test)] mod tests
#[cfg(test)]
mod tests {
    use super::*;

    // 单元测试：简单函数
    #[test]
    fn tenant_context_builds() {
        let ctx = TenantContext::new(
            "tenant-1",
            "user-1",
            vec!["admin".to_string()],
            vec![],
            None,
        );
        assert_eq!(ctx.tenant_id, "tenant-1");
    }

    // 异步测试：使用 tokio::test
    #[tokio::test]
    async fn login_success() {
        // 测试实现
    }

    // 测试构建辅助函数：创建测试用 AppState
    fn build_state() -> AppState {
        AppState {
            auth: Arc::new(AuthService::new(...)),
            db_pool: None,
            // ...
        }
    }
}

// 测试组织原则：
// 1. 测试文件：每个 crate 的 tests/ 目录（如 storage/tests/）
// 2. 测试函数：描述性强，明确测试目标
// 3. 断言：使用 assert_eq! / assert! / assert_ne! 等
// 4. 异步测试：使用 #[tokio::test]
// 5. 集成测试：模拟完整业务流程
```

### 异步编程规范

```rust
// 使用 async/await
use async_trait::async_trait;

// Trait 定义：所有方法都是 async
#[async_trait]
pub trait DeviceStore: Send + Sync {
    async fn list_devices(&self, ctx: &TenantContext, project_id: &str)
        -> Result<Vec<DeviceRecord>, StorageError>;
}

// 实现：impl 内部也使用 async
impl DeviceStore for PgDeviceStore {
    async fn list_devices(&self, ctx: &TenantContext, project_id: &str)
        -> Result<Vec<DeviceRecord>, StorageError> {
        let rows = sqlx::query!(...).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(...).collect())
    }
}

// 并发：使用 Arc<dyn Trait> 支持多线程
let store: Arc<dyn DeviceStore> = Arc::new(PgDeviceStore::new(pool));
tokio::spawn(async move {
    let result = store.list_devices(&ctx, "project-1").await;
});
```

### 日志与追踪规范

```rust
// 使用 tracing 进行结构化日志
use tracing::{info, warn, error, instrument};

// 简单日志：直接输出信息
info!("service started");

// 带字段的日志：使用 target 和字段
info!(
    target: "ems.control",
    tenant_id = %tenant_id,
    project_id = %project_id,
    command_id = %command_id,
    "command_issued"
);

// 带仪器化的函数：#[instrument] 自动添加 tracing span
#[instrument(skip(state))]
pub async fn handle_request(state: &AppState, ...) {
    info!("handling request");
    // 实现
}

// 错误日志：带错误上下文
error!(
    target: "ems.storage",
    error = %err,
    "query_failed"
);
```

### 依赖注入规范

```rust
// 使用 Arc 包装共享状态
use std::sync::Arc;

pub struct AppState {
    pub auth: Arc<AuthService>,
    pub project_store: Arc<dyn ProjectStore>,
    pub gateway_store: Arc<dyn GatewayStore>,
}

// Axum 提取器：使用 State<T>
use axum::extract::{State, Path, Query};
pub async fn handler(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Response {
    let projects = state.project_store.list_projects(&ctx, &project_id).await?;
    // ...
}

// 闭包中克隆 Arc：move 或 Arc::clone()
tokio::spawn(async move {
    let store = store.clone();
    let result = store.list_projects(&ctx, "project-1").await;
});
```

### 多租户隔离规范

```rust
// 所有存储操作必须接收 TenantContext
pub trait ProjectStore: Send + Sync {
    async fn list_projects(&self, ctx: &TenantContext)
        -> Result<Vec<ProjectRecord>, StorageError>;
}

// 使用 TenantContext 的 tenant_id 进行过滤
sqlx::query!(
    SELECT * FROM projects
    WHERE tenant_id = $1
)
.bind(ctx.tenant_id)
```

### 项目作用域验证

```rust
// 使用 ensure_project_scope 函数验证项目归属
use crate::storage::validation::ensure_project_scope;

pub async fn get_project(
    &self,
    ctx: &TenantContext,
    project_id: &str,
) -> Result<Option<ProjectRecord>, StorageError> {
    // 验证项目归属
    ensure_project_scope(ctx, project_id)?;
    
    // 查询数据
    let project = self.db_query(...).await?;
    Ok(project)
}
```

## 架构设计原则

### 模块边界

```
依赖方向（严格遵循）：
domain
  ↑
api-contract
  ↑
capability/*（auth/storage/control/telemetry）
  ↑
ems-api（仅负责装配，禁止反向依赖）
```

### Trait 抽象

```rust
// 使用 trait 定义接口，便于测试和替换实现
#[async_trait]
pub trait RealtimeStore: Send + Sync {
    async fn upsert_last_value(&self, ctx: &TenantContext, value: &PointValue)
        -> Result<(), StorageError>;
}

// 生产实现：PostgreSQL + Redis
pub struct PgMeasurementStore { ... }
impl RealtimeStore for PgMeasurementStore { ... }

// 测试实现：内存存储（用于快速测试）
pub struct InMemoryRealtimeStore { ... }
impl RealtimeStore for InMemoryRealtimeStore { ... }
```

### 仓储模式

```rust
// 分离存储逻辑到专门的 Store 结构
pub struct PgDeviceStore {
    pub pool: PgPool,
}

impl PgDeviceStore {
    // 所有 SQL 查询通过 sqlx 宏
    async fn list_devices(&self, ctx: &TenantContext, project_id: &str)
        -> Result<Vec<DeviceRecord>, StorageError> {
        let rows = sqlx::query!(
            SELECT device_id, tenant_id, project_id, name
            FROM devices
            WHERE tenant_id = $1 AND project_id = $2
        )
        .bind(ctx.tenant_id, project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| DeviceRecord {
            device_id: row.device_id,
            // ...
        }).collect())
    }
}
```

### 不可变性与业务规则

```rust
// 多租户隔离：所有存储操作验证 tenant_id
// 项目归属：项目级资源验证 project_id 属于当前租户
// 权限验证：使用 TenantContext.permissions 检查访问权限
// 级联删除：删除项目时级联删除其下所有资源
// Token 刷新：使用 refresh token 轮换机制
// 控制超时：未收到回执的命令自动流转为 timeout 状态
```

## 关键模块详解

### 认证模块（ems-auth）

```rust
// JWT 配置
pub struct JwtManager {
    pub secret: Vec<u8>,
    pub access_ttl_seconds: u64,
    pub refresh_ttl_seconds: u64,
}

// Token 类型
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_jti: String,
    pub expires_at: u64,
}

// 认证流程：
// 1. 用户登录 → 验证密码 → 签发 access/refresh token
// 2. 使用 access token 调用 API → 验证 token → 提取 TenantContext
// 3. access token 过期 → 使用 refresh token → 签发新 token 对
```

### 存储模块（ems-storage）

```rust
// 双实现模式：
// - InMemory*：用于测试和演示
// - Pg*：生产环境（PostgreSQL）

// 测试用辅助函数：
fn build_state() -> AppState {
    AppState {
        auth: Arc::new(AuthService::new(...)),
        db_pool: None, // 测试环境不使用真实数据库
        project_store: Arc::new(InMemoryProjectStore::with_default_project()),
        // ...
    }
}

// 默认数据：
// - InMemoryUserStore::with_default_admin() → admin/admin123
// - InMemoryProjectStore::with_default_project() → 默认项目
```

### 控制模块（ems-control）

```rust
// 命令状态流转：
// issued → accepted → success/failed/timeout

// MQTT 下发器：
pub trait CommandDispatcher: Send + Sync {
    async fn dispatch(&self, command: &CommandDispatch) -> Result<(), ControlError>;
}

// 回执监听：
pub fn spawn_receipt_listener(
    config: MqttReceiptListenerConfig,
    command_store: Arc<dyn CommandStore>,
    receipt_store: Aurora<dyn CommandReceiptStore>,
    audit_store: Arc<dyn AuditLogStore>,
) -> tokio::task::JoinHandle<()>
```

### 遥测模块（ems-telemetry）

```rust
// 初始化日志系统
pub fn init_tracing();

// 生成请求追踪 ID
pub fn new_request_ids() -> RequestIds {
    RequestIds {
        request_id: Uuid::new_v4().to_string(),
        trace_id: 外, // 从请求头提取
    }
}

// 指标记录：
pub fn record_raw_event();
pub fn record_write_success();
pub fn record_write_latency_ms(latency_ms: u64);
pub fn record_command_dispatch_success();
pub fn record_command_issue_latency_ms(latency_ms: u64);
```

### 前端对接规范

```rust
// API 响应格式
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

// 字段命名：camelCase（前端约定）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub access_token: String,     // accessToken
    pub refresh_token: String,   // refreshToken
    pub expires: u64,          // Unix 毫秒时间戳
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

// 动态路由：跳过空的 children
#[derive(Debug, Serialize)]
#[serde(skip_serializing_if = "Vec::is_empty")]
pub struct AsyncRoute {
    pub path: String,
    pub name: String,
    pub component: String,
    pub meta: RouteMeta,
    pub children: Vec<AsyncRoute>,  // 空时跳过
}
```

## 数据库规范

### 迁移脚本

```sql
-- 迁移文件命名：001_init.sql, 002_seed.sql, 003_assets.sql, ...
-- 所有表都包含 tenant_id 字段（多租户隔离）

-- 外键约束：使用 REFERENCES 和 ON DELETE CASCADE
CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    gateway_id TEXT NOT NULL REFERENCES gateways(gateway_id)
);

-- 索引：所有查询使用 tenant_id + project_id 复合索引
CREATE INDEX idx_devices_tenant_project ON devices(tenant_id, project_id);
```

### 查询模式

```rust
// 使用 sqlx 参数化查询防止 SQL 注入
sqlx::query!(
    SELECT * FROM devices
    WHERE tenant_id = $1 AND project_id = $2 AND gateway_id = $3
)
.bind(ctx.tenant_id, project_id, gateway_id)
    .fetch_all(&self.pool)
    .await?
```

## 协议模块

### Modbus TCP 支持

```rust
// 协议类型枚举
pub enum ModbusDataType {
    Int16, Uint16, Int32, Uint32, Float32, Float64,
}

// 功能码枚举
pub enum ModbusFunctionCode {
    ReadCoils = 1, ReadDiscreteInputs = 2,
    ReadHoldingRegisters = 3, ReadInputRegisters = 4,
}

// 点位协议详情（JSON 配置）
pub struct ModbusPointDetail {
    pub function_code: u8,
    pub register_address: u16,
    pub register_count: u16,
    pub data_type: ModbusDataType,
    pub byte_order: String, // big_endian / little_endian
}

// 设备地址配置
pub struct ModbusDeviceAddress {
    pub slave_id: u8, // 从站 ID (1-247)
}
```

### MQTT 协议

```rust
// 数据采集主题：{data_prefix}/{tenant_id}/{project_id}/{address}
// 控制命令主题：{command_prefix}/{tenant_id}/{project_id}/{command_id}
// 回执主题：{receipt_prefix}/{tenant_id}/{project_id}/{command_id}

// 接收器实现
pub struct MqttDispatcher {
    client: AsyncClient,
    command_topic_prefix: String,
    include_target_in_topic: bool,
    qos: QoS,
}
```

## 性能优化建议

### 异步并发

```rust
// 使用 tokio::spawn 并行处理
let (result1, result2) = tokio::join!(
    task1(),
    task2(),
).await;

// 使用 tokio::join! 顺序执行
let (result1, result2) = tokio::join!(
    task1(),
    task2(),
).await;
```

### 批量操作

```rust
// 批量写入减少数据库往返
pub async fn write_measurements_batch(
    &self,
    ctx: &TenantContext,
    values: Vec<PointValue>,
) -> Result<(), StorageError> {
    for chunk in values.chunks(100) {
        let tx = self.pool.begin().await?;
        for value in chunk {
            sqlx::query!(...).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}
```

### 缓存策略

```rust
// Redis 缓存实时数据（带 TTL）
pub async fn upsert_last_value(
    &self,
    ctx: &TenantContext,
    value: &PointValue,
) -> Result<(), StorageError> {
    let key = format!("tenant:{}:project:{}:point:{}:last_value", ...);
    self.client
        .set_ex(&key, &value, Some(ttl_seconds))
        .await?;
    Ok(())
}

// 在线状态缓存（短 TTL）
pub async fn update_online_status(
    &self,
    ctx: &TenantContext,
    gateway_id: &str,
) -> Result<(), StorageError> {
    let key = format!("tenant:{}:project:{}:gateway:{}:online", ...);
    self.client
        .set_ex(&key, &1, Some(60))
        .await?;
    Ok(())
}
```

## 常见任务模式

### 添加新的 API 端点

1. 在 `api-contract` 中定义请求/响应 DTO
2. 在 `handlers/` 中创建对应的 handler 函数
3. 在 `routes.rs` 中注册路由
4. 在 `middleware/auth.rs` 中配置权限检查（如需要）
5. 更新动态路由配置（如需要前端菜单）
6. 编写测试用例

### 添加新的存储操作

1. 在 `models.rs` 中定义数据模型
2. 在 `traits.rs` 中定义 trait 接口
3. 在 `in_memory/` 中实现内存版本（用于测试）
4. 在 `postgres/` 中实现 PostgreSQL 版本
5. 编写测试用例验证两种实现

### 添加新的协议支持

1. 在 `protocol/types.rs` 中定义协议类型
2. 创建协议实现文件（如 `modbus_tcp.rs`）
3. 实现 `Source` trait
4. 在 `ingest.rs` 中注册新协议类型
5. 更新网关配置格式文档

### 调试技巧

```bash
# 启用详细日志
export RUST_LOG=debug
cargo run -p ems-api

# 启用特定模块的详细日志
export RUST_LOG=ems_api=debug,sqlx=debug
cargo run -p ems-api

# 运行单个测试并查看输出
cargo test -p ems-api realtime_returns_values -- --nocapture
```

### 错误排查

1. 检查 TenantContext 是否正确传递
2. 验证 SQL 查询包含 tenant_id 过滤
3. 确认所有存储操作都返回 Result 类型
4. 检查权限检查是否配置正确
5. 使用 `cargo clippy` 检查潜在问题

## 代码质量检查

### 运行质量检查

```bash
# 运行所有检查
cargo clippy --all-targets -- -D warnings

# 格式化所有代码
cargo fmt --all

# 检查所有测试
cargo test --all-targets
```

### 常见 Clippy 警告及修复

```rust
// 避免：大型枚举上 derive Copy
// 建议：为大型枚举仅 derive Debug 和 Clone

// 避免：unwrap() 在可能失败的地方
// 建议：使用 ? 或 match 正确处理错误

// 避免：clone() 不必要的克隆
// 建议：使用引用或避免所有权转移
```

## Git 工作流

### 提交规范

```
提交格式：<范围>: <简短描述>

范围示例：
- ems-api: add MQTT command dispatch
- storage: implement Modbus TCP protocol
- docs: update AGENTS.md with build commands

提交示例：
- ems-api: add project CRUD endpoints
- storage: fix tenant isolation bug in project queries
- docs: update README with new architecture diagram
```

### 分支策略

```
main: 主分支（稳定版本）
develop: 开发分支
feature/*: 功能分支
bugfix/*: Bug 修复分支
```

---

## 附录：技术栈版本信息

- Rust: 1.92.0
- Tokio: 1.40+
- Axum: 0.7
- SQLx: 0.8
- Serde: 1.0
- PostgreSQL: 16+（可选 TimescaleDB）
- Redis: 7+
- MQTT Broker: Mosquitto 2+

---
