# ems-api 使用方法

## 模块结构

```
src/
├── main.rs              # 启动入口：装配依赖、初始化服务
├── routes.rs            # 路由定义：集中管理所有 API 路由
├── handlers/             # HTTP 处理器：按业务域分组
│   ├── mod.rs
│   ├── auth.rs         # 认证：登录、刷新 token、获取动态路由
│   ├── projects.rs     # 项目 CRUD
│   ├── gateways.rs     # 网关 CRUD
│   ├── devices.rs      # 设备 CRUD
│   ├── points.rs       # 点 CRUD
│   └── point_mappings.rs # 点映射 CRUD
├── middleware/          # 中间件：认证、授权、请求追踪
│   ├── mod.rs
│   └── auth.rs         # request_context、bearer_token、require_tenant_context、require_project_scope
└── utils/               # 工具函数
    ├── mod.rs
    ├── response.rs      # 错误响应、DTO 转换函数
    └── validation.rs    # 输入验证：normalize_required、normalize_optional
```

## 模块职责
- 启动 HTTP 服务并装配各 capability。
- 提供 M0 认证链路与动态路由接口。
- 输出 `x-request-id` / `x-trace-id` 响应头，便于链路追踪。

## 运行方式
应用会自动读取项目根目录 `.env`（若存在）。

必填环境变量：
- `EMS_DATABASE_URL`：Postgres 连接串（需先执行 migrations/seed）。
- `EMS_JWT_SECRET`：JWT 密钥。
- `EMS_JWT_ACCESS_TTL_SECONDS`：访问令牌有效期（秒）。
- `EMS_JWT_REFRESH_TTL_SECONDS`：刷新令牌有效期（秒）。

可选环境变量：
- `EMS_HTTP_ADDR`：监听地址，默认 `127.0.0.1:8080`。
- `EMS_REDIS_URL`：本地健康检查使用的 Redis 连接串，默认 `redis://default:admin123@localhost:6379`。
- `EMS_MQTT_HOST`：本地健康检查使用的 MQTT 主机，默认 `127.0.0.1`。
- `EMS_MQTT_PORT`：本地健康检查使用的 MQTT 端口，默认 `1883`。
- `EMS_MQTT_USERNAME`：本地健康检查使用的 MQTT 用户名，默认 `ems`。
- `EMS_MQTT_PASSWORD`：本地健康检查使用的 MQTT 密码，默认 `admin123`。
- `EMS_WEB_ADMIN`：前端启动模式（`off`/`on`/`only`），默认 `off`。

```bash
export EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
export EMS_JWT_SECRET="your-secret"
export EMS_JWT_ACCESS_TTL_SECONDS="3600"
export EMS_JWT_REFRESH_TTL_SECONDS="2592000"
export EMS_HTTP_ADDR="127.0.0.1:8080"

cargo run -p ems-api
```

前后端一键启动（开发）：
```bash
export EMS_WEB_ADMIN="on"
cargo run -p ems-api
```

仅启动前端（开发）：
```bash
export EMS_WEB_ADMIN="only"
cargo run -p ems-api
```

首次初始化：
```bash
scripts/db-init.sh
scripts/health-check.sh
```
说明：新增资产表（gateways/devices/points/point_sources）后需重新执行初始化脚本。

## 默认账号（数据库 seed）
仅用于 M0 演示，需先执行 seed 脚本：
- 用户名：`admin`
- 密码：`admin123`
初始化命令：
```bash
scripts/db-init.sh
```

## 依赖默认账号（本地）
- Redis：`default` / `admin123`
- MQTT（Mosquitto）：`ems` / `admin123`

## 接口说明
- `POST /login`（兼容 `/api/login`）：登录，返回 access/refresh token。
- `POST /refresh-token`（兼容 `/api/refresh-token`）：刷新 token。
- `GET /get-async-routes`（兼容 `/api/get-async-routes`）：动态路由，需 `Authorization: Bearer <access_token>`。
- `GET /health`：健康检查。
- `GET /projects`、`POST /projects`（兼容 `/api/*`）：项目列表与创建。
- `GET /projects/{project_id}`、`PUT /projects/{project_id}`、`DELETE /projects/{project_id}`（兼容 `/api/*`）。
- `GET /projects/{project_id}/gateways`、`POST /projects/{project_id}/gateways`（兼容 `/api/*`）。
- `GET /projects/{project_id}/gateways/{gateway_id}`、`PUT /projects/{project_id}/gateways/{gateway_id}`、`DELETE /projects/{project_id}/gateways/{gateway_id}`（兼容 `/api/*`）。
- `GET /projects/{project_id}/devices`、`POST /projects/{project_id}/devices`（兼容 `/api/*`）。
- `GET /projects/{project_id}/devices/{device_id}`、`PUT /projects/{project_id}/devices/{device_id}`、`DELETE /projects/{project_id}/devices/{device_id}`（兼容 `/api/*`）。
- `GET /projects/{project_id}/points`、`POST /projects/{project_id}/points`（兼容 `/api/*`）。
- `GET /projects/{project_id}/points/{point_id}`、`PUT /projects/{project_id}/points/{point_id}`、`DELETE /projects/{project_id}/points/{point_id}`（兼容 `/api/*`）。
- `GET /projects/{project_id}/point-mappings`、`POST /projects/{project_id}/point-mappings`（兼容 `/api/*`）。
- `GET /projects/{project_id}/point-mappings/{source_id}`、`PUT /projects/{project_id}/point-mappings/{source_id}`、`DELETE /projects/{project_id}/point-mappings/{source_id}`（兼容 `/api/*`）。

说明：
- 字段使用 camelCase（`accessToken`、`refreshToken`）。
- `expires` 为毫秒时间戳（Unix ms）。
- 动态路由 `meta.roles` / `meta.auths` 使用 `domain::permissions` 稳定权限码。
- 叶子节点 `children` 为空时会省略该字段，避免前端菜单过滤。
- 项目级接口需校验 `project_id` 归属，校验通过后写入 `TenantContext.project_scope`，归属失败返回 `AUTH.FORBIDDEN`。
- 动态路由 `component` 以 `src/views` 为基准，例如 `ems/projects/index` 对应 `src/views/ems/projects/index.vue`。
- 当前返回项目/网关/设备/点位/点位映射页面，前端需补齐对应视图或做映射。

## 最小验证
登录获取 token：
```bash
curl -sS -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'
```

携带 access_token 获取动态路由：
```bash
curl -sS http://127.0.0.1:8080/get-async-routes \
  -H "Authorization: Bearer <access_token>"
```

刷新 token：
```bash
curl -sS -X POST http://127.0.0.1:8080/refresh-token \
  -H "Content-Type: application/json" \
  -d '{"refreshToken":"<refresh_token>"}'
```

CRUD 验证（创建与清理）：
```bash
TOKEN_JSON=$(curl -sS -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}')

ACCESS_TOKEN=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("accessToken",""))' <<<"$TOKEN_JSON")

PROJECT_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Cleanup Project","timezone":"UTC"}')
PROJECT_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("projectId",""))' <<<"$PROJECT_JSON")

GATEWAY_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/gateways \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Cleanup-GW","status":"online"}')
GATEWAY_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("gatewayId",""))' <<<"$GATEWAY_JSON")

DEVICE_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/devices \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"gatewayId":"'"$GATEWAY_ID"'","name":"Cleanup-Device","model":"M1"}')
DEVICE_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("deviceId",""))' <<<"$DEVICE_JSON")

POINT_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/points \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"deviceId":"'"$DEVICE_ID"'","key":"temp","dataType":"float","unit":"C"}')
POINT_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("pointId",""))' <<<"$POINT_JSON")

MAPPING_JSON=$(curl -sS -X POST http://127.0.0.1:8080/projects/$PROJECT_ID/point-mappings \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"pointId":"'"$POINT_ID"'","sourceType":"mqtt","address":"topic/cleanup","scale":1.0,"offset":0.0}')
SOURCE_ID=$(python3 -c 'import json,sys; obj=json.load(sys.stdin); print(obj.get("data",{}).get("sourceId",""))' <<<"$MAPPING_JSON")

curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/point-mappings/$SOURCE_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/points/$POINT_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/devices/$DEVICE_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID/gateways/$GATEWAY_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
curl -sS -X DELETE http://127.0.0.1:8080/projects/$PROJECT_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"

curl -sS -o /tmp/project.json -w "%{http_code}\n" \
  http://127.0.0.1:8080/projects/$PROJECT_ID \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```
预期：项目查询返回 404；项目内资源列表返回 403（项目已删除，归属校验失败）。
