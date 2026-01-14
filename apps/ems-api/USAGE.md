# ems-api 使用方法

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

```bash
export EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
export EMS_JWT_SECRET="your-secret"
export EMS_JWT_ACCESS_TTL_SECONDS="3600"
export EMS_JWT_REFRESH_TTL_SECONDS="2592000"
export EMS_HTTP_ADDR="127.0.0.1:8080"

cargo run -p ems-api
```

首次初始化：
```bash
scripts/db-init.sh
scripts/health-check.sh
```

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

说明：
- 字段使用 camelCase（`accessToken`、`refreshToken`）。
- `expires` 为毫秒时间戳（Unix ms）。

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
