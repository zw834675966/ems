# EMS 生产环境部署指南

## 目录结构

```
deploy/
├── docker-compose.prod.yml  # 生产环境 Docker Compose
├── nginx.conf               # Nginx 反向代理配置
├── README.md               # 本文档
├── ssl/                    # SSL 证书目录（需创建）
│   ├── certificate.crt
│   └── private.key
├── mosquitto/              # MQTT 配置（需创建）
│   ├── mosquitto.conf
│   └── passwd
└── .env.prod               # 生产环境变量（需创建）
```

## 快速开始

### 1. 准备环境变量

```bash
# 复制示例并编辑
cp .env.example deploy/.env.prod

# 必须修改的变量：
# - POSTGRES_PASSWORD: 数据库密码（强密码）
# - JWT_SECRET: JWT 签名密钥（至少 32 字符随机字符串）
# - MQTT_PASSWORD: MQTT 密码
```

### 2. 准备 SSL 证书

```bash
mkdir -p deploy/ssl

# 方式 A：使用 Let's Encrypt（推荐）
# certbot certonly --webroot -w /var/www/certbot -d your-domain.com

# 方式 B：使用自签名证书（仅测试）
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout deploy/ssl/private.key \
    -out deploy/ssl/certificate.crt \
    -subj "/CN=your-domain.com"
```

### 3. 配置 MQTT

```bash
mkdir -p deploy/mosquitto

# 创建配置文件
cat > deploy/mosquitto/mosquitto.conf << 'EOF'
listener 1883
allow_anonymous false
password_file /mosquitto/passwd
persistence true
persistence_location /mosquitto/data/
EOF

# 创建密码文件
docker run --rm -v $(pwd)/deploy/mosquitto:/mosquitto/passwd eclipse-mosquitto:2 \
    mosquitto_passwd -c -b /mosquitto/passwd/passwd ems emsmqtt
```

### 4. 构建镜像

```bash
# 构建 API 镜像
docker build -t ems-api:latest -f Dockerfile.api .

# 构建前端镜像
docker build -t ems-web-admin:latest -f web/admin/Dockerfile web/admin
```

### 5. 启动服务

```bash
cd deploy
docker compose -f docker-compose.prod.yml --env-file .env.prod up -d
```

### 6. 验证部署

```bash
# 检查服务状态
docker compose -f docker-compose.prod.yml ps

# 查看日志
docker compose -f docker-compose.prod.yml logs -f ems-api

# 测试 API
curl https://your-domain.com/api/health
```

## 资源配置

| 服务 | 内存限制 | CPU 限制 | 说明 |
|------|----------|----------|------|
| PostgreSQL | 2GB | 2核 | 可根据数据量调整 |
| Redis | 512MB | 1核 | maxmemory 设为 450MB |
| EMS API | 1GB | 2核 | 根据请求量调整 |
| EMS Web | 128MB | 0.5核 | 静态资源服务 |
| Nginx | 128MB | 0.5核 | 反向代理 |
| MQTT | 256MB | 0.5核 | 消息队列 |

## 安全配置

### Nginx 安全 Headers

已配置的安全 Headers：

- **HSTS**: 强制 HTTPS，有效期 1 年
- **X-Frame-Options**: SAMEORIGIN，防止点击劫持
- **X-XSS-Protection**: 启用 XSS 过滤
- **X-Content-Type-Options**: nosniff，禁止 MIME 嗅探
- **Referrer-Policy**: strict-origin-when-cross-origin
- **Content-Security-Policy**: 限制资源加载来源

### /metrics 访问限制

`/metrics` 端点仅允许内网 IP 访问：
- 10.0.0.0/8
- 172.16.0.0/12
- 192.168.0.0/16
- 127.0.0.1

如需从外网访问指标，可配置 Token 验证：

```nginx
location /api/metrics {
    if ($http_x_metrics_token != "your-secret-token") {
        return 403;
    }
    # ...
}
```

## TimescaleDB 数据治理

生产环境自动应用的策略：

- **压缩策略**: 7 天后自动压缩，减少存储空间约 80%
- **保留策略**: 365 天后自动删除，防止磁盘爆满

查看策略状态：

```sql
-- 查看压缩策略
SELECT * FROM timescaledb_information.jobs 
WHERE proc_name = 'policy_compression';

-- 查看保留策略
SELECT * FROM timescaledb_information.jobs 
WHERE proc_name = 'policy_retention';
```

## 备份与恢复

### 数据库备份

```bash
# 备份
docker exec ems-postgres pg_dump -U ems -d ems | gzip > backup_$(date +%Y%m%d).sql.gz

# 恢复
gunzip -c backup_YYYYMMDD.sql.gz | docker exec -i ems-postgres psql -U ems -d ems
```

### 自动备份脚本

```bash
#!/bin/bash
# /opt/ems/backup.sh

BACKUP_DIR=/opt/ems/backups
RETAIN_DAYS=30

mkdir -p $BACKUP_DIR
docker exec ems-postgres pg_dump -U ems -d ems | gzip > $BACKUP_DIR/ems_$(date +%Y%m%d_%H%M%S).sql.gz

# 清理旧备份
find $BACKUP_DIR -name "*.sql.gz" -mtime +$RETAIN_DAYS -delete
```

添加到 crontab：
```
0 2 * * * /opt/ems/backup.sh
```

## 监控

### Prometheus 指标采集

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'ems-api'
    static_configs:
      - targets: ['ems-api:8080']
    metrics_path: /metrics
    scheme: http
```

### 关键指标

- `http_requests_total`: HTTP 请求总数
- `http_request_duration_seconds`: 请求延迟分布
- `db_connections_active`: 数据库活跃连接数

## 故障排查

### 服务无法启动

```bash
# 检查日志
docker compose -f docker-compose.prod.yml logs ems-api

# 检查依赖服务
docker compose -f docker-compose.prod.yml ps
```

### 数据库连接失败

```bash
# 检查 PostgreSQL 状态
docker exec ems-postgres pg_isready -U ems

# 检查网络连通性
docker exec ems-api nc -zv postgres 5432
```

### SSL 证书问题

```bash
# 检查证书有效性
openssl x509 -in deploy/ssl/certificate.crt -text -noout

# 检查证书链
openssl verify -CAfile ca-bundle.crt deploy/ssl/certificate.crt
```

## 升级流程

1. 拉取新镜像
2. 备份数据库
3. 停止服务
4. 启动新版本
5. 验证服务

```bash
# 1. 拉取/构建新镜像
docker pull ems-api:v2.0.0

# 2. 备份
docker exec ems-postgres pg_dump -U ems -d ems > backup_before_upgrade.sql

# 3. 滚动更新
docker compose -f docker-compose.prod.yml up -d --no-deps ems-api

# 4. 验证
curl https://your-domain.com/api/health
```
