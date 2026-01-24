# Web Admin 前端技术文档

## 项目概述

基于 `vue-pure-admin-thin`（精简版）构建的 EMS 管理后台前端项目。

- **框架**: Vue 3 + TypeScript + Vite
- **UI 库**: Element Plus + TailwindCSS
- **状态管理**: Pinia
- **路由**: Vue Router 4
- **HTTP 客户端**: Axios
- **Mock 服务**: vite-plugin-fake-server

---

## 1. API 接口

### 1.1 API 目录结构

```
src/api/
├── ems/                    # EMS 业务模块接口
│   ├── projects.ts         # 项目管理
│   ├── devices.ts          # 设备管理
│   ├── gateways.ts        # 网关管理
│   ├── points.ts          # 数据点管理
│   ├── pointMappings.ts   # 数据点映射
│   └── types.ts          # 通用类型定义
├── rbac/                   # RBAC 管理模块接口（tenant 级）
│   ├── users.ts
│   ├── roles.ts
│   └── permissions.ts
├── routes.ts            # 动态路由接口
└── user.ts             # 用户认证接口
```

### 1.2 HTTP 配置

**配置文件**: `src/utils/http/index.ts`

**关键特性**:

- 基于 Axios 封装
- 自动 token 管理和刷新
- 请求白名单机制
- 自动处理过期 token 刷新
- 默认超时: 10秒
- 请求白名单: `/login`, `/refresh-token`

**请求拦截器**:

```typescript
// 自动添加 Bearer Token（白名单除外）
config.headers["Authorization"] = formatToken(data.accessToken);

// token 过期自动刷新
if (expired) {
  useUserStoreHook().handRefreshToken({ refreshToken: data.refreshToken });
}
```

**响应拦截器**:

- 直接返回 `response.data`
- 支持自定义请求/响应回调

### 1.3 API 接口清单

#### 1.3.1 用户认证 (`src/api/user.ts`)

```typescript
// 登录
POST /login
Request: { username, password }
Response: {
  success: boolean;
  data: {
    avatar: string;
    username: string;
    nickname: string;
    roles: string[];
    permissions: string[];
    accessToken: string;
    refreshToken: string;
    expires: number; // Unix ms timestamp
  };
}

// 刷新 token
POST /refresh-token
Request: { refreshToken }
Response: {
  success: boolean;
  data: {
    accessToken: string;
    refreshToken: string;
    expires: number; // Unix ms timestamp
  };
}
```

#### 1.3.2 动态路由 (`src/api/routes.ts`)

```typescript
// 获取动态路由
GET /get-async-routes
Response: {
  success: boolean;
  data: RouteRecordRaw[];
}
```

#### 1.3.3 项目管理 (`src/api/ems/projects.ts`)

```typescript
// 获取项目列表
GET /projects
Response: ApiResponse<ProjectDto[]>

// 创建项目
POST /projects
Request: { name, timezone? }
Response: ApiResponse<ProjectDto>

type ProjectDto = {
  projectId: string;
  name: string;
  timezone: string;
};
```

#### 1.3.4 设备管理 (`src/api/ems/devices.ts`)

```typescript
// 获取设备列表
GET /projects/{projectId}/devices
Response: ApiResponse<DeviceDto[]>

// 创建设备
POST /projects/{projectId}/devices
Request: { gatewayId, name, model? }
Response: ApiResponse<DeviceDto>

type DeviceDto = {
  deviceId: string;
  projectId: string;
  gatewayId: string;
  name: string;
  model?: string;
};
```

#### 1.3.5 网关管理 (`src/api/ems/gateways.ts`)

```typescript
// 获取网关列表
GET /projects/{projectId}/gateways
Response: ApiResponse<GatewayDto[]>

// 创建网关
POST /projects/{projectId}/gateways
Request: { name, status? }
Response: ApiResponse<GatewayDto>

type GatewayDto = {
  gatewayId: string;
  projectId: string;
  name: string;
  status: string;
};
```

#### 1.3.6 数据点管理 (`src/api/ems/points.ts`)

```typescript
// 获取数据点列表
GET /projects/{projectId}/points
Response: ApiResponse<PointDto[]>

// 创建数据点
POST /projects/{projectId}/points
Request: { deviceId, key, dataType, unit? }
Response: ApiResponse<PointDto>

type PointDto = {
  pointId: string;
  projectId: string;
  deviceId: string;
  key: string;
  dataType: string;
  unit?: string;
};
```

#### 1.3.7 数据点映射 (`src/api/ems/pointMappings.ts`)

```typescript
// 获取数据点映射列表
GET /projects/{projectId}/point-mappings
Response: ApiResponse<PointMappingDto[]>

// 创建数据点映射
POST /projects/{projectId}/point-mappings
Request: { pointId, sourceType, address, scale?, offset? }
Response: ApiResponse<PointMappingDto>

type PointMappingDto = {
  sourceId: string;
  projectId: string;
  pointId: string;
  sourceType: string;
  address: string;
  scale?: number;
  offset?: number;
};
```

### 1.4 API 代理配置

**Vite 配置**: `vite.config.ts`

```typescript
proxy: {
  "^/(login|refresh-token|get-async-routes|projects|health|rbac)": {
    target: "http://127.0.0.1:8080",
    changeOrigin: true
  }
}
```

**环境变量**:

- 开发环境: `VITE_API_BASE = "http://127.0.0.1:8080"`
- 生产环境: 通过环境变量配置

---

## 2. Mock 模拟接口

### 2.1 Mock 配置

**Mock 框架**: `vite-plugin-fake-server`

**配置文件**: `build/plugins.ts`

```typescript
vitePluginFakeServer({
  logger: false,
  include: "mock",
  infixName: false,
  enableProd: true // 生产环境也启用
});
```

**开关环境变量**:

- `VITE_ENABLE_MOCK = true|false`（默认 false，关闭 mock 可直连后端）

### 2.2 Mock 接口清单

#### 2.2.1 登录接口 (`mock/login.ts`)

```typescript
POST /login
Response (admin):
  {
    success: true,
    data: {
      avatar: "https://avatars.githubusercontent.com/u/44761321",
      username: "admin",
      nickname: "小铭",
      roles: ["admin"],
      permissions: ["*:*:*"],  // 全部权限
      accessToken: "eyJhbGciOiJIUzUxMiJ9.admin",
      refreshToken: "eyJhbGciOiJIUzUxMiJ9.adminRefresh",
      expires: 1919548800000
    }
  }

Response (common):
  {
    success: true,
    data: {
      avatar: "https://avatars.githubusercontent.com/u/52823142",
      username: "common",
      nickname: "小林",
      roles: ["common"],
      permissions: ["permission:btn:add", "permission:btn:edit"],
      accessToken: "eyJhbGciOiJIUzUxMiJ9.common",
      refreshToken: "eyJhbGciOiJIUzUxMiJ9.commonRefresh",
      expires: 1919548800000
    }
  }
```

**支持的账号**:

- `admin` - 管理员角色，拥有全部权限 `*:*:*`
- `common` - 普通角色，拥有部分权限

#### 2.2.2 刷新 Token 接口 (`mock/refreshToken.ts`)

```typescript
POST /refresh-token
Request: { refreshToken }
Response:
  {
    success: true,
    data: {
      accessToken: "eyJhbGciOiJIUzUxMiJ9.newAdmin",
      refreshToken: "eyJhbGciOiJIUzUxMiJ9.newAdminRefresh",
      expires: 1919635199000
    }
  }
```

#### 2.2.3 动态路由接口 (`mock/asyncRoutes.ts`)

```typescript
GET /get-async-routes
Response:
  {
    success: true,
    data: [
      {
        path: "/permission",
        meta: {
          title: "权限管理",
          icon: "ep:lollipop",
          rank: 10
        },
        children: [
          {
            path: "/permission/page/index",
            name: "PermissionPage",
            meta: {
              title: "页面权限",
              roles: ["admin", "common"]
            }
          },
          {
            path: "/permission/button",
            meta: {
              title: "按钮权限",
              roles: ["admin", "common"]
            },
            children: [
              {
                path: "/permission/button/router",
                component: "permission/button/index",
                name: "PermissionButtonRouter",
                meta: {
                  title: "路由返回按钮权限",
                  auths: [
                    "permission:btn:add",
                    "permission:btn:edit",
                    "permission:btn:delete"
                  ]
                }
              },
              {
                path: "/permission/button/login",
                component: "permission/button/perms",
                name: "PermissionButtonLogin",
                meta: {
                  title: "登录接口返回按钮权限"
                }
              }
            ]
          }
        ]
      }
    ]
  }
```

---

## 3. 静态页面和动态页面实现方式

### 3.1 路由架构

**路由文件**: `src/router/index.ts`

**路由类型**:

1. **静态路由** - 固定在代码中，不依赖后端
2. **动态路由** - 从后端 `/get-async-routes` 获取
3. **剩余路由** - 登录页、错误页等不参与菜单的路由

### 3.2 静态路由

**目录**: `src/router/modules/`

**加载方式**: 自动导入

```typescript
// 自动导入 src/router/modules 目录下所有 .ts 文件（except remaining.ts）
const modules: Record<string, any> = import.meta.glob(
  ["./modules/**/*.ts", "!./modules/**/remaining.ts"],
  { eager: true }
);
```

**静态路由列表**:

#### 3.2.1 首页路由 (`router/modules/home.ts`)

```typescript
{
  path: "/",
  name: "Home",
  component: Layout,
  redirect: "/welcome",
  meta: {
    icon: "ep/home-filled",
    title: "首页",
    rank: 0
  },
  children: [
    {
      path: "/welcome",
      name: "Welcome",
      component: () => import("@/views/welcome/index.vue"),
      meta: {
        title: "首页",
        showLink: VITE_HIDE_HOME === "true" ? false : true
      }
    }
  ]
}
```

**特性**:

- 可通过 `VITE_HIDE_HOME` 环境变量隐藏首页
- 默认跳转到 `/welcome`
- 首页组件: `src/views/welcome/index.vue`

#### 3.2.2 错误页面路由 (`router/modules/error.ts`)

```typescript
{
  path: "/error",
  redirect: "/error/403",
  meta: {
    icon: "ri/information-line",
    title: "异常页面",
    rank: 9
  },
  children: [
    { path: "/error/403", component: () => import("@/views/error/403.vue") },
    { path: "/error/404", component: () => import("@/views/error/404.vue") },
    { path: "/error/500", component: () => import("@/views/error/500.vue") }
  ]
}
```

**错误页面**:

- `403` - 无权限访问
- `404` - 页面不存在
- `500` - 服务器错误

### 3.3 剩余路由 (`router/modules/remaining.ts`)

**不参与菜单的路由**:

```typescript
[
  {
    path: "/login",
    name: "Login",
    component: () => import("@/views/login/index.vue"),
    meta: { title: "登录", showLink: false }
  },
  {
    path: "/access-denied",
    name: "AccessDenied",
    component: () => import("@/views/error/403.vue"),
    meta: { title: "403", showLink: false }
  },
  {
    path: "/server-error",
    name: "ServerError",
    component: () => import("@/views/error/500.vue"),
    meta: { title: "500", showLink: false }
  },
  {
    path: "/redirect",
    component: Layout,
    meta: { title: "加载中...", showLink: false },
    children: [
      {
        path: "/redirect/:path(.*)",
        name: "Redirect",
        component: () => import("@/layout/redirect.vue")
      }
    ]
  }
];
```

### 3.4 动态路由

**获取接口**: `GET /get-async-routes`

**处理流程** (`src/router/utils.ts`):

```typescript
// 1. 初始化路由
function initRouter() {
  if (getConfig()?.CachingAsyncRoutes) {
    // 优先从 localStorage 读取缓存
    const asyncRouteList = storageLocal().getItem("async-routes");
    if (asyncRouteList) {
      handleAsyncRoutes(asyncRouteList);
    } else {
      // 从后端获取
      getAsyncRoutes().then(({ data }) => {
        handleAsyncRoutes(data);
        storageLocal().setItem("async-routes", data);
      });
    }
  } else {
    // 不缓存，每次从后端获取
    getAsyncRoutes().then(({ data }) => {
      handleAsyncRoutes(data);
    });
  }
}

// 2. 处理动态路由
function handleAsyncRoutes(routeList) {
  formatFlatteningRoutes(addAsyncRoutes(routeList)).map(v => {
    // 添加到路由
    router.options.routes[0].children.push(v);
    router.addRoute(v);
  });
  usePermissionStoreHook().handleWholeMenus(routeList);
  addPathMatch(); // 添加404匹配路由
}

// 3. 处理后端路由
function addAsyncRoutes(arrRoutes: Array<RouteRecordRaw>) {
  arrRoutes.forEach(v => {
    v.meta.backstage = true; // 标识为后端返回
    if (v?.children && v.children.length && !v.redirect)
      v.redirect = v.children[0].path;
    if (v?.children && v.children.length && !v.name)
      v.name = (v.children[0].name as string) + "Parent";

    // 处理组件
    if (v.component === "Layout") {
      v.component = Layout;
    } else if (v.meta?.frameSrc) {
      v.component = IFrame;
    } else {
      // 自动匹配组件
      const index = modulesRoutesKeys.findIndex(ev =>
        ev.includes(v.component as any)
      );
      v.component = modulesRoutes[modulesRoutesKeys[index]];
    }

    if (v?.children && v.children.length) {
      addAsyncRoutes(v.children);
    }
  });
  return arrRoutes;
}
```

**动态路由特性**:

- 支持 `component: "Layout"` - 使用主布局
- 支持 `meta.frameSrc` - 使用 iframe 嵌入
- 组件自动匹配: 查找 `src/views/**/*.{vue,tsx}`
- 自动处理 `redirect` 和 `name`
- 三级及以上路由自动拍成二级（keep-alive 限制）
- 叶子节点建议省略 `children` 字段，避免菜单过滤 `children.length === 0`

### 3.5 路由守卫

**文件**: `src/router/index.ts`

```typescript
router.beforeEach((to, _from, next) => {
  const userInfo = storageLocal().getItem<DataInfo<number>>(userKey);

  // 1. 已登录
  if (Cookies.get(multipleTabsKey) && userInfo) {
    // 权限检查
    if (to.meta?.roles && !isOneOfArray(to.meta?.roles, userInfo?.roles)) {
      next({ path: "/error/403" }); // 跳转403
    }
    // 隐藏首页检查
    if (VITE_HIDE_HOME === "true" && to.fullPath === "/welcome") {
      next({ path: "/error/404" });
    }
    // 刷新时初始化动态路由
    if (
      usePermissionStoreHook().wholeMenus.length === 0 &&
      to.path !== "/login"
    ) {
      initRouter().then(router => {
        if (isAllEmpty(to.name)) router.push(to.fullPath);
      });
    }
    toCorrectRoute();
  }
  // 2. 未登录
  else {
    if (to.path !== "/login") {
      if (whiteList.indexOf(to.path) !== -1) {
        next();
      } else {
        removeToken();
        next({ path: "/login" });
      }
    } else {
      next();
    }
  }
});
```

**路由白名单**: `/login`

### 3.6 页面组件目录

```
src/views/
├── login/                  # 登录页
│   ├── index.vue
│   ├── utils/
│   │   ├── rule.ts       # 表单验证规则
│   │   ├── static.ts     # 静态资源引用
│   │   └── motion.ts    # 动画配置
├── welcome/               # 首页
│   └── index.vue
├── error/                # 错误页
│   ├── 403.vue
│   ├── 404.vue
│   └── 500.vue
├── permission/           # 权限演示
│   ├── page/
│   │   └── index.vue    # 页面权限
│   └── button/
│       ├── index.vue     # 路由权限按钮
│       └── perms.vue     # 登录权限按钮
└── ems/                 # EMS 业务页面
    ├── projects/         # 项目管理
    ├── gateways/        # 网关管理
    ├── devices/         # 设备管理
    ├── points/          # 数据点管理
    └── point-mappings/  # 数据点映射
```

### 3.7 路由懒加载

所有页面组件使用动态导入:

```typescript
component: () => import("@/views/welcome/index.vue");
component: () => import("@/views/error/404.vue");
```

**Vite 自动代码分割**: 每个动态导入的组件会被分割成单独的 chunk。

### 3.8 路由历史模式

**配置**: `VITE_ROUTER_HISTORY`

**可选模式**:

- `hash` - Hash 模式（默认）
- `h5` - HTML5 模式
- `hash,/base` - Hash 模式带 base 路径
- `h5,/base` - HTML5 模式带 base 路径

**开发环境**: `VITE_ROUTER_HISTORY = "hash"`

---

## 4. 用户登录页面的权限与配置

### 4.1 登录流程

**组件**: `src/views/login/index.vue`

**完整流程**:

```
1. 用户输入账号密码
   ↓
2. 表单验证
   ↓
3. 调用 getLogin API
   ↓
4. 设置 Token 和用户信息
   ↓
5. 初始化动态路由 (initRouter)
   ↓
6. 跳转到第一个菜单页面
```

**关键代码**:

```typescript
const onLogin = async (formEl: FormInstance) => {
  await formEl.validate(valid => {
    if (valid) {
      loading.value = true;
      useUserStoreHook()
        .loginByUsername({ username, password })
        .then(res => {
          if (res.success) {
            // 获取后端路由
            return initRouter().then(() => {
              router.push(getTopMenu(true).path);
              message("登录成功", { type: "success" });
            });
          }
        })
        .finally(() => (loading.value = false));
    }
  });
};
```

### 4.2 Token 管理

**文件**: `src/utils/auth.ts`

**存储策略**:

| 数据         | 存储位置              | Key                              | 说明               |
| ------------ | --------------------- | -------------------------------- | ------------------ |
| accessToken  | Cookie                | `authorized-token`               | 访问令牌           |
| expires      | Cookie                | `authorized-token`               | 过期时间（时间戳） |
| refreshToken | Cookie + LocalStorage | `authorized-token` / `user-info` | 刷新令牌           |
| avatar       | LocalStorage          | `user-info`                      | 头像               |
| username     | LocalStorage          | `user-info`                      | 用户名             |
| nickname     | LocalStorage          | `user-info`                      | 昵称               |
| roles        | LocalStorage          | `user-info`                      | 页面权限角色       |
| permissions  | LocalStorage          | `user-info`                      | 按钮权限 code      |

**Cookie 配置**:

```typescript
Cookies.set(TokenKey, cookieString, {
  expires: (expires - Date.now()) / 86400000 // 自动过期
});
```

**多标签页支持**:

```typescript
export const multipleTabsKey = "multiple-tabs";

// 登录时设置
Cookies.set(
  multipleTabsKey,
  "true",
  isRemembered
    ? { expires: loginDay } // 记住我
    : {}
);
```

**Token 刷新机制**:

```typescript
// HTTP 请求拦截器中自动检查
const data = getToken();
const now = new Date().getTime();
const expired = parseInt(data.expires) - now <= 0;

if (expired && !PureHttp.isRefreshing) {
  PureHttp.isRefreshing = true;
  useUserStoreHook()
    .handRefreshToken({ refreshToken: data.refreshToken })
    .then(res => {
      const token = res.data.accessToken;
      config.headers["Authorization"] = formatToken(token);
      PureHttp.requests.forEach(cb => cb(token));
      PureHttp.requests = [];
    })
    .finally(() => {
      PureHttp.isRefreshing = false;
    });
}
```

### 4.3 权限体系

#### 4.3.1 页面级权限

**实现方式**: 路由守卫 + `roles` 字段

```typescript
// 路由定义
{
  path: "/permission/page/index",
  meta: {
    title: "页面权限",
    roles: ["admin", "common"]  // 允许访问的角色
  }
}

// 路由守卫检查
if (to.meta?.roles && !isOneOfArray(to.meta?.roles, userInfo?.roles)) {
  next({ path: "/error/403" });
}
```

**权限过滤**:

```typescript
// 过滤无权限的菜单
function filterNoPermissionTree(data: RouteComponent[]) {
  const currentRoles =
    storageLocal().getItem<DataInfo<number>>(userKey)?.roles ?? [];
  const newTree = cloneDeep(data).filter(v =>
    isOneOfArray(v.meta?.roles, currentRoles)
  );
  newTree.forEach(
    v => v.children && (v.children = filterNoPermissionTree(v.children))
  );
  return filterChildrenTree(newTree);
}
```

#### 4.3.2 按钮级权限

**两种实现方式**:

##### 方式一: 路由 meta.auths（推荐）

```typescript
// 路由定义
{
  path: "/permission/button/router",
  meta: {
    title: "路由返回按钮权限",
    auths: [
      "permission:btn:add",
      "permission:btn:edit",
      "permission:btn:delete"
    ]
  }
}

// 判断函数
function hasAuth(value: string | Array<string>): boolean {
  const metaAuths = getAuths();
  return isString(value)
    ? metaAuths.includes(value)
    : isIncludeAllChildren(value, metaAuths);
}
```

##### 方式二: 登录接口返回 permissions

```typescript
// 登录响应
{
  permissions: ["permission:btn:add", "permission:btn:edit"];
}

// 判断函数
export const hasPerms = (value: string | Array<string>): boolean => {
  const allPerms = "*:*:*";
  const { permissions } = useUserStoreHook();
  if (permissions.length === 1 && permissions[0] === allPerms) return true;
  return isString(value)
    ? permissions.includes(value)
    : isIncludeAllChildren(value, permissions);
};
```

#### 4.3.3 权限指令

**v-auth 指令**: 登录权限

```vue
<el-button v-auth="'permission:btn:add'" plain type="warning">
  拥有 code：'permission:btn:add' 权限可见
</el-button>
```

**v-perms 指令**: 路由权限（动态）

```vue
<el-button v-perms="'permission:btn:add'" plain type="warning">
  拥有 code：'permission:btn:add' 权限可见
</el-button>
```

**Auth 组件**: 登录权限组件方式

```vue
<Auth value="permission:btn:add">
  <el-button plain type="warning">
    拥有 code：'permission:btn:add' 权限可见
  </el-button>
</Auth>
```

**Perms 组件**: 路由权限组件方式

```vue
<Perms value="permission:btn:add">
  <el-button plain type="warning">
    拥有 code：'permission:btn:add' 权限可见
  </el-button>
</Perms>
```

### 4.4 用户 Store

**文件**: `src/store/modules/user.ts`

**State**:

```typescript
{
  avatar: string;        // 头像
  username: string;      // 用户名
  nickname: string;      // 昵称
  roles: string[];      // 页面权限角色
  permissions: string[];  // 按钮权限 code
  isRemembered: boolean;  // 是否记住登录
  loginDay: number;     // 记住天数（默认7天）
}
```

**Actions**:

```typescript
// 登录
async loginByUsername(data) {
  const res = await getLogin(data);
  if (res?.success) setToken(res.data);
  return res;
}

// 登出
logOut() {
  this.username = "";
  this.roles = [];
  this.permissions = [];
  removeToken();
  resetRouter();
  router.push("/login");
}

// 刷新 token
async handRefreshToken(data) {
  const res = await refreshTokenApi(data);
  if (res) setToken(res.data);
  return res;
}
```

### 4.5 权限 Store

**文件**: `src/store/modules/permission.ts`

**State**:

```typescript
{
  constantMenus: RouteComponent[];    // 静态路由菜单
  wholeMenus: RouteComponent[];       // 整体菜单（静态+动态）
  flatteningRoutes: RouteRecordRaw[]; // 一维数组格式路由
  cachePageList: RouteRecordName[];  // keep-alive 缓存列表
}
```

**Actions**:

```typescript
// 组装整体菜单
handleWholeMenus(routes: any[]) {
  this.wholeMenus = filterNoPermissionTree(
    filterTree(ascending(this.constantMenus.concat(routes)))
  );
  this.flatteningRoutes = formatFlatteningRoutes(
    this.constantMenus.concat(routes)
  );
}

// 缓存操作
cacheOperate({ mode, name }) {
  switch (mode) {
    case "add": this.cachePageList.push(name); break;
    case "delete": 删除并清理缓存; break;
    case "refresh": 先删除后添加; break;
  }
}

// 清空缓存
clearAllCachePage() {
  this.wholeMenus = [];
  this.cachePageList = [];
}
```

### 4.6 登录页配置

**默认账号**:

```typescript
const ruleForm = reactive({
  username: "admin",
  password: "admin123"
});
```

**表单验证规则**:

```typescript
export const loginRules = reactive<FormRules>({
  username: [{ required: true, message: "请输入账号", trigger: "blur" }],
  password: [{ required: true, message: "请输入密码", trigger: "blur" }]
});
```

**键盘快捷键**: Enter / NumpadEnter 提交登录

**主题切换**: 支持亮色/暗色主题切换

### 4.7 权限配置示例

#### 4.7.1 管理员权限

```json
{
  "username": "admin",
  "roles": ["admin"],
  "permissions": ["*:*:*"] // 全部权限
}
```

#### 4.7.2 普通用户权限

```json
{
  "username": "common",
  "roles": ["common"],
  "permissions": ["permission:btn:add", "permission:btn:edit"]
}
```

#### 4.7.3 权限使用示例

```vue
<template>
  <!-- 方式一: 函数判断 -->
  <el-button v-if="hasPerms('permission:btn:add')" @click="handleAdd">
    新增
  </el-button>

  <!-- 方式二: 指令 -->
  <el-button v-perms="'permission:btn:edit'" @click="handleEdit">
    编辑
  </el-button>

  <!-- 方式三: 组件 -->
  <Perms :value="['permission:btn:delete']">
    <el-button @click="handleDelete">删除</el-button>
  </Perms>
</template>
```

### 4.8 环境配置

**.env 文件**:

```bash
# 本地端口
VITE_PORT = 8848

# 是否隐藏首页
VITE_HIDE_HOME = false

# 路由模式
VITE_ROUTER_HISTORY = "hash"

# API 地址
VITE_API_BASE = "http://127.0.0.1:8080"

# 是否启用 mock（开发）
VITE_ENABLE_MOCK = false
```

---

## 5. 项目启动命令

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm dev

# 构建生产版本
pnpm build

# 构建 staging 版本
pnpm build:staging

# 预览构建结果
pnpm preview

# 类型检查
pnpm typecheck

# 代码检查
pnpm lint
```

---

## 6. 技术栈总结

| 技术         | 版本    | 用途        |
| ------------ | ------- | ----------- |
| Vue          | 3.5.22  | 前端框架    |
| Vue Router   | 4.6.3   | 路由管理    |
| Pinia        | 3.0.3   | 状态管理    |
| Element Plus | 2.11.5  | UI 组件库   |
| TailwindCSS  | 4.1.16  | CSS 框架    |
| Axios        | 1.12.2  | HTTP 客户端 |
| Vite         | 7.1.12  | 构建工具    |
| TypeScript   | 5.9.3   | 类型系统    |
| dayjs        | 1.11.18 | 日期处理    |
| js-cookie    | 3.0.5   | Cookie 管理 |
| localforage  | 1.10.0  | 本地存储    |

---

## 7. 关键特性

1. **动态路由**: 支持后端返回路由配置
2. **权限管理**: 页面级 + 按钮级权限控制
3. **Token 刷新**: 无感刷新 accessToken
4. **多标签页**: 支持多标签页共享登录状态
5. **路由缓存**: keep-alive 缓存页面
6. **Mock 服务**: 开发环境支持数据模拟
7. **代码分割**: 自动路由懒加载
8. **主题切换**: 支持亮色/暗色主题
