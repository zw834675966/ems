# EMS 项目商用部署验收报告（M0-M5）

- 生成日期：2026-01-16
- 验收对象：`/home/zw/projects/ems`
- 代码版本：`feaf3ffca022c0de5f096211de886fd64a07cbeb`（验收时工作区存在未提交变更）
- 验收口径：按“可商用部署的基本要求”对后端与管理端做基线验收

## 1. 验收结论

结论：通过（整改完成，复验通过）。

说明：
- 功能性（MVP 口径）已通过：后端一键验收、稳定性回归、RBAC 回归均通过；管理端完成登录与 EMS 项目页加载抽样验证。
- 商用部署基线阻断项（P0）已修复：账号口令已改为强哈希存储与校验，并支持旧数据的登录时升级迁移。

## 2. 验收范围与依据

范围：
- 后端：`apps/ems-api`（认证/RBAC、项目内资源 CRUD、采集与查询、控制链路、审计、/livez(/health)、/readyz、/metrics）
- 能力 crates：`crates/core/*`、`crates/capability/*`
- 数据库/依赖：Postgres（含 Timescale 规划）、Redis、MQTT
- 管理端：`web/admin`（pure-admin-thin）

依据（仓库文档）：
- `01_需求与范围_PRD.md`
- `02_关键决策记录_ADR.md`
- `03_系统架构与模块边界.md`
- `04_数据模型与存储设计.md`
- `05_API契约与前端对接.md`
- `07_项目执行任务.md`
- `10_里程碑完成总结_M0-M5.md`

## 3. 验收环境

运行与工具版本（本次会话）：
- Rust：`rustc 1.92.0` / `cargo 1.92.0`
- Postgres：`16.11`（本次连接实例未安装 timescaledb 扩展）
- Redis：`7.0.15`
- Mosquitto：`2.0.18`
- Node.js：`v24.12.0` / pnpm：`10.15.1`

关键运行配置（来源：`crates/capability/config/src/lib.rs` 与 `EMS_JWT_SECRET.md`）：
- 必需：`EMS_DATABASE_URL`、`EMS_JWT_SECRET`、`EMS_JWT_ACCESS_TTL_SECONDS`、`EMS_JWT_REFRESH_TTL_SECONDS`
- 可选/推荐：`EMS_REDIS_URL`、`EMS_REDIS_LAST_VALUE_TTL_SECONDS`、`EMS_REDIS_ONLINE_TTL_SECONDS`
- MQTT：`EMS_MQTT_HOST`、`EMS_MQTT_PORT`、`EMS_MQTT_USERNAME`、`EMS_MQTT_PASSWORD`、topic 前缀相关
- 开关：`EMS_INGEST`、`EMS_CONTROL`（默认 off）

## 4. 已执行验收项与结果（证据）

后端构建与单测：
- `cargo test --workspace`：通过。

依赖健康检查与 DB 初始化：
- `scripts/health-check.sh`：通过（Postgres/Redis/MQTT）。
- `scripts/db-init.sh`：通过（migrations 可重入）。
- Postgres 扩展检查（MCP postgres）：仅 `plpgsql`；未安装 `timescaledb`，导致 `migrations/004_timescale.sql` 打印 “skip hypertable”。

端到端（E2E）与回归脚本：
- `scripts/mvp-acceptance.sh`：通过（登录→创建资源→MQTT 上报→realtime/measurements→控制→回执→审计→/metrics 断言）。
- `scripts/stability-check.sh`：通过（批写、异常输入丢弃、租户隔离 403、控制闭环）。
- `scripts/rbac-acceptance.sh`：通过（角色/权限最小回归，operator 只读验证，禁用用户验证）。

管理端抽样（MCP Playwright）：
- 访问 `http://127.0.0.1:8848` 完成登录（admin/admin123），跳转至首页并导航到 `#/ems/projects` 成功渲染项目列表页。
- 证据截图：`artifacts/ems-web-admin-projects.png`

## 5. 商用部署基线审计（结论与评语）

### 5.1 多租户隔离
通过。
- 设计：TenantContext 贯穿 handler 与 storage（例如 `crates/capability/storage/src/postgres/project.rs` 强制 `tenant_id` 过滤）。
- 验证：`scripts/stability-check.sh` 已断言跨租户访问返回 `403`。

### 5.2 认证与账号安全
通过（整改完成）。
- 已修复：口令使用 Argon2id 强哈希存储与校验；seed 不再写明文；支持旧明文存储在登录成功时自动升级为哈希。

### 5.3 授权与审计
通过（MVP 口径），商用需加强。
- 授权：RBAC 权限码已在核心接口落地（见 `05_API契约与前端对接.md` 表格与 handlers 中的 `require_permission`）。
- 审计：控制回执写入审计（`CONTROL.COMMAND.RECEIPT`）已在 E2E 中验证。
- 建议：补齐“登录/刷新/关键 CRUD/权限变更”的审计与留存策略（商用合规通常需要）。

### 5.4 可观测性
通过（基线整改完成；生产可继续增强）。
- 已具备：`x-request-id` / `x-trace-id` 注入（`apps/ems-api/src/middleware/auth.rs`）、tracing 初始化（`crates/capability/telemetry/src/lib.rs`）、/metrics 快照（`apps/ems-api/src/handlers/metrics.rs`）。
- 已修复：`GET /metrics` 已增加鉴权与权限控制（`SYSTEM.METRICS.READ`），避免匿名访问泄露运行信息。
- 建议：仍建议仅内网开放或网段限制；若要接 Prometheus，建议提供 Prometheus exposition 格式或接入 exporter。

### 5.5 可靠性与运维
通过（基线整改完成；生产可继续增强）。
- 依赖编排：提供 `docker-compose.yml`（Postgres/Redis/MQTT）与脚本化检查。
- 不足：
  - 已补齐后端 Dockerfile/镜像构建（多阶段构建）。
  - 已补齐就绪探针（readiness）/存活探针（liveness）。
  - Timescale 依赖支持可选 fail-fast（`EMS_REQUIRE_TIMESCALE=on` 时启动与 db-init 检查扩展）。

## 6. 风险清单与整改建议（按优先级）

P0（阻断商用上线，必须修复）：
1) 口令安全：改为强哈希存储（Argon2id），并提供用户迁移策略（已落地：登录时检测并升级哈希）。

P1（建议在试点/生产前完成）：
1) /metrics 暴露面：已增加鉴权并区分 liveness/readiness；仍建议生产做网段限制。
2) 错误信息泄露：已对外收口为通用错误消息，对内保留结构化日志。
3) TLS 与反向代理：已补充生产拓扑建议与安全头策略，并提供 admin 容器反代示例配置。
4) Token 体系：已补齐 refresh token rotation 的失效策略（服务端绑定 refresh_jti，旧 refresh 自动失效）。
5) Timescale 依赖：已支持可选强依赖检查（`EMS_REQUIRE_TIMESCALE=on` 时 fail-fast）；retention/压缩/分区策略建议后续按数据量规划落地。

P2（可排期优化）：
1) 验收脚本数据清理：已增加清理逻辑，避免污染环境。
2) 后端发布形态：已提供 `apps/ems-api/Dockerfile`（多阶段构建）与部署补充说明（端口、env、健康探针、日志）。

## 7. 放行条件（复验标准）

满足以下条件后建议复验并放行试点：
- 完成 P0：口令哈希存储与校验上线，并验证 `scripts/rbac-acceptance.sh`、`scripts/mvp-acceptance.sh`、`scripts/stability-check.sh` 仍通过。（已完成）
- 完成至少 2 项 P1（/metrics 收口 + 错误信息收口）并更新 `README.md` 的生产部署说明。（已完成）
