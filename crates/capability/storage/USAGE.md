# storage 使用方法

## 模块职责
- 定义存储访问的 trait。
- 提供内存实现与 Postgres 实现。

## 模块结构
- `models.rs`：数据模型与更新结构。
- `traits.rs`：存储接口定义。
- `validation.rs`：TenantContext 与 project_scope 校验。
- `connection.rs`：Postgres 连接池初始化。
- `in_memory/*`：内存实现（用于本地测试）。
- `postgres/*`：Postgres 实现（用于运行时）。

## 边界与约束
- handler 禁止直接写 SQL，统一通过 storage 层。
- 所有访问必须显式接收 TenantContext（M0 为占位）。
- 项目内资源操作需保证 `TenantContext.project_scope` 与 project_id 一致。

## 对外能力
- `UserStore`：用户查询接口。
- `ProjectStore`：项目 CRUD 与归属校验接口。
- `GatewayStore`：网关 CRUD 接口。
- `DeviceStore`：设备 CRUD 接口。
- `PointStore`：点位 CRUD 接口。
- `PointMappingStore`：点位映射 CRUD 接口。
- `MeasurementStore`：时序写入接口。
- `RealtimeStore`：实时 last_value 接口。
- `CommandStore`：控制命令存储接口。
- `CommandReceiptStore`：命令回执存储接口。
- `AuditLogStore`：审计日志存储接口。
- `InMemoryUserStore`：本地演示实现。
- `InMemoryProjectStore`：本地测试实现。
- `InMemoryGatewayStore`：本地测试实现。
- `InMemoryDeviceStore`：本地测试实现。
- `InMemoryPointStore`：本地测试实现。
- `InMemoryPointMappingStore`：本地测试实现。
- `InMemoryMeasurementStore`：时序写入占位实现。
- `InMemoryRealtimeStore`：实时 last_value 占位实现。
- `InMemoryCommandStore`：控制命令占位实现。
- `InMemoryCommandReceiptStore`：命令回执占位实现。
- `InMemoryAuditLogStore`：审计日志占位实现。
- `PgMeasurementStore`：Timescale/PG 时序写入实现。
- `RedisRealtimeStore`：Redis 实时 last_value 实现。
- `PgCommandStore`：控制命令 PG 实现。
- `PgCommandReceiptStore`：命令回执 PG 实现。
- `PgAuditLogStore`：审计日志 PG 实现。

## Redis 约定
- key 格式：`tenant:{tid}:project:{pid}:point:{point_id}:last_value`
- payload：`{ ts_ms, value, quality }`
- TTL：可通过 `EMS_REDIS_LAST_VALUE_TTL_SECONDS` 配置（未设置或为 0 则不设置 TTL）。
- online TTL：可通过 `EMS_REDIS_ONLINE_TTL_SECONDS` 配置（默认 60 秒）。
- `PgUserStore`：Postgres 实现。
- `PgProjectStore`：Postgres 实现。
- `PgGatewayStore`：Postgres 实现。
- `PgDeviceStore`：Postgres 实现。
- `PgPointStore`：Postgres 实现。
- `PgPointMappingStore`：Postgres 实现。

## 默认账号权限
- `InMemoryUserStore` 的默认 admin 账号使用 `domain::permissions` 中的稳定权限码。

## 演示数据
- `migrations/002_seed.sql` 已包含最小资产数据：网关、设备、点位、点位映射。

## 最小示例
```rust
use ems_storage::{InMemoryUserStore, UserStore};
use domain::TenantContext;

let store = InMemoryUserStore::with_default_admin();
let ctx = TenantContext::default();
// 在异步上下文中调用：
// let user = store.find_by_username(&ctx, "admin").await?;
```

```rust
use ems_storage::{PgUserStore, UserStore};
use domain::TenantContext;

let store = PgUserStore::connect("postgresql://ems:admin123@localhost:5432/ems").await?;
let ctx = TenantContext::default();
// let user = store.find_by_username(&ctx, "admin").await?;
```

```rust
use ems_storage::{InMemoryProjectStore, ProjectStore};
use domain::TenantContext;

let store = InMemoryProjectStore::with_default_project();
let ctx = TenantContext::new("tenant-1", "user-1", vec![], vec![], None);
// let ok = store.project_belongs_to_tenant(&ctx, "project-1").await?;
```

## 验证命令
```bash
cargo check -p ems-storage
cargo test -p ems-storage
```
