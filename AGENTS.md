# 仓库指南

## 项目结构与模块组织
仓库包含文档与 Rust workspace 代码，文档主要位于根目录（例如：`00_项目总览.md`、`03_系统架构与模块边界.md`、`06_开发环境与工作流_WSL2.md`）。当前代码结构如下：
- `apps/ems-api`：Rust 可执行程序（唯一运行时二进制）。
- `crates/core/domain`：领域模型与业务不变量。
- `crates/core/api-contract`：DTO/OpenAPI/ErrorCode。
- `crates/capability/*`：能力模块（`auth`、`config`、`storage`、`telemetry`）。
- `migrations`：数据库初始化与种子数据 SQL。
- `scripts`：本地数据库初始化与依赖健康检查脚本。
- `web/admin`：pure-admin-thin 管理端（M1 最小联调已落地，M4 页面持续补齐）。

## 构建、测试与开发命令
当前仓库已有 Rust workspace 与脚本，常用命令如下：
- `cargo build` / `cargo run -p ems-api`：构建或运行 API。
- `cargo test`：运行 Rust 单元/集成测试。
- `scripts/db-init.sh`：初始化数据库（依赖 `EMS_DATABASE_URL`）。
- `scripts/health-check.sh`：检查 Postgres/Redis/MQTT 依赖（可选 `EMS_REDIS_URL`、`EMS_MQTT_HOST`、`EMS_MQTT_PORT`、`EMS_MQTT_USERNAME`、`EMS_MQTT_PASSWORD`）。
- `scripts/mqtt-simulate.sh`：模拟 MQTT 上报点位（用于 /realtime 与 /measurements 验证）。
- `scripts/control-receipt-simulate.sh`：模拟设备侧 MQTT 回执（用于控制链路闭环验证）。
- `scripts/device-emulator.sh`：模拟设备侧（订阅 commands 自动回执 receipts）。
- `scripts/mvp-acceptance.sh`：MVP 验收脚本（登录→创建资源→实时/历史→控制→审计/回执）。
- `scripts/stability-check.sh`：稳定性验证脚本（批写/异常输入/租户隔离/控制闭环）。
如后续新增脚本或 compose 文件，请在此补充，并简要说明用途。

## 编码风格与命名约定
仓库已有的工程约束：
- 严格遵循依赖方向：`domain` 无依赖；`api-contract` 不依赖 `storage/web`；业务模块只依赖 `domain`/`api-contract`（及必要 trait）。
- `ems-api` 仅负责装配与启动，其他 crate 禁止反向依赖 `ems-api`。
- handler 中禁止直接写 SQL，统一走 `storage` 层。
- 所有数据访问必须显式接收 `TenantContext`，并保证多租户隔离逻辑在链路内生效。
- 所有请求输出 `trace_id`/`request_id`，数据链路使用结构化日志（tracing）。
格式化工具尚未配置，保持与周边文件一致，变更尽量小。

## 测试指南
测试框架与覆盖率目标尚未定义。新增测试时建议按 crate 边界组织单元测试，并为关键 API 流程提供集成测试，同时在此记录运行方式。

## 提交与合并请求规范
当前目录未包含 Git 历史，无法推断提交风格。如后续初始化 Git，建议使用简洁的祈使句标题并标注范围（例如：`ems-api`、`storage`、`docs`）。PR 需包含变更说明、验证步骤，以及可能的数据库/数据影响。

## 安全与配置提示
运行期配置应通过环境变量提供（例如：`EMS_DATABASE_URL`、`EMS_JWT_SECRET`、`EMS_JWT_ACCESS_TTL_SECONDS`、`EMS_JWT_REFRESH_TTL_SECONDS`、`EMS_REDIS_URL`、`EMS_MQTT_HOST`、`EMS_MQTT_PORT`、`EMS_MQTT_USERNAME`、`EMS_MQTT_PASSWORD`）。请勿提交敏感信息，并在主 README 中记录新增配置项。
