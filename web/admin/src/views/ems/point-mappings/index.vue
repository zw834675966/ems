<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  listPointMappings,
  createPointMapping,
  updatePointMapping,
  deletePointMapping,
  type PointMappingDto
} from "@/api/ems/pointMappings";
import { listPoints, type PointDto } from "@/api/ems/points";
import { listDevices, type DeviceDto } from "@/api/ems/devices";
import { listGateways, type GatewayDto } from "@/api/ems/gateways";
import { getRealtime, type RealtimeValueDto } from "@/api/ems/realtime";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import { buildProtocolDetail, buildTcpProtocolDetail, parseProtocolDetail, parseTcpProtocolDetail } from "@/utils/protocol";

defineOptions({
  name: "EmsPointMappings"
});

const loading = ref(false);
const items = ref<PointMappingDto[]>([]);
const error = ref("");
const projectId = ref("");
const points = ref<PointDto[]>([]);
const devices = ref<DeviceDto[]>([]);
const gateways = ref<GatewayDto[]>([]);
const currentSourceId = ref<string>("");
const realtime = ref<RealtimeValueDto | null>(null);
const form = ref({
  gatewayId: "",
  pointId: "",
  sourceType: "mqtt",
  address: "",
  scale: null as number | null,
  offset: null as number | null,
  // Modbus TCP 专用字段
  modbusFunctionCode: 3,
  modbusRegisterAddress: 0,
  modbusRegisterCount: 1,
  modbusDataType: "int16",
  modbusEndian: "big_endian",
  modbusWordOrder: "",
  // TCP(TLV) 专用字段
  tcpTag: "",
  tcpValueType: "uint16",
  tcpEndian: "big_endian"
});

const gatewayOptions = computed(() =>
  gateways.value.map(item => ({
    label: `${item.name} (${item.protocolType})`,
    value: item.gatewayId
  }))
);

// 根据选中的网关过滤点位
const filteredPoints = computed(() => {
  if (!form.value.gatewayId) return points.value;
  
  // 找到该网关下的所有设备
  const gatewayDevices = devices.value.filter(
    d => d.gatewayId === form.value.gatewayId
  );
  const deviceIds = new Set(gatewayDevices.map(d => d.deviceId));
  
  // 过滤出这些设备下的点位
  return points.value.filter(p => deviceIds.has(p.deviceId));
});

const pointOptions = computed(() =>
  filteredPoints.value.map(item => {
    // 找到点位所属的设备
    const device = devices.value.find(d => d.deviceId === item.deviceId);
    const deviceName = device?.name || '未知设备';
    
    return {
      label: `${deviceName} - ${item.key}`,
      value: item.pointId
    };
  })
);

// 当前选中点位对应的设备和网关
const selectedPoint = computed(() =>
  points.value.find(p => p.pointId === form.value.pointId)
);

const selectedDevice = computed(() =>
  devices.value.find(d => d.deviceId === selectedPoint.value?.deviceId)
);

const selectedGateway = computed(() =>
  gateways.value.find(g => g.gatewayId === selectedDevice.value?.gatewayId)
);

const isModbusTcp = computed(() =>
  selectedGateway.value?.protocolType === "modbus_tcp"
);

const isTcp = computed(() =>
  selectedGateway.value?.protocolType === "tcp_server" ||
  selectedGateway.value?.protocolType === "tcp_client"
);

const effectiveSourceType = computed(() => {
  if (isModbusTcp.value) return "modbus";
  if (isTcp.value) return "tcp";
  return "mqtt";
});

const sourceTypeHint = computed(() => {
  if (isModbusTcp.value) return "当前点位所属网关为 Modbus TCP，因此 sourceType 固定为 modbus";
  if (isTcp.value) return "当前点位所属网关为 TCP，因此 sourceType 固定为 tcp";
  return "当前点位所属网关为 MQTT（或未选择），sourceType 默认为 mqtt";
});

// 刷新设备列表
const refreshDevices = async () => {
  const pid = projectId.value.trim();
  devices.value = [];
  if (!pid) return;
  try {
    const res = await listDevices(pid);
    if (!res.success) return;
    devices.value = res.data ?? [];
  } catch {}
};

// 刷新网关列表
const refreshGateways = async () => {
  const pid = projectId.value.trim();
  gateways.value = [];
  if (!pid) return;
  try {
    const res = await listGateways(pid);
    if (!res.success) return;
    gateways.value = res.data ?? [];
  } catch {}
};

const refreshPoints = async () => {
  const pid = projectId.value.trim();
  points.value = [];
  if (!pid) return;
  try {
    const res = await listPoints(pid);
    if (!res.success) return;
    points.value = res.data ?? [];
  } catch {}
};

const fetchList = async () => {
  error.value = "";
  const pid = projectId.value.trim();
  if (!pid) {
    error.value = "请选择项目";
    return;
  }
  loading.value = true;
  try {
    // 同时刷新点位、设备和网关列表，确保过滤逻辑正常工作
    await Promise.all([
      refreshPoints(),
      refreshDevices(),
      refreshGateways()
    ]);
    
    const res = await listPointMappings(pid);
    if (!res.success) {
      error.value = res.error?.message ?? "加载失败";
      return;
    }
    items.value = res.data ?? [];
  } catch (err) {
    error.value = "请求失败";
  } finally {
    loading.value = false;
  }
};

// 树形表格行数据类型
type TableRow = {
  id: string; // 唯一ID
  name: string; // 显示名称
  type: "gateway" | "device" | "point"; // 节点类型
  
  // 原始数据引用
  gateway?: GatewayDto;
  device?: DeviceDto;
  point?: PointDto;
  mapping?: PointMappingDto;
  
  // 树形结构
  children?: TableRow[];
  
  // 映射详情字段
  sourceId?: string;
  sourceType?: string;
  address?: string;
  scale?: number | null;
  offset?: number | null;
  protocolDetail?: string;
};

const treeData = ref<TableRow[]>([]);

const modbusRegisterCountMax = computed(() => {
  const fc = form.value.modbusFunctionCode;
  if (fc === 1 || fc === 2) return 2000;
  return 125;
});

// 构建工程树
const buildTree = () => {
  const tree: TableRow[] = [];
  
  // 1. 遍历网关
  gateways.value.forEach(gw => {
    // 查找网关下的设备
    const gwDevices = devices.value.filter(d => d.gatewayId === gw.gatewayId);
    
    const deviceNodes: TableRow[] = gwDevices.map(dev => {
      // 查找设备下的点位
      const devPoints = points.value.filter(p => p.deviceId === dev.deviceId);
      
      const pointNodes: TableRow[] = devPoints.map(pt => {
        // 查找点位的映射
        const mapping = items.value.find(m => m.pointId === pt.pointId);
        
        return {
          id: `point_${pt.pointId}`,
          name: pt.key,
          type: "point",
          point: pt,
          mapping: mapping,
          // 如果有映射，填充映射字段
          sourceId: mapping?.sourceId,
          sourceType: mapping?.sourceType,
          address: mapping?.address,
          scale: mapping?.scale,
          offset: mapping?.offset,
          protocolDetail: mapping?.protocolDetail
        };
      });
      
      return {
        id: `device_${dev.deviceId}`,
        name: dev.name,
        type: "device",
        device: dev,
        children: pointNodes,
        // 其他字段置空
      };
    });
    
    // 只有当网关下有设备或本来就是空网关时才显示
    tree.push({
      id: `gateway_${gw.gatewayId}`,
      name: `${gw.name} (${gw.protocolType})`,
      type: "gateway",
      gateway: gw,
      children: deviceNodes
    });
  });
  
  // 过滤树：如果选择了网关，只显示该网关
  if (form.value.gatewayId) {
    return tree.filter(node => node.gateway?.gatewayId === form.value.gatewayId);
  }
  
  return tree;
};

// 监听数据变化重建树
watch(
  [gateways, devices, points, items, () => form.value.gatewayId], 
  () => {
    treeData.value = buildTree();
  },
  { deep: true }
);

watch(
  () => form.value.modbusFunctionCode,
  (fc) => {
    if (fc === 1 || fc === 2) {
      form.value.modbusDataType = "bool";
      if (form.value.modbusRegisterCount > 2000) {
        form.value.modbusRegisterCount = 2000;
      }
      return;
    }

    if ((fc === 3 || fc === 4) && form.value.modbusDataType === "bool") {
      form.value.modbusDataType = "int16";
    }
    if (form.value.modbusRegisterCount > 125) {
      form.value.modbusRegisterCount = 125;
    }
  }
);

const submit = async () => {
  error.value = "";
  const pid = projectId.value.trim();
  if (!pid) {
    error.value = "请选择项目";
    return;
  }
  const pointId = form.value.pointId.trim();
  if (!pointId) {
    error.value = "请选择 point";
    return;
  }
  const sourceType = effectiveSourceType.value;
  if (!sourceType) {
    error.value = "请输入 sourceType";
    return;
  }

  let address = form.value.address.trim();

  const tcpDeviceDevId = (() => {
    try {
      const device = selectedDevice.value;
      if (!device?.addressConfig) return "";
      const cfg = JSON.parse(device.addressConfig);
      return cfg.devId ?? "";
    } catch {
      return "";
    }
  })();

  const modbusDeviceUnitId = (() => {
    try {
      const device = selectedDevice.value;
      if (!device?.addressConfig) return "";
      const cfg = JSON.parse(device.addressConfig);
      return cfg.unitId ?? cfg.slave_id ?? "";
    } catch {
      return "";
    }
  })();

  if (sourceType === "tcp" && !tcpDeviceDevId) {
    error.value = "TCP 点位需要先在设备中配置 addressConfig.devId（DEV_ID）";
    return;
  }
  if (sourceType === "modbus" && (modbusDeviceUnitId === "" || modbusDeviceUnitId === null || modbusDeviceUnitId === undefined)) {
    error.value = "Modbus 点位需要先在设备中配置 addressConfig.unitId（Unit ID）";
    return;
  }

  // address 对于 TCP/Modbus 主要用于展示与检索；这里做一个默认生成，避免空值被 API 拒绝
  if (!address) {
    if (sourceType === "tcp") {
      address = `dev/${tcpDeviceDevId}/tag/${form.value.tcpTag || "?"}`;
    } else if (sourceType === "modbus") {
      address = `unit/${modbusDeviceUnitId}/fc/${form.value.modbusFunctionCode}/addr/${form.value.modbusRegisterAddress}`;
    } else {
      error.value = "请输入 address";
      return;
    }
  }

  let protocolDetail: string | undefined;
  if (sourceType === "modbus") {
    if ((form.value.modbusFunctionCode === 1 || form.value.modbusFunctionCode === 2) && form.value.modbusDataType !== "bool") {
      error.value = "Modbus 功能码 01/02 的 dataType 必须为 bool";
      return;
    }
    if ((form.value.modbusFunctionCode === 3 || form.value.modbusFunctionCode === 4) && form.value.modbusDataType === "bool") {
      error.value = "Modbus 功能码 03/04 的 dataType 不能为 bool";
      return;
    }
    if (form.value.modbusRegisterCount > modbusRegisterCountMax.value) {
      error.value = `Modbus registerCount/quantity 不能超过 ${modbusRegisterCountMax.value}`;
      return;
    }
    if (["int32", "uint32", "float32"].includes(form.value.modbusDataType) && !form.value.modbusWordOrder) {
      error.value = "Modbus 32-bit 点位必须设置 wordOrder";
      return;
    }
    protocolDetail = buildProtocolDetail("modbus_tcp", form.value);
  } else if (sourceType === "tcp") {
    protocolDetail = buildTcpProtocolDetail(form.value);
    if (!protocolDetail) {
      error.value = "请填写 TCP tag";
      return;
    }
  }

  loading.value = true;
  try {
    const payload: {
      pointId: string;
      sourceType: string;
      address: string;
      scale?: number;
      offset?: number;
      protocolDetail?: string;
    } = {
      pointId,
      sourceType,
      address
    };
    if (form.value.scale !== null) {
      payload.scale = form.value.scale;
    }
    if (form.value.offset !== null) {
      payload.offset = form.value.offset;
    }
    payload.protocolDetail = protocolDetail;
    
    const res = currentSourceId.value
      ? await updatePointMapping(pid, currentSourceId.value, payload)
      : await createPointMapping(pid, payload);
    if (!res.success) {
      error.value = res.error?.message ?? (currentSourceId.value ? "更新失败" : "创建失败");
      return;
    }
    ElMessage.success(currentSourceId.value ? "更新成功" : "创建成功");
    form.value.address = address;
    await fetchList();
  } catch (err) {
    error.value = "请求失败";
  } finally {
    loading.value = false;
  }
};

const handleDeleteMapping = async () => {
  const pid = projectId.value.trim();
  if (!pid || !currentSourceId.value) return;
  await ElMessageBox.confirm("确认删除该点位映射？", "提示", { type: "warning" }).catch(() => false);
  try {
    const res = await deletePointMapping(pid, currentSourceId.value);
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "删除失败");
      return;
    }
    ElMessage.success("删除成功");
    currentSourceId.value = "";
    form.value.address = "";
    await fetchList();
  } catch {
    ElMessage.error("删除请求失败");
  }
};

const fetchRealtime = async () => {
  realtime.value = null;
  const pid = projectId.value.trim();
  const pointId = form.value.pointId.trim();
  if (!pid || !pointId) {
    ElMessage.warning("请先选择项目与点位");
    return;
  }
  try {
    const res = await getRealtime(pid, pointId);
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "查询实时数据失败");
      return;
    }
    const arr = res.data ?? [];
    realtime.value = arr.length > 0 ? arr[0] : null;
    if (!realtime.value) {
      ElMessage.info("暂无实时数据（可能尚未通信或未写入）");
    }
  } catch {
    ElMessage.error("查询实时数据异常");
  }
};

watch(
  () => projectId.value,
  async () => {
    form.value.pointId = "";
    currentSourceId.value = "";
    await refreshPoints();
    await refreshDevices();
    await refreshGateways();
  }
);

watch(
  () => form.value.pointId,
  () => {
    realtime.value = null;
    currentSourceId.value = "";
    const mapping = items.value.find(m => m.pointId === form.value.pointId);
    if (!mapping) return;
    currentSourceId.value = mapping.sourceId;
    form.value.sourceType = mapping.sourceType;
    form.value.address = mapping.address;
    form.value.scale = (mapping.scale ?? null) as any;
    form.value.offset = (mapping.offset ?? null) as any;
    if (mapping.protocolDetail) {
      if (effectiveSourceType.value === "modbus") {
        parseProtocolDetail(mapping.protocolDetail, form.value);
      } else if (effectiveSourceType.value === "tcp") {
        parseTcpProtocolDetail(mapping.protocolDetail, form.value);
      }
    }
  }
);
</script>

<template>
  <div class="ems-page">
    <el-card shadow="never" class="mb-4!">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div>
            <div class="text-base font-medium">点位映射</div>
            <div class="text-xs text-gray-500">Point ↔ Source 映射（MQTT / Modbus / TCP）</div>
          </div>
          <EmsProjectSelector v-model="projectId" @change="fetchList" />
        </div>
      </template>
      <div class="flex items-center gap-2">
        <el-button type="primary" :loading="loading" @click="fetchList">刷新列表</el-button>
        <span v-if="error" class="text-red-500">{{ error }}</span>
      </div>
      <el-table 
        class="mt-4" 
        :data="treeData" 
        border 
        row-key="id"
        default-expand-all
        :tree-props="{ children: 'children' }"
      >
        <el-table-column prop="name" label="名称 (网关/设备/点位)" min-width="300">
          <template #default="{ row }">
            <div class="flex items-center gap-2">
               <!-- 图标区分类型 -->
               <el-tag v-if="row.type === 'gateway'" type="success" size="small">GW</el-tag>
               <el-tag v-else-if="row.type === 'device'" type="warning" size="small">DEV</el-tag>
               <el-tag v-else-if="row.type === 'point'" size="small">PT</el-tag>
               <span :class="{'font-bold': row.type === 'gateway', 'font-medium': row.type === 'device'}">
                 {{ row.name }}
               </span>
               <span v-if="row.type === 'point'" class="text-gray-400 text-xs ml-2">
                 ({{ row.point?.dataType }})
               </span>
            </div>
          </template>
        </el-table-column>
        
        <el-table-column prop="sourceType" label="类型/协议" min-width="120">
          <template #default="{ row }">
            <span v-if="row.type === 'point'">
              {{ row.sourceType || '-' }}
            </span>
            <span v-else-if="row.type === 'gateway'">
              {{ row.gateway?.protocolType }}
            </span>
          </template>
        </el-table-column>
        
        <el-table-column prop="address" label="映射地址 / 配置" min-width="250">
           <template #default="{ row }">
             <div v-if="row.type === 'point'">
               <div v-if="row.address">
                 MQTT: {{ row.address }}
               </div>
               <div v-if="row.protocolDetail">
                 <code class="text-xs bg-gray-50 px-1 rounded block truncate max-w-[300px]" :title="row.protocolDetail">
                   {{ row.protocolDetail }}
                 </code>
               </div>
               <span v-if="!row.address && !row.protocolDetail" class="text-gray-300">未配置</span>
             </div>
           </template>
        </el-table-column>
        
        <el-table-column prop="sourceId" label="映射ID" min-width="150" show-overflow-tooltip />
        
        <el-table-column label="状态" width="100" align="center">
          <template #default="{ row }">
            <div v-if="row.type === 'point'">
              <el-tag v-if="row.mapping || row.protocolDetail" type="success" size="small">已配置</el-tag>
              <el-tag v-else type="info" size="small">未配置</el-tag>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-card shadow="never">
      <template #header>
        <div>创建点位映射</div>
      </template>
      <el-form :model="form" label-width="120px">
        <el-form-item label="选择网关">
          <el-select
            v-model="form.gatewayId"
            class="w-full"
            filterable
            clearable
            placeholder="选择网关（可选，用于过滤点位）"
          >
            <el-option
              v-for="opt in gatewayOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <div class="mt-1 text-xs text-gray-500">
            选择网关后，下方点位列表将只显示该网关下的点位
          </div>
        </el-form-item>
        <el-form-item label="pointId">
          <el-select
            v-model="form.pointId"
            class="w-full"
            filterable
            placeholder="选择点位"
          >
            <el-option
              v-for="opt in pointOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <div class="mt-1 text-xs text-gray-500">
            {{ form.gatewayId ? '已按网关过滤' : '若下拉为空，请先在"点位"页创建点位并刷新列表' }}
          </div>
        </el-form-item>
        <el-form-item label="sourceType">
          <el-input v-model="form.sourceType" :disabled="true" :placeholder="effectiveSourceType" />
          <div class="mt-1 text-xs text-gray-500">{{ sourceTypeHint }}</div>
        </el-form-item>
        <el-form-item label="address">
          <el-input v-model="form.address" placeholder="topic/xxx" />
          <div v-if="effectiveSourceType !== 'mqtt'" class="mt-1 text-xs text-gray-500">
            TCP/Modbus 场景 address 主要用于展示与检索；留空时会自动生成默认值。
          </div>
        </el-form-item>
        <el-form-item label="scale">
          <el-input-number v-model="form.scale" :step="0.1" />
        </el-form-item>
        <el-form-item label="offset">
          <el-input-number v-model="form.offset" :step="0.1" />
        </el-form-item>

        <template v-if="effectiveSourceType === 'modbus'">
          <el-divider content-position="left">Modbus 点位配置</el-divider>
          <el-form-item label="functionCode">
            <el-select v-model="form.modbusFunctionCode" class="w-full">
              <el-option label="03 读保持寄存器" :value="3" />
              <el-option label="04 读输入寄存器" :value="4" />
              <el-option label="01 读线圈" :value="1" />
              <el-option label="02 读离散输入" :value="2" />
            </el-select>
          </el-form-item>
          <el-form-item label="registerAddress (0-based)">
            <el-input-number v-model="form.modbusRegisterAddress" :min="0" :max="65535" class="w-full" />
          </el-form-item>
          <el-form-item label="registerCount / quantity">
            <el-input-number v-model="form.modbusRegisterCount" :min="1" :max="modbusRegisterCountMax" class="w-full" />
            <div class="mt-1 text-xs text-gray-500">
              01/02 最大 2000，03/04 最大 125（与后端校验一致）
            </div>
          </el-form-item>
          <el-form-item label="dataType">
            <el-select v-model="form.modbusDataType" class="w-full">
              <el-option label="bool" value="bool" />
              <el-option label="int16" value="int16" />
              <el-option label="uint16" value="uint16" />
              <el-option label="int32" value="int32" />
              <el-option label="uint32" value="uint32" />
              <el-option label="float32" value="float32" />
              <el-option label="float64" value="float64" />
            </el-select>
          </el-form-item>
          <el-form-item label="endian">
            <el-select v-model="form.modbusEndian" class="w-full">
              <el-option label="big_endian" value="big_endian" />
              <el-option label="little_endian" value="little_endian" />
            </el-select>
          </el-form-item>
          <el-form-item label="wordOrder (32-bit required)">
            <el-select v-model="form.modbusWordOrder" class="w-full" clearable placeholder="ABCD">
              <el-option label="ABCD" value="ABCD" />
              <el-option label="CDAB" value="CDAB" />
              <el-option label="BADC" value="BADC" />
              <el-option label="DCBA" value="DCBA" />
            </el-select>
          </el-form-item>
        </template>

        <template v-else-if="effectiveSourceType === 'tcp'">
          <el-divider content-position="left">TCP(TLV) 点位配置</el-divider>
          <el-form-item label="tag">
            <el-input v-model="form.tcpTag" placeholder="例如：1" />
          </el-form-item>
          <el-form-item label="valueType">
            <el-select v-model="form.tcpValueType" class="w-full">
              <el-option label="uint8" value="uint8" />
              <el-option label="uint16" value="uint16" />
              <el-option label="uint32" value="uint32" />
              <el-option label="int32" value="int32" />
            </el-select>
          </el-form-item>
          <el-form-item label="endian">
            <el-select v-model="form.tcpEndian" class="w-full">
              <el-option label="big_endian" value="big_endian" />
              <el-option label="little_endian" value="little_endian" />
            </el-select>
          </el-form-item>
        </template>

        <el-form-item label="通信返回数据">
          <div class="flex items-center gap-2">
            <el-button @click="fetchRealtime">查看实时值</el-button>
            <span v-if="realtime" class="text-xs text-gray-600">
              ts={{ realtime.tsMs }} value={{ realtime.value }} quality={{ realtime.quality || "-" }}
            </span>
            <span v-else class="text-xs text-gray-400">未查询</span>
          </div>
        </el-form-item>
        
        <el-form-item>
          <el-button type="primary" :loading="loading" @click="submit">
            {{ currentSourceId ? "更新" : "创建" }}
          </el-button>
          <el-button v-if="currentSourceId" type="danger" plain :loading="loading" @click="handleDeleteMapping">
            删除映射
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<style scoped>
.ems-page {
  padding: var(--space-6);
  max-width: 1600px;
  margin: 0 auto;
}
</style>
