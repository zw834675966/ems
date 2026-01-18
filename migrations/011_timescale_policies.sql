-- ============================================================================
-- TimescaleDB 数据治理策略
-- ============================================================================
-- 
-- 本迁移脚本为 measurement 和 event 超表配置数据生命周期管理策略：
-- 1. 压缩策略：数据写入 7 天后自动压缩，减少存储空间
-- 2. 保留策略：数据保留 365 天后自动删除，防止磁盘爆满
--
-- 注意：策略仅在 TimescaleDB 扩展启用时生效，普通 Postgres 环境跳过
-- ============================================================================

DO $$
DECLARE
    has_timescaledb BOOLEAN;
BEGIN
    -- 检查 TimescaleDB 扩展是否可用
    SELECT EXISTS(
        SELECT 1 FROM pg_extension WHERE extname = 'timescaledb'
    ) INTO has_timescaledb;

    IF NOT has_timescaledb THEN
        RAISE NOTICE '[011] timescaledb not installed, skipping data governance policies';
        RETURN;
    END IF;

    RAISE NOTICE '[011] TimescaleDB detected, configuring data governance policies...';

    -- ========================================================================
    -- 1. measurement 表：压缩与保留策略
    -- ========================================================================
    
    -- 1.1 启用压缩
    BEGIN
        -- 按 tenant_id, project_id, point_id 分段压缩（保持查询效率）
        ALTER TABLE measurement SET (
            timescaledb.compress,
            timescaledb.compress_segmentby = 'tenant_id, project_id, point_id'
        );
        RAISE NOTICE '[011] measurement: compression enabled (segmentby: tenant_id, project_id, point_id)';
    EXCEPTION 
        WHEN duplicate_object THEN
            RAISE NOTICE '[011] measurement: compression already enabled';
        WHEN undefined_table THEN
            RAISE NOTICE '[011] measurement: table not found, skipping';
            RETURN;
        WHEN OTHERS THEN
            RAISE NOTICE '[011] measurement: compression setup error: %', SQLERRM;
    END;

    -- 1.2 添加压缩策略：7天后自动压缩
    BEGIN
        PERFORM add_compression_policy('measurement', INTERVAL '7 days', if_not_exists => TRUE);
        RAISE NOTICE '[011] measurement: compression policy added (after 7 days)';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '[011] measurement: compression policy error: %', SQLERRM;
    END;

    -- 1.3 添加保留策略：365天后自动删除（含压缩数据）
    BEGIN
        PERFORM add_retention_policy('measurement', INTERVAL '365 days', if_not_exists => TRUE);
        RAISE NOTICE '[011] measurement: retention policy added (365 days)';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '[011] measurement: retention policy error: %', SQLERRM;
    END;

    -- ========================================================================
    -- 2. event 表：压缩与保留策略
    -- ========================================================================
    
    -- 2.1 启用压缩
    BEGIN
        ALTER TABLE event SET (
            timescaledb.compress,
            timescaledb.compress_segmentby = 'tenant_id, project_id'
        );
        RAISE NOTICE '[011] event: compression enabled (segmentby: tenant_id, project_id)';
    EXCEPTION 
        WHEN duplicate_object THEN
            RAISE NOTICE '[011] event: compression already enabled';
        WHEN undefined_table THEN
            RAISE NOTICE '[011] event: table not found, skipping';
        WHEN OTHERS THEN
            RAISE NOTICE '[011] event: compression setup error: %', SQLERRM;
    END;

    -- 2.2 添加压缩策略：7天后自动压缩
    BEGIN
        PERFORM add_compression_policy('event', INTERVAL '7 days', if_not_exists => TRUE);
        RAISE NOTICE '[011] event: compression policy added (after 7 days)';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '[011] event: compression policy error: %', SQLERRM;
    END;

    -- 2.3 添加保留策略：365天后自动删除
    BEGIN
        PERFORM add_retention_policy('event', INTERVAL '365 days', if_not_exists => TRUE);
        RAISE NOTICE '[011] event: retention policy added (365 days)';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '[011] event: retention policy error: %', SQLERRM;
    END;

    -- ========================================================================
    -- 3. 验证策略配置
    -- ========================================================================
    RAISE NOTICE '[011] Data governance policies configured successfully';
    RAISE NOTICE '[011]   - Compression: after 7 days';
    RAISE NOTICE '[011]   - Retention: 365 days (1 year)';

END $$;

-- ============================================================================
-- 策略查询（可选，用于验证）
-- ============================================================================
-- 
-- 查看压缩策略：
--   SELECT * FROM timescaledb_information.jobs 
--   WHERE proc_name = 'policy_compression';
--
-- 查看保留策略：
--   SELECT * FROM timescaledb_information.jobs 
--   WHERE proc_name = 'policy_retention';
--
-- 查看压缩状态：
--   SELECT * FROM timescaledb_information.compression_settings;
--
-- 手动执行压缩（测试用）：
--   SELECT compress_chunk(c) FROM show_chunks('measurement', older_than => INTERVAL '7 days') c;
-- ============================================================================
