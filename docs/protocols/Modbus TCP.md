
# Modbus TCP 网关设备配置规范（定义文档）

**文档版本**：V1.0
**适用范围**：网关通过 Modbus TCP 采集/控制下挂设备（PLC、电表、仪表等）
**目标**：统一网关配置字段、地址规则、数据解析规则、轮询与异常策略，确保不同设备可一致接入。

---

## 1. 术语与基本约定

| 术语            | 含义                                          |
| ------------- | ------------------------------------------- |
| 网关（Gateway）   | 作为 Modbus TCP Client（主站）发起请求                |
| 设备（Device）    | Modbus TCP Server（从站）响应请求                   |
| Unit ID / 从站号 | MBAP 的 Unit Identifier，用于网关区分下挂设备（常见 1–247） |
| 点位（Point/Tag） | 一个可采集或可写入的数据项（如电压、电流、开关量）                   |
| 寄存器（Register） | Modbus 数据单元，16-bit（1 个寄存器=2字节）              |

**字节序约定**

* 单寄存器（16-bit）：Big-Endian（Modbus 标准）
* 多寄存器（32-bit/Float）：必须明确 **Word Order（字序）** 与 **Byte Order（字节序）**（见第 4 章）

---

## 2. 网关连接（Connection）配置字段规范

每个 Modbus TCP 连接对应一个目标 IP/端口（可承载多个 Unit ID 设备）。

| 字段名                | 类型     | 必填 | 取值/范围      | 说明                    |
| ------------------ | ------ | -: | ---------- | --------------------- |
| connectionName     | string |  是 | 唯一         | 连接名称                  |
| host               | string |  是 | IPv4/域名    | 设备/网关侧服务端地址           |
| port               | uint16 |  否 | 默认 502     | Modbus TCP 端口         |
| connectTimeoutMs   | int    |  否 | 1000–10000 | 建连超时                  |
| requestTimeoutMs   | int    |  否 | 200–5000   | 单次请求超时                |
| maxRetries         | int    |  否 | 0–5        | 超时重试次数                |
| retryIntervalMs    | int    |  否 | 50–2000    | 重试间隔                  |
| maxInflight        | int    |  否 | 1–16       | 并发事务数（Transaction ID） |
| keepAlive          | bool   |  否 | true/false | 是否长连接                 |
| reconnectBackoffMs | int    |  否 | 500–60000  | 断线重连退避                |

---

## 3. 设备（Device）配置字段规范

一个 Device 对应一个 Unit ID（从站号），挂在某个 Connection 下。

| 字段名           | 类型     | 必填 | 取值/范围       | 说明            |
| ------------- | ------ | -: | ----------- | ------------- |
| deviceName    | string |  是 | 唯一          | 设备名称          |
| unitId        | uint8  |  是 | 1–247（建议）   | 从站号 / Unit ID |
| enabled       | bool   |  否 | true/false  | 是否启用          |
| scanGroup     | string |  否 | default/自定义 | 轮询分组（不同周期）    |
| offlinePolicy | enum   |  否 | see §6      | 离线判定策略        |
| tags          | array  |  是 | 点位列表        | 采集/控制点        |

> 说明：Modbus TCP 标准中 Unit ID 在纯 TCP 场景有时不关键，但**在网关下挂多设备/网关转发场景非常关键**，规范必须要求填写。

---

## 4. 点位（Tag/Point）配置规范（重点）

### 4.1 点位字段定义（表格）

| 字段名               | 类型     |    必填 | 取值/范围                                       | 说明                           |
| ----------------- | ------ | ----: | ------------------------------------------- | ---------------------------- |
| tagName           | string |     是 | 唯一（同设备内）                                    | 点位名（如 voltageA）              |
| functionCode      | enum   |     是 | 01/02/03/04/05/06/0F/10                     | 功能码（读/写）                     |
| address           | uint16 |     是 | 0–65535                                     | **起始地址（偏移地址）**               |
| quantity          | uint16 |   读必填 | 1–125（03/04）                                | 读寄存器数量；线圈可更大但建议限制            |
| dataType          | enum   |   读必填 | bool/int16/uint16/int32/uint32/float/string | 解析类型                         |
| byteOrder         | enum   |     否 | BE/LE                                       | 单寄存器通常 BE；需要时明确              |
| wordOrder         | enum   | 32位必填 | AB/CD / CD/AB（常见：ABCD、CDAB、BADC、DCBA）       | 多寄存器字序/字节序组合                 |
| scale             | number |     否 | 默认 1                                        | 比例系数（原始值×scale）              |
| offset            | number |     否 | 默认 0                                        | 偏移（×scale 后再 +offset 或相反需约定） |
| unit              | string |     否 | V/A/kW…                                     | 工程单位                         |
| pollIntervalMs    | int    |     否 | 200–60000                                   | 单点轮询周期（或继承 scanGroup）        |
| rw                | enum   |     是 | R/W/RW                                      | 读写属性                         |
| writeFunctionCode | enum   |   写可选 | 05/06/0F/10                                 | 写入使用的功能码                     |
| writeAddress      | uint16 |   写可选 | 0–65535                                     | 写入地址（可与读相同）                  |
| writeDataType     | enum   |   写可选 | 同 dataType                                  | 写入类型                         |
| deadband          | number |     否 | 默认 0                                        | 死区，减少上报                      |
| qualityPolicy     | enum   |     否 | see §6                                      | 数据质量策略                       |

### 4.2 功能码允许的寄存器类型（规范约束）

| 功能码    | 名称                       | 目标区         |  quantity 上限建议 |
| ------ | ------------------------ | ----------- | -------------: |
| 01     | Read Coils               | Coil（0x）    | ≤ 2000（建议≤512） |
| 02     | Read Discrete Inputs     | DI（1x）      | ≤ 2000（建议≤512） |
| 03     | Read Holding Registers   | Holding（4x） |          ≤ 125 |
| 04     | Read Input Registers     | Input（3x）   |          ≤ 125 |
| 05     | Write Single Coil        | Coil        |           固定 1 |
| 06     | Write Single Register    | Holding     |           固定 1 |
| 0F(15) | Write Multiple Coils     | Coil        |         建议≤512 |
| 10(16) | Write Multiple Registers | Holding     |  建议≤60（很多设备更小） |

> 网关实现应在配置校验阶段阻止非法组合（比如用 03 去读离散输入）。

---

## 5. 地址规则（你说的“地址码”重点在这里）

### 5.1 地址填写规范（强制）

**网关配置的 address 字段必须填写“偏移地址（0-based）”。**

* 即报文中的 Start Address 直接等于 `address`
* 不允许填写 3xxxx/4xxxx 这种带前缀的“人类地址”

✅ 示例（设备手册写 40001）

* 很多手册写法：Holding Register 40001
* 规范要求：`address = 0`（因为 40001 对应偏移 0）

### 5.2 手册地址换算规则（附录公式）

| 手册写法  | 区域          | 偏移 address 计算         |
| ----- | ----------- | --------------------- |
| 0xxxx | Coil        | address = 手册号 - 00001 |
| 1xxxx | DI          | address = 手册号 - 10001 |
| 3xxxx | Input Reg   | address = 手册号 - 30001 |
| 4xxxx | Holding Reg | address = 手册号 - 40001 |

> 你可以在文档里明确：若设备手册已给出“Start Address(0-based)”，则直接填；若给的是 3xxxx/4xxxx，则按上表换算。

---

## 6. 轮询与异常处理规范

### 6.1 轮询策略（建议规范）

| 项目              | 建议                                               |
| --------------- | ------------------------------------------------ |
| scanGroup       | 支持按组配置周期（如 fast=1s, normal=5s, slow=30s）         |
| 合并读（Batch Read） | 同一 unitId、同一 functionCode、地址连续的点位应合并为一次读取，减少请求次数 |
| 最大合并长度          | 03/04 合并后 quantity ≤125；建议≤60 以兼容更多设备            |
| 写入优先            | 写入请求应优先于轮询，写后可触发读回校验                             |

### 6.2 异常/离线判定

| 场景                   | 规则                                 |
| -------------------- | ---------------------------------- |
| 单次超时                 | 标记本次点位 quality=BAD，按 maxRetries 重试 |
| 连续失败 N 次             | 标记 device offline（N 建议 3–5）        |
| Modbus 异常响应（0x80+fc） | 记录 exceptionCode；按配置决定是否继续轮询该点位    |
| 非法地址（0x02）           | 默认：禁用该点位并告警（避免无限失败）                |

---

## 7. 数据类型解析规则（寄存器到工程值）

### 7.1 基本类型占用寄存器数

| dataType           |  寄存器数 | 说明                     |
| ------------------ | ----: | ---------------------- |
| bool               | 1 bit | 对应 01/02/05/0F         |
| int16/uint16       |     1 | 16-bit                 |
| int32/uint32/float |     2 | 32-bit（必须指定 wordOrder） |
| string             |     N | 需要指定长度与编码（ASCII/UTF-8） |

### 7.2 32-bit 字序/字节序（必须在点位上明确）

推荐定义一个枚举 `wordOrder`：

* **ABCD**：寄存器顺序=高字在前，字节=高字节在前（常见默认）
* **CDAB**：寄存器交换
* **BADC / DCBA**：字节交换等组合（某些设备会用）

> 文档里要明确：**未指定 wordOrder 的 32-bit 点位配置校验不通过**。

---

## 8. 配置示例（可直接落库/JSON）

### 8.1 设备示例：电表（unitId=1）

* 读保持寄存器 03
* 手册：40001 电压、40003 电流、40005 功率（假设）

换算后：

* 40001 → address=0
* 40003 → address=2
* 40005 → address=4

示例（TL;DR）：

| tagName | functionCode | address | quantity | dataType | scale | unit |
| ------- | ------------ | ------: | -------: | -------- | ----: | ---- |
| voltage | 03           |       0 |        1 | uint16   |   0.1 | V    |
| current | 03           |       2 |        1 | uint16   |   0.1 | A    |
| power   | 03           |       4 |        2 | int32    |   0.1 | kW   |

---

## 9. 配置校验规则（上线前必须做）

1. unitId ∈ [1,247]（默认）
2. functionCode 与 dataType 匹配（bool ↔ 01/02/05/0F；寄存器 ↔ 03/04/06/10）
3. 03/04：quantity ∈ [1,125]
4. 32-bit/float：必须填写 wordOrder
5. 地址换算后的 address 必须 ≥0
6. 合并读时不得跨越不同 functionCode / unitId

---

## 10. 快速联调脚本（本仓库自带）

仓库自带两个无第三方依赖的 Modbus/TCP 联调脚本，适合在现场快速验证网关与设备是否可稳定读写：

### 10.1 基础读写：`scripts/modbus_tcp_tool.py`

* 读保持寄存器（手册 40001 → address=0）：

`python3 scripts/modbus_tcp_tool.py --host <ip> --port <port> --unit-id 1 read --function 3 --address 0 --quantity 1`

* 写保持寄存器（把 40001 写成 1）：

`python3 scripts/modbus_tcp_tool.py --host <ip> --port <port> --unit-id 1 write --function 6 --address 0 --value 1`

### 10.2 继电器稳定性循环测试：`scripts/modbus_relay_loop_test.py`

用途：持续把输出置 True，并同时校验输出读回值（可选再校验一个“反馈输入”地址），任何一次失败立即退出并返回非 0。

* 仅校验输出（默认 out-ref=40001）：

`python3 scripts/modbus_relay_loop_test.py --host <ip> --port <port> --unit-id 1 --required-successes 100 --verbose`

* 同时校验反馈输入（示例：把反馈当作 00001；如你的设备反馈是 10001/30001/40002 等，改成对应 ref 即可）：

`python3 scripts/modbus_relay_loop_test.py --host <ip> --port <port> --unit-id 1 --out-ref 40001 --fb-ref 00001 --required-successes 100 --verbose`

> `--required-successes 0` 表示无限循环直到失败或 Ctrl+C。
