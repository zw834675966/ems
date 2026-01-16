CREATE TABLE IF NOT EXISTS commands (
    command_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    target TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL,
    issued_by TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_commands_tenant_project_issued_at
    ON commands (tenant_id, project_id, issued_at DESC);

CREATE TABLE IF NOT EXISTS command_receipts (
    receipt_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    command_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    message TEXT
);

CREATE INDEX IF NOT EXISTS idx_command_receipts_tenant_project_ts
    ON command_receipts (tenant_id, project_id, ts DESC);

CREATE INDEX IF NOT EXISTS idx_command_receipts_command
    ON command_receipts (command_id);

CREATE TABLE IF NOT EXISTS audit_logs (
    audit_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    result TEXT NOT NULL,
    detail TEXT,
    ts TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_project_ts
    ON audit_logs (tenant_id, project_id, ts DESC);
