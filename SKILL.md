# EMS 项目执行约束（给 Codex 的硬性指令）

你是 EMS（Energy Management System）项目的工程协作者。目标是在约束内实现可运行、可测试、可维护、可演进的工程模块。若与其他文档冲突，以本文件为准。

## 1. 核心目标（必须达成）
- 设备数据采集、时序存储、实时状态、历史查询、控制下发、审计与后续告警的完整链路。
- WSL2 本地一键启动形成闭环（采集→规范化→入库→查询；控制→回执→审计）。
- 后端 Rust 模块化单体，边界清晰，未来可拆服务。
- 多租户 SaaS：tenant + project 从第一天生效。
- 前端 pure-admin-thin，对接登录/刷新/动态路由 API 契约。

## 2. 非目标（MVP 阶段禁止）
- 多区域部署、复杂计费、全量规则引擎、全协议覆盖（优先 MQTT）。
- K8s 生产化落地（只允许结构占位，不投入实现）。

## 3. 环境与依赖约束
- Windows 仅作为 IDE 外壳（VSCode Remote WSL），所有进程运行在 WSL2。
- 依赖组件：Postgres + TimescaleDB、Redis、MQTT Broker。
- `apps/ems-api` 是唯一可执行；其他 crate 只提供能力与领域实现。

## 4. 架构与模块边界（强约束）
- Monorepo：后端 Rust workspace（apps + crates），前端 `web/admin`。
- 分层：
  - core：`domain`（实体/值对象/业务不变量）、`api-contract`（DTO/OpenAPI/ErrorCode）。
  - capability：`auth/storage/ingest/normalize/pipeline/control/alarm/scheduler/telemetry/config`。
- 依赖方向：
  - `domain` 不依赖任何 crate。
  - `api-contract` 不依赖 `storage/web`。
  - 业务模块只依赖 `domain/api-contract`（和必要 trait）。
  - `ems-api` 仅负责装配与启动，其他 crate 禁止反向依赖 `ems-api`。

## 5. 多租户与安全不变量（必须全链路生效）
- 每个请求与后台任务携带 `TenantContext`：
  `tenant_id / user_id / roles / permissions / scope(project_id?)`。
- `tenant_id` 禁止出现在 URL；必须从 JWT/Context 提取。
- `project_id` 出现在 URL 时，必须校验归属当前 tenant。

## 6. 数据流与控制流约束
- 数据流（采集）：
  `Source(MQTT/Modbus/TCP) → RawEvent → normalize → PointValue → pipeline →`
  `Timescale(measurement)` 与 `Redis(last_value/online)`。
- 控制流：
  `API → auth/permission → command service → dispatcher → receipt → audit/event`。

## 7. 实现与开发规范（必须）
- handler 禁止直接写 SQL，统一走 `storage` 层。
- 所有数据访问方法必须显式接收 `TenantContext`。
- 每个请求输出 `trace_id/request_id`，关键链路用结构化日志（tracing）。
- 变更优先最小化，保持与周边文件一致。

## 8. 模块完成与交付（Definition of Done）
- 模块完成必须同时满足：
  - 提供 `USAGE.md`（或等价文件），说明职责、对外能力、最小示例、TenantContext/权限前置条件。
  - 至少有单元测试或集成测试，且通过。
- 未通过测试的模块不得作为其他模块依赖。
- 注释与模块级文档后置：功能完成 + 测试通过后再补充。

## 9. 行为准则
- 优先保证：能跑、可测、可演进。
- 禁止：过度抽象、为“未来可能”提前引入复杂机制。
- 所有设计必须能在当前 PRD/约束下自证合理性。

## 10. 输出要求（对 AI 的明确指令）
- 生成代码/设计时明确所属模块与依赖边界。
- 说明是否满足 DoD 与核心约束。
- 默认采用 Rust + async + trait + 明确边界。
