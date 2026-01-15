# auth 使用方法

## 模块职责
- 登录认证与 JWT 签发/校验。
- 从 token 提取 TenantContext。

## 边界与约束
- 不直接访问数据库，依赖 `UserStore` 接口。
- 不处理 HTTP 路由，仅提供能力接口。

## 对外能力
- `AuthService`：登录、校验、刷新。
- `JwtManager`：JWT 生成与解析。
- 角色与权限码使用 `domain::permissions` 的稳定清单。

## 最小示例
```rust
use ems_auth::{AuthService, JwtManager};
use ems_storage::PgUserStore;
use std::sync::Arc;

// 在异步上下文中调用：
let user_store = Arc::new(PgUserStore::connect("postgresql://ems:admin123@localhost:5432/ems").await?);
let jwt = JwtManager::new("secret".to_string(), 3600, 2592000);
let auth = AuthService::new(user_store, jwt);
```

## TenantContext 要求
- 由 `AuthService` 从 token 中提取。
- 业务模块不得绕过该上下文。
