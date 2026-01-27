INSERT INTO tenants (tenant_id, name)
VALUES ('tenant-1', 'Default Tenant')
ON CONFLICT (tenant_id) DO NOTHING;

INSERT INTO tenants (tenant_id, name)
VALUES ('tenant-2', 'Second Tenant')
ON CONFLICT (tenant_id) DO NOTHING;

INSERT INTO projects (project_id, tenant_id, name, timezone)
VALUES ('project-1', 'tenant-1', 'Default Project', 'UTC')
ON CONFLICT (project_id) DO NOTHING;

INSERT INTO projects (project_id, tenant_id, name, timezone)
VALUES ('project-2', 'tenant-2', 'Second Project', 'UTC')
ON CONFLICT (project_id) DO NOTHING;

-- NOTE: SQLx migrations cannot use psql variables like :'EMS_SEED_ADMIN_PASSWORD_HASH'.
-- Default password for seeded users is "admin123" (Argon2id hash embedded below).
INSERT INTO users (user_id, tenant_id, username, password_hash)
VALUES ('user-1', 'tenant-1', 'admin', '$argon2id$v=19$m=19456,t=2,p=1$DbDw3oOhVbslpjGMB85ubw$0Waoa6461p4fptzz/m6YhiOZpR8JKFBWsxYUmMaSyIE')
ON CONFLICT (user_id) DO NOTHING;

INSERT INTO users (user_id, tenant_id, username, password_hash)
VALUES ('user-2', 'tenant-2', 'admin2', '$argon2id$v=19$m=19456,t=2,p=1$DbDw3oOhVbslpjGMB85ubw$0Waoa6461p4fptzz/m6YhiOZpR8JKFBWsxYUmMaSyIE')
ON CONFLICT (user_id) DO NOTHING;

INSERT INTO roles (role_code, name)
VALUES ('admin', 'Administrator')
ON CONFLICT (role_code) DO NOTHING;

INSERT INTO permissions (permission_code, description)
VALUES ('PROJECT.READ', '读取项目'),
       ('PROJECT.WRITE', '编辑项目'),
       ('ASSET.GATEWAY.READ', '读取网关'),
       ('ASSET.GATEWAY.WRITE', '编辑网关'),
       ('ASSET.DEVICE.READ', '读取设备'),
       ('ASSET.DEVICE.WRITE', '编辑设备'),
       ('ASSET.POINT.READ', '读取点位'),
       ('ASSET.POINT.WRITE', '编辑点位'),
       ('DATA.REALTIME.READ', '读取实时数据'),
       ('DATA.MEASUREMENTS.READ', '读取历史数据'),
       ('CONTROL.COMMAND.ISSUE', '下发控制'),
       ('CONTROL.COMMAND.READ', '读取控制'),
       ('ALARM.RULE.READ', '读取告警规则'),
       ('ALARM.RULE.WRITE', '编辑告警规则'),
       ('ALARM.EVENT.READ', '读取告警事件'),
       ('RBAC.USER.READ', '读取用户'),
       ('RBAC.USER.WRITE', '编辑用户'),
       ('RBAC.ROLE.READ', '读取角色'),
       ('RBAC.ROLE.WRITE', '编辑角色'),
       ('SYSTEM.METRICS.READ', '读取系统指标')
ON CONFLICT (permission_code) DO NOTHING;

INSERT INTO user_roles (user_id, role_code)
VALUES ('user-1', 'admin')
ON CONFLICT (user_id, role_code) DO NOTHING;

INSERT INTO user_roles (user_id, role_code)
VALUES ('user-2', 'admin')
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
       ('admin', 'ALARM.EVENT.READ'),
       ('admin', 'RBAC.USER.READ'),
       ('admin', 'RBAC.USER.WRITE'),
       ('admin', 'RBAC.ROLE.READ'),
       ('admin', 'RBAC.ROLE.WRITE'),
       ('admin', 'SYSTEM.METRICS.READ')
ON CONFLICT (role_code, permission_code) DO NOTHING;