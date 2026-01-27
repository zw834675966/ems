# EMS 项目商用就绪性审计报告 (Production Readiness Audit)

**审计日期**：2026-01-24
**审计范围**：全栈项目（Rust 后端 + Vue 3 前端）
**总体结论**：后端存储层安全性良好，但启动依赖、配置规范及前端类型安全存在阻断级或高危风险，需在正式上线前修复。

---

## 🛑 阻断级问题 (Must Fix for Production)

### 1. 启动依赖项导致的崩溃风险 (Panic Risk)
*   **文件路径**：[`crates/capability/ingest/src/lib.rs:266`](file:///home/zw/projects/ems/crates/capability/ingest/src/lib.rs#L266)
*   **问题描述**：在加载系统原生根证书（CA Certs）时使用了 `.expect("could not load platform certs")`。
*   **风险评估**：在精简版生产环境（如专用的嵌入式 Linux 发行版）中，若缺失 `ca-certificates` 库，整个采集子系统将直接崩溃且无法启动。
*   **修改建议**：将 `expect` 改为 `?` 错误传播，并在 `main` 启动阶段捕获，记录 `error!` 日志并尝试优雅退出。

### 2. 前端 API 代理配置不严谨
*   **文件路径**：[`web/admin/vite.config.ts:21`](file:///home/zw/projects/ems/web/admin/vite.config.ts#L21)
*   **问题描述**：`const apiBase = VITE_API_BASE || "http://127.0.0.1:8080"` 配置了硬编码的回退地址。
*   **风险评估**：如果生产环境环境变量未正确注入，前端请求将默认指向 `localhost`，产生难以排查的连接错误。
*   **修改建议**：移除默认值，若 `VITE_API_BASE` 缺失应在构建阶段报错。

### 3. 未闭环的动态权限路由
*   **文件路径**：[`apps/ems-api/src/handlers/auth.rs:336-344`](file:///home/zw/projects/ems/apps/ems-api/src/handlers/auth.rs#L336-L344)
*   **问题描述**：`get_async_routes` 接口当前返回空数组，注释标注为「依赖前端静态路由以避免菜单重复」。
*   **风险评估**：由于后端不返回权限过滤后的路由树，导致 RBAC 权限变更无法实时作用于前端菜单，属于功能性残留。
*   **修改建议**：应按商用标准实现路由树的下发及根据权限的过滤逻辑。

### 4. 默认凭据与弱口令安全隐患
*   **主要涉及**：`.env` 配置文件, `migrations/002_seed.sql`
*   **问题描述**：项目内置了 `admin123` 作为默认管理密码和数据库密码。
*   **风险评估**：极易形成生产部署习惯，极大地增加了勒索病毒和未授权访问的风险。
*   **修改建议**：部署脚本必须强制要求生成随机密码，且在生产初始化后强制用户修改初始密码。

---

## ⚠️ 警告与优化建议 (Should Fix)

### 1. 前端类型安全等级偏低 (Overuse of Any)
*   **涉及范围**：`web/admin/src` 目录下 100+ 处 `any` 或 `as any`。
*   **风险点**：在处理复杂的业务逻辑（如点位数据映射、时序曲线绘制）时，TypeScript 无法提供类型保护，可能导致运行时 `undefined` 白屏错误。
*   **建议**：参照后端 DTO 完善前端 `interface` 定义。

### 2. 核心异步链路的健壮性 (Robustness)
*   **涉及文件**：[`apps/ems-api/src/handlers/auth.rs:141`](file:///home/zw/projects/ems/apps/ems-api/src/handlers/auth.rs#L141) (审计日志写入), [`apps/ems-api/src/ingest.rs:158`](file:///home/zw/projects/ems/apps/ems-api/src/ingest.rs#L158) (WAL 确认)
*   **问题描述**：关键合规性数据（审计/WAL）写入失败仅打印警告，未实施重试或熔断。
*   **风险点**：在高并发场景下可能发生静默数据丢失。
*   **建议**：增加失败补偿机制或在核心组件监控中加入此类错误的计数告警。

### 3. 逻辑分层不规范 (Code Smell)
*   **涉及文件**：[`apps/ems-api/src/handlers/gateways.rs`](file:///home/zw/projects/ems/apps/ems-api/src/handlers/gateways.rs)
*   **问题描述**：`test_gateway` 和 `update_gateway` 函数过长（超过 140 行），混合了参数验证、协议转换、指标记录和状态持久化。
*   **建议**：将协议具体实现搬迁至 `ems_protocol` 或专门的 `service` 层。

---

## ✅ 模拟/测试残留清理清单

请在发布前逐项确认以下内容是否已剔除/重写：

- [ ] **Mock 处理器**：`apps/ems-api/src/handlers/system_logs.rs` 中的 `create_mock_ctx()`。
- [ ] **内存存储**：`ems_storage` 中所有 `InMemory*Store` 的调用入口必须在生产二进制中被排除（`#[cfg(test)]` 已覆盖部分，但需检查 `main.rs` 的动态选择分支）。
- [ ] **硬编码测试链接**：`web/admin/src/utils/sso.ts` 中的 `http://localhost:8848`。
- [ ] **调试日志**：确保 `vite.config.ts` 已启用 `drop_console` 配置。

---
**审计负责人**：AI Auditor (EMS Team)
