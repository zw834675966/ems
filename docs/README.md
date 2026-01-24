# EMS 项目文档

本文档目录包含了 EMS 项目的所有相关文档。

## 📁 目录结构

### [project-management/](./project-management/)
项目管理相关文档，包括需求、架构、任务和进度等。
- `README.md` - 项目总览
- `01_需求与范围_PRD.md` - 产品需求文档
- `02_关键决策记录_ADR.md` - 架构决策记录
- `03_系统架构与模块边界.md` - 系统架构设计
- `04_数据模型与存储设计.md` - 数据模型设计
- `05_API契约与前端对接.md` - API 接口规范
- `06_开发环境与工作流_WSL2.md` - 开发环境配置
- `07_项目执行任务.md` - 开发任务清单
- `08_最小可运行骨架.md` - MVP 实现
- `09_完成进度.md` - 项目进度跟踪
- `10_里程碑完成总结_M0-M5.md` - 里程碑总结
- `11_项目深度诊断.md` - 项目诊断报告
- `ACCEPTANCE_REPORT.md` - 验收报告
- `API_ALIGNMENT_REPORT.md` - API 对齐报告
- `COMMERCIAL_MANUAL.md` - 商业手册
- `OPERATION_MANUAL.md` - 运维手册
- `PROJECT_FLOW_DIAGRAMS.md` - 项目流程图
- `legacy/` - 旧版文档归档

### [docs-guides/](./docs-guides/)
开发指南和代理相关文档。
- `AGENTS.md` - AI 代理使用指南
- `SKILL.md` - 技能说明

### [protocols/](./protocols/)
通信协议相关文档。
- `TCP.md` - TCP 协议
- `MQTT.md` - MQTT 协议
- `Modbus TCP.md` - Modbus TCP 协议
- `架构与实施主线总结.md` - 协议架构总结

### [api/](./api/)
API 相关文档。

### [architecture/](./architecture/)
架构设计文档。

### [deploy/](./deploy/)
部署相关文档。
- `README.md` - 部署指南

### [web/](./web/)
前端相关文档。
- `TECHNICAL_DOCUMENTATION.md` - 技术文档
- `UI_REDESIGN_SUMMARY.md` - UI 重构总结
- `FRONTEND_ANALYSIS_REPORT.md` - 前端分析报告
- `README.md` - 前端 README
- `REDESIGN_COMPLETE.md` - 重构完成
- `VISUAL_GUIDE.md` - 视觉指南

## 📖 模块使用文档

各模块的使用文档保留在对应的代码目录中：

### 应用层
- [apps/ems-api/USAGE.md](../apps/ems-api/USAGE.md) - API 服务使用文档
- [apps/ems-api/src/handlers/USAGE.md](../apps/ems-api/src/handlers/USAGE.md) - 请求处理器使用文档

### 核心层
- [crates/core/domain/USAGE.md](../crates/core/domain/USAGE.md) - 领域模型使用文档
- [crates/core/api-contract/USAGE.md](../crates/core/api-contract/USAGE.md) - API 契约使用文档

### 能力层
- [crates/capability/auth/USAGE.md](../crates/capability/auth/USAGE.md) - 认证能力使用文档
- [crates/capability/config/USAGE.md](../crates/capability/config/USAGE.md) - 配置能力使用文档
- [crates/capability/control/USAGE.md](../crates/capability/control/USAGE.md) - 控制能力使用文档
- [crates/capability/ingest/USAGE.md](../crates/capability/ingest/USAGE.md) - 采集能力使用文档
- [crates/capability/normalize/USAGE.md](../crates/capability/normalize/USAGE.md) - 归一化能力使用文档
- [crates/capability/pipeline/USAGE.md](../crates/capability/pipeline/USAGE.md) - 管道能力使用文档
- [crates/capability/protocol/USAGE.md](../crates/capability/protocol/USAGE.md) - 协议能力使用文档
- [crates/capability/storage/USAGE.md](../crates/capability/storage/USAGE.md) - 存储能力使用文档
- [crates/capability/telemetry/USAGE.md](../crates/capability/telemetry/USAGE.md) - 遥测能力使用文档

## 🚀 快速开始

1. **新项目成员**：阅读 [project-management/README.md](./project-management/README.md) 和 [project-management/06_开发环境与工作流_WSL2.md](./project-management/06_开发环境与工作流_WSL2.md)
2. **了解需求**：阅读 [project-management/01_需求与范围_PRD.md](./project-management/01_需求与范围_PRD.md)
3. **架构设计**：阅读 [project-management/03_系统架构与模块边界.md](./project-management/03_系统架构与模块边界.md)
4. **API 接口**：阅读 [project-management/05_API契约与前端对接.md](./project-management/05_API契约与前端对接.md)
