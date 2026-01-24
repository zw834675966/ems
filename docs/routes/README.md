# 页面路由实现原理 (Page Routing)

本文档详细介绍了本项目（Vue 3 + TypeScript）的前端路由实现原理，包括静态路由自动加载、动态路由权限控制、以及路由扁平化处理等核心机制。

## 1. 核心架构

项目采用 `vue-router` 4.x 进行路由管理，路由模式支持 `hash` 和 `h5`（history）模式。核心逻辑位于 `web/admin/src/router` 目录下：

- **`index.ts`**: 路由入口，负责创建 Router 实例、全局路由守卫（Guards）以及静态路由的自动化导入。
- **`utils.ts`**: 路由工具函数库，包含动态路由处理、权限过滤、路由扁平化、菜单升序排序等核心算法。

## 2. 静态路由 (Static Routes)

静态路由主要用于无需权限控制的页面（如登录页、重定向页）以及基础布局。

### 自动化导入
项目使用 Vite 的 `import.meta.glob` 功能自动导入 `src/router/modules` 目录下的所有定义文件，无需手动注册。

```typescript
// src/router/index.ts
const modules: Record<string, any> = import.meta.glob(
  ["./modules/**/*.ts", "!./modules/**/remaining.ts"],
  { eager: true }
);
```

### 路由处理
导入的静态路由会被处理为二级路由结构，并进行扁平化，以确保 `keep-alive` 缓存机制的正常工作（Vue Router 的 keep-alive 在多级嵌套下往往存在问题，因此这里统一拍平）。

## 3. 动态路由 (Dynamic Routes)

动态路由是本项目权限控制的核心，依据后端返回的菜单数据动态生成。

### 初始化流程 (`initRouter`)
1.  **获取数据**: 调用 `getAsyncRoutes()` 接口从后端获取当前用户的菜单数据。
2.  **处理路由**: 调用 `handleAsyncRoutes(data)` 进行处理：
    *   **后端格式转换**: 将后端返回的菜单结构转换为 Vue Router 可识别的 `RouteRecordRaw` 结构。
    *   **组件映射**: 将后端返回的字符串格式 component 路径映射为真实的组件导入函数。
    *   **权限过滤**: 虽然通常后端已过滤，前端 `usePermissionStoreHook` 也会进行二次校验和状态存储。
3.  **动态添加**: 使用 `router.addRoute()` 将处理好的路由动态添加到 Router 实例中。

### 关键函数 (`src/router/utils.ts`)

*   **`addAsyncRoutes`**: 递归处理后端路由，填充 `meta` 信息（如 `backstage: true` 标识），处理重定向逻辑，并进行组件路径匹配。
*   **`formatFlatteningRoutes`**: 将多级嵌套路由扁平化为一维数组。
*   **`formatTwoStageRoutes`**: 将一维数组重新组合为二级路由结构（所有业务页面挂载在 Layout 下），确保存储和展示的层级一致性。
*   **`ascending`**: 根据 `meta.rank` 字段对菜单进行升序排序。

## 4. 路由守卫 (Navigation Guards)

全局前置守卫 `router.beforeEach` 负责：

1.  **进度条控制**: `NProgress.start()`。
2.  **权限校验**:
    *   检查是否已登录（Cookie/Token）。
    *   检查是否前往白名单页面（如 `/login`）。
    *   **动态路由加载**: 如果发现当前路由未加载（`wholeMenus.length === 0`），则触发 `initRouter()` 初始化动态路由，然后使用 `next({ ...to, replace: true })` 重新跳转，确保路由已添加。
3.  **标题设置**: 根据路由元信息设置页面 Title。
4.  **标签页管理**: 通过 `useMultiTagsStoreHook` 同步更新多标签页（Tags View）状态。

## 5. 权限控制 (RBAC)

虽然路由主要是根据后端返回生成的，但在前端也实现了基于角色的权限控制逻辑：

*   **`filterNoPermissionTree`**: 根据用户角色 (`userInfo.roles`) 过滤无权限的静态路由。
*   **`hasAuth` / `getAuths`**: 用于按钮级别的细粒度权限控制，判断当前用户是否拥有特定操作权限。

## 6. 后端路由对应

前端路由最终请求的数据通常对应后端的 API 路由。后端路由定义在 `apps/ems-api/src/routes.rs` 中，使用 `AddRoute` trait 将不同的 Handler 模块（如 `projects`, `auth`, `system_logs`）注册到 Axum 的 Router 中。

---

**总结**: 本项目的路由系统是一个高度自动化、结合了前后端权限控制的动态路由系统。它解决了传统多级嵌套路由的缓存痛点，并提供了灵活的静态/动态路由混合管理能力。
