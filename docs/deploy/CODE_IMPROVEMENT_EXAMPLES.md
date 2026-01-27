# EMS 代码改进示例文档

本文档展示如何改进 EMS 系统的关键代码部分，符合商用部署标准。

## 1. 认证 Handler 改进示例

### 问题：硬编码测试凭证

**原始代码** (`apps/ems-api/src/handlers/auth.rs:683-689`):
```rust
// ❌ 问题：硬编码的测试凭证
let jwt = JwtManager::new("test-secret".to_string(), 3600, 7200);
let tokens = jwt.issue_tokens(&ctx).expect("tokens");
```

**改进后**:
```rust
// ✅ 改进：从环境变量读取 JWT 密钥
let jwt_secret = std::env::var("EMS_JWT_SECRET")
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to read EMS_JWT_SECRET");
        ApiError::ConfigurationError(e.to_string())
    })?;

let jwt = JwtManager::new(&jwt_secret, 3600, 7200);

// ✅ 改进：使用 Result 处理 token 生成失败
let tokens = jwt.issue_tokens(&ctx)
    .map_err(|e| {
        tracing::error!(error = %e, user_id = %user_id, "Token generation failed");
        ApiError::TokenGenerationError(e.to_string())
    })?;
```

## 2. 数据库查询错误处理示例

### 问题：unwrap() 在生产代码中使用

**原始代码** (`apps/ems-api/src/handlers/collection_strategy.rs:293-298`):
```rust
// ❌ 问题：unwrap 可能导致 panic
let protocol_config = gateway.protocol_config.as_deref().unwrap_or("{}");
let config_res = ModbusTcpSource::from_json(protocol_config);
```

**改进后**:
```rust
// ✅ 改进：使用 Result 和适当的错误消息
let protocol_config = gateway.protocol_config.as_deref().unwrap_or("{}");
let config_res = ModbusTcpSource::from_json(protocol_config)
    .map_err(|e| {
        tracing::error!(
            error = %e,
            protocol_config = %protocol_config,
            gateway_id = %gateway_id,
            "Failed to parse Modbus config"
        );
        ApiError::InvalidConfig(format!(
            "Invalid Modbus config for gateway {}: {}",
            gateway_id,
            e.to_string()
        ))
    })?;
```

## 3. 协议错误处理示例

### 问题：设备连接失败

**原始代码**:
```rust
// ❌ 问题：直接返回错误字符串
pub fn connect(config: &ModbusConfig) -> Result<ModbusClient, String> {
    let client = tokio_modbus::tcp::Context::new(config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(client)
}
```

**改进后**:
```rust
// ✅ 改进：使用自定义错误类型和结构化日志
use ems_protocol::{ModbusError, ModbusClient};

pub fn connect(config: &ModbusConfig) -> Result<ModbusClient, ModbusError> {
    tokio_modbus::tcp::Context::new(config)
        .await
        .map_err(|e| {
            ModbusError::ConnectionFailed {
                host: config.host.clone(),
                port: config.port,
                message: e.to_string(),
            }
        })
}
```

## 4. 日志记录最佳实践

### 4.1 生产环境配置

```bash
# 推荐的生产环境日志配置
RUST_LOG=warn,ems_api=info,ems_collector=info,ems_protocol=warn
EMS_LOG_FORMAT=json
```

### 4.2 日志级别选择指南

| 场景 | 推荐级别 | 示例 |
|------|----------|------|
| 正常业务流程 | info | 用户登录、设备操作、数据采集 |
| 认证失败 | warn | 密码错误、token 过期、权限拒绝 |
| 系统错误 | error | 数据库连接失败、协议错误、panic |
| 调试信息 | debug | 请求详情、中间件状态（仅开发） |

### 4.3 结构化日志示例

```rust
// ✅ 推荐：使用 tracing 的结构化日志
use tracing::{info, warn, error};

// 信息日志
info!(
    target = "ems_api",
    user_id = %user_id,
    action = "login",
    result = "success"
);

// 警日志
warn!(
    target = "ems_api",
    error = %err,
    device_id = %device_id,
    "Device connection unstable"
);

// 错误日志
error!(
    target = "ems_api",
    error = %err,
    point_id = %point_id,
    request_id = %request_id,
    "Failed to write measurement"
);
```

## 5. 前端虚拟滚动优化建议

### 5.1 何时需要虚拟滚动

当前项目使用分页控制数据量（`pageSize`），以下情况不需要虚拟滚动：
- **列表数据 < 1000 条**：当前分页已足够
- **表格高度固定**：已配置高度限制
- **不需要搜索/过滤**：静态列表渲染速度快

### 5.2 虚拟滚动实施方案（未来需要时）

**方案 A：使用 el-table-v2（推荐）**
```vue
<template>
  <el-table-v2
    :data="largeData"
    :columns="columns"
    :height="400"
    fixed
  />
</template>

<script lang="ts" setup>
import { ref } from 'vue'

// ✅ 优势：仅渲染视口内元素
const largeData = ref(Array.from({ length: 10000 }).map((_, i) => ({
  id: i,
  name: `Item ${i}`,
  value: Math.random() * 100
})))
</script>
```

**方案 B：自定义虚拟滚动（高性能）**
```vue
<template>
  <div class="virtual-scroll-container" style="height: 400px; overflow-y: auto;">
    <div
      v-for="item in visibleItems"
      :key="item.id"
      class="virtual-item"
      :style="{ height: itemHeight + 'px' }"
    >
      {{ item.name }}
    </div>
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted } from 'vue'

const allItems = ref(/* 大数据源 */);
const itemHeight = 40;
const containerHeight = 400;
const visibleCount = Math.ceil(containerHeight / itemHeight);

const visibleItems = computed(() => {
  const start = scrollTop.value / itemHeight;
  const end = start + visibleCount;
  return allItems.value.slice(Math.max(0, start - 5), end + 5);
});

const scrollTop = ref(0);

const onScroll = (e: Event) => {
  scrollTop.value = (e.target as HTMLElement).scrollTop;
};
</script>

<style>
.virtual-scroll-container {
  position: relative;
}

.virtual-item {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  will-change: transform;
}
</style>
```

## 6. 配置管理改进

### 6.1 环境变量验证

```rust
// ✅ 改进：启动时验证所有必要的环境变量
pub fn validate_env_vars() -> Result<(), String> {
    let required_vars = vec![
        "EMS_DATABASE_URL",
        "EMS_JWT_SECRET",
    ];

    for var in required_vars {
        if std::env::var(var).is_err() {
            return Err(format!("Required environment variable {} is not set", var));
        }
    }

    Ok(())
}
```

### 6.2 配置加载改进

```rust
// ✅ 改进：结构化配置加载
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub http_addr: String,
    pub log_level: String,
    pub log_format: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: std::env::var("EMS_DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://ems:admin123@localhost:5432/ems".to_string()),
            jwt_secret: std::env::var("EMS_JWT_SECRET")
                .unwrap_or_else(|_| {
                    tracing::warn!("JWT_SECRET not set, using default (INSECURE!)");
                    "default-secret-change-in-production".to_string()
                }),
            http_addr: std::env::var("EMS_HTTP_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            log_level: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,ems_api=info".to_string()),
            log_format: std::env::var("EMS_LOG_FORMAT")
                .unwrap_or_else(|_| "text".to_string()),
        })
    }
}
```

## 7. 测试与生产分离

### 7.1 使用条件编译标记测试代码

```rust
// ✅ 推荐：使用 #[cfg(test)] 和 #[cfg(not(test))] 分离测试和生产代码

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login_with_invalid_credentials() {
        // 测试代码只编译到测试目标
        assert!(login_invalid_credentials().await.is_err());
    }
}

#[cfg(not(test))]
pub async fn login_with_validation(
    state: &AppState,
    req: LoginRequest,
) -> Result<LoginResponse, ApiError> {
    // 生产代码：不包含测试逻辑
    let user = state.user_store
        .find_by_username(&ctx, &req.username)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, username = %req.username, "User lookup failed");
            ApiError::DatabaseError(e.to_string())
        })?;

    // ... 正常的业务逻辑
}
```

### 7.2 使用 feature flags 控制可选功能

```toml
[features]
default = []

# 开发工具（包含测试接口）
dev-tools = []

# 高级监控（需要 Prometheus）
advanced-monitoring = ["prometheus-exporter"]

# 生产功能（默认启用）
production = []

[dependencies]
# dev-tools 特性依赖
anyhow = { version = "1.0", optional = true }
serde_json = { version = "1.0", optional = true }
```

```rust
#[cfg(feature = "dev-tools")]
pub mod dev_utils {
    pub fn debug_snapshot() -> Value {
        // 仅在开发工具时编译
    }
}

#[cfg(not(feature = "dev-tools"))]
pub mod production_only {
    pub fn business_logic() {
        // 始终编译
    }
}
```

## 8. 性能优化清单

### 8.1 查询优化

- [ ] 使用数据库索引（已存在）
- [ ] 批量查询减少数据库往返（部分实现）
- [ ] 缓存频繁访问的数据（使用 Redis）
- [ ] 使用连接池（已实现）

### 8.2 前端性能优化

- [x] 使用分页控制数据量（已实现）
- [ ] 虚拟滚动（大数据量时需要）
- [ ] 图片懒加载
- [ ] 请求防抖/节流
- [ ] Web Worker 处理复杂计算

### 8.3 后端性能优化

- [x] 异步处理（Tokio）
- [ ] 连接池优化（当前使用 sqlx 默认）
- [ ] 批量写入
- [ ] 查询结果缓存

## 9. 安全加固检查清单

### 9.1 身份认证

- [ ] JWT 密钥强生成（使用 openssl rand）
- [ ] 密码哈希存储（bcrypt）
- [ ] Token 刷新机制安全（撤销旧 token）
- [ ] 登录失败限流（防止暴力破解）

### 9.2 网络安全

- [ ] TLS 1.2/1.3 配置（已提供配置）
- [ ] CORS 白名单（仅允许前端域名）
- [ ] 请求速率限制（防止 DDoS）
- [ ] WebSocket 安全配置

### 9.3 数据安全

- [ ] SQL 注入防护（使用 sqlx 参数化查询）
- [ ] 敏感数据加密（数据库字段加密）
- [ ] 审计日志保留策略（已配置）
- [ ] 数据脱敏（日志中不记录完整信息）

### 9.4 运维安全

- [ ] 容器/进程隔离（Systemd ProtectSystem）
- [ ] 文件系统权限（限制可写路径）
- [ ] 资源限制（CPU/内存配额）
- [ ] 崩溃自动恢复（Watchdog）

## 10. 监控与告警

### 10.1 关键指标

| 指标名称 | 告警阈值 | 严重度 | 建议响应 |
|----------|----------|--------|----------|
| ems_write_failure_total | >10/5min | Critical | 立即检查数据库连接 |
| ems_command_dispatch_failure_total | >5/5min | Warning | 检查 MQTT 连接 |
| ems_backpressure_total | >100/5min | Warning | 增加处理容量 |
| ems_dropped_invalid_total | >50/5min | Warning | 检查数据质量 |
| 请求延迟 >1000ms | >5/min | Info | 检查网络/数据库性能 |

### 10.2 告警规则配置（Prometheus）

```yaml
groups:
  - name: ems-alerts
    interval: 30s
    rules:
      - alert: EmsCriticalWriteFailures
        expr: rate(ems_write_failure_total[5m]) > 2
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "EMS write failure rate is critical"
          description: "More than 2 write failures per 5 minutes"

      - alert: EmsHighCommandFailures
        expr: rate(ems_command_dispatch_failure_total[5m]) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "EMS command dispatch failure rate is high"
          description: "More than 1 command dispatch failure per 5 minutes"

      - alert: EmsHighBackpressure
        expr: ems_backpressure_total > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "EMS backpressure is high"
          description: "System experiencing high backpressure"

      - alert: EmsHighDroppedData
        expr: rate(ems_dropped_invalid_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "EMS dropped data rate is high"
          description: "More than 10 invalid data points dropped per 5 minutes"
```

## 11. 实施优先级

### 阶段 1：立即执行（本周）

1. ✅ TimescaleDB 数据保留策略 - 已完成
2. ✅ Prometheus 监控指标接口 - 已完成
3. ✅ Systemd 部署单元模板 - 已完成
4. ✅ .env 生产环境配置模板 - 已完成
5. ✅ TLS/MQTT 安全配置文档 - 已完成
6. ✅ 错误处理和日志配置改进指南 - 已完成
7. **执行数据保留策略迁移**

### 阶段 2：短中期（2-4 周）

1. 配置 Prometheus 抓取和告警
2. 实施错误处理改进（逐步替换 unwrap）
3. 添加请求速率限制
4. 添加 CORS 白名单

### 阶段 3：中长期（1-3 月）

1. 添加虚拟滚动优化（需要时）
2. 实施批量查询优化
3. 添加 Redis 缓存
4. 添加数据库查询优化

---

**结论**

本文档提供了：
1. 代码改进示例（认证、错误处理、日志记录）
2. 虚拟滚动优化建议（当前不需要，未来方案）
3. 配置管理改进（环境变量验证）
4. 测试与生产分离（条件编译和 feature flags）
5. 性能优化清单（查询、前端、后端）
6. 安全加固检查清单（认证、网络、数据、运维）
7. 监控与告警配置（Prometheus 规则）
8. 实施优先级（分三个阶段）

建议按阶段逐步实施，确保系统稳定性和可维护性。
