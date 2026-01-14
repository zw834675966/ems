# domain 使用方法

## 模块职责
- 定义领域层通用值对象与不变量。
- 提供 `TenantContext` 作为全链路必传上下文。

## 边界与约束
- 不依赖任何其他 crate。
- 不包含存储、网络或框架代码。

## 对外能力
- `TenantContext`：租户与权限上下文。

## 最小示例
```rust
use domain::TenantContext;

let ctx = TenantContext::new(
    "tenant-1",
    "user-1",
    vec!["admin".to_string()],
    vec![],
    None,
);
```

## TenantContext 要求
- 必须由上游（如鉴权层）显式传入。
- 禁止在业务模块内自行构造租户 ID。
