# ADR：关键决策记录

> 格式：背景 / 决策 / 备选方案 / 影响 / 何时复审

## ADR-001：部署与开发环境全在 WSL2
- 背景：Windows 只做 IDE 外壳，减少跨平台差异
- 决策：所有进程跑 WSL2；依赖用 docker-compose 起
- 影响：开发一致性强；对未来协作需写明步骤
- 复审：当需要 CI Linux/Windows 双环境验证时

## ADR-002：后端架构采用模块化单体（Modular Monolith）
- 决策：仅一个可执行 ems-api；模块用 crates 边界隔离
- 备选：一开始就拆微服务
- 影响：MVP 交付快；未来拆分 ingest/control 可复用接口
- 复审：当 ingest/control 需要独立扩容或部署隔离时

## ADR-003：多租户隔离策略 MVP 使用 tenant_id 应用层强制
- 备选：Postgres RLS
- 决策：先应用层 tenant_id 强制 + 统一 TenantContext
- 影响：实现快；后续可逐步升级到 RLS
- 复审：当租户数量/合规要求上升，评估上 RLS

## ADR-004：数据存储采用 Postgres + TimescaleDB（同实例）
- 元数据：普通表
- 时序：hypertable（measurement/event）
- 影响：运维简单；需要提前规划 retention/压缩策略
- 复审：当数据量导致单实例瓶颈时（写入/存储成本）

## ADR-005：Redis 作为实时状态镜像（last_value/online）
- 决策：last_value 与 online 状态走 Redis，提高实时查询性能
- 备选：全部走 Timescale/PG
- 影响：需要统一 key 规范与一致性策略
- 复审：当一致性需求提升或 Redis 成本/可用性约束变化时

## ADR-006：采集链路协议无关：RawEvent → PointValue → Pipeline
- 决策：ingest 只负责连接与收包；normalize 做映射；pipeline 做去重/质量/批写
- 备选：不同协议独立写入逻辑
- 影响：协议扩展成本显著降低
- 复审：当某协议需要特殊高性能路径时

## ADR-007：前端采用 pure-admin-thin，后端兼容其登录/刷新/动态路由契约
- 决策：提供 /login /refresh-token /get-async-routes
- 影响：降低前端改造成本；后端需维护权限码与路由元数据
- 复审：当迁移到其他前端框架或前后端分离策略变化时

## ADR-008：错误码与 API 响应结构稳定化（api-contract）
- 决策：统一 ApiResponse + ErrorCode
- 影响：前后端联调与排障效率大幅提升
- 复审：当需要兼容外部第三方 API 或 SDK 输出格式时
