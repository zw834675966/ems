# PRD：EMS MVP 需求与范围

## 1. 用户与场景
### 1.1 用户角色
- SaaS 管理员（平台级）：创建租户、管理套餐/全局策略（MVP 可弱化）
- 租户管理员：管理项目、用户、角色权限
- 项目运维人员：管理网关/设备/点位映射、查看实时与历史、下发控制
- 只读观察者：仅查看数据与告警

### 1.2 核心场景（MVP）
1) 登录进入后台  
2) 创建项目、配置网关/设备/点位映射  
3) 设备通过 MQTT 上报数据  
4) 系统写入时序库并更新实时 last_value  
5) 前端查询实时与历史曲线/列表  
6) 用户发起控制命令，系统下发并记录回执与审计  

## 2. 功能范围
### 2.1 必须有（MVP）
- 鉴权：JWT access + refresh  
- 权限：RBAC + 项目 scope  
- 项目内资源：Gateway/Device/Point/PointSource CRUD  
- 数据链路：ingest(MQTT) → normalize → pipeline → storage(timescale + redis)  
- 查询：realtime(last_value) + measurements(历史)  
- 控制：commands 下发 + receipt + audit  

### 2.2 应该有（MVP+）
- 在线状态：gateway/device online  
- 基础质量：坏质量标记/离线标记/简单范围校验  
- 批写与背压：写入保护  

### 2.3 可以延后（Post-MVP）
- Modbus TCP 等协议扩展  
- 告警规则引擎（表达式/阈值/离线）  
- 聚合报表（日/月/分时）  
- RLS（数据库行级安全）  

## 3. 非功能需求
- 多租户隔离：任何 API 不允许跨租户读写（默认从 JWT 取 tenant）  
- 可观测性：日志/trace_id/metrics  
- 性能：采集写入可批处理；历史查询支持分页/聚合  
- 可维护性：模块边界固定，接口优先  

## 4. 验收标准（Definition of Done）
- 本地 WSL2：一键启动依赖 + ems-api  
- seed 后可：  
  - 登录获得 token  
  - 创建项目资源  
  - 用 MQTT 模拟写入点位  
  - 查询 realtime 与历史  
  - 发起控制并看到审计与回执
