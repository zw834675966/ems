---
name: ems-skill
description: EMS project scope, architecture, and implementation constraints.
---

# EMS 项目约束

本文件用于明确 EMS 项目的范围、架构与实现约束，作为设计与实现的统一准绳。

## 1. 范围与目标
- 支持设备数据采集、时序存储、实时状态、历史查询、控制下发、审计与后续告警。
- WSL2 本地一键启动形成闭环（采集→规范化→入库→查询；控制→回执→审计）。
- 后端 Rust 模块化单体，边界清晰，未来可拆服务。
- 多租户 SaaS：tenant + project 隔离从第一天生效。
- 前端采用 pure-admin-thin，后端提供兼容登录/刷新/动态路由的契约。

## 2. 非目标（明确不做）
- MVP 阶段不做：多区域部署、复杂计费、全量规则引擎、全协议覆盖（优先 MQTT）。
- 暂不做：K8s 生产化落地（先占位，但不投入实现）。

## 3. 技术与运行环境约束
- Windows 仅作为 IDE 外壳（VSCode Remote WSL），所有进程运行在 WSL2。
- 依赖组件：Postgres+Timescale、Redis、MQTT Broker。
- apps/ems-api 为唯一可执行，其他 crate 只提供能力与领域实现。

## 4. 架构与模块边界
- Monorepo：后端为 Rust workspace（apps + crates），前端为 web/admin。
- 模块分层：
  - core：domain（实体/值对象/业务规则）、api-contract（DTO/OpenAPI/ErrorCode）。
  - capability：auth/storage/ingest/normalize/pipeline/control/alarm/scheduler/telemetry/config。
- 依赖方向硬规则：
  - domain 不依赖任何 crate。
  - api-contract 不依赖 storage/web。
  - 业务模块只依赖 domain/api-contract（以及必要的 trait）。
  - ems-api 负责装配与启动，其他 crate 禁止反向依赖 ems-api。

## 5. 多租户与安全不变量
- 每个请求与后台任务必须携带 TenantContext：tenant_id / user_id / roles / permissions / scope(project_id?)。
- tenant_id 不写入 URL，从 JWT/Context 提取，避免串租。
- project_id 出现在 URL 时，必须校验归属当前 tenant。

## 6. 数据流与控制流约束
- 数据流（采集）：
  Source(MQTT/Modbus/TCP) → RawEvent → normalize → PointValue → pipeline →
  Timescale(measurement) 与 Redis(last_value/online)。
- 控制流：
  API → auth/permission → command service → dispatcher → receipt → audit/event。

## 7. 开发规范与实现约束
- handler 中禁止直接写 SQL，统一走 storage。
- 任何访问数据的方法必须显式接收 TenantContext。
- trace_id/request_id 每个请求都打印，关键链路使用结构化日志。
