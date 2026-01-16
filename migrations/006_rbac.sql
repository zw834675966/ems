-- Tenant-scoped RBAC tables
--
-- Why: `roles/user_roles/role_permissions` are global in 001_init.sql.
-- For multi-tenant isolation, RBAC management should be tenant-scoped.

CREATE TABLE IF NOT EXISTS tenant_roles (
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    role_code TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (tenant_id, role_code)
);

CREATE TABLE IF NOT EXISTS tenant_user_roles (
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    role_code TEXT NOT NULL,
    PRIMARY KEY (tenant_id, user_id, role_code),
    FOREIGN KEY (tenant_id, role_code) REFERENCES tenant_roles(tenant_id, role_code) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tenant_role_permissions (
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    role_code TEXT NOT NULL,
    permission_code TEXT NOT NULL REFERENCES permissions(permission_code) ON DELETE CASCADE,
    PRIMARY KEY (tenant_id, role_code, permission_code),
    FOREIGN KEY (tenant_id, role_code) REFERENCES tenant_roles(tenant_id, role_code) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tenant_roles_tenant ON tenant_roles(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_user_roles_tenant_user ON tenant_user_roles(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_tenant_role_permissions_tenant_role ON tenant_role_permissions(tenant_id, role_code);
