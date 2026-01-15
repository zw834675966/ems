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
VALUES ('PROJECT.READ', 'Read projects'),
       ('PROJECT.WRITE', 'Write projects'),
       ('ASSET.GATEWAY.READ', 'Read gateways'),
       ('ASSET.GATEWAY.WRITE', 'Write gateways'),
       ('ASSET.DEVICE.READ', 'Read devices'),
       ('ASSET.DEVICE.WRITE', 'Write devices'),
       ('ASSET.POINT.READ', 'Read points'),
       ('ASSET.POINT.WRITE', 'Write points'),
       ('DATA.REALTIME.READ', 'Read realtime data'),
       ('DATA.MEASUREMENTS.READ', 'Read measurements'),
       ('CONTROL.COMMAND.ISSUE', 'Issue commands'),
       ('CONTROL.COMMAND.READ', 'Read commands'),
       ('ALARM.RULE.READ', 'Read alarm rules'),
       ('ALARM.RULE.WRITE', 'Write alarm rules'),
       ('ALARM.EVENT.READ', 'Read alarm events')
ON CONFLICT (permission_code) DO NOTHING;

INSERT INTO user_roles (user_id, role_code)
VALUES ('user-1', 'admin')
ON CONFLICT (user_id, role_code) DO NOTHING;

INSERT INTO role_permissions (role_code, permission_code)
VALUES ('admin', 'PROJECT.READ'),
       ('admin', 'PROJECT.WRITE'),
       ('admin', 'ASSET.GATEWAY.READ'),
       ('admin', 'ASSET.GATEWAY.WRITE'),
       ('admin', 'ASSET.DEVICE.READ'),
       ('admin', 'ASSET.DEVICE.WRITE'),
       ('admin', 'ASSET.POINT.READ'),
       ('admin', 'ASSET.POINT.WRITE'),
       ('admin', 'DATA.REALTIME.READ'),
       ('admin', 'DATA.MEASUREMENTS.READ'),
       ('admin', 'CONTROL.COMMAND.ISSUE'),
       ('admin', 'CONTROL.COMMAND.READ'),
       ('admin', 'ALARM.RULE.READ'),
       ('admin', 'ALARM.RULE.WRITE'),
       ('admin', 'ALARM.EVENT.READ')
ON CONFLICT (role_code, permission_code) DO NOTHING;

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
