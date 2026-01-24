# EMS 代码库 Agent 指南

本指南为 AI 编码代理提供 EMS 项目的工作规范和最佳实践。项目采用 Rust + Axum 技术栈，构建高性能多租户 SaaS 能源管理系统。

## 构建与测试命令

### 基本命令
```bash
# 构建项目
cargo build

# 运行 API 服务器
cargo run -p ems-api

# 运行所有测试
cargo test
```

### 运行单个测试（推荐方式）
```bash
# 方式 1：运行特定测试函数
cargo test -p ems-api realtime_returns_values

# 方式 2：运行测试模块
cargo test -p ems-api tests

# 方式 3：显示测试输出详细信息
cargo test -p ems-api -- --nocapture

# 方式 4：按测试名称过滤（支持通配符）
cargo test -p ems-api test_name
```

### 检查与格式化
```bash
# 检查代码编译
cargo check

# 格式化代码
cargo fmt

# 运行 Clippy linter
cargo clippy

# 自动修复 Clippy 警告
cargo clippy --fix
```

### 前端命令（web/admin/）
```bash
# 开发服务器
pnpm dev

# 类型检查
pnpm typecheck

# 代码检查
pnpm lint

# 构建
pnpm build
```

## 编码风格与规范

### 导入顺序
```rust
// 1. 标准库 std/core/alloc
// 2. 外部依赖（workspace 或第三方）
// 3. 本地模块（crate::）

use std::sync::Arc;
use axum::{Router, Json};
use crate::AppState;
```

### 类型命名规范
```rust
// 结构体：PascalCase，字段 snake_case
#[derive(Debug, Clone)]
pub struct PointValue {
    pub tenant_id: String,
    pub project_id: String,
}

// API DTO：使用 camelCase
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    pub data: Option<T>,
}

// 函数：snake_case
pub async fn list_projects(ctx: &TenantContext) -> Result<Vec<ProjectRecord>>

// CRUD 操作：list/find/create/update/delete
pub trait DeviceStore: Send + Sync {
    async fn list_devices(&self, ctx: &TenantContext, project_id: &str)
        -> Result<Vec<DeviceRecord>, StorageError>;
}
```

### 错误处理
```rust
// 使用 thiserror 定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("internal error: {0}")]
    Internal(String),
}

// 使用 Result<T, Error> 模式
pub async fn get_project(&self, ctx: &TenantContext, project_id: &str)
    -> Result<Option<ProjectRecord>, StorageError> {
    let project = self.db_query(...).await?;
    Ok(project)
}

// 错误转换：From trait
impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(err.to_string())
    }
}
```

### 异步编程
```rust
// Trait 定义使用 #[async_trait]
#[async_trait]
pub trait DeviceStore: Send + Sync {
    async fn list_devices(&self, ctx: &TenantContext) -> Result<Vec<DeviceRecord>, StorageError>;
}

// 并发：使用 Arc<dyn Trait>
let store: Arc<dyn DeviceStore> = Arc::new(PgDeviceStore::new(pool));
tokio::spawn(async move {
    let result = store.list_projects(&ctx).await;
});
```

### 日志与追踪
```rust
// 使用 tracing
use tracing::{info, instrument};

// 简单日志
info!("service started");

// 带字段日志
info!(tenant_id = %tenant_id, "handling request");

// 函数仪器化
#[instrument(skip(state))]
pub async fn handle_request(state: &AppState, ...) {
    info!("handling request");
}
```

### 多租户隔离
```rust
// 所有存储操作必须接收 TenantContext
pub trait ProjectStore: Send + Sync {
    async fn list_projects(&self, ctx: &TenantContext) -> Result<Vec<ProjectRecord>, StorageError>;
}

// SQL 查询使用 tenant_id 过滤
sqlx::query!(
    SELECT * FROM projects WHERE tenant_id = $1
)
.bind(ctx.tenant_id)
```

## 架构设计原则

### 模块边界（严格遵循）
```
domain → api-contract → capability/* → ems-api
```

### Trait 抽象
```rust
// 使用 trait 定义接口，便于测试和替换实现
#[async_trait]
pub trait RealtimeStore: Send + Sync {
    async fn upsert_last_value(&self, ctx: &TenantContext, value: &PointValue)
        -> Result<(), StorageError>;
}
```

## 测试编写
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_context_builds() {
        let ctx = TenantContext::new("tenant-1", "user-1", vec![], vec![], None);
        assert_eq!(ctx.tenant_id, "tenant-1");
    }

    #[tokio::test]
    async fn login_success() {
        // 测试实现
    }
}
```

## 环境变量
```bash
# 必需环境变量
export EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
export EMS_JWT_SECRET="dev"

# 功能开关
export EMS_INGEST="off"
export EMS_CONTROL="off"
export EMS_WEB_ADMIN="off"
```

## 常见任务

### 添加新的 API 端点
1. 在 `api-contract` 中定义请求/响应 DTO
2. 在 `handlers/` 中创建对应的 handler 函数
3. 在 `routes.rs` 中注册路由
4. 编写测试用例

### 添加新的存储操作
1. 在 `models.rs` 中定义数据模型
2. 在 `traits.rs` 中定义 trait 接口
3. 在 `in_memory/` 中实现内存版本
4. 在 `postgres/` 中实现 PostgreSQL 版本

### 调试技巧
```bash
# 启用详细日志
export RUST_LOG=debug
cargo run -p ems-api

# 运行单个测试并查看输出
cargo test -p ems-api realtime_returns_values -- --nocapture
```

## 技术栈
- Rust: 1.92.0
- Tokio: 1+
- Axum: 0.7
- SQLx: 0.8
- PostgreSQL: 16+
- Redis: 7+
- Vue 3 + TypeScript + Vite（前端）
