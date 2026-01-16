# WSL2 MCP 环境说明

本文档记录此 WSL2 环境中已安装的依赖与 MCP 配置，便于维护。

## 系统
- 操作系统: Ubuntu 24.04.3 LTS (WSL2)
- 内核: 6.6.87.2-microsoft-standard-WSL2
- Node.js: v24.12.0
- npm: 11.6.2
- Python: 3.12.3

## Codex MCP 配置
配置文件: /home/zw/.codex/config.toml

已配置 MCP servers:
- filesystem
  - command: /home/zw/.local/bin/mcp-server-filesystem
  - args: /home/zw/projects/ems
- git
  - command: /home/zw/.local/mcp-git-venv/bin/mcp-server-git
- shell
  - command: /home/zw/.local/bin/mcp-shell
- process
  - command: /home/zw/.local/bin/mcp-shell
- browser (puppeteer)
  - command: /home/zw/.local/bin/mcp-server-puppeteer
  - env:
    - MCP_PUPPETEER_HEADLESS=true
    - MCP_PUPPETEER_NAVIGATION_TIMEOUT_MS=20000
    - MCP_PUPPETEER_VIEWPORT_WIDTH=1280
    - MCP_PUPPETEER_VIEWPORT_HEIGHT=720
- playwright
  - command: /home/zw/.local/bin/mcp-server-playwright
  - env:
    - PLAYWRIGHT_BROWSERS_PATH=/home/zw/.cache/ms-playwright
    - PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
    - PLAYWRIGHT_TIMEOUT=20000
- postgres
  - command: /home/zw/.local/bin/mcp-server-postgres
  - args: postgresql://ems:admin123@localhost:5432/ems
- redis
  - command: /home/zw/.local/bin/mcp-server-redis
  - args: redis://localhost:6379
  - startup_timeout_sec: 20

## 已安装的 MCP server 包
全局 npm 安装 (prefix: /home/zw/.local):
- @mako10k/mcp-shell-server@2.6.2
- mcp-server-filesystem@0.6.2
- @mkusaka/mcp-shell-server@0.1.1 (binary: mcp-shell)
- @modelcontextprotocol/server-postgres@0.6.2
- @modelcontextprotocol/server-redis@2025.4.25
- @modelcontextprotocol/server-puppeteer@2025.5.12
- @playwright/mcp@0.0.55

Python 虚拟环境:
- /home/zw/.local/mcp-git-venv
  - mcp-server-git@0.6.2 (安装自 https://github.com/modelcontextprotocol/servers.git#subdirectory=src/git, commit 861c11b)

Playwright 浏览器缓存:
- /home/zw/.cache/ms-playwright

## 数据库配置
PostgreSQL:
- 版本: 16.11 (Ubuntu 16.11-0ubuntu0.24.04.1)
- 数据库: ems
- 用户: ems
- 密码: admin123
- 认证: pg_hba.conf (权限受限，未重新验证具体方法)

Redis:
- 版本: 7.0.15 (Ubuntu 包)
- ACL: user default on >admin123 ~* +@all；user ems on >admin123 ~* +@all

MQTT (Mosquitto):
- 版本: 2.0.18 (Ubuntu 包)
- 用户名: ems
- 密码: admin123

## 服务检查
- PostgreSQL:
  - pg_isready
  - psql "postgresql://ems:admin123@localhost:5432/ems" -c "select 1;"
- Redis:
  - redis-cli -u redis://localhost:6379 ping
- 统一脚本:
  - scripts/db-init.sh
  - scripts/health-check.sh
  - health-check 可选环境变量: EMS_REDIS_URL, EMS_MQTT_HOST, EMS_MQTT_PORT, EMS_MQTT_USERNAME, EMS_MQTT_PASSWORD
  - 默认 Redis URL: redis://default:admin123@localhost:6379

## WSL2 服务自动启动
WSL2 需要启用 systemd 才能让 PostgreSQL/Redis/MQTT 开机自启。

1) 启用 systemd（需要 sudo）：
```bash
sudo tee /etc/wsl.conf >/dev/null <<'EOF'
[boot]
systemd=true
EOF
```

2) 重启 WSL：
```bash
wsl --shutdown
```

3) 启用并启动服务：
```bash
sudo systemctl enable --now postgresql redis-server mosquitto
```

4) 校验：
```bash
systemctl is-enabled postgresql redis-server mosquitto
systemctl status postgresql redis-server mosquitto
```

## EMS API（无 DB 阶段）
- 默认账号: admin / admin123

## 生产建议（基线）
- 健康探针：`GET /livez`（存活）、`GET /readyz`（就绪：检查 Postgres）
- 指标接口：`GET /metrics` 需要 Bearer token 且具备权限 `SYSTEM.METRICS.READ`，建议仅内网开放
- Timescale 依赖：设置 `EMS_REQUIRE_TIMESCALE=on` 时会检查 `timescaledb` 扩展并 fail-fast
- Docker Compose：使用 `docker compose --profile app up -d` 启动应用栈（需要本机 Docker）
- JWT 配置: EMS_JWT_SECRET, EMS_JWT_ACCESS_TTL_SECONDS, EMS_JWT_REFRESH_TTL_SECONDS
- 数据库配置: EMS_DATABASE_URL
- Redis 配置: EMS_REDIS_URL, EMS_REDIS_LAST_VALUE_TTL_SECONDS（可选）, EMS_REDIS_ONLINE_TTL_SECONDS（默认 60 秒）
- 采集配置: EMS_INGEST, EMS_MQTT_HOST, EMS_MQTT_PORT, EMS_MQTT_USERNAME, EMS_MQTT_PASSWORD, EMS_MQTT_TOPIC_PREFIX, EMS_MQTT_DATA_TOPIC_PREFIX（可选）
- 控制配置: EMS_CONTROL, EMS_MQTT_COMMAND_TOPIC_PREFIX, EMS_MQTT_RECEIPT_TOPIC_PREFIX（可选）, EMS_MQTT_COMMAND_QOS（可选）, EMS_MQTT_RECEIPT_QOS（可选）, EMS_CONTROL_DISPATCH_MAX_RETRIES（可选）, EMS_CONTROL_DISPATCH_BACKOFF_MS（可选）
- 说明: 当前登录使用 Postgres 用户表（需先执行 migrations/seed）
- 接口路径兼容 `/login` 与 `/api/login`（同理适用于 refresh-token/get-async-routes）
- `expires` 为 Unix 毫秒时间戳
- 动态路由叶子节点省略 `children` 字段，避免前端菜单过滤

### 控制链路 MQTT 示例
- 采集主题：`ems/data/tenant-1/project-1/demo/topic`
- 命令主题：`ems/commands/tenant-1/project-1/command-123`
- 命令 payload（服务端发布）：`{"action":"set","value":42}`
- 回执主题：`ems/receipts/tenant-1/project-1/command-123`
- 回执 payload（设备侧发布）：`{"status":"success","message":"applied","tsMs":1700000000000}`

### 开启 EMS_CONTROL 的最小运行步骤
1) 启动依赖并初始化数据库：
```bash
scripts/db-init.sh
scripts/health-check.sh
```

2) 启动 ems-api（启用控制链路）：
```bash
EMS_CONTROL=on \
EMS_DATABASE_URL=postgresql://ems:admin123@localhost:5432/ems \
EMS_JWT_SECRET=dev \
EMS_JWT_ACCESS_TTL_SECONDS=3600 \
EMS_JWT_REFRESH_TTL_SECONDS=7200 \
EMS_MQTT_USERNAME=ems \
EMS_MQTT_PASSWORD=admin123 \
EMS_MQTT_TOPIC_PREFIX=ems \
cargo run -p ems-api
```

3) 从 /commands 获取 command_id（示例）：
```bash
ACCESS_TOKEN=$(curl -sS -X POST http://127.0.0.1:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["accessToken"])')

curl -sS -X POST http://127.0.0.1:8080/projects/project-1/commands \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d '{"target":"demo-target","payload":{"action":"set","value":42}}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["commandId"])'
```

4) 设备侧发布回执（示例，需替换命令 ID）：
```bash
mosquitto_pub -h 127.0.0.1 -p 1883 -u ems -P admin123 \
  -t "ems/receipts/tenant-1/project-1/command-123" \
  -m '{"status":"success","message":"applied","tsMs":1700000000000}'
```
也可使用脚本：
```bash
EMS_COMMAND_ID=command-123 \
EMS_MQTT_USERNAME=ems EMS_MQTT_PASSWORD=admin123 EMS_MQTT_TOPIC_PREFIX=ems \
scripts/control-receipt-simulate.sh
```
或启动“模拟设备”（自动订阅命令并回执）：
```bash
EMS_MQTT_USERNAME=ems EMS_MQTT_PASSWORD=admin123 EMS_MQTT_TOPIC_PREFIX=ems \
EMS_DEVICE_COMMAND_QOS=1 EMS_DEVICE_RECEIPT_QOS=1 \
scripts/device-emulator.sh
```

### 设备侧回执联调清单 + 验收步骤
联调清单：
- 设备侧发布回执 topic：`{EMS_MQTT_RECEIPT_TOPIC_PREFIX}/{tenant_id}/{project_id}/{command_id}`
- payload 字段：`status`（必填）、`message`（可选）、`tsMs`（可选，毫秒）
- status 建议枚举：`accepted`/`success`/`failed`/`timeout`
- 服务端行为：写入 `command_receipts`，更新 `commands.status`，写入 `audit_logs`（`CONTROL.COMMAND.RECEIPT`）

验收步骤（最小闭环）：
1) 启动 `ems-api` 并开启 `EMS_CONTROL=on`
2) 调用 `/projects/{project_id}/commands` 发起命令，得到 `command_id`
3) 设备侧按主题发布回执（或用 `scripts/control-receipt-simulate.sh`）
4) 查询 `/projects/{project_id}/commands/{command_id}/receipts`，确认回执存在
5) 查询 `/projects/{project_id}/audit`，确认有 `CONTROL.COMMAND.RECEIPT` 记录
6) 查询 `/projects/{project_id}/commands`，确认该命令 `status` 已更新

也可使用一键验收脚本（会启动一个临时 `ems-api` 实例，默认端口 `18080`）：
```bash
scripts/mvp-acceptance.sh
```
RBAC 管理面回归（会启动一个临时 `ems-api` 实例，默认端口 `18082`）：
```bash
scripts/rbac-acceptance.sh
```

### 认证接口验证
1) 登录获取 access/refresh token:
   - curl -sS -X POST http://localhost:8080/login \\
     -H "Content-Type: application/json" \\
     -d '{"username":"admin","password":"admin123"}'
2) 刷新 access token:
   - curl -sS -X POST http://localhost:8080/refresh-token \\
     -H "Content-Type: application/json" \\
     -d '{"refreshToken":"<refreshToken>"}'
3) 获取动态路由（需 Bearer access token）:
   - curl -sS http://localhost:8080/get-async-routes \\
     -H "Authorization: Bearer <accessToken>"

### 资源与数据接口示例（最小联动）
1) 统一准备（获取 token）：
```bash
BASE_URL=http://127.0.0.1:8080
ACCESS_TOKEN=$(curl -sS -X POST "$BASE_URL/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["accessToken"])')
AUTH_HEADER="Authorization: Bearer $ACCESS_TOKEN"
```

2) 健康检查（可选）：
```bash
curl -sS "$BASE_URL/livez"
curl -sS "$BASE_URL/readyz"
```

3) 创建项目/网关/设备/点位/点位映射（用于后续采集与查询）：
```bash
PROJECT_ID=$(curl -sS -X POST "$BASE_URL/projects" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d '{"name":"demo-project","timezone":"Asia/Shanghai"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["projectId"])')

GATEWAY_ID=$(curl -sS -X POST "$BASE_URL/projects/$PROJECT_ID/gateways" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d '{"name":"gw-1","status":"online"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["gatewayId"])')

DEVICE_ID=$(curl -sS -X POST "$BASE_URL/projects/$PROJECT_ID/devices" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d "{\"gatewayId\":\"$GATEWAY_ID\",\"name\":\"dev-1\",\"model\":\"m1\"}" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["deviceId"])')

POINT_ID=$(curl -sS -X POST "$BASE_URL/projects/$PROJECT_ID/points" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d "{\"deviceId\":\"$DEVICE_ID\",\"key\":\"temperature\",\"dataType\":\"f64\",\"unit\":\"C\"}" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["pointId"])')

SOURCE_ID=$(curl -sS -X POST "$BASE_URL/projects/$PROJECT_ID/point-mappings" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d "{\"pointId\":\"$POINT_ID\",\"sourceType\":\"mqtt\",\"address\":\"demo/topic\"}" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["sourceId"])')
```

4) 列表与详情查询：
```bash
curl -sS "$BASE_URL/projects" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/gateways" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/gateways/$GATEWAY_ID" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/devices" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/devices/$DEVICE_ID" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/points" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/points/$POINT_ID" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/point-mappings" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/point-mappings/$SOURCE_ID" -H "$AUTH_HEADER"
```

5) 采集模拟（需 `EMS_INGEST=on`，topic 地址需与 point-mapping.address 对齐）：
```bash
EMS_TENANT_ID=tenant-1 EMS_PROJECT_ID="$PROJECT_ID" EMS_POINT_ADDRESS=demo/topic \
EMS_MQTT_USERNAME=ems EMS_MQTT_PASSWORD=admin123 EMS_MQTT_TOPIC_PREFIX=ems EMS_MQTT_DATA_TOPIC_PREFIX=ems/data \
scripts/mqtt-simulate.sh
```

6) 查询实时与历史：
```bash
curl -sS "$BASE_URL/projects/$PROJECT_ID/realtime?pointId=$POINT_ID" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/measurements?pointId=$POINT_ID&limit=10" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/measurements?pointId=$POINT_ID&bucketMs=1000&agg=count&limit=10" -H "$AUTH_HEADER"
```

7) 控制命令、回执与审计：
```bash
COMMAND_ID=$(curl -sS -X POST "$BASE_URL/projects/$PROJECT_ID/commands" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d "{\"target\":\"device:$DEVICE_ID\",\"payload\":{\"action\":\"set\",\"value\":42}}" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["commandId"])')

curl -sS "$BASE_URL/projects/$PROJECT_ID/commands" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/commands/$COMMAND_ID/receipts" -H "$AUTH_HEADER"
curl -sS "$BASE_URL/projects/$PROJECT_ID/audit?limit=20" -H "$AUTH_HEADER"
```

8) 更新/删除示例（其余资源同理）：
```bash
curl -sS -X PUT "$BASE_URL/projects/$PROJECT_ID" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d '{"name":"demo-project-updated"}'

curl -sS -X DELETE "$BASE_URL/projects/$PROJECT_ID" -H "$AUTH_HEADER"
```

## 前端联动（开发）
- 前后端一起启动：`EMS_WEB_ADMIN=on cargo run`
- 仅后端：`cargo run` 或 `cargo run -p ems-api`
- 前端 mock 关闭：`web/admin/.env.development` 中设置 `VITE_ENABLE_MOCK = false`

## 备注
- Codex MCP 列表: `codex mcp list`
- Playwright 浏览器下载已通过 Playwright CLI 完成; puppeteer 按需下载。
