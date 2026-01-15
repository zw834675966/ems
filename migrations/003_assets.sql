CREATE TABLE IF NOT EXISTS gateways (
    gateway_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'offline',
    last_seen_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gateways_tenant_project
    ON gateways (tenant_id, project_id);

CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    gateway_id TEXT NOT NULL REFERENCES gateways(gateway_id),
    name TEXT NOT NULL,
    model TEXT
);

CREATE INDEX IF NOT EXISTS idx_devices_tenant_project
    ON devices (tenant_id, project_id);

CREATE TABLE IF NOT EXISTS points (
    point_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    device_id TEXT NOT NULL REFERENCES devices(device_id),
    key TEXT NOT NULL,
    data_type TEXT NOT NULL,
    unit TEXT
);

CREATE INDEX IF NOT EXISTS idx_points_tenant_project
    ON points (tenant_id, project_id);

CREATE TABLE IF NOT EXISTS point_sources (
    source_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    point_id TEXT NOT NULL REFERENCES points(point_id),
    source_type TEXT NOT NULL,
    address TEXT NOT NULL,
    scale DOUBLE PRECISION,
    offset_value DOUBLE PRECISION
);

CREATE INDEX IF NOT EXISTS idx_point_sources_tenant_project
    ON point_sources (tenant_id, project_id);
