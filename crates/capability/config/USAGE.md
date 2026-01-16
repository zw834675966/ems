# config 使用方法

## 模块职责
- 从环境变量读取应用配置。
- 统一配置校验与默认值。

## 边界与约束
- 不包含业务逻辑，仅提供配置能力。

## 对外能力
- `AppConfig::from_env()`：读取并校验配置。

## 关键环境变量
- `EMS_DATABASE_URL`、`EMS_JWT_SECRET`、`EMS_JWT_ACCESS_TTL_SECONDS`、`EMS_JWT_REFRESH_TTL_SECONDS`
- `EMS_REDIS_URL`、`EMS_REDIS_LAST_VALUE_TTL_SECONDS`、`EMS_REDIS_ONLINE_TTL_SECONDS`
- `EMS_MQTT_HOST`、`EMS_MQTT_PORT`、`EMS_MQTT_USERNAME`、`EMS_MQTT_PASSWORD`
- `EMS_MQTT_TOPIC_PREFIX`、`EMS_MQTT_DATA_TOPIC_PREFIX`、`EMS_MQTT_COMMAND_TOPIC_PREFIX`、`EMS_MQTT_RECEIPT_TOPIC_PREFIX`
- `EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET`
- `EMS_MQTT_COMMAND_QOS`、`EMS_MQTT_RECEIPT_QOS`
- `EMS_CONTROL_DISPATCH_MAX_RETRIES`、`EMS_CONTROL_DISPATCH_BACKOFF_MS`
- `EMS_INGEST`、`EMS_CONTROL`
- `EMS_REQUIRE_TIMESCALE`（生产建议开启：要求 timescaledb 扩展存在，否则启动 fail-fast）

## 最小示例
```rust
use ems_config::AppConfig;

let config = AppConfig::from_env()?;
println!("{}", config.http_addr);
println!("{}", config.database_url);
```
