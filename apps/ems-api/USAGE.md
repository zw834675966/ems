# ems-api 使用方法

## 模块结构

```
src/
├── main.rs              # 启动入口：装配依赖、初始化服务、启动 HTTP 服务器
├── routes.rs            # 路由定义：集中管理所有 API 路由
├── ingest.rs            # 采集链路装配：MQTT 数据采集处理
├── handlers/             # HTTP 处理器：按业务域分组
│   ├── mod.rs
│   ├── auth.rs         # 认证：health/livez/readyz、login、refresh_token、get_async_routes
│   ├── projects.rs     # 项目 CRUD
│   ├── gateways.rs     # 网关 CRUD
│   ├── devices.rs      # 设备 CRUD
│   ├── points.rs       # 点 CRUD
│   ├── point_mappings.rs # 点映射 CRUD
│   ├── realtime.rs     # 实时查询
│   └── measurements.rs # 历史查询
├── middleware/          # 中间件：认证、授权、请求追踪
│   ├── mod.rs
│   └── auth.rs         # request_context、bearer_token、require_tenant_context、require_project_scope
└── utils/               # 工具函数
    ├── mod.rs
    ├── response.rs      # 错误响应、DTO 转换函数
    └── validation.rs    # 输入验证：normalize_required、normalize_optional
```

## 模块职责
- **main.rs**：启动 HTTP 服务并装配各 capability（auth、config、storage、telemetry）
- **routes.rs**：集中管理所有 API 路由定义
- **ingest.rs**：装配 MQTT 数据采集链路（配置驱动的采集任务）
- **handlers/**：HTTP 请求处理器，按业务域分组（auth、projects、gateways、devices、points、point_mappings、realtime、measurements）
- **middleware/**：认证、授权和请求追踪中间件
- **utils/**：响应处理和输入验证工具函数

**核心功能**：
- 提供 M0 认证链路（JWT）与动态路由接口
- 支持 MQTT 数据采集（可通过环境变量 `EMS_INGEST` 启用/禁用）
- 输出 `x-request-id` / `x-trace-id` 响应头，便于链路追踪
- 多租户隔离和项目级权限控制

## 运行方式
应用会自动读取项目根目录 `.env`（若存在）。

### 环境变量配置

**必填环境变量**：
- `EMS_DATABASE_URL`：PostgreSQL 连接串（需先执行 migrations/seed）
- `EMS_JWT_SECRET`：JWT 签名密钥
- `EMS_JWT_ACCESS_TTL_SECONDS`：访问令牌有效期（秒）
- `EMS_JWT_REFRESH_TTL_SECONDS`：刷新令牌有效期（秒）

**可选环境变量**：
- `EMS_HTTP_ADDR`：HTTP 监听地址，默认 `127.0.0.1:8080`
- `EMS_REDIS_URL`：Redis 连接串（用于实时数据缓存），默认 `redis://default:admin123@localhost:6379`
- `EMS_REDIS_LAST_VALUE_TTL_SECONDS`：last_value 过期秒数（可选，未设置或为 0 则不设置 TTL）
- `EMS_REDIS_ONLINE_TTL_SECONDS`：online 过期秒数（默认 60 秒）
- `EMS_MQTT_HOST`：MQTT Broker 主机，默认 `127.0.0.1`
- `EMS_MQTT_PORT`：MQTT Broker 端口，默认 `1883`
- `EMS_MQTT_USERNAME`：MQTT 用户名（可选）
- `EMS_MQTT_PASSWORD`：MQTT 密码（可选）
- `EMS_MQTT_TOPIC_PREFIX`：MQTT 根前缀，默认 `ems`
- `EMS_MQTT_DATA_TOPIC_PREFIX`：采集订阅前缀，默认 `{EMS_MQTT_TOPIC_PREFIX}/data`（主题形如 `{dataPrefix}/{tenant_id}/{project_id}/{address}`）
- `EMS_MQTT_DATA_TOPIC_HAS_SOURCE_ID`：采集 topic 是否包含 source_id（默认 `off`；开启后主题形如 `{dataPrefix}/{tenant_id}/{project_id}/{source_id}/{address}`）
- `EMS_MQTT_COMMAND_TOPIC_PREFIX`：控制下发主题前缀，默认 `{EMS_MQTT_TOPIC_PREFIX}/commands`
- `EMS_MQTT_RECEIPT_TOPIC_PREFIX`：回执订阅主题前缀，默认 `{EMS_MQTT_TOPIC_PREFIX}/receipts`
- `EMS_MQTT_COMMAND_QOS`：控制下发 QoS（0/1/2），默认 `1`
- `EMS_MQTT_RECEIPT_QOS`：回执订阅 QoS（0/1/2），默认 `1`
- `EMS_CONTROL_DISPATCH_MAX_RETRIES`：控制下发重试次数（默认 2，表示最多尝试 3 次）
- `EMS_CONTROL_DISPATCH_BACKOFF_MS`：控制下发重试退避毫秒（默认 200）
- `EMS_CONTROL_RECEIPT_TIMEOUT_SECONDS`：等待设备回执超时秒数（默认 30 秒；到期仍为 accepted 则自动置为 timeout）
- `EMS_INGEST`：是否启用 MQTT 数据采集（`off`/`on`/`true`/`1`），默认 `off`
- `EMS_CONTROL`：是否启用控制下发与回执订阅（默认 `off`）
- `EMS_WEB_ADMIN`：前端启动模式（`off`/`on`/`only`），默认 `off`
- `EMS_REQUIRE_TIMESCALE`：是否强依赖 timescaledb（`off`/`on`/`true`/`1`），默认 `off`

```bash
export EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
export EMS_JWT_SECRET="your-secret"
export EMS_JWT_ACCESS_TTL_SECONDS="3600"
export EMS_JWT_REFRESH_TTL_SECONDS="2592000"
export EMS_HTTP_ADDR="127.0.0.1:8080"

cargo run -p ems-api
```

前后端一键启动（开发）：
```bash
export EMS_WEB_ADMIN="on"
cargo run -p ems-api
```

仅启动前端（开发）：
```bash
export EMS_WEB_ADMIN="only"
cargo run -p ems-api
```

首次初始化：
```bash
scripts/db-init.sh
scripts/health-check.sh
```
说明：新增资产表（gateways/devices/points/point_sources）后需重新执行初始化脚本。

## 数据采集链路

`ingest.rs` 模块负责装配 MQTT 数据采集链路，根据配置自动选择数据源：

### 采集流程

```
MQTT Broker → MqttSource → Normalizer → Pipeline → StorageWriter
                            ↓                        ↓
                     (根据点映射归一化)    (写入实时和历史表)
```

### 配置说明

当 `EMS_INGEST=on` 时启用 MQTT 采集：

1. **MqttSource**：连接 MQTT Broker，订阅主题：
   - 默认：`{EMS_MQTT_DATA_TOPIC_PREFIX}/{tenant_id}/{project_id}/{address}`
   - 开启 `EMS_MQTT_DATA_TOPIC_HAS_SOURCE_ID=on`：`{EMS_MQTT_DATA_TOPIC_PREFIX}/{tenant_id}/{project_id}/{source_id}/{address}`
2. **Normalizer**：根据 `point_mappings` 表配置，将原始数据归一化为 `PointValue`
3. **Pipeline**：将归一化后的数据写入存储层（实时数据 + 历史数据）

### 点映射配置

点映射表 `point_mappings` 用于将 MQTT 主题映射到具体的点位：

```json
{
  "source_id": "uuid",
  "point_id": "uuid",
  "source_type": "mqtt",
  "address": "temperature",  // MQTT 主题的最后一段
  "scale": 0.1,            // 缩放因子
  "offset": 0.0            // 偏移量
}
```

### 数据处理

- 原始 MQTT 消息 → `RawEvent`
- 根据点映射匹配 → `PointValue`（应用 scale 和 offset）
- 写入 `realtime_store`（Redis）：最新值
- 写入 `measurement_store`（PostgreSQL）：历史记录

### 启用采集

```bash
export EMS_INGEST="on"
export EMS_MQTT_HOST="127.0.0.1"
export EMS_MQTT_PORT="1883"
export EMS_MQTT_USERNAME="ems"
export EMS_MQTT_PASSWORD="admin123"
export EMS_MQTT_TOPIC_PREFIX="ems"

cargo run -p ems-api
```

## 默认账号（数据库 seed）
仅用于 M0 演示，需先执行 seed 脚本：
- 用户名：`admin`
- 密码：`admin123`
初始化命令：
```bash
scripts/db-init.sh
```

## 依赖默认账号（本地）
- Redis：`default` / `admin123`
- MQTT（Mosquitto）：`ems` / `admin123`

## 接口说明

### 公开端点（无需认证）

- `GET /health`：健康检查（等价于 `GET /livez`），返回 `{"ok": true}`
- `GET /livez`：liveness 探针，返回 `{"ok": true}`
- `GET /readyz`：readiness 探针（检查 Postgres 连接）
- `POST /login`：用户登录，返回 access/refresh token（兼容 `/api/login`）
- `POST /refresh-token`：刷新 token（兼容 `/api/refresh-token`）

### 私有端点（需 Bearer token 认证）

- `GET /get-async-routes`：动态路由配置，根据用户权限返回前端路由（兼容 `/api/get-async-routes`）
- `GET /metrics`：Telemetry 指标快照（需要权限 `SYSTEM.METRICS.READ`；兼容 `/api/metrics`）
- `GET /projects`：列出项目
- `POST /projects`：创建项目
- `GET /projects/{project_id}`：获取项目详情
- `PUT /projects/{project_id}`：更新项目
- `DELETE /projects/{project_id}`：删除项目
- `GET /projects/{project_id}/gateways`：列出网关
- `POST /projects/{project_id}/gateways`：创建网关
- `GET /projects/{project_id}/gateways/{gateway_id}`：获取网关详情
- `PUT /projects/{project_id}/gateways/{gateway_id}`：更新网关
- `DELETE /projects/{project_id}/gateways/{gateway_id}`：删除网关
- `GET /projects/{project_id}/devices`：列出设备
- `POST /projects/{project_id}/devices`：创建设备
- `GET /projects/{project_id}/devices/{device_id}`：获取设备详情
- `PUT /projects/{project_id}/devices/{device_id}`：更新设备
- `DELETE /projects/{project_id}/devices/{device_id}`：删除设备
- `GET /projects/{project_id}/points`：列出点
- `POST /projects/{project_id}/points`：创建点
- `GET /projects/{project_id}/points/{point_id}`：获取点详情
- `PUT /projects/{project_id}/points/{point_id}`：更新点
- `DELETE /projects/{project_id}/points/{point_id}`：删除点
- `GET /projects/{project_id}/point-mappings`：列出点映射
- `POST /projects/{project_id}/point-mappings`：创建点映射
- `GET /projects/{project_id}/point-mappings/{source_id}`：获取点映射详情
- `PUT /projects/{project_id}/point-mappings/{source_id}`：更新点映射
- `DELETE /projects/{project_id}/point-mappings/{source_id}`：删除点映射
- `GET /projects/{project_id}/realtime?pointId=`：实时数据查询（可选指定点 ID）
- `GET /projects/{project_id}/measurements?pointId=&from=&to=&limit=&cursorTsMs=&order=&bucketMs=&agg=`：历史数据查询（支持 keyset 分页与聚合）
- `GET /projects/{project_id}/commands`：列出控制命令
- `POST /projects/{project_id}/commands`：下发控制命令
- `GET /projects/{project_id}/commands/{command_id}/receipts`：查询命令回执
- `GET /projects/{project_id}/audit`：查询审计日志

### 路径兼容性

所有接口支持 `/path` 和 `/api/path` 两种前缀（例如 `/login` 和 `/api/login` 都有效）。

### 响应格式

**成功响应**：
```json
{
  "success": true,
  "data": { ... }
}
```

**错误响应**：
```json
{
  "success": false,
  "error": {
    "code": "ERROR.CODE",
    "message": "error description"
  }
}
```

### 常见错误码

| 错误码 | HTTP 状态 | 说明 |
|--------|----------|------|
| `AUTH.UNAUTHORIZED` | 401 | 认证失败（token 无效或过期） |
| `AUTH.FORBIDDEN` | 403 | 无权限访问（项目归属校验失败或缺少权限码） |
| `INVALID.REQUEST` | 400 | 请求参数错误 |
| `RESOURCE.NOT_FOUND` | 404 | 资源不存在 |
| `INTERNAL.ERROR` | 500 | 服务器内部错误 |

### 字段说明

- 所有请求/响应字段使用 camelCase（如 `accessToken`、`refreshToken`、`projectId`）
- `expires` 为毫秒时间戳（Unix ms）
- 动态路由 `meta.roles` / `meta.auths` 使用 `domain::permissions` 稳定权限码
- 叶子节点 `children` 为空时会省略该字段（`#[serde(skip_serializing_if = "Vec::is_empty")]`），避免前端菜单过滤
- 项目级接口需校验 `project_id` 归属，校验通过后写入 `TenantContext.project_scope`
- 服务端 RBAC：部分接口会校验 `TenantContext.permissions`；缺少权限码返回 `AUTH.FORBIDDEN`
- 动态路由 `component` 以 `src/views` 为基准，例如 `ems/projects/index` 对应 `src/views/ems/projects/index.vue`

## 服务端 RBAC（权限矩阵）

服务端对以下端点进行权限码校验（详情见 `05_API契约与前端对接.md`）：
- projects：`PROJECT.READ` / `PROJECT.WRITE`
- gateways：`ASSET.GATEWAY.READ` / `ASSET.GATEWAY.WRITE`
- devices：`ASSET.DEVICE.READ` / `ASSET.DEVICE.WRITE`
- points & point-mappings：`ASSET.POINT.READ` / `ASSET.POINT.WRITE`
- realtime：`DATA.REALTIME.READ`
- measurements：`DATA.MEASUREMENTS.READ`
- commands：list/receipts 需要 `CONTROL.COMMAND.READ` 或 `CONTROL.COMMAND.ISSUE`；create 需要 `CONTROL.COMMAND.ISSUE`
- audit：`CONTROL.COMMAND.READ`
- rbac/users：`RBAC.USER.READ` / `RBAC.USER.WRITE`
- rbac/roles & rbac/permissions：`RBAC.ROLE.READ` / `RBAC.ROLE.WRITE`

## 最小验证

### 登录获取 token
```bash
curl -sS -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'
```

预期响应：
```json
{
  "success": true,
  "data": {
    "accessToken": "...",
    "refreshToken": "...",
    "expires": 1704067200000,
    "username": "admin",
    "nickname": "admin",
    "avatar": "",
    "roles": ["admin"],
    "permissions": [...]
  }
}
```

### 携带 access_token 获取动态路由
```bash
curl -sS http://127.0.0.1:8080/get-async-routes \
  -H "Authorization: Bearer <access_token>"
```

### 刷新 token
```bash
curl -sS -X POST http://127.0.0.1:8080/refresh-token \
  -H "Content-Type: application/json" \
  -d '{"refreshToken":"<refresh_token>"}'
```

### CRUD 验证（创建与清理）
```bash
TOKEN_JSON=$(curl -sS -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}')

ACCESS_TOKEN=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("accessToken",""))' <<<"$TOKEN_JSON")

# 创建项目
PROJECT_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Cleanup Project","timezone":"UTC"}')
PROJECT_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("projectId",""))' <<<"$PROJECT_JSON")

# 创建网关
GATEWAY_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/gateways \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Cleanup-GW","status":"online"}')
GATEWAY_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("gatewayId",""))' <<<"$GATEWAY_JSON")

# 创建设备
DEVICE_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/devices \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"gatewayId":"'"$GATEWAY_ID"'","name":"Cleanup-Device","model":"M1"}')
DEVICE_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("deviceId",""))' <<<"$DEVICE_JSON")

# 创建点
POINT_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/points \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"deviceId":"'"$DEVICE_ID"'","key":"temp","dataType":"float","unit":"C"}')
POINT_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("pointId",""))' <<<"$POINT_JSON")

# 创建点映射
MAPPING_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/point-mappings \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"pointId":"'"$POINT_ID"'","sourceType":"mqtt","address":"topic/cleanup","scale":1.0,"offset":0.0}')
SOURCE_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("sourceId",""))' <<<"$MAPPING_JSON")

# 清理（按依赖顺序倒序删除）
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/point-mappings/$SOURCE_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/points/$POINT_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/devices/$DEVICE_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/gateways/$GATEWAY_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"

# 验证删除结果
curl -sS http://127.0.0.1:8080/projects/$PROJECT_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

预期结果：
- 项目查询返回 404（`{"success": false, "error": {"code": "RESOURCE.NOT_FOUND", "message": "not found"}}`）
- 项目内资源列表返回 403（项目已删除，归属校验失败，`{"success": false, "error": {"code": "AUTH.FORBIDDEN", "message": "forbidden"}}`）

## 链路追踪

### 请求追踪

所有请求都会自动生成 `request_id` 和 `trace_id`，并通过响应头返回：

```http
x-request-id: 550e8400-e29b-41d4-a716-446655440000
x-trace-id: 550e8400-e29b-41d4-a716-446655440001
```

### 日志结构

使用 `tracing` 框架进行结构化日志记录：

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(state))]
pub async fn handle_request(/* ... */) -> Response {
    info!("handling request");
    // ...
}
```

日志包含字段：
- `request_id`: 唯一请求标识
- `trace_id`: 分布式追踪标识
- `method`: HTTP 方法
- `path`: 请求路径
- `user_id`: 用户 ID（已认证）
- `tenant_id`: 租户 ID（已认证）
- `project_id`: 项目 ID（项目级接口）

## 测试

### 运行测试

```bash
# 运行所有测试
cargo test -p ems-api

# 运行特定测试
cargo test -p ems-api realtime_returns_values
cargo test -p ems-api measurements_returns_values

# 运行集成测试
cargo test -p ems-api --test '*'
```

### 测试策略

**单元测试**：
- `handlers/auth.rs`：Bearer token 提取逻辑测试
- `handlers/projects.rs`：项目上下文和归属验证测试

**集成测试**（`main.rs`）：
- `realtime_returns_values`：实时数据查询测试
- `measurements_returns_values`：历史数据查询测试

测试使用内存存储实现（`InMemory*Store`）进行快速测试，无需数据库。

## 依赖说明

### Workspace 依赖

```toml
[dependencies]
api-contract = { workspace = true }       # API 契约和 DTO
axum = { workspace = true }               # Web 框架
ems-auth = { workspace = true }            # 认证服务
ems-config = { workspace = true }         # 配置加载
ems-ingest = { workspace = true }         # 数据采集
ems-normalize = { workspace = true }     # 数据归一化
ems-pipeline = { workspace = true }       # 数据处理管道
ems-storage = { workspace = true }        # 存储层
ems-telemetry = { workspace = true }       # 追踪和日志
domain = { workspace = true }             # 领域模型
```

### 外部依赖

- `tokio`：异步运行时
- `uuid`：唯一标识符生成
- `serde` / `serde_json`：序列化/反序列化
- `dotenvy`：环境变量加载
- `tracing`：结构化日志和追踪

## 开发建议

### 添加新 API 端点

1. 在 `api-contract` 中定义请求/响应 DTO
2. 在 `handlers/` 中创建对应的 handler 函数
3. 在 `routes.rs` 中注册路由
4. 更新动态路由配置（如需要前端菜单）
5. 添加测试用例

### 添加新的数据源

1. 在 `ems-ingest` 中实现 `Source` trait
2. 在 `ingest.rs` 中添加新的源类型选择逻辑
3. 更新配置项（如需要）

### 调试技巧

- 设置 `RUST_LOG=debug` 或 `RUST_LOG=trace` 查看详细日志
- 使用 `x-request-id` 响应头追踪特定请求
- 检查 `TenantContext.project_scope` 确认项目归属状态
- 使用内存存储进行快速迭代测试
