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

## EMS API（无 DB 阶段）
- 默认账号: admin / admin123
- JWT 配置: EMS_JWT_SECRET, EMS_JWT_ACCESS_TTL_SECONDS, EMS_JWT_REFRESH_TTL_SECONDS
- 数据库配置: EMS_DATABASE_URL
- 说明: 当前登录使用 Postgres 用户表（需先执行 migrations/seed）
- 接口路径兼容 `/login` 与 `/api/login`（同理适用于 refresh-token/get-async-routes）
- `expires` 为 Unix 毫秒时间戳
- 动态路由叶子节点省略 `children` 字段，避免前端菜单过滤

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

## 前端联动（开发）
- 前后端一起启动：`EMS_WEB_ADMIN=on cargo run`
- 仅后端：`cargo run` 或 `cargo run -p ems-api`
- 前端 mock 关闭：`web/admin/.env.development` 中设置 `VITE_ENABLE_MOCK = false`

## 备注
- Codex MCP 列表: `codex mcp list`
- Playwright 浏览器下载已通过 Playwright CLI 完成; puppeteer 按需下载。
