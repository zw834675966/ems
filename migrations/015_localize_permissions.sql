-- Localize permission descriptions to Chinese

INSERT INTO permissions (permission_code, description)
VALUES 
    ('PROJECT.READ', '读取项目'),
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
ON CONFLICT (permission_code) 
DO UPDATE SET description = EXCLUDED.description;
