# telemetry 使用方法

## 模块职责
- 初始化 tracing 日志。
- 生成 request_id/trace_id。
- 提供基础指标埋点（MVP）。

## 边界与约束
- 不包含业务逻辑，仅提供观测能力。

## 对外能力
- `init_tracing()`：初始化日志。
- `new_request_ids()`：生成请求追踪 ID。
- `metrics()`：访问全局指标实例。
- `record_*()`：记录 RawEvent/PointValue/写入/控制链路等指标。
- `record_write_latency_ms()`/`record_end_to_end_latency_ms()`：记录写入/端到端延迟。
 - `record_command_issued()`/`record_command_dispatch_success()`/`record_command_dispatch_failure()`：记录命令下发指标。
 - `record_command_issue_latency_ms()`：记录命令下发处理耗时。
 - `record_receipt_processed()`：记录回执处理次数。

## 最小示例
```rust
use ems_telemetry::{init_tracing, new_request_ids};

init_tracing();
let ids = new_request_ids();
println!("request_id={}, trace_id={}", ids.request_id, ids.trace_id);
```

### 指标埋点示例
```rust
use ems_telemetry::{record_raw_event, record_write_success};

record_raw_event();
record_write_success();
```

### 延迟指标示例
```rust
use ems_telemetry::{record_end_to_end_latency_ms, record_write_latency_ms};

record_write_latency_ms(12);
record_end_to_end_latency_ms(350);
```
