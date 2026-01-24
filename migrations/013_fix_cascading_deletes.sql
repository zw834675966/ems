-- Add ON DELETE CASCADE to foreign keys to prevent internal server errors during deletion of parent assets

-- 1. Gateways -> Projects
ALTER TABLE gateways DROP CONSTRAINT IF EXISTS gateways_project_id_fkey;
ALTER TABLE gateways ADD CONSTRAINT gateways_project_id_fkey 
    FOREIGN KEY (project_id) REFERENCES projects(project_id) ON DELETE CASCADE;

-- 2. Devices -> Gateways
ALTER TABLE devices DROP CONSTRAINT IF EXISTS devices_gateway_id_fkey;
ALTER TABLE devices ADD CONSTRAINT devices_gateway_id_fkey 
    FOREIGN KEY (gateway_id) REFERENCES gateways(gateway_id) ON DELETE CASCADE;

-- 3. Points -> Devices
ALTER TABLE points DROP CONSTRAINT IF EXISTS points_device_id_fkey;
ALTER TABLE points ADD CONSTRAINT points_device_id_fkey 
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE;

-- 4. Point Sources -> Points
ALTER TABLE point_sources DROP CONSTRAINT IF EXISTS point_sources_point_id_fkey;
ALTER TABLE point_sources ADD CONSTRAINT point_sources_point_id_fkey 
    FOREIGN KEY (point_id) REFERENCES points(point_id) ON DELETE CASCADE;

-- 5. Hierarchy Tables (008)
ALTER TABLE buildings DROP CONSTRAINT IF EXISTS buildings_area_id_fkey;
ALTER TABLE buildings ADD CONSTRAINT buildings_area_id_fkey
    FOREIGN KEY (area_id) REFERENCES areas(area_id) ON DELETE CASCADE;

ALTER TABLE floors DROP CONSTRAINT IF EXISTS floors_building_id_fkey;
ALTER TABLE floors ADD CONSTRAINT floors_building_id_fkey
    FOREIGN KEY (building_id) REFERENCES buildings(building_id) ON DELETE CASCADE;

ALTER TABLE rooms DROP CONSTRAINT IF EXISTS rooms_floor_id_fkey;
ALTER TABLE rooms ADD CONSTRAINT rooms_floor_id_fkey
    FOREIGN KEY (floor_id) REFERENCES floors(floor_id) ON DELETE CASCADE;
