-- 为 points 表添加 protocol_detail 列
-- 用于存储 Modbus TCP 等协议的详细配置

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'points' AND column_name = 'protocol_detail'
    ) THEN
        ALTER TABLE points ADD COLUMN protocol_detail JSONB;
    END IF;
END $$;

-- 添加注释
COMMENT ON COLUMN points.protocol_detail IS 'Modbus TCP等协议的详细配置（JSON格式），例如：{"function_code": 3, "register_address": 100, "register_count": 2, "data_type": "float"}';
