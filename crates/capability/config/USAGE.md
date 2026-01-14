# config 使用方法

## 模块职责
- 从环境变量读取应用配置。
- 统一配置校验与默认值。

## 边界与约束
- 不包含业务逻辑，仅提供配置能力。

## 对外能力
- `AppConfig::from_env()`：读取并校验配置。

## 最小示例
```rust
use ems_config::AppConfig;

let config = AppConfig::from_env()?;
println!("{}", config.http_addr);
println!("{}", config.database_url);
```
