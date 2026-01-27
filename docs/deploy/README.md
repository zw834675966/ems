# EMS 生产环境部署指南

## 目录结构

```
deploy/
├── ems-api.service          # Systemd 服务单元模板
├── README.md                # 本文档
├── ssl/                     # SSL 证书目录（需创建）
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
password_file /etc/mosquitto/passwd
persistence true
persistence_location /var/lib/mosquitto/
EOF

# 创建密码文件 (需安装 mosquitto-clients)
# sudo mosquitto_passwd -c /etc/mosquitto/passwd ems
```

### 4. 编译构建

```bash
# 构建后端
cargo build --release -p ems-api

# 构建前端
cd web/admin
pnpm build:production
```

### 5. 安装与启动服务

```bash
# 1. 复制二进制文件
sudo cp target/release/ems-api /usr/local/bin/

# 2. 配置 Systemd
sudo cp deploy/ems-api.service /etc/systemd/system/
sudo systemctl daemon-reload

# 3. 启动依赖服务
sudo systemctl enable --now postgresql redis-server mosquitto

# 4. 启动应用
sudo systemctl enable --now ems-api
```

### 6. 验证部署

```bash
# 检查服务状态
systemctl status ems-api

# 查看日志
journalctl -u ems-api -f

# 测试 API
curl http://your-domain.com/api/health
```

## 资源配置

| 服务 | 内存限制 | CPU 限制 | 说明 |
|------|----------|----------|------|
| PostgreSQL | 2GB | 2核 | 可根据数据量调整 |
| Redis | 512MB | 1核 | maxmemory 设为 450MB |
| EMS API | 1GB | 2核 | 根据请求量调整 (集成静态资源服务) |
| MQTT | 256MB | 0.5核 | 消息队列 |

## 安全配置

### /metrics 访问限制

`/metrics` 端点建议仅内网 IP 访问：
- 10.0.0.0/8
- 172.16.0.0/12
- 192.168.0.0/16
- 127.0.0.1

生产环境下应配置网段限制或在 Systemd 单元中使用 IP 访问控制。

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
pg_dump -U ems ems | gzip > backup_$(date +%Y%m%d).sql.gz

# 恢复
gunzip -c backup_YYYYMMDD.sql.gz | psql -U ems -d ems
```

### 自动备份脚本

```bash
#!/bin/bash
# /opt/ems/backup.sh

BACKUP_DIR=/opt/ems/backups
RETAIN_DAYS=30

mkdir -p $BACKUP_DIR
pg_dump -U ems ems | gzip > $BACKUP_DIR/ems_$(date +%Y%m%d_%H%M%S).sql.gz

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
      - targets: ['localhost:8080']
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
journalctl -u ems-api -f

# 检查依赖服务
systemctl status postgresql redis-server mosquitto
```

### 数据库连接失败

```bash
# 检查 PostgreSQL 状态
systemctl status postgresql

# 检查网络连通性
nc -zv localhost 5432
```

### SSL 证书问题

```bash
# 检查证书有效性
openssl x509 -in deploy/ssl/certificate.crt -text -noout

# 检查证书链
openssl verify -CAfile ca-bundle.crt deploy/ssl/certificate.crt
```

## 升级流程

1. 编译新版本
2. 备份数据库
3. 停止服务
4. 替换二进制
5. 启动新版本
6. 验证服务

```bash
# 1. 编译
cargo build --release -p ems-api

# 2. 备份
pg_dump -U ems ems > backup_before_upgrade.sql

# 3. 停止
sudo systemctl stop ems-api

# 4. 替换
sudo cp target/release/ems-api /usr/local/bin/

# 5. 启动
sudo systemctl start ems-api

# 6. 验证
curl http://your-domain.com/api/health
```
