# API 契约与前端对接（pure-admin-thin + EMS）

## 1. 全局约定
- Base URL：/（兼容 /api 前缀）
- 认证：Authorization: Bearer <access_token>
- 响应结构：ApiResponse<T>（success/data/error）
- 错误码：稳定字符串（例如 `AUTH.UNAUTHORIZED`、`AUTH.FORBIDDEN`、`INVALID.REQUEST`、`RESOURCE.NOT_FOUND`、`INTERNAL.ERROR`）
- 授权（服务端强制）：项目归属校验 + RBAC 权限码校验；无权限返回 `403` + `AUTH.FORBIDDEN`

## 2. 后台模板兼容接口（必须）
### 2.1 登录
- POST /login
- req：{ username, password }
- resp：
  - accessToken
  - refreshToken
  - expires（毫秒时间戳，Unix ms）
  - username / nickname / avatar
  - roles: []
  - permissions: []（按钮权限码）

### 2.2 刷新 token（无感刷新链路）
- POST /refresh-token
- req：{ refreshToken }
- resp：{ accessToken, refreshToken, expires }
  - 说明：refresh token 采用 rotation；每次刷新都会签发新的 refresh token，旧 refresh token 立即失效。

### 2.3 动态路由
- GET /get-async-routes
 - resp：[{ path,name,component,meta:{title,icon,rank,roles?,auths?},children? }]

> roles：页面级访问  
> auths：按钮级权限（与 permissions 对应）  
> 叶子节点建议省略 `children` 字段（避免前端菜单过滤 `children.length === 0`）。

## 3. EMS 业务接口（项目内）
- /projects
- /projects/{project_id}/gateways
- /projects/{project_id}/devices
- /projects/{project_id}/points
- /projects/{project_id}/point-mappings
- /projects/{project_id}/measurements?pointId=&from=&to=&limit=&cursorTsMs=&order=&bucketMs=&agg=
- /projects/{project_id}/realtime?pointId=（响应为列表；指定 pointId 时列表长度为 0 或 1）
- /projects/{project_id}/commands
- /projects/{project_id}/audit
- /projects/{project_id}/alarms（规划中）

## 3.1 RBAC 管理接口（tenant 级）
- `GET /rbac/users`（list users）
- `POST /rbac/users`（create user）
- `PUT /rbac/users/{user_id}`（update user: status/password）
- `PUT /rbac/users/{user_id}/roles`（replace roles）
- `GET /rbac/roles`（list roles, include permissions）
- `POST /rbac/roles`（create role）
- `DELETE /rbac/roles/{role_code}`（delete role, cascade bindings）
- `PUT /rbac/roles/{role_code}/permissions`（replace permissions）
- `GET /rbac/permissions`（list permission codes）

在线状态口径补充：
- gateways/devices 的响应 DTO 增加 `online` 与 `lastSeenAtMs` 字段（由 Redis TTL 推导）。
- `status` 字段为元数据（人工配置 online/offline），不等同于 `online`（实时在线）。

#### measurements 查询参数补充（分页/聚合）
- `cursorTsMs`：可选，毫秒时间戳；与 `order` 配合实现 keyset 分页（`asc`: ts > cursor；`desc`: ts < cursor）
- `order`：可选，`asc`/`desc`（默认 `asc`）
- `bucketMs`：可选，毫秒桶大小；提供后返回聚合结果（`tsMs` 为桶起始，`value` 为聚合值字符串，`quality` 为空）
- `agg`：可选，`avg|min|max|sum|count`（默认 `avg`；仅在提供 `bucketMs` 时生效）

### 控制与审计（M3 基础）
- `POST /projects/{project_id}/commands`
  - req: `{ target, payload }`
- `GET /projects/{project_id}/commands?limit=`
- `GET /projects/{project_id}/commands/{command_id}/receipts`
- `GET /projects/{project_id}/audit?from=&to=&limit=`

## 4. 多租户规则
- tenant_id 不出现在 URL
- tenant 从 JWT/Context 读取
- project_id 在 URL 中出现，且必须校验属于该 tenant

## 5. 权限码规划（建议先定一版）
- PROJECT.READ / PROJECT.WRITE
- ASSET.GATEWAY.READ / ASSET.GATEWAY.WRITE
- ASSET.DEVICE.READ / ASSET.DEVICE.WRITE
- ASSET.POINT.READ / ASSET.POINT.WRITE
- DATA.REALTIME.READ / DATA.MEASUREMENTS.READ
- CONTROL.COMMAND.ISSUE / CONTROL.COMMAND.READ
- ALARM.RULE.READ / ALARM.RULE.WRITE / ALARM.EVENT.READ
- RBAC.USER.READ / RBAC.USER.WRITE
- RBAC.ROLE.READ / RBAC.ROLE.WRITE
- SYSTEM.METRICS.READ

## 6. 服务端 RBAC 授权矩阵（已落地）
说明：
- 登录响应中的 `permissions` 会写入 JWT，并在服务端用于接口授权判断。
- 返回 `AUTH.FORBIDDEN` 的两类常见原因：
  1) 项目不属于当前 tenant（project scope 校验失败）
  2) 当前用户缺少接口所需权限码

| 接口 | 权限要求 |
|------|----------|
| `GET /projects`、`GET /projects/{project_id}` | `PROJECT.READ` |
| `POST/PUT/DELETE /projects/{project_id?}` | `PROJECT.WRITE` |
| `GET /projects/{project_id}/gateways*` | `ASSET.GATEWAY.READ` |
| `POST/PUT/DELETE /projects/{project_id}/gateways*` | `ASSET.GATEWAY.WRITE` |
| `GET /projects/{project_id}/devices*` | `ASSET.DEVICE.READ` |
| `POST/PUT/DELETE /projects/{project_id}/devices*` | `ASSET.DEVICE.WRITE` |
| `GET /projects/{project_id}/points*` | `ASSET.POINT.READ` |
| `POST/PUT/DELETE /projects/{project_id}/points*` | `ASSET.POINT.WRITE` |
| `GET /projects/{project_id}/point-mappings*` | `ASSET.POINT.READ` |
| `POST/PUT/DELETE /projects/{project_id}/point-mappings*` | `ASSET.POINT.WRITE` |
| `GET /projects/{project_id}/realtime` | `DATA.REALTIME.READ` |
| `GET /projects/{project_id}/measurements` | `DATA.MEASUREMENTS.READ` |
| `GET /projects/{project_id}/commands`、`GET /projects/{project_id}/commands/{command_id}/receipts` | `CONTROL.COMMAND.READ` 或 `CONTROL.COMMAND.ISSUE`（任一满足） |
| `POST /projects/{project_id}/commands` | `CONTROL.COMMAND.ISSUE` |
| `GET /projects/{project_id}/audit` | `CONTROL.COMMAND.READ` |
| `GET /rbac/users` | `RBAC.USER.READ` |
| `POST/PUT /rbac/users*` | `RBAC.USER.WRITE` |
| `GET /rbac/roles`、`GET /rbac/permissions` | `RBAC.ROLE.READ` |
| `POST/PUT/DELETE /rbac/roles*` | `RBAC.ROLE.WRITE` |
| `GET /metrics` | `SYSTEM.METRICS.READ` |
