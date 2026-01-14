INSERT INTO tenants (tenant_id, name)
VALUES ('tenant-1', 'Default Tenant')
ON CONFLICT (tenant_id) DO NOTHING;

INSERT INTO projects (project_id, tenant_id, name, timezone)
VALUES ('project-1', 'tenant-1', 'Default Project', 'UTC')
ON CONFLICT (project_id) DO NOTHING;

INSERT INTO users (user_id, tenant_id, username, password_hash)
VALUES ('user-1', 'tenant-1', 'admin', 'admin123')
ON CONFLICT (user_id) DO NOTHING;

INSERT INTO roles (role_code, name)
VALUES ('admin', 'Administrator')
ON CONFLICT (role_code) DO NOTHING;

INSERT INTO permissions (permission_code, description)
VALUES ('*:*:*', 'All permissions')
ON CONFLICT (permission_code) DO NOTHING;

INSERT INTO user_roles (user_id, role_code)
VALUES ('user-1', 'admin')
ON CONFLICT (user_id, role_code) DO NOTHING;

INSERT INTO role_permissions (role_code, permission_code)
VALUES ('admin', '*:*:*')
ON CONFLICT (role_code, permission_code) DO NOTHING;
