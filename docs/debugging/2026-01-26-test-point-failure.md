# 故障分析报告：Modbus 测试失败 "missing field registerAddress"

**日期**: 2026-01-26  
**分析对象**: 测点测试功能 (`test_point`) 报错问题

## 1. 问题描述
用户在执行 "测试网关" 下的 "继电器" 点位连接测试时，系统返回以下错误：
> Invalid configuration: config parse error: point detail: missing field `registerAddress` at line 1 column 2

## 2. 根因分析 (Root Cause Analysis)

经过代码走查与数据库状态确认，该错误由 **后端测试逻辑读取配置的位置与前端保存配置的位置不一致** 导致。

### 证据链：
1.  **前端行为**：
    *   用户使用的配置界面组件为 `EmsPointMappings.vue` / `ModbusConfigForm.vue`。
    *   该组件将点位的协议配置保存到了 `point_sources` 表（即点位映射表）中。
    *   数据库验证：查询 `point_sources` 表，"继电器" 点位存在完整的配置 JSON（包含 `registerAddress`）。

2.  **后端逻辑** (`test_point` handler)：
    *   位置：`apps/ems-api/src/handlers/collection_strategy.rs`
    *   代码逻辑：
        ```rust
        // 仅从 points 表读取 protocol_detail
        let protocol_detail = point.protocol_detail.as_deref().unwrap_or("{}");
        // 如果为空，则默认使用 "{}"
        ```
    *   数据库验证：查询 `points` 表，"继电器" 点位的 `protocol_detail` 字段为 `null`。

3.  **错误触发**：
    *   `test_point` 函数读取到空的 `protocol_detail` (即 `null` -> `"{}"`)。
    *   尝试将空 JSON `{}` 反序列化为 `ModbusPointDetail` 结构体。
    *   结构体定义要求 `registerAddress` 字段必填（见 `ModbusPointDetail` 定义），因此抛出 `missing field` 错误。

## 3. 解决方案 (Proposed Solution)

修改 `test_point` 函数的实现逻辑，使其具有回退查找机制。

### 修改计划：
1.  **文件**: `apps/ems-api/src/handlers/collection_strategy.rs`
2.  **逻辑变更**:
    *   保持获取 `point` 的逻辑不变。
    *   在获取 `protocol_detail` 时，先检查 `point.protocol_detail` 是否为空。
    *   如果为空，则调用 `point_mapping_store.list_point_mappings` 获取该项目的映射列表。
    *   在列表中查找与当前 `point_id` 匹配的记录，并使用其 `protocol_detail`。
    *   如果两者都为空，再报错提示配置缺失。

### 优化建议 (后续):
*   当前 `PointMappingStore` 仅提供了 `list_point_mappings` (查询全量) 接口，对于单点测试性能稍低（但项目级数据量通常可接受）。
*   建议后续在 `ems_storage` 中增加 `find_mappings_by_point_id` 接口以优化性能。

## 4. 待确认事项
*   [ ] 是否同意执行上述后端的代码修改？
