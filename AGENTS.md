# 仓库指南

## 项目结构与模块组织
当前仓库主要是规划与设计文档，均位于根目录（例如：`00_项目总览.md`、`03_系统架构与模块边界.md`、`06_开发环境与工作流_WSL2.md`）。代码目录尚未落地，但架构文档中已明确规划：
- `apps/ems-api`：Rust 可执行程序（唯一运行时二进制）。
- `crates/*`：Rust workspace crates，按 `core` 与 `capability` 分组。
- `web/admin`：前端（pure-admin-thin）集成目录。

## 构建、测试与开发命令
当前未提交构建/测试脚本。代码建立后预计使用以下命令：
- `docker-compose up -d`：启动 Postgres/Timescale、Redis、MQTT（需补充 compose 文件）。
- `cargo build` / `cargo run -p ems-api`：构建或运行 API（需 Rust workspace 就绪）。
- `cargo test`：运行 Rust 单元/集成测试（需测试用例存在）。
如后续新增脚本，请在此补充，并简要说明用途。

## 编码风格与命名约定
仓库已有的工程约束：
- 严格遵循依赖方向：`domain` 无依赖；`api-contract` 不依赖 storage/web；业务模块只依赖 `domain`/`api-contract`。
- handler 中禁止直接写 SQL，统一走 `storage` 层。
- 所有数据访问必须显式接收 `TenantContext`。
- 所有请求输出 `trace_id`/`request_id`，数据链路使用结构化日志。
格式化工具尚未配置，保持与周边文件一致，变更尽量小。

## 测试指南
测试框架与覆盖率目标尚未定义。新增测试时建议按 crate 边界组织单元测试，并为关键 API 流程提供集成测试，同时在此记录运行方式。

## 提交与合并请求规范
当前目录未包含 Git 历史，无法推断提交风格。如后续初始化 Git，建议使用简洁的祈使句标题并标注范围（例如：`ems-api`、`storage`、`docs`）。PR 需包含变更说明、验证步骤，以及可能的数据库/数据影响。

## 安全与配置提示
运行期配置应通过环境变量提供（例如：`EMS_JWT_SECRET`、`EMS_JWT_ACCESS_TTL_SECONDS`、`EMS_JWT_REFRESH_TTL_SECONDS`）。请勿提交敏感信息，并在主 README 中记录新增配置项。
