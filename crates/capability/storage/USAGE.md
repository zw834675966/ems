# storage 使用方法

## 模块职责
- 定义存储访问的 trait。
- 提供内存实现与 Postgres 实现。

## 边界与约束
- handler 禁止直接写 SQL，统一通过 storage 层。
- 所有访问必须显式接收 TenantContext（M0 为占位）。

## 对外能力
- `UserStore`：用户查询接口。
- `InMemoryUserStore`：本地演示实现。
- `PgUserStore`：Postgres 实现。

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
