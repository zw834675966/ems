-- 系统日志表
-- 用于存储操作日志、错误日志和警告日志，供前端消息通知组件展示

CREATE TABLE IF NOT EXISTS system_logs (
    -- 主键与基础信息
    log_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT,  -- 可选，某些系统级日志不关联项目
    
    -- 日志分类
    category TEXT NOT NULL,  -- 'operation' | 'error' | 'warning'
    level TEXT NOT NULL,     -- 'info' | 'warn' | 'error'
    
    -- 日志内容
    title TEXT NOT NULL,     -- 日志标题（简短描述）
    message TEXT NOT NULL,   -- 详细消息
    source TEXT,             -- 来源模块（如 'ems.ingest', 'ems.api.handlers'）
    
    -- 关联信息
    resource TEXT,           -- 关联资源（如 'gateway:gw-123'）
    actor TEXT,              -- 触发者（用户ID或系统）
    
    -- 元数据
    metadata JSONB,          -- 额外元数据（堆栈信息、请求ID等）
    
    -- 状态与时间
    is_read BOOLEAN DEFAULT false NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    read_at TIMESTAMPTZ
);

-- 索引优化：按租户、分类、创建时间查询
CREATE INDEX IF NOT EXISTS idx_system_logs_tenant_category_created 
    ON system_logs (tenant_id, category, created_at DESC);

-- 索引优化：快速查询未读日志
CREATE INDEX IF NOT EXISTS idx_system_logs_unread 
    ON system_logs (tenant_id, is_read, created_at DESC) 
    WHERE is_read = false;

-- 索引优化：按项目查询
CREATE INDEX IF NOT EXISTS idx_system_logs_project 
    ON system_logs (project_id, created_at DESC) 
    WHERE project_id IS NOT NULL;

-- 注释说明
COMMENT ON TABLE system_logs IS '系统日志表，用于前端消息通知展示';
COMMENT ON COLUMN system_logs.category IS '日志分类: operation(操作日志), error(错误), warning(警告)';
COMMENT ON COLUMN system_logs.level IS '日志级别: info, warn, error';
COMMENT ON COLUMN system_logs.is_read IS '是否已读标记';
