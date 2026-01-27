# EMS 项目逐文件对照版系统架构与代码功能映射

> 目标：逐文件描述后端 Rust (*.rs) 与前端 `web/admin/src/views/ems/**` 的核心职责、关键函数、数据结构与调用链，便于快速定位“某段代码在系统中的作用”。

---

## 后端（Rust）逐文件说明

### apps/ems-api/src/main.rs
- 作用：API 服务主入口，装配配置、日志、存储层、MQTT 控制/采集，并启动 Axum HTTP 服务。
- 关键流程：
  - `WebAdminMode::from_env()`：读取 `EMS_WEB_ADMIN`，决定是否启动前端 dev server。
  - `spawn_web_admin()`：在 `web/admin` 目录执行 `pnpm dev`（WSL2 下 Linux 路径）。
  - `run_local_services_script()`：尝试运行 `scripts/start-local-services.sh`（本地依赖服务）。
  - `connect_pool(&config.database_url)`：创建 PG 连接池。
  - `AuthService::new(...)`：认证服务装配。
  - Storage 装配：`PgProjectStore/PgGatewayStore/...`、`InMemoryRealtimeStore/InMemoryOnlineStore`。
  - Control 装配：`MqttDispatcher::connect` / `NoopDispatcher`；`CommandService::new_with_config`。
  - Ingest 装配：`ingest::spawn_ingest(...)`；含 MQTT、Modbus/TCP 多网关管理。
  - Router：`routes::create_api_router()`；挂载在 `/` 与 `/api`；`request_context` 中间件注入 `request_id/trace_id`。
- 监听地址：来自 `AppConfig.http_addr`（默认 `127.0.0.1:8080`）。

### apps/ems-api/src/app_state.rs
- 作用：全局依赖注入容器（`AppState`），保存 Auth、各 Store、控制服务等 `Arc<dyn Trait>`。
- 使用方式：Axum `State<AppState>` 提取器。

### apps/ems-api/src/routes.rs
- 作用：集中注册 API 路由，绑定到 handlers。
- 关键映射（仅列要点）：
  - `/health|/livez|/readyz` -> `handlers::auth::{health, livez, readyz}`
  - `/login|/refresh-token|get-async-routes` -> `handlers::auth::*`
  - `/rbac/*` -> `handlers::rbac::*`
  - `/projects/*` -> `handlers::{projects,gateways,devices,points,point_mappings,realtime,measurements,commands,audit,system_logs,collection_strategy}::*`

### apps/ems-api/src/middleware/auth.rs
- 作用：认证、鉴权与请求上下文。
- 关键函数：
  - `request_context`: 生成 `request_id/trace_id` 并写入响应头。
  - `bearer_token`: 解析 Authorization: Bearer。
  - `require_tenant_context`: 校验 JWT，生成 `TenantContext`。
  - `require_project_scope`: 校验项目归属并写入 `ctx.project_scope`。
  - `require_permission/require_any_permission`: 权限校验工具。

### apps/ems-api/src/utils/response.rs
- 作用：统一错误响应与 Record -> DTO 转换。
- 关键函数：
  - `auth_error/forbidden_error/bad_request_error/not_found_error/internal_auth_error/storage_error`
  - `project_to_dto/gateway_to_dto/device_to_dto/point_to_dto/point_mapping_to_dto/command_to_dto/audit_log_to_dto/command_receipt_to_dto`

### apps/ems-api/src/utils/validation.rs
- 作用：字符串字段校验（去空格 + 非空），失败直接构造 `400` 响应。

### apps/ems-api/src/utils/audit.rs
- 作用：从 Header 提取 IP / trace_id / request_id。

### apps/ems-api/src/handlers/auth.rs
- 端点：`GET /health|/livez|/readyz`, `POST /login`, `POST /refresh-token`, `GET /get-async-routes`
- 关键逻辑：
  - `login`: 调 `AuthService::login` -> 生成 `LoginResponse` -> 异步写入审计日志。
  - `refresh_token`: 校验 refresh token -> 生成新 token -> 写入审计日志。
  - `get_async_routes`: 校验 token 后返回空数组（前端采用静态路由）。

### apps/ems-api/src/handlers/projects.rs
- 端点：`GET/POST /projects`, `GET/PUT/DELETE /projects/:id`
- 权限：`PROJECT_READ/PROJECT_WRITE`。
- 调用链：Handler -> `ProjectStore` -> DTO 转换 -> `ApiResponse`。

### apps/ems-api/src/handlers/gateways.rs
- 端点：`GET/POST/PUT/DELETE /projects/:id/gateways`, `POST /projects/:id/gateways/:gid/test`
- 权限：`ASSET_GATEWAY_READ/WRITE`。
- 关键逻辑：
  - 列表/详情：结合内存 `OnlineStore` 补充 `online/last_seen_at/last_error`。
  - `validate_protocol_config`: 校验协议 JSON 字段（modbus/tcp）。
  - `test_gateway`: modbus 连接测试，更新在线状态与错误信息。

### apps/ems-api/src/handlers/devices.rs
- 端点：`GET/POST/PUT/DELETE /projects/:id/devices`
- 权限：`ASSET_DEVICE_READ/WRITE`。
- 关键逻辑：
  - 创建设备前校验网关存在。
  - `validate_address_config`: 按协议解析 `addressConfig`（modbus/tcp）。
  - 列表/详情：从内存 OnlineStore 补充在线状态。

### apps/ems-api/src/handlers/points.rs
- 端点：`GET/POST/PUT/DELETE /projects/:id/points`
- 权限：`ASSET_POINT_READ/WRITE`。
- 关键逻辑：创建点位前校验设备存在。

### apps/ems-api/src/handlers/point_mappings.rs
- 端点：`GET/POST/PUT/DELETE /projects/:id/point-mappings`
- 权限：`ASSET_POINT_READ/WRITE`。
- 关键逻辑：
  - 创建前校验点位存在。
  - `validate_point_mapping_protocol_detail`: 针对 modbus/tcp/mqtt 校验 `protocolDetail`。
  - 更新时根据最终 `source_type` 再次校验。

### apps/ems-api/src/handlers/realtime.rs
- 端点：`GET /projects/:id/realtime`
- 权限：`DATA_REALTIME_READ`。
- 逻辑：若传 `point_id` 则查单点，否则查询项目全部实时值（内存）。

### apps/ems-api/src/handlers/measurements.rs
- 端点：`GET /projects/:id/measurements`
- 权限：`DATA_MEASUREMENTS_READ`。
- 逻辑：参数校验（from/to/limit/order/aggregation），下发到 `MeasurementStore`。

### apps/ems-api/src/handlers/commands.rs
- 端点：`GET/POST /projects/:id/commands`, `GET /projects/:id/commands/:command_id/receipts`
- 权限：`CONTROL_COMMAND_READ/ISSUE`。
- 逻辑：`create_command` 调用 `CommandService::issue_command` 下发并记录审计。

### apps/ems-api/src/handlers/audit.rs
- 端点：`GET /projects/:id/audit`
- 权限：`CONTROL_COMMAND_READ`。

### apps/ems-api/src/handlers/metrics.rs
- 端点：`GET /metrics`
- 权限：`SYSTEM_METRICS_READ`。

### apps/ems-api/src/handlers/system_logs.rs
- 端点：`GET /projects/:id/system-logs`, `GET /projects/:id/system-logs/unread`, `POST /projects/:id/system-logs/read`
- 权限：`PROJECT_READ`（当前配置）。
- 逻辑：SystemLogQuery -> Storage Query -> DTO。注意前端 API 当前未解包 ApiResponse。

### apps/ems-api/src/handlers/collection_strategy.rs
- 端点：
  - `GET /projects/:id/collection-strategies`
  - `POST /projects/:id/collection-strategies`
  - `DELETE /projects/:id/collection-strategies/:strategy_id`
  - `POST /projects/:id/collection-strategies/batch-enabled`
  - `GET /points/batch`
  - `POST /projects/:id/points/:point_id/test`
- 权限：读取 `DATA_REALTIME_READ`，写入 `ASSET_POINT_WRITE`。

### apps/ems-api/src/ingest.rs
- 作用：采集链路装配与后台任务管理。
- 关键结构与函数：
  - `PipelineHandler::handle`：WAL -> normalize -> pipeline -> online update。
  - `spawn_ingest`：初始化 Normalizer、Pipeline、MQTT Source、Modbus/TCP 管理器。
  - `ModbusProtocolManager/TcpServerProtocolManager`：周期扫描网关并创建/销毁采集任务。
  - `touch_online_from_point`：通过点位反查设备/网关并更新内存在线状态。

### crates/core/api-contract/src/lib.rs
- 作用：后端 API DTO 统一定义（`ApiResponse`, `LoginRequest`, `ProjectDto`, `CommandDto` 等）。
- 特性：`#[serde(rename_all = "camelCase")]` 与前端 TS 类型对齐。

### crates/core/domain/src/lib.rs
- 作用：`TenantContext`（多租户/权限上下文）与 `PointValue` 等领域模型出口。

### crates/capability/auth/src/lib.rs
- 作用：认证能力（登录/刷新/JWT 验证）。
- 关键方法：`AuthService::login/refresh/verify_access_token`。

### crates/capability/config/src/lib.rs
- 作用：从环境变量加载运行配置（数据库、MQTT、JWT、HTTP 地址等）。
- 注意：`EMS_HTTP_ADDR` 默认 `127.0.0.1:8080`，与 main.rs 注释不同。

### crates/capability/control/src/lib.rs
- 作用：控制命令下发（MQTT）+ 回执处理 + 超时流转。
- 关键结构：
  - `CommandService::issue_command`：创建记录、下发、记录审计、超时任务。
  - `MqttDispatcher::dispatch`：发布到命令 topic。
  - `spawn_receipt_listener`：订阅回执，写入 `command_receipts`。

### crates/capability/ingest/src/lib.rs
- 作用：MQTT 采集源实现与 TLS 文件读取（`std::fs::File::open`）。

### crates/capability/normalize/src/lib.rs
- 作用：原始事件 -> 点位值的规整化（依赖映射规则）。

### crates/capability/pipeline/src/lib.rs
- 作用：写入流水线（缓冲、去重、批量写入、背压）。
- 关键结构：`Pipeline`, `DedupState`, `PointValueWriter`。

### crates/capability/storage/src/lib.rs
- 作用：存储抽象入口，导出 traits、models、postgres/in_memory 实现。

### crates/capability/storage/src/models.rs
- 作用：所有数据模型（User/Project/Gateway/Device/Point/PointMapping/Measurement/Command/Audit/SystemLog/CollectionStrategy）。

### crates/capability/storage/src/traits.rs
- 作用：所有 Store trait 定义（含 project_scope 校验）。

### crates/capability/storage/src/error.rs
- 作用：`StorageError` 统一封装（SQLx）。

### crates/capability/storage/src/postgres/*.rs
- 作用：各资源的 SQLx 实现，直接映射表结构与 CRUD 语义。
- 示例：`postgres/project.rs` 包含项目级联删除的事务逻辑。

### crates/capability/storage/src/in_memory/*.rs
- 作用：测试/演示用内存存储实现（`RwLock<HashMap>`）。

### crates/capability/storage/src/online.rs / ingest_wal.rs / token_blacklist.rs
- 作用：实时值、在线状态、WAL 存储。

### crates/capability/telemetry/src/lib.rs
- 作用：指标计数器与 tracing 初始化（`AtomicU64` + `EnvFilter`）。

---

## 前端（Vue 3）逐视图说明

> 所有 EMS 视图均位于 `web/admin/src/views/ems/**`，核心交互结构为：用户选择项目 -> 触发 API -> Pinia/本地 state 更新 -> Element Plus 组件渲染。

### web/admin/src/views/ems/projects/index.vue
- 作用：项目列表与 CRUD。
- 关键状态：`useCrud(listProjects, deleteProject)`, `form{name, timezone}`。
- 调用链：按钮 -> `createProject/updateProject/deleteProject` -> `fetchList()` -> 表格刷新。

### web/admin/src/views/ems/gateways/index.vue
- 作用：网关 CRUD + 协议配置 + 连接测试。
- 关键状态：`form{protocolType, modbusHost/tcp...}`、`testingGateways`。
- 调用链：
  - 新建/编辑 -> `buildProtocolConfig` -> `createGateway/updateGateway` -> `fetchList`。
  - 测试 -> `testGateway` -> 后端 Modbus 测试 -> 刷新在线状态。

### web/admin/src/views/ems/devices/index.vue
- 作用：设备 CRUD，依赖网关协议类型。
- 关键状态：`gateways`, `form{gatewayId, name, model, unitId/devId}`。
- 调用链：
  - 选择项目 -> `listDevices` + `listGateways`。
  - 提交 -> `buildAddressConfig` -> `createDevice/updateDevice`。

### web/admin/src/views/ems/points/index.vue
- 作用：点位 CRUD。
- 关键状态：`devices`, `gateways`, `form{deviceId,key,dataType}`。
- 调用链：选择项目 -> `listPoints` + `listDevices`/`listGateways` -> 提交 `createPoint/updatePoint`。

### web/admin/src/views/ems/point-mappings/index.vue
- 作用：点位映射配置（MQTT/Modbus/TCP）。
- 关键状态：`form{pointId, sourceType, protocolDetail*}`、`treeData`。
- 调用链：
  - 选择项目 -> `listPointMappings` + 点位/设备/网关 -> 构建树。
  - 提交 -> `buildProtocolDetail`/`buildTcpProtocolDetail` -> `createPointMapping/updatePointMapping`。
  - 实时值查询 -> `getRealtime`。

### web/admin/src/views/ems/realtime/index.vue
- 作用：实时点位监控。
- 关键状态：`realtimeValues: Map<pointId, value>`、`autoRefresh`。
- 调用链：
  - 项目变更 -> `loadMetadata` -> `getRealtime` -> 表格更新。
  - 自动刷新 -> `setInterval` 每 3s 调用 `getRealtime`。

### web/admin/src/views/ems/measurements/index.vue
- 作用：历史趋势查询 + 图表。
- 关键状态：`pointId`, `timeRange`, `limit`, `items`, `nextCursor`。
- 调用链：
  - 选择点位 -> `listMeasurements`（含 `from/to/cursorTsMs`）
  - 更新 ECharts 图表与表格。

### web/admin/src/views/ems/commands/index.vue
- 作用：控制命令下发与回执查看。
- 关键状态：`mode(simple/advanced)`, `targetId`, `rawJson`。
- 调用链：
  - 选择项目 -> `listPoints/listDevices` 作为目标候选。
  - 下发 -> `createCommand` -> 刷新命令列表 -> 拉取 `listCommandReceipts`。

### web/admin/src/views/ems/collection-strategies/index.vue
- 作用：采集策略批量配置。
- 关键状态：`points`, `strategies`, `selectedPointIds`, `editForm`。
- 调用链：
  - 选择项目 -> `listPoints` + `listStrategies`。
  - 批量保存 -> `upsertStrategies`。
  - 批量启停 -> `batchUpdateEnabled`。
  - 测试点位 -> `testPoint`。

### web/admin/src/views/ems/audit/index.vue
- 作用：审计日志查询。
- 关键状态：`form{from,to,limit}`。
- 调用链：点击“加载” -> `listAuditLogs` -> 表格渲染。

### web/admin/src/views/ems/rbac/users/index.vue
- 作用：RBAC 用户管理。
- 关键状态：`users`, `roles`, `createForm`, `editForm`, `rolesForm`。
- 调用链：
  - `listRbacUsers/listRbacRoles` 并行刷新。
  - 创建用户 -> `createRbacUser`。
  - 更新状态/密码 -> `updateRbacUser`。
  - 设定角色 -> `setRbacUserRoles`。

### web/admin/src/views/ems/rbac/roles/index.vue
- 作用：RBAC 角色管理。
- 关键状态：`roles`, `permissions`, `createForm`, `permsForm`。
- 调用链：
  - `listRbacRoles/listRbacPermissions` 并行刷新。
  - 创建角色 -> `createRbacRole`。
  - 设置权限 -> `setRbacRolePermissions`。
  - 删除角色 -> `deleteRbacRole`。

---

## 前后端核心调用闭环示例（命令下发）
- `web/admin/src/views/ems/commands/index.vue` -> `createCommand` -> `POST /projects/:id/commands`
- `apps/ems-api/src/handlers/commands.rs::create_command` -> `CommandService::issue_command` -> `MqttDispatcher::dispatch` -> `command_receipts` 回执写入
- 前端 `listCommandReceipts` -> 表格显示回执

---

## 待确认/潜在不一致点
- `AppConfig.http_addr` 默认值是 `127.0.0.1:8080`，与 `apps/ems-api/src/main.rs` 注释 `0.0.0.0:8080` 不一致。
- `web/admin/src/api/system-logs.ts` 未使用 `ApiResponse` 包装，但后端实际返回 `ApiResponse`。
- Vite 代理仅覆盖部分路径，`/projects/:id/*` 中的深层路径需确认是否命中代理规则或使用 `VITE_API_BASE`。
