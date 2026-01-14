# telemetry 使用方法

## 模块职责
- 初始化 tracing 日志。
- 生成 request_id/trace_id。

## 边界与约束
- 不包含业务逻辑，仅提供观测能力。

## 对外能力
- `init_tracing()`：初始化日志。
- `new_request_ids()`：生成请求追踪 ID。

## 最小示例
```rust
use ems_telemetry::{init_tracing, new_request_ids};

init_tracing();
let ids = new_request_ids();
println!("request_id={}, trace_id={}", ids.request_id, ids.trace_id);
```
