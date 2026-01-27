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
import {
  formatMappingAddressLine,
  mappingAddressFormLabel,
  mappingAddressPlaceholder
} from "@/utils/mappingAddress";
import ModbusConfigForm from "./components/ModbusConfigForm.vue";
import TcpConfigForm from "./components/TcpConfigForm.vue";

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
const drawerVisible = ref(false);
const editorError = ref("");
const rwFilter = ref<"all" | "ro" | "rw">("all");
const gatewayFilterId = ref("");

// Unified Form structure
const form = ref({
  pointId: "",
  address: "",
  writable: false,
  scale: null as number | null,
  offset: null as number | null,
  protocolDetailJson: "" // Raw JSON string managed by sub-components
});

const gatewayOptions = computed(() =>
  gateways.value.map(item => ({
    label: `${item.name} (${item.protocolType})`,
    value: item.gatewayId
  }))
);

const selectedPoint = computed(() =>
  points.value.find(p => p.pointId === form.value.pointId)
);

const selectedDevice = computed(() =>
  devices.value.find(d => d.deviceId === selectedPoint.value?.deviceId)
);

const selectedGateway = computed(() =>
  gateways.value.find(g => g.gatewayId === selectedDevice.value?.gatewayId)
);

const isModbusTcp = computed(
  () => selectedGateway.value?.protocolType === "modbus_tcp"
);

const isTcp = computed(
  () =>
    selectedGateway.value?.protocolType === "tcp_server" ||
    selectedGateway.value?.protocolType === "tcp_client"
);

const effectiveSourceType = computed(() => {
  if (isModbusTcp.value) return "modbus";
  if (isTcp.value) return "tcp";
  return "mqtt";
});

const sourceTypeHint = computed(() => {
  if (isModbusTcp.value)
    return "当前点位所属网关为 Modbus TCP，因此 sourceType 固定为 modbus";
  if (isTcp.value) return "当前点位所属网关为 TCP，因此 sourceType 固定为 tcp";
  return "当前点位所属网关为 MQTT（或未选择），sourceType 默认为 mqtt";
});

const addressFormLabel = computed(() =>
  mappingAddressFormLabel(effectiveSourceType.value)
);

const addressPlaceholder = computed(() =>
  mappingAddressPlaceholder(effectiveSourceType.value)
);

const rwMode = computed({
  get: () => (form.value.writable ? "rw" : "ro"),
  set: (v: string) => {
    form.value.writable = v === "rw";
  }
});

const drawerTitle = computed(() => {
  const pt = selectedPoint.value;
  const dev = selectedDevice.value;
  const gw = selectedGateway.value;
  const name = pt ? pt.key : "未选择点位";
  const devName = dev?.name ? `${dev.name} / ` : "";
  const gwName = gw?.name ? `${gw.name} / ` : "";
  return `配置点位映射：${gwName}${devName}${name}`;
});

const resetEditorForm = () => {
  form.value.address = "";
  form.value.writable = false;
  form.value.scale = null as any;
  form.value.offset = null as any;
  form.value.protocolDetailJson = "";
};

const refreshAll = async () => {
  const pid = projectId.value.trim();
  if (!pid) return;
  try {
    const [pts, devs, gws] = await Promise.all([
        listPoints(pid),
        listDevices(pid),
        listGateways(pid)
    ]);
    if (pts.success) points.value = pts.data ?? [];
    if (devs.success) devices.value = devs.data ?? [];
    if (gws.success) gateways.value = gws.data ?? [];
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
    await refreshAll();
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

type TableRow = {
  id: string; 
  name: string; 
  type: "gateway" | "device" | "point"; 
  gateway?: GatewayDto;
  device?: DeviceDto;
  point?: PointDto;
  mapping?: PointMappingDto;
  children?: TableRow[];
  sourceId?: string;
  sourceType?: string;
  address?: string;
  writable?: boolean;
  scale?: number | null;
  offset?: number | null;
  protocolDetail?: string;
};

const treeData = ref<TableRow[]>([]);

const buildTree = () => {
  const tree: TableRow[] = [];
  gateways.value.forEach(gw => {
    const gwDevices = devices.value.filter(d => d.gatewayId === gw.gatewayId);
    const deviceNodes: TableRow[] = gwDevices.map(dev => {
      const devPoints = points.value.filter(p => p.deviceId === dev.deviceId);
      const pointNodes: TableRow[] = devPoints.map(pt => {
        const mapping = items.value.find(m => m.pointId === pt.pointId);
        return {
          id: `point_${pt.pointId}`,
          name: pt.key,
          type: "point",
          point: pt,
          mapping: mapping,
          sourceId: mapping?.sourceId,
          sourceType: mapping?.sourceType,
          address: mapping?.address,
          writable: mapping?.writable,
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
        children: pointNodes
      };
    });
    tree.push({
      id: `gateway_${gw.gatewayId}`,
      name: `${gw.name} (${gw.protocolType})`,
      type: "gateway",
      gateway: gw,
      children: deviceNodes
    });
  });

  const filteredByGateway = gatewayFilterId.value
    ? tree.filter(node => node.gateway?.gatewayId === gatewayFilterId.value)
    : tree;

  const filterByRw = (nodes: TableRow[]): TableRow[] => {
    return nodes.map(node => {
        if (!node.children || node.children.length === 0) return node;
        return { ...node, children: filterByRw(node.children) };
      }).filter(node => {
        if (node.type === "point") {
          if (rwFilter.value === "all") return true;
          if (!node.mapping) return false;
          return rwFilter.value === "rw" ? !!node.writable : !node.writable;
        }
        if (node.type === "gateway") return true;
        return (node.children?.length ?? 0) > 0;
      });
  };

  return filterByRw(filteredByGateway);
};

watch(
  [gateways, devices, points, items, gatewayFilterId, rwFilter],
  () => { treeData.value = buildTree(); },
  { deep: true }
);

const submit = async () => {
  editorError.value = "";
  const pid = projectId.value.trim();
  if (!pid) return;
  
  const pointId = form.value.pointId.trim();
  if (!pointId) {
    editorError.value = "请选择点位";
    return;
  }
  
  const sourceType = effectiveSourceType.value;
  let address = form.value.address.trim();

  // Validate Modbus / TCP prerequisites
  if (sourceType === 'tcp' || sourceType === 'modbus') {
      try {
        const dCfg = JSON.parse(selectedDevice.value?.addressConfig || "{}");
        if (sourceType === 'tcp' && !dCfg.devId) {
            editorError.value = "TCP设备未配置 devId"; 
            return;
        }
        if (sourceType === 'modbus' && (dCfg.unitId === undefined && dCfg.slave_id === undefined)) {
            editorError.value = "Modbus设备未配置 unitId"; 
            return;
        }
      } catch {
          editorError.value = "设备地址配置解析失败";
          return;
      }
  }

  // Auto-generate display address if empty
  if (!address && form.value.protocolDetailJson) {
      try {
          const detail = JSON.parse(form.value.protocolDetailJson);
          if (sourceType === "modbus") {
             // 格式: FC{code}:Addr{addr}
             address = `FC${detail.functionCode}:@${detail.registerAddress}`;
          } else if (sourceType === "tcp") {
             address = `Tag${detail.tag}`;
          }
      } catch {}
  }
  
  if (!address) {
      address = "auto_generated";
  }

  loading.value = true;
  try {
    const payload: any = {
      pointId,
      sourceType,
      address,
      writable: !!form.value.writable,
      scale: form.value.scale !== null ? form.value.scale : undefined,
      offset: form.value.offset !== null ? form.value.offset : undefined,
      protocolDetail: form.value.protocolDetailJson || undefined
    };

    const res = currentSourceId.value
      ? await updatePointMapping(pid, currentSourceId.value, payload)
      : await createPointMapping(pid, payload);
      
    if (!res.success) {
      editorError.value = res.error?.message ?? "保存失败";
    } else {
      ElMessage.success("保存成功");
      await fetchList();
      drawerVisible.value = false;
    }
  } catch (err) {
    editorError.value = "请求异常";
  } finally {
    loading.value = false;
  }
};

const handleDeleteMapping = async () => {
  if (!currentSourceId.value) return;
  await ElMessageBox.confirm("确认删除？", "提示", { type: "warning" });
  const res = await deletePointMapping(projectId.value, currentSourceId.value);
  if (res.success) {
      ElMessage.success("删除成功");
      currentSourceId.value = "";
      fetchList();
      drawerVisible.value = false;
  } else {
      ElMessage.error(res.error?.message || "删除失败");
  }
};

const fetchRealtime = async () => {
  realtime.value = null;
  if (!form.value.pointId) return;
  const res = await getRealtime(projectId.value, form.value.pointId);
  if (res.success && res.data?.length) {
      realtime.value = res.data[0];
  } else {
      ElMessage.info("暂无数据");
  }
};

watch(() => projectId.value, async () => {
    form.value.pointId = "";
    drawerVisible.value = false;
    refreshAll();
});

watch(() => form.value.pointId, () => {
    realtime.value = null;
    currentSourceId.value = "";
    resetEditorForm();
    const mapping = items.value.find(m => m.pointId === form.value.pointId);
    if (mapping) {
        currentSourceId.value = mapping.sourceId;
        form.value.address = mapping.address;
        form.value.writable = !!mapping.writable;
        form.value.scale = mapping.scale ?? null;
        form.value.offset = mapping.offset ?? null;
        form.value.protocolDetailJson = mapping.protocolDetail || "";
    }
});

const openEditor = (row: TableRow) => {
  if (row.type === "point" && row.point) {
      form.value.pointId = row.point.pointId;
      drawerVisible.value = true;
  }
};

const closeEditor = () => {
    drawerVisible.value = false;
};
</script>

<template>
  <div class="ems-page">
    <el-card shadow="never" class="mb-4!">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div>
            <div class="text-base font-medium">点位映射</div>
            <div class="text-xs text-gray-500">
              Point ↔ Source 映射工场（MQTT / Modbus / TCP）
            </div>
          </div>
          <EmsProjectSelector v-model="projectId" @change="fetchList" />
        </div>
      </template>
      <div class="flex items-center justify-between flex-wrap gap-2">
        <div class="flex items-center gap-2 flex-wrap">
          <el-button type="primary" :loading="loading" @click="fetchList">
            刷新列表
          </el-button>
          <el-select
            v-model="gatewayFilterId"
            class="w-[280px]"
            filterable
            clearable
            placeholder="按网关筛选"
          >
            <el-option
              v-for="opt in gatewayOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <el-radio-group v-model="rwFilter" size="small">
            <el-radio-button label="all">全选</el-radio-button>
            <el-radio-button label="ro">只读</el-radio-button>
            <el-radio-button label="rw">可写</el-radio-button>
          </el-radio-group>
        </div>
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
        <el-table-column prop="name" label="资源层级 (网关 > 设备 > 点位)" min-width="300">
          <template #default="{ row }">
            <div class="flex items-center gap-2">
              <el-tag v-if="row.type === 'gateway'" type="success" size="small" effect="dark">GW</el-tag>
              <el-tag v-else-if="row.type === 'device'" type="warning" size="small" effect="plain">DEV</el-tag>
              <el-tag v-else-if="row.type === 'point'" size="small" effect="light">PT</el-tag>
              
              <span :class="{'font-bold': row.type === 'gateway', 'font-medium': row.type==='device'}">
                  {{ row.name }}
              </span>
              <span v-if="row.type === 'point'" class="text-gray-400 text-xs font-mono ml-1">
                  [{{ row.point?.dataType }}]
              </span>
            </div>
          </template>
        </el-table-column>

        <el-table-column prop="sourceType" label="协议" width="100" align="center">
           <template #default="{ row }">
               <span v-if="row.type === 'point'" class="uppercase text-xs font-bold text-gray-600">
                   {{ row.sourceType || '-' }}
               </span>
               <span v-else-if="row.type === 'gateway'" class="uppercase text-xs text-primary">
                   {{ row.gateway?.protocolType }}
               </span>
           </template>
        </el-table-column>

        <el-table-column label="映射详情" min-width="200">
             <template #default="{ row }">
                 <div v-if="row.type === 'point'">
                     <div v-if="row.mapping">
                         <div class="text-xs font-mono text-gray-700 break-all">{{ row.protocolDetail }}</div>
                         <div v-if="row.address !== 'auto_generated'" class="text-xs text-gray-400 transform scale-90 origin-left">Addr: {{ row.address }}</div>
                     </div>
                     <span v-else class="text-gray-300 text-xs italic">未配置</span>
                 </div>
             </template>
        </el-table-column>

        <el-table-column label="权限" width="80" align="center">
             <template #default="{ row }">
                 <div v-if="row.type === 'point' && row.mapping">
                     <el-tag v-if="row.writable" type="warning" size="small" effect="dark">R/W</el-tag>
                     <el-tag v-else type="info" size="small" effect="plain">RO</el-tag>
                 </div>
             </template>
        </el-table-column>
        
        <el-table-column label="操作" width="100" align="center" fixed="right">
          <template #default="{ row }">
            <el-button v-if="row.type === 'point'" link type="primary" size="small" @click="openEditor(row)">
                {{ row.mapping ? '配置' : '添加' }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 配置面板 -->
    <el-drawer
      v-model="drawerVisible"
      :title="drawerTitle"
      size="500px"
      destroy-on-close
      @close="closeEditor"
    >
      <div v-if="selectedPoint" class="mb-4 p-3 bg-blue-50 rounded border border-blue-100 text-sm">
        <div class="flex gap-4 mb-1">
            <span class="text-gray-500">点位:</span>
            <span class="font-bold">{{ selectedPoint.key }}</span>
            <span class="text-gray-400 font-mono text-xs bg-gray-200 px-1 rounded">{{ selectedPoint.dataType }}</span>
        </div>
        <div class="flex gap-4">
             <span class="text-gray-500">所属:</span>
             <span>{{ selectedGateway?.name }} > {{ selectedDevice?.name }}</span>
        </div>
      </div>

      <el-alert v-if="editorError" :title="editorError" type="error" show-icon class="mb-4" :closable="false"/>

      <el-form :model="form" label-width="120px" label-position="top">
        
        <el-form-item label="Source Type (自动推断)">
            <el-input :model-value="effectiveSourceType" disabled>
                <template #append>{{ isModbusTcp ? 'Modbus' : (isTcp ? 'TCP' : 'MQTT') }}</template>
            </el-input>
        </el-form-item>

        <!-- 动态表单组件 -->
        <div v-if="effectiveSourceType === 'modbus'">
             <ModbusConfigForm 
                v-model="form.protocolDetailJson" 
                v-model:writable="form.writable" 
             />
        </div>

        <div v-else-if="effectiveSourceType === 'tcp'">
             <TcpConfigForm v-model="form.protocolDetailJson" />
        </div>
        
        <div v-else>
             <el-form-item :label="addressFormLabel" required>
                 <el-input v-model="form.address" :placeholder="addressPlaceholder" />
             </el-form-item>
             <el-form-item label="读写权限">
                  <el-switch v-model="form.writable" active-text="可写 (R/W)" inactive-text="只读 (RO)" />
             </el-form-item>
        </div>

        <el-divider content-position="left">数值修正 (可选)</el-divider>
        <div class="flex gap-4">
            <el-form-item label="缩放 (Scale)" class="flex-1">
                <el-input-number v-model="form.scale" :step="0.1" controls-position="right" class="w-full" />
            </el-form-item>
            <el-form-item label="偏移 (Offset)" class="flex-1">
                <el-input-number v-model="form.offset" :step="1" controls-position="right" class="w-full" />
            </el-form-item>
        </div>

      </el-form>

      <template #footer>
          <div class="flex justify-between items-center">
              <div class="flex items-center gap-2">
                  <el-button v-if="currentSourceId" type="danger" link @click="handleDeleteMapping">删除</el-button>
                  <el-button v-if="currentSourceId" link @click="fetchRealtime">读取当前值</el-button>
                  <span v-if="realtime" class="text-xs font-mono bg-black text-green-400 px-2 py-1 rounded">
                      {{ realtime.value }}
                  </span>
              </div>
              <div>
                  <el-button @click="closeEditor">取消</el-button>
                  <el-button type="primary" :loading="loading" @click="submit">保存配置</el-button>
              </div>
          </div>
      </template>
    </el-drawer>
  </div>
</template>
