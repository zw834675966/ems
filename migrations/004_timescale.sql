CREATE TABLE IF NOT EXISTS measurement (
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    point_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    value TEXT NOT NULL,
    quality TEXT
);

CREATE INDEX IF NOT EXISTS idx_measurement_tenant_project_point_ts
    ON measurement (tenant_id, project_id, point_id, ts DESC);

CREATE TABLE IF NOT EXISTS event (
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    type TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_event_tenant_project_ts
    ON event (tenant_id, project_id, ts DESC);

DO $$
DECLARE
    has_timescaledb BOOLEAN := true;
BEGIN
    BEGIN
        CREATE EXTENSION IF NOT EXISTS timescaledb;
    EXCEPTION WHEN others THEN
        RAISE NOTICE 'timescaledb extension not available, skip hypertable';
        has_timescaledb := false;
    END;

    IF has_timescaledb THEN
        BEGIN
            -- When TimescaleDB is installed after the table already has data,
            -- enable migrate_data so re-running migrations can still convert it.
            PERFORM create_hypertable('measurement', 'ts', if_not_exists => TRUE, migrate_data => TRUE);
            PERFORM create_hypertable('event', 'ts', if_not_exists => TRUE, migrate_data => TRUE);
        EXCEPTION WHEN others THEN
            RAISE NOTICE 'timescaledb create_hypertable failed: %', SQLERRM;
        END;
    END IF;
END $$;
