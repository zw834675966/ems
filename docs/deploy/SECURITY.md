# EMS 安全配置指南

本文档描述 EMS 系统的安全配置要求，包括 TLS 加密、MQTT 认证和网络隔离。

## 1. MQTT TLS 配置

### 1.1 启用 TLS 加密

在 `.env` 文件中配置：

```bash
# MQTT 主机地址
EMS_MQTT_HOST=your-mqtt-broker-host

# 使用 TLS 端口（推荐 8883）
EMS_MQTT_PORT=8883

# 启用 TLS
EMS_MQTT_USE_TLS=true

# CA 证书路径（可选，用于验证 broker 证书）
EMS_MQTT_CA_CERT_PATH=/etc/ssl/certs/ca-certificates.crt
```

### 1.2 MQTT 认证配置

```bash
EMS_MQTT_USERNAME=ems-service
EMS_MQTT_PASSWORD=your-strong-mqtt-password
```

### 1.3 推荐 MQTT Broker 配置（Mosquitto）

创建 `/etc/mosquitto/conf.d/ems.conf`：

```bash
# 基础配置
listener 8883
protocol mqtt

# TLS 配置
cafile /etc/mosquitto/ca_certificates/ca.crt
certfile /etc/mosquitto/certs/server.crt
keyfile /etc/mosquitto/certs/server.key
require_certificate false

# 用户认证
password_file /etc/mosquitto/passwords.txt
allow_anonymous false

# ACL 权限控制（可选）
acl_file /etc/mosquitto/acl.txt
```

生成密码文件：
```bash
mosquitto_passwd -c /etc/mosquitto/passwords.txt ems-service
```

### 1.4 生成自签名证书（测试环境）

```bash
# 创建 CA
openssl genrsa -out ca.key 2048
openssl req -new -x509 -days 365 -key ca.key -out ca.crt

# 创建服务器证书
openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr
openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt
```

## 2. 网络安全配置

### 2.1 防火墙规则（iptables）

```bash
# 允许 SSH 入站
iptables -A INPUT -p tcp --dport 22 -j ACCEPT

# 允许已建立的连接
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# 允许 EMS API 入站（仅内网）
iptables -A INPUT -s 192.168.0.0/16 -p tcp --dport 8080 -j ACCEPT

# 允许 MQTT 入站（仅内网或 VPN）
iptables -A INPUT -s 192.168.0.0/16 -p tcp --dport 8883 -j ACCEPT

# 允许 Prometheus 抓取指标（仅监控服务器）
iptables -A INPUT -s 192.168.100.0/24 -p tcp --dport 8080 -j ACCEPT

# 拒绝其他入站连接
iptables -A INPUT -j DROP

# 保存规则
iptables-save > /etc/iptables/rules.v4
```

### 2.2 Systemd 网络隔离

在 `ems-api.service` 中已配置：

```ini
[Service]
# 禁止访问主机的 /home /root 等目录
ProtectHome=true

# 只读根文件系统
ProtectSystem=strict

# 私有临时目录
PrivateTmp=true

# 禁止获取新权限
NoNewPrivileges=true
```

## 3. 数据库安全

### 3.1 PostgreSQL TLS 配置

编辑 `postgresql.conf`：

```bash
ssl = on
ssl_cert_file = '/etc/ssl/certs/server.crt'
ssl_key_file = '/etc/ssl/private/server.key'
ssl_ca_file = '/etc/ssl/certs/ca.crt'
```

修改连接字符串使用 TLS：
```
DATABASE_URL=postgresql://ems:password@postgres-host:5432/ems?sslmode=verify-ca
```

### 3.2 数据库用户权限

```sql
-- 创建只读用户（用于监控）
CREATE USER ems_readonly WITH PASSWORD 'readonly_password';
GRANT SELECT ON ALL TABLES IN SCHEMA public TO ems_readonly;

-- 创建应用用户（限制权限）
CREATE USER ems_app WITH PASSWORD 'app_password';
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ems_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO ems_app;
```

## 4. 密钥管理

### 4.1 生产环境密钥生成

```bash
# 生成强 JWT 密钥（至少 32 字符）
openssl rand -base64 32

# 生成数据库密码
openssl rand -base64 24

# 生成 MQTT 密码
mosquitto_passwd -c /etc/mosquitto/passwords.txt ems-service
```

### 4.2 密钥轮换策略

建议每 90 天轮换一次密钥：

```bash
# 轮换 JWT 密钥
1. 修改 EMS_JWT_SECRET
2. 重启服务
3. 通知用户重新登录

# 轮换数据库密码
1. 修改数据库用户密码
2. 更新 .env 中的 DATABASE_URL
3. 重启服务

# 轮换 MQTT 密码
1. 更新密码文件
2. 重启 MQTT Broker
3. 更新 .env 中的 EMS_MQTT_PASSWORD
4. 重启 EMS 服务
```

## 5. 审计日志

### 5.1 启用审计日志

确保 `audit_logs` 表已创建，EMS 会自动记录：
- 用户登录/登出
- 关键操作（创建/删除/修改资源）
- 控制指令下发

### 5.2 日志保留

```sql
-- 清理 90 天前的审计日志
DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '90 days';
```

## 6. 合规性检查清单

| 检查项 | 状态 | 说明 |
|--------|------|------|
| MQTT 使用 TLS 加密 | ☐ | 推荐端口 8883 |
| API 使用 HTTPS/TLS 终止 | ☐ | 在负载均衡器配置 |
| 数据库使用 TLS | ☐ | sslmode=verify-ca |
| 强密码策略 | ☐ | 最小 16 字符 |
| 密钥定期轮换 | ☐ | 建议 90 天 |
| 防火墙规则配置 | ☐ | 仅开放必要端口 |
| 审计日志启用 | ☐ | 默认已启用 |
| 日志保留策略 | ☐ | 建议 90 天 |
| Systemd 安全加固 | ☐ | 已配置在 service 文件 |

## 7. 监控与告警

### 7.1 推荐监控指标

通过 `GET /metrics/prometheus` 监控：
- `ems_write_failure_total` - 写入失败次数（告警阈值：>10/分钟）
- `ems_command_dispatch_failure_total` - 指令下发失败次数
- `ems_backpressure_total` - 背压事件次数

### 7.2 告警规则示例（Prometheus）

```yaml
groups:
  - name: ems-alerts
    rules:
      - alert: EmsHighWriteFailures
        expr: increase(ems_write_failure_total[5m]) > 10
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "EMS 写入失败次数过多"

      - alert: EmsHighCommandFailures
        expr: increase(ems_command_dispatch_failure_total[5m]) > 5
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "EMS 指令下发失败次数过多"
```

## 8. 应急响应

### 8.1 服务熔断

如果检测到异常，可以手动停止服务：

```bash
# 停止服务
sudo systemctl stop ems-api

# 查看日志
sudo journalctl -u ems-api -n 100

# 重启服务
sudo systemctl start ems-api
```

### 8.2 数据恢复

```bash
# 从备份恢复
pg_restore -U ems -d ems /backup/ems_20240101.sql
```
