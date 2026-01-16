# Handlers 模块使用文档

本目录为 `apps/ems-api/src/handlers/*` 的实现说明与导航（以 `apps/ems-api/USAGE.md` 为主文档）。

## 统一响应结构

所有 HTTP 接口统一返回 `api_contract::ApiResponse<T>`：

```json
{
  "success": true,
  "data": {},
  "error": null
}
```

失败时：

```json
{
  "success": false,
  "data": null,
  "error": { "code": "SOME.CODE", "message": "..." }
}
```

## 路由与模块

- 认证：`apps/ems-api/src/handlers/auth.rs`
  - `POST /login`
  - `POST /refresh-token`
  - `GET /get-async-routes`
- 观测：`apps/ems-api/src/handlers/metrics.rs`
  - `GET /metrics`
- 项目与资产：`apps/ems-api/src/handlers/projects.rs`、`gateways.rs`、`devices.rs`、`points.rs`、`point_mappings.rs`
- 数据查询：`apps/ems-api/src/handlers/realtime.rs`、`measurements.rs`
- 控制与审计：`apps/ems-api/src/handlers/commands.rs`、`audit.rs`

## 参考（完整示例）

- `apps/ems-api/USAGE.md`
- `05_API契约与前端对接.md`
