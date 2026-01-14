# API 契约与前端对接（pure-admin-thin + EMS）

## 1. 全局约定
- Base URL：/api
- 认证：Authorization: Bearer <access_token>
- 响应结构：ApiResponse<T>（success/data/error）
- 错误码：稳定字符串（如 AUTH.INVALID_TOKEN）

## 2. 后台模板兼容接口（必须）
### 2.1 登录
- POST /login
- req：{ username, password }
- resp：
  - accessToken
  - refreshToken
  - expires（时间戳或秒）
  - username / nickname / avatar
  - roles: []
  - permissions: []（按钮权限码）

### 2.2 刷新 token（无感刷新链路）
- POST /refresh-token
- req：{ refreshToken }
- resp：{ accessToken, refreshToken?, expires }

### 2.3 动态路由
- GET /get-async-routes
- resp：[{ path,name,component,meta:{title,icon,rank,roles?,auths?},children:[] }]

> roles：页面级访问  
> auths：按钮级权限（与 permissions 对应）  

## 3. EMS 业务接口（项目内）
- /projects
- /projects/{project_id}/gateways
- /projects/{project_id}/devices
- /projects/{project_id}/points
- /projects/{project_id}/measurements?point_id=&from=&to=&agg=
- /projects/{project_id}/realtime?point_id=
- /projects/{project_id}/commands
- /projects/{project_id}/alarms

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
