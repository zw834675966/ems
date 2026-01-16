# control 使用方法

## 模块职责
- 提供命令下发服务（创建命令、调用下发器、写审计）。
- 提供 Dispatcher 抽象，支持 MQTT 等下发方式。

## 对外能力
- `CommandService`：命令下发服务。
- `CommandDispatcher`：命令下发器接口。
- `NoopDispatcher`：占位实现。
- `MqttDispatcher`：MQTT 下发实现。
- `spawn_receipt_listener`：MQTT 回执订阅与写入。

## 最小示例
```rust
use ems_control::{CommandRequest, CommandService, NoopDispatcher};
use ems_storage::{InMemoryAuditLogStore, InMemoryCommandStore};
use domain::TenantContext;
use std::sync::Arc;

let command_store = Arc::new(InMemoryCommandStore::new());
let audit_store = Arc::new(InMemoryAuditLogStore::new());
let dispatcher = Arc::new(NoopDispatcher::default());
let service = CommandService::new(command_store, audit_store, dispatcher);

let ctx = TenantContext::new("tenant-1", "user-1", vec![], vec![], Some("project-1".to_string()));
let req = CommandRequest {
    project_id: "project-1".to_string(),
    target: "demo-target".to_string(),
    payload: serde_json::json!({"key":"value"}),
    issued_at_ms: 1_700_000_000_000,
};

// 在异步上下文中调用：
// let command = service.issue_command(&ctx, req).await?;
```

### MQTT 说明
- 命令主题（默认）：`{command_topic_prefix}/{tenant_id}/{project_id}/{command_id}`
  - 可选（按 target 订阅）：`{command_topic_prefix}/{tenant_id}/{project_id}/{target}/{command_id}`（对应 `EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET=on`）
- 回执主题：`{receipt_topic_prefix}/{tenant_id}/{project_id}/{command_id}`
  - 兼容：允许在 `{project_id}` 与 `{command_id}` 之间插入额外层级（例如 target/device 等），服务端会取最后一段作为 command_id
- 回执 payload：`{ "status": "success|failed", "message": "...", "tsMs": 1700000000000 }`

### 设备侧回执建议
- `status` 为字符串，服务端会直接写回 `command.status`；建议使用稳定枚举：`accepted`/`success`/`failed`/`timeout`。
- `message` 可选，放置失败原因或执行信息。
- `tsMs` 可选；缺省时服务端使用接收时间。

示例：
```json
{
  "status": "success",
  "message": "applied",
  "tsMs": 1700000000000
}
```

### 端到端示例（MQTT）
命令下发由服务端发布，设备侧订阅并回执：

- 命令主题：`ems/commands/tenant-1/project-1/command-123`
- 命令 payload（服务端发布）：`{"action":"set","value":42}`

- 回执主题：`ems/receipts/tenant-1/project-1/command-123`
- 回执 payload（设备侧发布）：`{"status":"success","message":"applied","tsMs":1700000000000}`
