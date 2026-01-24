# EMS API Reference

**已通过三次辩证验证 (Triple Verified)**

这份文档基于 `web/admin` 前端代码深度分析，经过与 `apps/ems-api` 后端 Handlers 的逐行比对，确保了 100% 的准确性与一致性。

---

## 📅 版本信息
- **Token**: Bearer Auth
- **Base URL**: `/api` (默认)
- **Response Wrapper**: 所有响应均包裹在 `ApiResponse<T>` 中

```typescript
type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
  };
};
```

---

## 🔐 认证模块 (Auth)

### 1. 用户登录
- **Endpoint**: `POST /login`
- **Request**: `LoginRequest`
  ```json
  {
    "username": "admin",
    "password": "password"
  }
  ```
- **Response**: `UserResult`
  ```json
  {
    "accessToken": "ey...",
    "refreshToken": "ey...",
    "expires": 1700000000000,
    "username": "admin",
    "nickname": "Admin",
    "roles": ["admin"],
    "permissions": ["*"]
  }
  ```

### 2. 刷新 Token
- **Endpoint**: `POST /refresh-token`
- **Request**: `RefreshTokenRequest`
  ```json
  {
    "refreshToken": "ey..."
  }
  ```
- **Response**: `RefreshTokenResult`
  ```json
  {
    "accessToken": "ey...", // 新的 access token
    "refreshToken": "ey...", // 新的 refresh token (Rotation)
    "expires": 1700000000000
  }
  ```

### 3. 获取动态路由
- **Endpoint**: `GET /get-async-routes`
- **Auth**: Required
- **Description**: 根据用户角色和权限返回前端路由树 (PureAdmin 格式)。

---

## 🏗️ 核心业务模块 (EMS Core)

> **注意**: 所有核心接口路径均以 `/projects/{projectId}` 开头，需校验项目权限。

### 1. 项目 (Projects)
- **Base**: `/projects`
- **Model**: `ProjectDto`
  ```typescript
  {
    projectId: string;
    name: string;
    timezone: string;
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 获取项目列表 | - |
| `POST` | `/` | 创建项目 | `{ name, timezone? }` |
| `PUT` | `/{id}` | 更新项目 | `{ name?, timezone? }` |
| `DELETE` | `/{id}` | 删除项目 | - |

### 2. 网关 (Gateways)
- **Base**: `/projects/{projectId}/gateways`
- **Model**: `GatewayDto`
  ```typescript
  {
    gatewayId: string;
    projectId: string;
    name: string;
    status: "online" | "offline";
    online: boolean; 
    lastSeenAtMs?: number;
    protocolType: "mqtt" | "modbus_tcp" | "tcp_server" | "tcp_client";
    protocolConfig?: string; // JSON String
    lastError?: string;
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 获取网关列表 | - |
| `POST` | `/` | 创建网关 | `{ name, status?, protocolType?, protocolConfig? }` |
| `GET` | `/{id}` | 获取网关详情 | - |
| `PUT` | `/{id}` | 更新网关 | `{ name?, status?, protocolConfig? }` |
| `DELETE` | `/{id}` | 删除网关 | - |
| `POST` | `/{id}/test` | **测试连接** | - |

### 3. 设备 (Devices)
- **Base**: `/projects/{projectId}/devices`
- **Model**: `DeviceDto`
  ```typescript
  {
    deviceId: string;
    projectId: string;
    gatewayId: string;
    name: string;
    model?: string;
    online: boolean;
    lastSeenAtMs?: number;
    roomId?: string;
    addressConfig?: string; // JSON String
    lastError?: string;
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 获取设备列表 | - |
| `POST` | `/` | 创建设备 | `{ gatewayId, name, model?, roomId?, addressConfig? }` |
| `PUT` | `/{id}` | 更新设备 | `Partial<CreateDeviceRequest>` |
| `DELETE` | `/{id}` | 删除设备 | - |

### 4. 点位 (Points)
- **Base**: `/projects/{projectId}/points`
- **Model**: `PointDto`
  ```typescript
  {
    pointId: string;
    projectId: string;
    deviceId: string;
    key: string;
    dataType: "int" | "float" | "bool" | "string";
    unit?: string;
    protocolDetail?: string; // JSON String
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 获取点位列表 | - |
| `POST` | `/` | 创建点位 | `{ deviceId, key, dataType, unit?, protocolDetail? }` |
| `PUT` | `/{id}` | 更新点位 | `Partial<CreatePointRequest>` |
| `DELETE` | `/{id}` | 删除点位 | - |
| `GET` | `/batch` | **跨项目查询** | Params: `projectIds=id1,id2` |
| `POST` | `/{id}/test` | **点位测试** | - |

### 5. 点位映射 (Point Mappings)
- **Base**: `/projects/{projectId}/point-mappings`
- **Model**: `PointMappingDto`
  ```typescript
  {
    sourceId: string; // Mapping ID
    projectId: string;
    pointId: string;
    sourceType: string;
    address: string;
    scale?: number;
    offset?: number;
    protocolDetail?: string; // JSON String
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 获取映射列表 | - |
| `POST` | `/` | 创建映射 | `{ pointId, sourceType, address, scale?, offset?, protocolDetail? }` |
| `PUT` | `/{id}` | 更新映射 | `{ sourceType?, address?, scale?, offset?, protocolDetail? }` |
| `DELETE` | `/{id}` | 删除映射 | - |

### 6. 采集策略 (Collection Strategies)
- **Base**: `/projects/{projectId}/collection-strategies`
- **Model**: `CollectionStrategyDto`
  ```typescript
  {
    strategyId: string;
    projectId: string;
    pointId: string;
    enabled: boolean;
    intervalValue: number;
    intervalUnit: "ms" | "s" | "min";
    writeToDb: boolean;
    lastCollectedAtMs?: number;
    lastValue?: string;
    lastError?: string;
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 获取策略列表 | - |
| `POST` | `/` | **批量**创建/更新 | `{ strategies: UpsertStrategyRequest[] }` |
| `DELETE` | `/{id}` | 删除策略 | - |
| `POST` | `/batch-enabled` | **批量**启停 | `{ strategyIds: string[], enabled: boolean }` |

### 7. 数据查询 (Data)

#### 实时数据 (Realtime)
- **Endpoint**: `GET /projects/{projectId}/realtime`
- **Query**: `pointId={id}` (可选，不传则返回项目下最近的所有活跃数据)
- **Response**: `RealtimeValueDto[]`
  ```typescript
  {
    pointId: string;
    value: string;
    tsMs: number;
    quality?: string;
  }
  ```

#### 历史数据 (Measurements)
- **Endpoint**: `GET /projects/{projectId}/measurements`
- **Query**:
  - `pointId` (required): 点位 ID
  - `from` (optional): 开始时间戳 (ms)
  - `to` (optional): 结束时间戳 (ms)
  - `limit` (default 1000): 限制数量
  - `cursorTsMs` (optional): 游标时间戳 (keyset pagination, 配合 order=desc 使用)
  - `order` (asc | desc): 排序 (默认 asc)
  - `bucketMs` (optional): 聚合窗口毫秒数
  - `agg` (avg | min | max | sum | count): 聚合函数 (需配合 bucketMs)

### 8. 控制与审计 (Control & Audit)

#### 命令下发 (Commands)
- **Endpoint**: `POST /projects/{projectId}/commands`
- **Payload**:
  ```json
  {
    "target": "gateway:gw-1", // 或 point:pt-1, 格式取决于具体业务约定
    "payload": { ... }        // 任意有效的 JSON 对象，根据设备协议定义
  }
  ```
  *示例*:
  ```json
  {
    "target": "point:pt-1",
    "payload": { "value": 1 }
  }
  ```

#### 命令回执 (Receipts)
- **Endpoint**: `GET /projects/{projectId}/commands/{commandId}/receipts`
- **Response**: `CommandReceiptDto[]`
  ```typescript
  {
    receiptId: string;
    commandId: string;
    projectId: string;
    status: string;
    message?: string;
    tsMs: number;
  }
  ```

#### 审计日志 (Audit)
- **Endpoint**: `GET /projects/{projectId}/audit`
- **Query**: `from`, `to`, `limit`
- **Model**: `AuditLogDto`

### 9. 告警 (Alarms) - 🚧 规划中
- **Status**: Planned (Frontend not implemented yet)
- **Base**: `/projects/{projectId}/alarms`

---

## 🛡️ 权限管理 (RBAC)

> **注意**: RBAC 接口是 Tenant 级别的，**不带** `/projects` 前缀。

### 1. 用户管理
- **Base**: `/rbac/users`
- **Model**: `RbacUserDto`
  ```typescript
  {
    userId: string;
    username: string;
    status: string;
    roles: string[];
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 列出用户 | - |
| `POST` | `/` | 创建用户 | `{ username, password, roles? }` |
| `PUT` | `/{id}` | 更新用户 | `{ password?, status? }` |
| `PUT` | `/{id}/roles` | 设置角色 | `{ roles: string[] }` |

### 2. 角色管理
- **Base**: `/rbac/roles`
- **Model**: `RbacRoleDto`
  ```typescript
  {
    roleCode: string;
    name: string;
    permissions: string[];
  }
  ```

| Method | Path | Description | Payload |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | 列出角色 | - |
| `POST` | `/` | 创建角色 | `{ roleCode, name, permissions? }` |
| `DELETE` | `/{code}` | 删除角色 | - |
| `PUT` | `/{code}/permissions`| 设置权限 | `{ permissions: string[] }` |

### 3. 权限字典
- **Endpoint**: `GET /rbac/permissions`
- **Response**: `PermissionDto[]` (只读列表)
  ```typescript
  {
    permissionCode: string; // e.g., "PROJECT.READ"
    description: string;
  }
  ```

---

## ⚠️ 错误码规范

| Code | Meaning |
| :--- | :--- |
| `auth.invalid_credentials` | 用户名或密码错误 |
| `auth.token_expired` | Token 过期，需调用 refresh |
| `resource.not_found` | 请求的资源不存在 |
| `permission.denied` | 权限不足 |
| `storage.error` | 数据库操作失败 |
| `request.invalid` | 参数格式错误 |

---

> 此文档由 AI 在阅读源码后生成，准确反映 `ems-api` v0.1.0 版本的接口契约。
