# api-contract 使用方法

## 模块职责
- 定义稳定的 DTO、错误码与响应封装。
- 保障前后端对接契约一致。

## 边界与约束
- 不依赖 storage/web。
- 不包含业务逻辑或数据库访问。

## 对外能力
- `ApiResponse<T>`：统一响应封装。
- 登录/刷新/动态路由相关 DTO。

## JSON 命名约定
- 请求/响应字段使用 camelCase。
- `expires` 为 Unix 毫秒时间戳。

## 最小示例
```rust
use api_contract::ApiResponse;

let ok = ApiResponse::success("ok");
let err = ApiResponse::<()>::error("AUTH.UNAUTHORIZED", "unauthorized");
```
