-- ============================================================================
-- TimescaleDB 持续聚合 (Continuous Aggregates)
-- ============================================================================
-- 
-- 本迁移脚本为 measurement 表创建按小时聚合的物化视图，
-- 用于提升大时间跨度（如月度报表）的查询性能。
--
-- 聚合指标：
-- 1. 平均值 (Avg)
-- 2. 最小值 (Min)
-- 3. 最大值 (Max)
-- 4. 样本数 (Count)
--
-- 注意：对于非数字类型的数据，聚合函数可能会返回 NULL
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
        RAISE NOTICE '[012] timescaledb not installed, skipping continuous aggregates';
        RETURN;
    END IF;

    RAISE NOTICE '[012] TimescaleDB detected, configuring continuous aggregates...';

    -- ========================================================================
    -- 1. 创建按小时聚合的物化视图
    -- ========================================================================
    
    -- 检查视图是否已存在
    IF EXISTS (SELECT 1 FROM pg_matviews WHERE matviewname = 'measurement_hourly') THEN
        RAISE NOTICE '[012] measurement_hourly already exists, skipping creation';
    ELSE
        -- 创建持续聚合视图
        -- 我们尝试将 value 转为 double precision 进行数学运算
        -- 提示：TimescaleDB 不允许在持续聚合中使用复杂的 CASE/ERROR 处理，
        -- 所以这里假设参与聚合的主要是数值型点位。
        EXECUTE 'CREATE MATERIALIZED VIEW measurement_hourly
        WITH (timescaledb.continuous)
        AS
        SELECT
            time_bucket(''1 hour'', ts) AS bucket,
            tenant_id,
            project_id,
            point_id,
            AVG(CAST(value AS DOUBLE PRECISION)) AS avg_value,
            MIN(CAST(value AS DOUBLE PRECISION)) AS min_value,
            MAX(CAST(value AS DOUBLE PRECISION)) AS max_value,
            COUNT(*) AS sample_count
        FROM measurement
        GROUP BY bucket, tenant_id, project_id, point_id
        WITH NO DATA';
        
        RAISE NOTICE '[012] measurement_hourly continuous aggregate created';
    END IF;

    -- ========================================================================
    -- 2. 配置刷新策略
    -- ========================================================================
    
    -- 每天刷新最近 24 小时的数据，处理可能的补报数据
    -- start_offset: 刷新多久以前的数据
    -- end_offset: 刷新到多久以前（通常预留 1 小时以包含完整桶）
    -- schedule_interval: 检查刷新的频率
    BEGIN
        PERFORM add_continuous_aggregate_policy('measurement_hourly',
            start_offset      => INTERVAL '24 hours',
            end_offset        => INTERVAL '1 hour',
            schedule_interval => INTERVAL '1 hour',
            if_not_exists     => TRUE
        );
        RAISE NOTICE '[012] measurement_hourly: refresh policy added (24h lookback)';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '[012] measurement_hourly: policy error: %', SQLERRM;
    END;

    -- ========================================================================
    -- 3. （可选）为聚合视图创建索引
    -- ========================================================================
    -- 持续聚合视图会自动为 bucket 列创建索引，
    -- 但我们可以根据查询模式添加额外索引提升维度过滤效率。
    EXECUTE 'CREATE INDEX IF NOT EXISTS idx_meas_hourly_lookup 
             ON measurement_hourly (tenant_id, project_id, point_id, bucket DESC)';

    RAISE NOTICE '[012] Continuous aggregates configured successfully';

END $$;
