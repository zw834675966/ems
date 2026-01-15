# EMS 项目运行逻辑流程图

## 目录
1. [系统整体架构](#系统整体架构)
2. [用户登录与认证流程](#用户登录与认证流程)
3. [API 请求处理流程](#api-请求处理流程)
4. [数据 CRUD 操作流程](#数据-crud-操作流程)
5. [多租户上下文传播流程](#多租户上下文传播流程)
6. [JWT Token 刷新流程](#jwt-token-刷新流程)
7. [动态路由加载流程](#动态路由加载流程)

---

## 系统整体架构

```mermaid
graph TB
    subgraph "前端层 (Vue 3)"
        FE[web/admin<br/>Element Plus]
        Router[Vue Router]
        Store[Pinia Store]
        API[API Client]
    end

    subgraph "后端层 (Rust + Axum)"
        MW1[request_context<br/>注入 trace_id]
        MW2[认证中间件<br/>JWT 验证]
        Handler[Handler 层]
    end

    subgraph "能力层 (Capabilities)"
        Auth[Auth Service<br/>JWT 管理]
        Storage[Storage 层<br/>PostgreSQL]
        Config[Config<br/>环境变量]
        Telemetry[Telemetry<br/>结构化日志]
    end

    subgraph "数据层"
        PG[(PostgreSQL<br/>元数据)]
        TS[(TimescaleDB<br/>时序数据 - 规划中)]
        REDIS[(Redis<br/>缓存 - 规划中)]
    end

    subgraph "外部服务"
        MQTT[(MQTT Broker<br/>Mosquitto - 规划中)]
    end

    FE --> Router
    Router --> API
    API -->|HTTP 请求| MW1
    MW1 --> MW2
    MW2 --> Handler
    Handler --> Auth
    Handler --> Storage
    Auth --> Storage
    Storage --> PG
    Storage -.->|规划中| TS
    Storage -.->|规划中| REDIS
    Handler -.->|规划中| MQTT

    style FE fill:#e1f5ff
    style Handler fill:#fff4e6
    style Storage fill:#f0fff4f
    style PG fill:#4caf50
    style TS fill:#9e9e9e,stroke-dasharray: 5 5
    style REDIS fill:#9e9e9e,stroke-dasharray: 5 5
    style MQTT fill:#9e9e9e,stroke-dasharray: 5 5
```

---

## 用户登录与认证流程

```mermaid
sequenceDiagram
    participant User as 👤 用户
    participant FE as 🌐 前端 (Vue)
    participant API as 🚀 后端 API
    participant Auth as 🔐 Auth Service
    participant Storage as 💾 Storage
    participant PG as 🗄️ PostgreSQL

    User->>FE: 输入用户名密码
    FE->>FE: 前端验证 (非空检查)
    FE->>API: POST /login<br/>{username, password}
    API->>Storage: UserStore.find_by_username()
    Storage->>PG: SELECT * FROM users<br/>WHERE username = $1
    PG-->>Storage: UserRecord
    Storage-->>API: Option<UserRecord>

    alt 用户不存在或密码错误
        API-->>FE: 401 Unauthorized<br/>{success: false}
        FE-->>User: 显示错误提示
    else 认证成功
        API->>Auth: 验证密码
        Auth->>Auth: 生成 TenantContext
        Auth->>Auth: JWT 签发<br/>(access_token + refresh_token)
        Auth-->>API: AuthTokens
        API-->>FE: 200 OK<br/>{success: true, data: {...}}
        FE->>FE: 存储 tokens 到 localStorage
        FE->>FE: 存储 user/roles/permissions 到 store
        FE->>FE: 跳转到 /ems 首页
    end
```

**关键点：**
- 密码当前为明文存储（待修复）
- JWT 包含 `tenant_id`、`user_id`、`roles`、`permissions`
- `expires` 为 Unix 毫秒时间戳

---

## API 请求处理流程

```mermaid
sequenceDiagram
    participant FE as 🌐 前端
    participant MW1 as request_context
    participant MW2 as 认证中间件
    participant Handler as Handler
    participant Storage as Storage
    participant PG as PostgreSQL

    FE->>MW1: HTTP 请求<br/>(可选 Authorization 头)

    rect rgb(240, 248, 255)
        Note over MW1: 1. 生成 UUID<br/>request_id + trace_id
        MW1->>MW1: new_request_ids()
        MW1->>MW2: 注入到 extensions
    end

    alt 需要认证的端点
        rect rgb(255, 245, 238)
            Note over MW2: 2. 提取 Bearer token
            MW2->>MW2: bearer_token(headers)

            alt 无 token
                MW2-->>FE: 401 Unauthorized<br/>missing auth
            else token 无效或过期
                MW2-->>FE: 401 Unauthorized<br/>invalid/expired token
            else token 有效
                Note over MW2: 3. 解码 JWT<br/>提取 TenantContext
                MW2->>MW2: auth.verify_access_token()
                MW2->>MW2: 注入到 state
                MW2->>Handler: 传递 TenantContext
            end
        end

        alt 需要项目范围验证
            Note over Handler: 4. 验证项目归属
            Handler->>Storage: project_belongs_to_tenant()
            Storage->>PG: SELECT tenant_id FROM projects<br/>WHERE project_id = $1
            PG-->>Storage: tenant_id
            Storage-->>Handler: bool

            alt 不属于当前租户
                Handler-->>FE: 403 Forbidden
            else 归属正确
                Note over Handler: 5. 设置 project_scope
                Handler->>Handler: ctx.project_scope = Some(project_id)
                Handler->>Storage: 执行业务逻辑
            end
        else 无需项目范围
            Handler->>Storage: 执行业务逻辑
        end
    else 公开端点 (如 /login, /health)
        MW1->>Handler: 直接传递
    end

    Handler->>Storage: 调用 storage trait 方法
    Storage->>PG: SQL 查询 (带 tenant_id 过滤)
    PG-->>Storage: 数据结果
    Storage-->>Handler: Vec<T> 或 Option<T>
    Handler->>Handler: 数据转换 (DTO)
    Handler-->>FE: 200 OK<br/>{success: true, data: [...]}
    FE->>FE: 更新 UI 或 store
```

**核心原则：**
1. **所有请求**都生成 `request_id` 和 `trace_id`
2. **认证端点**需要有效的 Bearer token
3. **项目级操作**验证项目归属当前租户
4. **所有 SQL**查询都带 `tenant_id` 过滤

---

## 数据 CRUD 操作流程

以"创建项目"为例：

```mermaid
sequenceDiagram
    participant User as 👤 用户
    participant FE as 🌐 前端页面
    participant API as 🚀 projects Handler
    participant Storage as ProjectStore
    participant PG as 🗄️ PostgreSQL
    participant Telemetry as 📊 Tracing

    User->>FE: 填写表单<br/>{name, timezone}
    FE->>FE: 前端验证<br/>name 非空
    FE->>API: POST /projects<br/>{name, timezone}
    Note over API,Telemetry: span: request<br/>trace_id: xxx
    API->>API: normalize_required(req.name, "name")
    API->>API: normalize_optional(req.timezone, "timezone")

    alt 验证失败
        API-->>FE: 400 Bad Request<br/>{error: "字段不能为空"}
        FE-->>User: 显示错误提示
    else 验证通过
        API->>API: 生成 UUID<br/>project_id
        API->>Storage: create_project(ctx, record)
        Storage->>PG: INSERT INTO projects<br/>(project_id, tenant_id, name, timezone)

        alt 数据库约束冲突
            PG-->>Storage: UNIQUE violation
            Storage-->>API: StorageError::Conflict
            API-->>FE: 409 Conflict<br/>{error: "项目已存在"}
            FE-->>User: 显示冲突提示
        else 插入成功
            PG-->>Storage: 1 row affected
            Storage-->>API: ProjectRecord
            API->>API: project_to_dto(record)
            API-->>FE: 201 Created<br/>{success: true, data: ProjectDto}
            FE->>FE: 更新项目列表
            FE->>FE: 显示成功提示
        end
    end
```

**验证流程：**
```mermaid
graph LR
    A[用户输入] --> B{normalize_required}
    B -->|空字符串| C[返回错误]
    B -->|有效字符串| D{normalize_optional}
    D -->|None 值| E[使用默认值]
    D -->|有效字符串| F[验证通过]
    E --> G[生成 UUID]
    F --> G
    G --> H[调用 Storage]
```

---

## 多租户上下文传播流程

```mermaid
graph TB
    subgraph "JWT Token 内容"
        JWT[JWT Payload]
        TID["tenant_id: "tenant-1""]
        UID["user_id: "user-1""]
        ROLE["roles: ["admin"]""]
        PERM["permissions: ["PROJECT.READ", ...]""]
        JWT --> TID
        JWT --> UID
        JWT --> ROLE
        JWT --> PERM
    end

    subgraph "后端处理链"
        AUTH[auth.verify_access_token]
        CTX[TenantContext<br/>包含上述 4 个字段]
        MW[认证中间件]
        HS[Handler]
        ST[Storage Trait]
        PGSQL[PostgreSQL]
    end

    JWT --> AUTH
    AUTH --> CTX
    CTX --> MW
    MW -->|注入到请求状态| HS

    subgraph "项目级操作"
        HS -->|require_project_scope| VALID{验证项目归属}
        VALID -->|true| SCOPE["ctx.project_scope = Some(project_id)""]
        VALID -->|false| ERR[403 Forbidden]
    end

    SCOPE --> ST
    HS -->|无需项目范围| ST
    ST -->|显式传递 &ctx| PGSQL
    PGSQL -->|WHERE tenant_id = $1<br/>AND project_id = $2| RESULT[租户隔离的结果]

    style JWT fill:#fff3cd
    style CTX fill:#d1c4e9
    style RESULT fill:#4caf50
    style ERR fill:#f44336
```

**关键不变量：**
1. `tenant_id` 从 JWT 提取，不在 URL 中
2. 所有数据库查询自动带 `WHERE tenant_id = ?`
3. `project_scope` 只在 URL 包含 `project_id` 时设置
4. 跨租户访问在架构层面被阻止

---

## JWT Token 刷新流程

```mermaid
sequenceDiagram
    participant FE as 🌐 前端
    participant API as 🚀 后端 API
    participant Auth as 🔐 Auth Service
    participant LocalStorage as 💾 localStorage

    Note over FE,LocalStorage: 场景：access_token 过期

    FE->>FE: 检测到 401 响应
    FE->>FE: 从 localStorage 读取 refresh_token

    alt refresh_token 存在
        FE->>API: POST /refresh-token<br/>{refreshToken}
        API->>Auth: refresh(token)
        Auth->>Auth: 解码 refresh_token<br/>提取 TenantContext
        Auth->>Auth: 重新签发 tokens<br/>(新的 access + refresh)
        Auth-->>API: AuthTokens
        API-->>FE: 200 OK<br/>{success: true, data: {...}}

        Note over FE: 更新存储的 tokens
        FE->>LocalStorage: 更新 access_token
        FE->>LocalStorage: 更新 refresh_token
        FE->>LocalStorage: 更新 expires

        FE->>FE: 重试原始请求
    else refresh_token 不存在
        FE->>FE: 清除 localStorage
        FE->>FE: 跳转到 /login
        FE-->>👤 用户: 显示登录页面
    end
```

**安全设计：**
- `access_token` TTL 短（如 1 小时）
- `refresh_token` TTL 长（如 7 天）
- 每次刷新都生成新的 refresh_token（防止重放攻击）

---

## 动态路由加载流程

```mermaid
sequenceDiagram
    participant FE as 🌐 前端 (Vue Router)
    participant Store as 🗃️ Pinia Store
    participant API as 🚀 后端 API
    participant Auth as 🔐 认证中间件

    Note over FE: 用户已登录，首次访问或刷新页面

    FE->>FE: 检查 localStorage.async-routes
    alt 路由缓存存在且未过期
        Note over FE: 使用缓存的路由
        FE->>FE: 直接加载路由
    else 无缓存或缓存过期
        FE->>API: GET /get-async-routes<br/>Authorization: Bearer xxx
        API->>Auth: 验证 token
        Auth->>Auth: 提取 TenantContext.roles
        Auth->>Auth: 提取 TenantContext.permissions

        alt 用户是 admin
            Note over API: 返回完整 EMS 菜单
            API-->>FE: 200 OK<br/>{
  routes: [
    {path: "/ems", children: [
      {path: "/ems/projects", ...},
      {path: "/ems/gateways", ...},
      {path: "/ems/devices", ...},
      {path: "/ems/points", ...},
      {path: "/ems/point-mappings", ...}
    ]}
  ]
}
        else 用户有特定角色
            Note over API: 基于权限过滤路由
            API-->>FE: 200 OK<br/>{
  routes: [
    {path: "/ems", children: [
      {path: "/ems/projects", meta: {auths: ["PROJECT.READ"]}},
      ...
    ]}
  ]
}
        end

        FE->>FE: 解析异步路由
        FE->>FE: 查找 /src/views/** 对应组件
        Note over FE: import.meta.glob 动态导入
        FE->>FE: 注册到 Vue Router
        FE->>FE: 存储到 localStorage.async-routes
        FE->>FE: 生成侧边栏菜单
    end
```

**路由结构：**
```mermaid
graph TB
    Root["/ems<br/>Layout 组件"]
    P1["/ems/projects<br/>ems/projects/index.vue"]
    P2["/ems/gateways<br/>ems/gateways/index.vue"]
    P3["/ems/devices<br/>ems/devices/index.vue"]
    P4["/ems/points<br/>ems/points/index.vue"]
    P5["/ems/point-mappings<br/>ems/point-mappings/index.vue"]

    Root --> P1
    Root --> P2
    Root --> P3
    Root --> P4
    Root --> P5

    style Root fill:#3f51b5,color:#fff
    style P1 fill:#e1f5ff
    style P2 fill:#e1f5ff
    style P3 fill:#e1f5ff
    style P4 fill:#e1f5ff
    style P5 fill:#e1f5ff
```

---

## 错误处理流程

```mermaid
graph TB
    subgraph "错误来源"
        AUTH[AuthError]
        STORAGE[StorageError]
        VALID[ValidationError]
    end

    subgraph "错误类型"
        E1[InvalidCredentials<br/>401]
        E2[TokenExpired/Invalid<br/>401]
        E3[NotFound<br/>404]
        E4[Conflict<br/>409]
        E5[Forbidden<br/>403]
        E6[Internal<br/>500]
    end

    subgraph "统一响应格式"
        APIR[ApiResponse<br/>{
  success: false,
  error: {code, message}
}]
    end

    AUTH --> E1
    AUTH --> E2
    STORAGE --> E3
    STORAGE --> E4
    STORAGE --> E5
    VALID --> E6

    E1 --> APIR
    E2 --> APIR
    E3 --> APIR
    E4 --> APIR
    E5 --> APIR
    E6 --> APIR

    APIR -->|响应头 x-request-id| FE[前端]
    FE -->|显示错误提示| USER[用户]

    style AUTH fill:#ff6b6b
    style STORAGE fill:#ffa726
    style VALID fill:#4db6ac
    style APIR fill:#ffd93d
```

---

## 数据库查询示例（以 Project 为例）

```mermaid
graph LR
    A[Handler 调用] --> B[TenantContext<br/>tenant_id = "tenant-1"]
    B --> C[ProjectStore.list_projects]
    C --> D[SQL 查询生成]
    D --> E["SELECT * FROM projects<br/>WHERE tenant_id = $1"]
    E --> F[PostgreSQL 执行]
    F --> G[返回 ProjectRecord[]]
    G --> H[Handler 接收]
    H --> I[转换为 ProjectDto]
    I --> J[ApiResponse 包装]
    J --> K[HTTP 响应]

    style B fill:#d1c4e9
    style E fill:#4caf50
    style I fill:#2196f3
```

**租户隔离保证：**
- 所有查询自动带 `WHERE tenant_id = ?`
- 跨租户数据访问在 SQL 层面被阻止
- `tenant_id` 从 JWT 提取，不在 API 参数中

---

## 完整用户操作流程示例

### 场景：用户登录后创建一个网关

```mermaid
sequenceDiagram
    participant U as 👤 用户
    participant FE as 🌐 前端
    participant API as 🚀 后端
    participant DB as 🗄️ PostgreSQL

    U->>FE: 1. 输入用户名密码登录
    FE->>API: POST /login
    API->>DB: 验证用户
    DB-->>API: 用户信息
    API-->>FE: 返回 tokens
    FE->>FE: 存储 tokens

    U->>FE: 2. 访问网关管理页面
    FE->>FE: 加载动态路由
    FE->>API: GET /get-async-routes
    API-->>FE: 返回 EMS 路由
    FE->>FE: 显示网关菜单

    U->>FE: 3. 点击"创建网关"
    FE->>API: GET /projects/:id/gateways
    API->>DB: WHERE tenant_id=? AND project_id=?
    DB-->>API: 网关列表
    API-->>FE: 返回网关列表
    FE-->>U: 显示网关表格

    U->>FE: 4. 填写网关信息
    FE->>API: POST /projects/:id/gateways<br/>{name, status}
    API->>API: 验证字段
    API->>API: 生成 UUID (gateway_id)
    API->>DB: INSERT INTO gateways<br/>(gateway_id, tenant_id, project_id, name, status)
    DB-->>API: 插入成功
    API-->>FE: 返回新网关
    FE->>FE: 刷新列表
    FE-->>U: 显示新网关
```

---

## 规划中的功能（M3-M5）

### M3: MQTT 数据采集闭环（规划中）

```mermaid
graph TB
    subgraph "数据采集"
        MQTT[MQTT Broker]
        INGEST[Ingest Capability<br/>订阅主题]
        NORM[Normalize Capability<br/>点位映射]
        PIPE[Pipeline Capability<br/>去重/质量/批量写入]
    end

    subgraph "存储"
        TSDB[(TimescaleDB<br/>measurement hypertable)]
        REDIS[(Redis<br/>last_value 缓存)]
    end

    MQTT --> INGEST
    INGEST -->|RawEvent| NORM
    NORM -->|PointValue| PIPE
    PIPE -->|批量写入| TSDB
    PIPE -->|实时更新| REDIS

    style INGEST fill:#9e9e9e,stroke-dasharray: 5 5
    style NORM fill:#9e9e9e,stroke-dasharray: 5 5
    style PIPE fill:#9e9e9e,stroke-dasharray: 5 5
    style TSDB fill:#9e9e9e,stroke-dasharray: 5 5
    style REDIS fill:#9e9e9e,stroke-dasharray: 5 5
```

### M4: 控制下发闭环（规划中）

```mermaid
graph TB
    subgraph "控制流程"
        UI[前端控制界面]
        API[Control API]
        CMD[Command Service]
        DISP[Dispatcher<br/>MQTT 发布]
    end

    subgraph "执行与反馈"
        DEV[设备执行]
        MQTT[MQTT Broker]
        RCP[Receipt 处理]
    end

    subgraph "存储"
        CMDT[(commands 表)]
        RCTT[(command_receipts 表)]
        ADT[(audit_logs 表)]
    end

    UI --> API
    API --> CMD
    CMD --> CMDT
    CMD --> DISP
    DISP --> MQTT
    MQTT --> DEV
    DEV --> MQTT
    MQTT --> RCP
    RCP --> RCTT
    RCP --> ADT

    style CMD fill:#9e9e9e,stroke-dasharray: 5 5
    style DISP fill:#9e9e9e,stroke-dasharray: 5 5
    style RCP fill:#9e9e9e,stroke-dasharray: 5 5
```

### M5: 告警框架（规划中）

```mermaid
graph TB
    subgraph "告警管理"
        UI[告警规则 UI]
        API[Alarm API]
        RULE[Rule Service]
    end

    subgraph "告警引擎"
        ENGINE[Engine 接口<br/>规则评估]
        EVENTS[(alarm_events 表)]
    end

    UI --> API
    API --> RULE
    RULE --> RULES[(alarm_rules 表)]
    ENGINE --> RULES
    ENGINE --> EVENTS

    style RULE fill:#9e9e9e,stroke-dasharray: 5 5
    style ENGINE fill:#9e9e9e,stroke-dasharray: 5 5
```

---

## 总结

**当前运行的核心流程：**

1. ✅ **认证流程**：用户登录 → JWT 签发 → Token 存储 → 后续请求携带
2. ✅ **API 请求**：request_context → JWT 验证 → TenantContext 提取 → 项目归属验证 → Storage 调用
3. ✅ **数据 CRUD**：前端验证 → Handler 验证 → Storage 执行 → PostgreSQL 查询 → DTO 转换 → 响应返回
4. ✅ **动态路由**：Token 验证 → 角色权限提取 → 路由生成 → 前端注册 → 菜单显示
5. ✅ **多租户隔离**：JWT 提取 tenant_id → SQL 过滤 tenant_id → 跨租户访问阻止

**关键设计原则：**
- 📐 **依赖方向**：domain → storage → handler → api
- 🔒 **租户隔离**：所有数据访问显式传递 TenantContext
- 🚪 **中间件链**：request_id/trace_id → JWT 验证 → 项目归属
- 📦 **统一响应**：ApiResponse 包装所有 API 输出
- 🗄️ **SQL 集中**：所有数据库操作在 storage 层，handler 无 SQL

**下一步扩展方向：**
- 📡 实现 MQTT 采集（M3）
- 🎮 实现控制下发（M4）
- 🚨 实现告警引擎（M5）
- 📊 集成 TimescaleDB 时序存储
- ⚡ 集成 Redis 实时缓存
