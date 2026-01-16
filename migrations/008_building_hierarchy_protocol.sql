-- EMS 楼宇层级与网关协议扩展
-- 迁移版本：008
-- 描述：添加区域→楼宇→楼层→房间层级结构，扩展网关协议配置和设备地址配置

-- ============================================================================
-- 1. 区域表 (areas)
-- ============================================================================
-- 区域是最顶层的地理划分，如"东区"、"西区"、"园区A"等
CREATE TABLE IF NOT EXISTS areas (
    area_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_areas_tenant_project
    ON areas (tenant_id, project_id);

-- ============================================================================
-- 2. 楼宇表 (buildings)
-- ============================================================================
-- 楼宇隶属于区域，如"A栋"、"1号楼"等
CREATE TABLE IF NOT EXISTS buildings (
    building_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    area_id TEXT NOT NULL REFERENCES areas(area_id),
    name TEXT NOT NULL,
    address TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_buildings_tenant_project
    ON buildings (tenant_id, project_id);

CREATE INDEX IF NOT EXISTS idx_buildings_area
    ON buildings (area_id);

-- ============================================================================
-- 3. 楼层表 (floors)
-- ============================================================================
-- 楼层隶属于楼宇，支持负数表示地下层（如 B1=-1）
CREATE TABLE IF NOT EXISTS floors (
    floor_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    building_id TEXT NOT NULL REFERENCES buildings(building_id),
    floor_number INTEGER NOT NULL,
    floor_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_floors_tenant_project
    ON floors (tenant_id, project_id);

CREATE INDEX IF NOT EXISTS idx_floors_building
    ON floors (building_id);

-- ============================================================================
-- 4. 房间表 (rooms)
-- ============================================================================
-- 房间隶属于楼层，如"101室"、"会议室A"等
CREATE TABLE IF NOT EXISTS rooms (
    room_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(tenant_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    floor_id TEXT NOT NULL REFERENCES floors(floor_id),
    room_number TEXT NOT NULL,
    room_name TEXT,
    room_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_rooms_tenant_project
    ON rooms (tenant_id, project_id);

CREATE INDEX IF NOT EXISTS idx_rooms_floor
    ON rooms (floor_id);

-- ============================================================================
-- 5. 扩展网关表 (gateways)
-- ============================================================================
-- 添加协议类型和协议配置字段
-- protocol_type: mqtt | modbus_tcp | tcp_server | tcp_client
-- protocol_config: JSONB 存储协议特定参数

-- 检查列是否已存在，避免重复执行迁移时报错
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'gateways' AND column_name = 'protocol_type'
    ) THEN
        ALTER TABLE gateways ADD COLUMN protocol_type TEXT NOT NULL DEFAULT 'mqtt';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'gateways' AND column_name = 'protocol_config'
    ) THEN
        ALTER TABLE gateways ADD COLUMN protocol_config JSONB;
    END IF;
END$$;

-- ============================================================================
-- 6. 扩展设备表 (devices)
-- ============================================================================
-- 添加房间关联和协议地址配置
-- room_id: 设备所在房间（可选）
-- address_config: JSONB 存储协议特定地址

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'devices' AND column_name = 'room_id'
    ) THEN
        ALTER TABLE devices ADD COLUMN room_id TEXT REFERENCES rooms(room_id);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'devices' AND column_name = 'address_config'
    ) THEN
        ALTER TABLE devices ADD COLUMN address_config JSONB;
    END IF;
END$$;

-- ============================================================================
-- 7. 扩展点位映射表 (point_sources)
-- ============================================================================
-- 添加协议细节配置
-- protocol_detail: JSONB 存储协议特定的点位配置

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'point_sources' AND column_name = 'protocol_detail'
    ) THEN
        ALTER TABLE point_sources ADD COLUMN protocol_detail JSONB;
    END IF;
END$$;

-- ============================================================================
-- 协议配置示例（仅供参考，不执行）
-- ============================================================================
-- 
-- Modbus TCP 网关配置:
-- {"host":"192.168.1.100","port":502,"unit_id":1,"timeout_ms":5000}
--
-- TCP 服务器网关配置:
-- {"bind_host":"0.0.0.0","bind_port":9000,"max_connections":100}
--
-- TCP 客户端网关配置:
-- {"host":"192.168.1.100","port":9000,"reconnect_ms":5000}
--
-- MQTT 网关配置:
-- {"broker_host":"127.0.0.1","broker_port":1883,"client_id":"gw-001","qos":1,"topic_prefix":"ems/telemetry"}
--
-- Modbus 设备地址配置:
-- {"slave_address":1}
--
-- TCP 设备地址配置:
-- {"start_byte":6,"length":10,"device_id_offset":0}
--
-- MQTT 设备地址配置:
-- {"topic_filter":"device/+/data","id_field":"device_id"}
--
-- Modbus 点位配置:
-- {"function_code":3,"register_address":100,"register_count":1,"data_type":"int16"}
--
-- TCP 点位配置:
-- {"byte_offset":2,"byte_length":2,"data_type":"uint16","endian":"big"}
--
-- MQTT 点位配置:
-- {"json_path":"$.sensors.temperature","data_type":"float"}
