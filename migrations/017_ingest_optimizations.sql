-- Ingest 性能优化索引
-- 迁移版本：017
-- 描述：为采集引擎频繁查询的字段添加索引

-- 1. 网关协议类型索引
-- ems-collector 会频繁调用 `list_gateways_by_protocol_type`
-- 添加索引 (protocol_type) 以加速该查询
CREATE INDEX IF NOT EXISTS idx_gateways_protocol_type 
    ON gateways (protocol_type);

-- 2. 设备所属网关索引
-- 采集任务初始化时需根据网关ID快速查找下属设备
-- 虽然目前 list_devices 是全量加载，但为将来 filter_by_gateway 预留索引
CREATE INDEX IF NOT EXISTS idx_devices_gateway_id
    ON devices (gateway_id);

-- 3. （可选）设备在线状态索引
-- 如果未来将在线状态持久化，该索引将有助于过滤在线设备
CREATE INDEX IF NOT EXISTS idx_gateways_status
    ON gateways (status);
