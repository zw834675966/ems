# EMS 错误处理与日志配置改进指南

本文档描述对 EMS 系统的错误处理和日志配置改进，符合商用部署标准。

## 1. 错误处理改进原则

### 1.1 避免使用 `.unwrap()` 和 `.expect()`

在商用环境中，所有可能的错误都应该通过 `Result` 类型处理并返回友好的错误信息，而不是直接 panic。

### 1.2 错误处理策略

#### 对于配置项（应该永远成功）
- **JWT 密钥配置**、**数据库连接字符串**、**协议配置解析**：
  ```rust
  let jwt_secret = std::env::var("EMS_JWT_SECRET")
      .expect("EMS_JWT_SECRET must be set");
  ```
  - 这是可接受的，因为这些是启动时的必要配置，失败就应该 panic。

#### 对于运行时错误（可能失败）
- **HTTP 响应解析**、**数据库查询**、**设备连接**：
  ```rust
  let tokens = state.auth.login(username, password).await
      .map_err(|e| {
          tracing::error!(error = %e, username = %username, "Authentication failed");
          ApiError::AuthenticationFailed(e.to_string())
      })?;
  ```
  - 使用 `?` 运算符或显式的 `.map_err()` 处理错误。

#### 对于数据解析
- **JSON 解析**、**协议详情解析**：
  ```rust
  let protocol_detail = point.protocol_detail.as_deref().unwrap_or("");
  let config_res = ModbusTcpSource::from_json(protocol_detail)
      .map_err(|e| {
          tracing::error!(error = %e, protocol_detail = %protocol_detail, "Invalid protocol config");
          ApiError::InvalidConfig(e.to_string())
      });
  ```

## 2. 日志配置改进

### 2.1 使用环境变量控制日志级别

不硬编码日志级别，使用 `RUST_LOG` 环境变量：

```bash
# 开发环境
RUST_LOG=debug,ems_api=debug,ems_collector=debug,ems_protocol=debug

# 生产环境
RUST_LOG=warn,ems_api=info,ems_collector=info,ems_protocol=warn
```

### 2.2 日志格式选择

通过 `EMS_LOG_FORMAT` 环境变量控制：
- `text`: 人类可读格式（开发环境）
- `json`: JSON 格式（生产环境，便于日志聚合）

### 2.3 结构化日志示例

```rust
// 使用 tracing 记录结构化日志
use tracing::{info, warn, error};

info!(target = "ems_api", "User {} logged in", username);
warn!(target = "ems_api", error = %err, username = %username, "Login failed due to internal error");
error!(target = "ems_api", error = %err, point_id = %point_id, "Failed to write measurement");
```

## 3. 常见错误处理模式

### 3.1 认证错误

```rust
pub async fn login(state: &AppState, req: LoginRequest) -> Result<LoginResponse, ApiError> {
    let user = state.user_store
        .find_by_username(&ctx, &req.username)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, username = %req.username, "User lookup failed");
            ApiError::DatabaseError(e.to_string())
        })?;

    // 验证密码（使用 bcrypt 避免明文比较）
    if !user.verify_password(&req.password) {
        tracing::warn!(target = "auth", username = %req.username, "Invalid password attempt");
        return Err(ApiError::InvalidCredentials);
    }

    let tokens = state.auth.issue_tokens(&ctx, &user.user_id).await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %user.user_id, "Token generation failed");
            ApiError::TokenGenerationError(e.to_string())
        })?;

    Ok(LoginResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
}
```

### 3.2 资源操作错误

```rust
pub async fn delete_device(state: &AppState, path: ProjectPath, device_id: String) -> Result<DeleteResponse, ApiError> {
    // 检查设备是否属于当前项目
    let device = state.device_store
        .find_device(&ctx, &path.project_id, &device_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, device_id = %device_id, "Device lookup failed");
            ApiError::NotFound(format!("Device not found: {}", device_id))
        })?;

    // 级联删除设备、点位、映射
    state.device_store
        .delete_device(&ctx, &path.project_id, &device_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, device_id = %device_id, "Device deletion failed");
            ApiError::DatabaseError(e.to_string())
        })?;

    Ok(DeleteResponse { deleted: true })
}
```

### 3.3 协议错误

```rust
pub fn test_modbus_connection(config: &ModbusConfig) -> Result<ModbusTestResult, ModbusError> {
    let mut client = tokio_modbus::tcp::Context::new(config)
        .map_err(|e| {
            tracing::error!(error = %e, host = %config.host, port = %config.port, "Modbus connection failed");
            ModbusError::ConnectionFailed(e.to_string())
        })?;

    // 测试读取
    let result = client.read_registers(config.unit_id, config.address, config.quantity).await
        .map_err(|e| {
            tracing::warn!(error = %e, address = %config.address, "Modbus read failed");
            ModbusError::ReadFailed(e.to_string())
        })?;

    Ok(result)
}
```

## 4. 测试与生产代码分离

### 4.1 使用条件编译

对于测试代码，使用 `#[cfg(test)]` 标记，避免在生产构建中包含测试逻辑：

```rust
#[cfg(test)]
async fn test_authentication() {
    // 测试代码只编译到测试目标
}

#[cfg(not(test))]
async fn production_login() {
    // 生产代码不包含测试逻辑
}
```

### 4.2 使用特性开关

对于开发时的测试功能，使用 feature flags：

```rust
#[cfg(feature = "dev-tools")]
pub fn dev_test_utils() {
    // 仅在开发工具时编译
}
```

## 5. 错误响应格式

### 5.1 统一错误响应结构

```rust
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
    pub details: Option<String>,
    pub request_id: String,  // 用于追踪
}

pub fn to_error_response(e: &ApiError, request_id: String) -> ErrorResponse {
    ErrorResponse {
        error_code: e.code().to_string(),
        message: e.user_facing_message(),
        details: e.details(),
        request_id,
    }
}
```

### 5.2 HTTP 状态码映射

| 错误类型 | HTTP 状态码 | 说明 |
|---------|-----------|------|
| 认证失败 | 401 | `UNAUTHORIZED` |
| 资源不存在 | 404 | `NOT_FOUND` |
| 权限不足 | 403 | `FORBIDDEN` |
| 验证失败 | 422 | `UNPROCESSABLE_ENTITY` |
| 内部错误 | 500 | `INTERNAL_SERVER_ERROR` |
| 服务不可用 | 503 | `SERVICE_UNAVAILABLE` |

## 6. 部署检查清单

### 6.1 日志配置验证

- [ ] 确认 `RUST_LOG` 环境变量已设置
- [ ] 确认日志级别适用于所有模块
- [ ] 确认日志格式（JSON/text）正确

### 6.2 错误处理验证

- [ ] 所有 handler 都返回 `Result` 类型
- [ ] 所有数据库错误都记录到日志
- [ ] 所有协议错误都返回用户友好的错误信息
- [ ] 避免 `.unwrap()` 在生产代码路径中

### 6.3 监控告警配置

- [ ] Prometheus 告警规则已配置
- [ ] 错误日志监控已启用
- [ ] 关键错误有告警阈值

## 7. 推荐的 cargo 依赖

```toml
[dev-dependencies]
anyhow = "1.0"  # 提供更好的错误处理上下文
thiserror = "1.0"  # 派生自定义错误类型
tracing = "0.1"  # 结构化日志（已存在）
tracing-subscriber = "0.3"  # 日志格式化和过滤
```
