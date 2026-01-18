-- 采集策略表
-- 存储每个点位的自动采集配置，支持定时采集和时序数据库写入

CREATE TABLE IF NOT EXISTS collection_strategies (
    -- 主键
    strategy_id TEXT PRIMARY KEY,
    
    -- 归属关系
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    point_id TEXT NOT NULL,
    
    -- 采集配置
    enabled BOOLEAN NOT NULL DEFAULT false,           -- 是否启用采集
    interval_value INTEGER NOT NULL DEFAULT 1000,     -- 采集间隔数值
    interval_unit TEXT NOT NULL DEFAULT 'ms',         -- 时间单位: ms, s, min
    write_to_db BOOLEAN NOT NULL DEFAULT false,       -- 是否写入时序数据库
    
    -- 运行状态
    last_collected_at TIMESTAMPTZ,                    -- 最后采集时间
    last_value TEXT,                                  -- 最后采集值
    last_error TEXT,                                  -- 最后错误信息
    
    -- 审计字段
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- 唯一约束：每个点位只能有一个策略
    UNIQUE(tenant_id, project_id, point_id),
    
    -- 外键约束：关联点位表，级联删除
    FOREIGN KEY (point_id) REFERENCES points(point_id) ON DELETE CASCADE
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_collection_strategies_project 
    ON collection_strategies(tenant_id, project_id);

CREATE INDEX IF NOT EXISTS idx_collection_strategies_enabled 
    ON collection_strategies(enabled) WHERE enabled = true;

CREATE INDEX IF NOT EXISTS idx_collection_strategies_point_id
    ON collection_strategies(point_id);

-- 注释
COMMENT ON TABLE collection_strategies IS '采集策略配置表 - 存储每个点位的自动采集和数据写入配置';
COMMENT ON COLUMN collection_strategies.strategy_id IS '策略唯一标识';
COMMENT ON COLUMN collection_strategies.interval_value IS '采集间隔数值';
COMMENT ON COLUMN collection_strategies.interval_unit IS '采集间隔单位: ms(毫秒), s(秒), min(分钟)';
COMMENT ON COLUMN collection_strategies.write_to_db IS '是否将采集数据写入 TimescaleDB 时序数据库';
COMMENT ON COLUMN collection_strategies.last_collected_at IS '最后一次成功采集的时间戳';
COMMENT ON COLUMN collection_strategies.last_value IS '最后一次采集到的值（字符串格式）';
COMMENT ON COLUMN collection_strategies.last_error IS '最后一次采集失败的错误信息（如有）';
