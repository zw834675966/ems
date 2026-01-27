-- 为 point_sources 表添加 writable 列（点位映射读写标记）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'point_sources' AND column_name = 'writable'
    ) THEN
        ALTER TABLE point_sources ADD COLUMN writable BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END$$;

