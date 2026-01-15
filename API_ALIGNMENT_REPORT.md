# API 对齐报告（后端 vs Web 管理后台）

范围：基于 `web/admin/TECHNICAL_DOCUMENTATION.md` 和当前 Rust API 契约/代码，使后端 API 契约与 `web/admin` 前端期望对齐。

## 审阅来源
- 后端路由：`apps/ems-api/src/routes.rs`
- 后端认证/异步路由：`apps/ems-api/src/handlers/auth.rs`
- 后端 DTO 契约：`crates/core/api-contract/src/lib.rs`
- 前端 API 类型：`web/admin/src/api/user.ts`，`web/admin/src/api/ems/*.ts`
- 前端认证/令牌处理：`web/admin/src/utils/auth.ts`，`web/admin/src/utils/http/index.ts`
- 前端路由映射：`web/admin/src/router/utils.ts`

## 对齐总结
- 认证端点匹配：`POST /login`，`POST /refresh-token`，`GET /get-async-routes`。
- CRUD 端点匹配：
  - `GET/POST /projects`
  - `GET/PUT/DELETE /projects/{projectId}`
  - `GET/POST /projects/{projectId}/gateways`
  - `GET/POST /projects/{projectId}/devices`
  - `GET/POST /projects/{projectId}/points`
  - `GET/POST /projects/{projectId}/point-mappings`
- 响应包装匹配：`ApiResponse { success, data, error }`。
- DTO 字段名对齐（驼峰命名）：`projectId`，`gatewayId`，`deviceId`，`pointId`，
  `sourceId`，`sourceType`，`dataType`。
- 异步路由对齐：`component` 通过 `import.meta.glob` 映射到 `/src/views/**`。

## 为对齐已应用的变更
- 更新前端认证类型使用 `expires: number`（Unix 毫秒时间戳）：
  - `web/admin/src/api/user.ts`
- 更新令牌存储将 `expires` 视为 Unix 毫秒：
  - `web/admin/src/utils/auth.ts`
- 更新 SSO 参数类型以匹配时间戳用法：
  - `web/admin/src/utils/sso.ts`
- 更新 API 文档以反映 `expires` 为 Unix 毫秒：
  - `web/admin/TECHNICAL_DOCUMENTATION.md`
- 更新模拟认证响应以返回 Unix 毫秒时间戳：
  - `web/admin/mock/login.ts`
  - `web/admin/mock/refreshToken.ts`
- 更新后端动态路由 meta.auths，按读/写权限码绑定 EMS 菜单：
  - `apps/ems-api/src/handlers/auth.rs`

## 剩余风险 / 检查项
- 动态路由缓存：如果启用了 `CachingAsyncRoutes`，localStorage 中 `async-routes` 的旧路由可能隐藏新菜单。
- 角色过滤：`meta.roles` 用于菜单可见性。确保登录响应包含预期角色（如 `admin`）以显示菜单。
- 生产环境基础 URL：Axios 默认使用相对路径。确保反向代理或同源部署，或根据需要添加 `baseURL`。

## 建议验证
1. `POST /login` 并确认 `expires` 是一个数字（Unix 毫秒）。
2. 登录 UI 加载后，`GET /get-async-routes` 返回 EMS 路由且菜单显示。
3. 调用 `GET /projects` 并从 UI 创建一个项目。

## 验证记录（当前会话）
- 前端开发服务器响应（通过模拟）：
  - `POST /login` 返回 `expires` 为数字。
  - `POST /refresh-token` 返回 `expires` 为数字。
  - `GET /get-async-routes` 返回模拟路由。
- 后端验证完成（Postgres 已启动）：
  - `POST /login` 返回 `expires` 为数字（Unix 毫秒）。
  - `GET /get-async-routes` 认证后返回 EMS 路由。
- Mock 已禁用；前端开发服务器现在代理到后端：
  - 通过 `http://127.0.0.1:8848/login` 的 `POST /login` 返回 `expires` 为数字。
  - 通过 `http://127.0.0.1:8848/get-async-routes` 的 `GET /get-async-routes` 返回顶级路由 `/ems`。
- UI 端数据验证（无浏览器自动化可用）：
  - `GET /get-async-routes` 无 Authorization 返回 `AUTH.UNAUTHORIZED`（代理到后端）。
  - 路由 `component` 路径解析到 `web/admin/src/views` 下的现有文件。
  - `admin` 无 `meta.roles` 不匹配。
- UI 渲染验证（Playwright + 系统 Chrome）：
  - 后端省略空 `children` 数组后，动态菜单渲染（EMS 菜单可见）。
- UI M1 联调验证（Playwright + 系统 Chrome）：
  - 项目/网关/设备/点位/点位映射页面完成列表与新增操作。
