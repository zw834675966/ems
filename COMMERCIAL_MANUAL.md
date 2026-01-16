# EMS 商用部署与使用说明书

**版本**: 1.0.0
**日期**: 2026-01-16
**适用对象**: 运维工程师、系统管理员

---

## 1. 系统简介
EMS (Energy Management System) 是一个基于云原生架构的高性能能源管理系统。
- **后端核心**: Rust + Axum (极速、安全)
- **数据存储**: TimescaleDB (时序数据) + Redis (实时缓存)
- **前端界面**: Vue 3 + Element Plus (现代交互体验)
- **通信协议**: MQTT (广泛的设备兼容性)

## 2. 部署前准备 (Prerequisites)

### 2.1 硬件要求 (最低配置)
- **CPU**: 2 Core
- **内存**: 4 GB (推荐 8 GB，因 JVM/TimescaleDB 主要消耗)
- **磁盘**: 50 GB SSD (取决于数据保留策略)

### 2.2 软件依赖
- **操作系统**: Linux (Ubuntu 22.04 LTS / CentOS 7+ 推荐)
- **容器环境**: Docker 24.0+, Docker Compose v2.0+
- **数据库**: PostgreSQL 16+（建议安装 TimeScaleDB 插件；可通过 `EMS_REQUIRE_TIMESCALE=on` 强制要求）
- **缓存**: Redis 7.0+
- **消息队列**: MQTT Broker (如 EMQX 或 Mosquitto)

---

## 3. 安装与部署 (Installation)

推荐使用 Docker Compose 进行部署，以确保环境一致性。

### 3.1 获取部署包
将项目根目录下的 `docker-compose.yml` 及相关配置文件复制到服务器 `/opt/ems` 目录。

### 3.2 环境变量配置
在部署目录下创建 `.env` 文件（参考下文“配置详解”）。
**安全警告**: 生产环境**必须**修改默认密码和密钥！

### 3.3 启动服务
```bash
# 进入部署目录
cd /opt/ems

# 启动依赖（Postgres/Redis/MQTT）
docker compose up -d

# 启动应用（包含 db-init + ems-api + web-admin）
docker compose --profile app up -d

# 查看日志确认启动状态（可选）
docker compose logs -f ems-api
```

### 3.4 验证部署
- **API 存活探针**: `curl http://localhost:8080/livez` (应返回 200 OK)
- **API 就绪探针**: `curl http://localhost:8080/readyz` (应返回 200 OK)
- **Web 访问**: 打开浏览器访问 `http://<服务器IP>:8848` (默认账号: admin / admin123)
- **指标接口**: `GET /metrics` 需要 Bearer token（权限 `SYSTEM.METRICS.READ`），不建议对公网暴露

---

## 4. 配置详解 (Configuration Reference)

所有配置均通过环境变量及其对应的 `.env` 文件进行管理。

### 4.1 基础与安全 (必填)
| 变量名 | 说明 | 示例/默认值 |
| :--- | :--- | :--- |
| `EMS_HTTP_ADDR` | API 监听地址 | `0.0.0.0:8080` |
| `EMS_JWT_SECRET` | **[重要]** JWT 签名密钥 | 生产环境请生成随机长字符串 |
| `EMS_JWT_ACCESS_TTL_SECONDS` | Access Token 有效期(秒) | `3600` (1小时) |
| `EMS_JWT_REFRESH_TTL_SECONDS` | Refresh Token 有效期(秒) | `86400` (24小时) |

### 4.2 数据库与存储
| 变量名 | 说明 | 默认值 |
| :--- | :--- | :--- |
| `EMS_DATABASE_URL` | PostgreSQL 连接串 | `postgresql://ems:admin123@postgres:5432/ems` |
| `EMS_REDIS_URL` | Redis 连接串 | `redis://:admin123@redis:6379` |
| `EMS_REDIS_ONLINE_TTL_SECONDS` | 设备在线状态缓存时长 | `60` |
| `EMS_REQUIRE_TIMESCALE` | 是否强依赖 timescaledb（生产建议开启） | `off` |

### 4.3 MQTT 消息总线
| 变量名 | 说明 | 默认值 |
| :--- | :--- | :--- |
| `EMS_MQTT_HOST` | Broker 地址 | `mosquitto` |
| `EMS_MQTT_PORT` | Broker 端口 | `1883` |
| `EMS_MQTT_USERNAME` | 连接用户名 | (可选) |
| `EMS_MQTT_PASSWORD` | 连接密码 | (可选) |
| `EMS_MQTT_TOPIC_PREFIX` | 根主题前缀 | `ems` |

### 4.4 功能开关
| 变量名 | 说明 | 默认值 |
| :--- | :--- | :--- |
| `EMS_INGEST` | 开启数据采集模块 | `false` (设为 `true` 以启用) |
| `EMS_CONTROL` | 开启反向控制模块 | `false` |
| `EMS_WEB_ADMIN` | 开发模式前端代理 | `off` (生产环境请勿开启) |

---

## 5. 运维操作指南 (Operations)

### 5.0 生产拓扑建议（TLS 与反向代理）
- 建议由 Nginx/Envoy/Ingress 终止 TLS，后端 `ems-api` 仅提供 HTTP（内网）。
- 反向代理需透传转发头：`X-Forwarded-For`、`X-Forwarded-Proto`、`X-Forwarded-Host`。
- 建议加安全响应头（示例）：`X-Content-Type-Options: nosniff`、`X-Frame-Options: SAMEORIGIN`、`Referrer-Policy: no-referrer`。
- `GET /metrics` 需要鉴权（权限 `SYSTEM.METRICS.READ`），仍建议仅内网开放或网段限制，避免对公网暴露运行指标。

### 5.1 数据备份
建议每日备份 PostgreSQL 数据库。
```bash
# 备份命令
docker exec -t ems-postgres pg_dump -U ems ems > /backup/ems_$(date +%Y%m%d).sql
```

### 5.2 日志查看
系统采用结构化日志 (JSON/Text)，支持通过 Docker logs 查看。
```bash
# 查看实时日志
docker compose logs -f --tail=100 ems-api
```
可配合 EFK (Elasticsearch-Fluentd-Kibana) 或 Loki 进行日志聚合。

### 5.3 故障排查 (Troubleshooting)
- **问题**: 启动报错 `relation "measurement" does not exist`
  - **原因**: 数据库迁移未执行或 TimescaleDB 插件未加载。
  - **解决**: 检查 Postgres 容器日志，确保 `check_extension` 通过；手动执行 `sqlx migrate run`。

- **问题**: 设备数据不更新
  - **原因**: MQTT 连接失败或 Topic 不匹配。
  - **解决**: 检查 `EMS_INGEST=true` 是否设置；使用 MQTT 客户端（如 MQTTX）订阅 `ems/#` 验证数据流。

---

## 6. 技术支持
如遇无法解决的问题，请联系技术支持团队获取协助，并提供以下信息：
1. `docker compose logs` 的输出片段。
2. `.env` 配置脱敏副本。
3. 问题复现步骤描述。
