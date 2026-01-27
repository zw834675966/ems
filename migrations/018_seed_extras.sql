-- Extra seed data that depends on tables added after 001/002.
-- Why: SQLx runs migrations in version order. Keep `002_seed.sql` limited to 001_init.sql tables,
-- and move tenant-scoped RBAC + demo assets here (after 003_assets.sql + 006_rbac.sql).

-- Tenant-scoped RBAC
INSERT INTO tenant_roles (tenant_id, role_code, name)
VALUES ('tenant-1', 'admin', 'Administrator'),
       ('tenant-2', 'admin', 'Administrator')
ON CONFLICT (tenant_id, role_code) DO NOTHING;

INSERT INTO tenant_user_roles (tenant_id, user_id, role_code)
VALUES ('tenant-1', 'user-1', 'admin'),
       ('tenant-2', 'user-2', 'admin')
ON CONFLICT (tenant_id, user_id, role_code) DO NOTHING;

INSERT INTO tenant_role_permissions (tenant_id, role_code, permission_code)
SELECT tenant_id, 'admin', permission_code
FROM (VALUES ('tenant-1'), ('tenant-2')) t(tenant_id)
CROSS JOIN (
  VALUES
    ('PROJECT.READ'),
    ('PROJECT.WRITE'),
    ('ASSET.GATEWAY.READ'),
    ('ASSET.GATEWAY.WRITE'),
    ('ASSET.DEVICE.READ'),
    ('ASSET.DEVICE.WRITE'),
    ('ASSET.POINT.READ'),
    ('ASSET.POINT.WRITE'),
    ('DATA.REALTIME.READ'),
    ('DATA.MEASUREMENTS.READ'),
    ('CONTROL.COMMAND.ISSUE'),
    ('CONTROL.COMMAND.READ'),
    ('ALARM.RULE.READ'),
    ('ALARM.RULE.WRITE'),
    ('ALARM.EVENT.READ'),
    ('RBAC.USER.READ'),
    ('RBAC.USER.WRITE'),
    ('RBAC.ROLE.READ'),
    ('RBAC.ROLE.WRITE'),
    ('SYSTEM.METRICS.READ')
) p(permission_code)
ON CONFLICT (tenant_id, role_code, permission_code) DO NOTHING;

-- Demo assets (depends on 003_assets.sql)
INSERT INTO gateways (gateway_id, tenant_id, project_id, name, status)
VALUES ('gateway-1', 'tenant-1', 'project-1', 'Demo Gateway', 'online')
ON CONFLICT (gateway_id) DO NOTHING;

INSERT INTO devices (device_id, tenant_id, project_id, gateway_id, name, model)
VALUES ('device-1', 'tenant-1', 'project-1', 'gateway-1', 'Demo Device', 'M1')
ON CONFLICT (device_id) DO NOTHING;

INSERT INTO points (point_id, tenant_id, project_id, device_id, key, data_type, unit)
VALUES ('point-1', 'tenant-1', 'project-1', 'device-1', 'temp', 'float', 'C')
ON CONFLICT (point_id) DO NOTHING;

INSERT INTO point_sources (source_id, tenant_id, project_id, point_id, source_type, address, scale, offset_value)
VALUES ('source-1', 'tenant-1', 'project-1', 'point-1', 'mqtt', 'demo/topic', 1.0, 0.0)
ON CONFLICT (source_id) DO NOTHING;