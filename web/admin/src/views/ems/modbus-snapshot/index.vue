<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from "vue";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";
import { listGateways, type GatewayDto } from "@/api/ems/gateways";
import { listDevices, type DeviceDto } from "@/api/ems/devices";
import { modbusSnapshot } from "@/api/ems/modbus";

defineOptions({
  name: "EmsModbusSnapshot"
});

type BlockKey = "coil_rw" | "discrete_ro" | "holding_rw" | "input_ro";

type BlockDef = {
  key: BlockKey;
  label: string;
  readFc: number;
  writeFc?: number;
  valueType: "bit" | "register";
  writable: boolean;
};

const BLOCKS: BlockDef[] = [
  {
    key: "coil_rw",
    label: "线圈状态(RW)",
    readFc: 1,
    writeFc: 5,
    valueType: "bit",
    writable: true
  },
  {
    key: "discrete_ro",
    label: "离散输入(RO)",
    readFc: 2,
    valueType: "bit",
    writable: false
  },
  {
    key: "holding_rw",
    label: "保持寄存器(RW)",
    readFc: 3,
    writeFc: 6,
    valueType: "register",
    writable: true
  },
  {
    key: "input_ro",
    label: "输入寄存器(RO)",
    readFc: 4,
    valueType: "register",
    writable: false
  }
];

type RowItem = {
  id: string;
  name: string;
  blockKey: BlockKey;
  address: number;
  quantity: number; // fixed 1 per rule
  bitOffset: number; // fixed 0 for bit types
  bitCount: number; // fixed 1 for bit types

  // value for UI
  valueBit: boolean;
  valueRegister: number;

  // status
  writing: boolean;
};

const projectId = ref("");
const gateways = ref<GatewayDto[]>([]);
const devices = ref<DeviceDto[]>([]);
const loading = ref(false);

const iconRefresh = useRenderIcon("ep:refresh");
const iconAdd = useRenderIcon("ri:add-circle-line");
const iconSnapshot = useRenderIcon("ep:camera");
const iconClear = useRenderIcon("ep:delete");

const form = reactive({
  gatewayId: "",
  deviceId: ""
});

const selectedGateway = computed(() =>
  gateways.value.find(g => g.gatewayId === form.gatewayId)
);
const selectedDevice = computed(() =>
  devices.value.find(d => d.deviceId === form.deviceId)
);

const isModbusGateway = computed(
  () => selectedGateway.value?.protocolType === "modbus_tcp"
);

const deviceUnitId = computed(() => {
  const cfgStr = selectedDevice.value?.addressConfig;
  if (!cfgStr) return null;
  try {
    const cfg = JSON.parse(cfgStr);
    const v = cfg?.unitId ?? cfg?.slave_id ?? null;
    const n = Number(v);
    if (!Number.isFinite(n)) return null;
    return n;
  } catch {
    return null;
  }
});

const gatewayOptions = computed(() =>
  gateways.value
    .filter(g => g.protocolType === "modbus_tcp")
    .map(g => ({ label: `${g.name} (${g.protocolType})`, value: g.gatewayId }))
);

const deviceOptions = computed(() => {
  if (!form.gatewayId) return [];
  return devices.value
    .filter(d => d.gatewayId === form.gatewayId)
    .map(d => ({ label: d.name, value: d.deviceId }));
});

const rows = ref<RowItem[]>([]);

const addDialogVisible = ref(false);
const addFormRef = ref<FormInstance>();
const addForm = reactive({
  count: 1,
  blockKey: "coil_rw" as BlockKey,
  startAddress: 0
});

const countMaxForBlock = (blockKey: BlockKey) => {
  const def = blockByKey(blockKey);
  // Best-practice defaults per field devices:
  // - bits: suggest <=512
  // - registers: <=125
  return def.valueType === "bit" ? 512 : 125;
};

const countMax = computed(() => countMaxForBlock(addForm.blockKey));

const addRules = reactive<FormRules>({
  count: [
    { required: true, message: "请输入数据条数", trigger: "blur" },
    {
      validator: (_rule, value, callback) => {
        const n = Number(value);
        const max = countMaxForBlock(addForm.blockKey);
        if (!Number.isFinite(n) || n < 1 || n > max) {
          callback(new Error(`数据条数范围为 1-${max}`));
          return;
        }
        callback();
      },
      trigger: "blur"
    }
  ],
  blockKey: [{ required: true, message: "请选择区块", trigger: "change" }],
  startAddress: [
    { required: true, message: "请输入起始数据地址", trigger: "blur" },
    {
      validator: (_rule, value, callback) => {
        const n = Number(value);
        if (!Number.isFinite(n) || n < 0 || n > 65535) {
          callback(new Error("地址范围为 0-65535"));
          return;
        }
        callback();
      },
      trigger: "blur"
    }
  ]
});

const resetAddForm = () => {
  addForm.count = 1;
  addForm.blockKey = "coil_rw";
  addForm.startAddress = 0;
  nextTick(() => addFormRef.value?.clearValidate());
};

const openAddDialog = () => {
  if (!projectId.value) {
    ElMessage.warning("请先选择项目");
    return;
  }
  if (!form.gatewayId || !form.deviceId) {
    ElMessage.warning("请先选择网关与设备");
    return;
  }
  if (!isModbusGateway.value) {
    ElMessage.warning("当前选择的网关不是 Modbus TCP");
    return;
  }
  if (!deviceUnitId.value) {
    ElMessage.warning("该设备未配置 addressConfig.unitId（Unit ID）");
    return;
  }
  resetAddForm();
  addDialogVisible.value = true;
};

const blockByKey = (key: BlockKey) => BLOCKS.find(b => b.key === key)!;

type ApiError = { code: string; message: string };
type ApiResponse<T> = { success: boolean; data?: T; error?: ApiError };

const extractApiResponse = <T,>(input: any): ApiResponse<T> | null => {
  if (!input || typeof input !== "object") return null;
  if (typeof input.success === "boolean") return input as ApiResponse<T>;
  return null;
};

const makeRow = (blockKey: BlockKey, address: number): RowItem => {
  const def = blockByKey(blockKey);
  const id = `${Date.now()}-${Math.random().toString(16).slice(2)}-${address}`;
  return {
    id,
    name: "--",
    blockKey,
    address,
    quantity: 1,
    bitOffset: def.valueType === "bit" ? 0 : 0,
    bitCount: def.valueType === "bit" ? 1 : 0,
    valueBit: false,
    valueRegister: 0,
    writing: false
  };
};

const confirmAdd = async () => {
  const ok = await addFormRef.value?.validate().catch(() => false);
  if (!ok) return;

  const count = Number(addForm.count);
  const start = Number(addForm.startAddress);
  const max = countMaxForBlock(addForm.blockKey);
  if (count < 1 || count > max) {
    ElMessage.error(`数据条数范围为 1-${max}`);
    return;
  }
  const end = start + count - 1;
  if (end > 65535) {
    ElMessage.error("地址越界：start + count - 1 超出 65535");
    return;
  }

  const toAdd: RowItem[] = [];
  for (let i = 0; i < count; i++) {
    toAdd.push(makeRow(addForm.blockKey, start + i));
  }
  rows.value = rows.value.concat(toAdd);
  addDialogVisible.value = false;
  ElMessage.success(`已新增 ${count} 条数据项`);
};

const clearRows = () => {
  rows.value = [];
};

const refreshMeta = async () => {
  const pid = projectId.value.trim();
  gateways.value = [];
  devices.value = [];
  if (!pid) return;
  try {
    const [gwRes, devRes] = await Promise.all([
      listGateways(pid),
      listDevices(pid)
    ]);
    if (gwRes.success) gateways.value = gwRes.data ?? [];
    if (devRes.success) devices.value = devRes.data ?? [];
  } catch {
    ElMessage.error("加载网关/设备失败");
  }
};

watch(
  () => projectId.value,
  async () => {
    form.gatewayId = "";
    form.deviceId = "";
    rows.value = [];
    await refreshMeta();
  }
);

watch(
  () => form.gatewayId,
  () => {
    form.deviceId = "";
  }
);

const fcLabel = (blockKey: BlockKey) => {
  const def = blockByKey(blockKey);
  const read = `FC${def.readFc.toString().padStart(2, "0")}`;
  if (def.writable && def.writeFc) {
    const write = `FC${def.writeFc.toString().padStart(2, "0")}`;
    return `${read}/${write}`;
  }
  return read;
};

const readSnapshot = async () => {
  const pid = projectId.value.trim();
  if (!pid) return;
  if (!form.gatewayId || !form.deviceId) {
    ElMessage.warning("请选择网关与设备");
    return;
  }
  if (!deviceUnitId.value) {
    ElMessage.error("该设备未配置 addressConfig.unitId（Unit ID）");
    return;
  }
  if (rows.value.length === 0) {
    ElMessage.warning("请先添加数据项");
    return;
  }

  loading.value = true;
  try {
    const reads = rows.value.map(r => ({
      functionCode: blockByKey(r.blockKey).readFc,
      address: r.address,
      quantity: r.quantity
    }));
    const res = await modbusSnapshot(pid, {
      gatewayId: form.gatewayId,
      deviceId: form.deviceId,
      unitId: deviceUnitId.value,
      reads,
      writes: [],
      lockTimeoutMs: 500
    });

    const api = extractApiResponse<any>(res);
    if (!api) {
      ElMessage.error("读取失败：响应格式异常");
      return;
    }
    if (api.error) {
      ElMessage.error(`${api.error.code}: ${api.error.message}`);
    }
    const data = api.data;
    if (!data) return;

    const map = new Map<string, any>();
    data.results.forEach(item => {
      map.set(`${item.functionCode}:${item.address}:${item.quantity}`, item);
    });

    // Update rows: each row is qty=1, so we use the first value.
    rows.value.forEach(r => {
      const fc = blockByKey(r.blockKey).readFc;
      const key = `${fc}:${r.address}:${r.quantity}`;
      const found = map.get(key);
      if (!found) return;
      if (found.error?.code) {
        // keep default/previous value on error
        return;
      }
      if (found.raw?.bits?.length) {
        r.valueBit = !!found.raw.bits[0];
      }
      if (found.raw?.registers?.length) {
        r.valueRegister = Number(found.raw.registers[0] ?? 0);
      }
    });

    if (api.success) {
      ElMessage.success(`读取成功（${data.latencyMs}ms）`);
    }
  } catch (e: any) {
    const api = extractApiResponse<any>(e);
    if (api?.error) {
      ElMessage.error(`${api.error.code}: ${api.error.message}`);
      const data = api.data;
      if (data?.results?.length) {
        const map = new Map<string, any>();
        data.results.forEach(item => {
          map.set(
            `${item.functionCode}:${item.address}:${item.quantity}`,
            item
          );
        });
        rows.value.forEach(r => {
          const fc = blockByKey(r.blockKey).readFc;
          const key = `${fc}:${r.address}:${r.quantity}`;
          const found = map.get(key);
          if (!found || found.error?.code) return;
          if (found.raw?.bits?.length) r.valueBit = !!found.raw.bits[0];
          if (found.raw?.registers?.length)
            r.valueRegister = Number(found.raw.registers[0] ?? 0);
        });
      }
    } else {
      ElMessage.error("读取异常");
    }
  } finally {
    loading.value = false;
  }
};

const writeRow = async (row: RowItem) => {
  const pid = projectId.value.trim();
  if (!pid) return;
  if (!form.gatewayId || !form.deviceId) {
    ElMessage.warning("请选择网关与设备");
    return;
  }
  if (!deviceUnitId.value) {
    ElMessage.error("该设备未配置 addressConfig.unitId（Unit ID）");
    return;
  }

  const def = blockByKey(row.blockKey);
  if (!def.writable || !def.writeFc) return;

  let value = 0;
  if (def.valueType === "bit") {
    value = row.valueBit ? 1 : 0;
  } else {
    const n = Number(row.valueRegister);
    if (!Number.isFinite(n) || n < 0 || n > 65535) {
      ElMessage.error("寄存器值范围为 0-65535");
      return;
    }
    value = n;
  }

  row.writing = true;
  try {
    const res = await modbusSnapshot(pid, {
      gatewayId: form.gatewayId,
      deviceId: form.deviceId,
      unitId: deviceUnitId.value,
      reads: [],
      writes: [
        {
          functionCode: def.writeFc,
          address: row.address,
          value
        }
      ],
      lockTimeoutMs: 500
    });

    const api = extractApiResponse<any>(res);
    if (!api) {
      ElMessage.error("写入失败：响应格式异常");
      return;
    }
    if (api.error) {
      // still continue; backend may return writeResults in data
      ElMessage.error(`${api.error.code}: ${api.error.message}`);
    }
    const data = api.data;
    if (!data) return;
    const wr = data.writeResults?.[0];
    if (!wr?.ok) {
      ElMessage.error(
        `${wr?.error?.code ?? "ERROR"}: ${wr?.error?.message ?? "写入失败"}`
      );
      return;
    }
    ElMessage.success("写入成功");
  } catch (e: any) {
    const api = extractApiResponse<any>(e);
    if (api?.error) {
      ElMessage.error(`${api.error.code}: ${api.error.message}`);
      const data = api.data;
      const wr = data?.writeResults?.[0];
      if (wr && !wr.ok) {
        ElMessage.error(
          `${wr?.error?.code ?? "ERROR"}: ${wr?.error?.message ?? "写入失败"}`
        );
      }
    } else {
      ElMessage.error("写入异常");
    }
  } finally {
    row.writing = false;
  }
};
</script>

<template>
  <div class="ems-page animate-fade-in-up">
    <!-- Toolbar -->
    <el-card shadow="never" class="mb-4">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-3">
          <div>
            <div class="text-base font-medium">Modbus 截图/读取</div>
            <div class="text-xs text-gray-400">
              按表格配置合并批读，支持行内写入（FC05/FC06）
            </div>
          </div>
          <div class="flex items-center gap-3 flex-wrap">
            <EmsProjectSelector v-model="projectId" />
            <el-button
              :icon="iconRefresh"
              circle
              :disabled="!projectId"
              @click="refreshMeta"
            />
          </div>
        </div>
      </template>

      <div class="flex items-end justify-between flex-wrap gap-3">
        <div class="flex items-end gap-3 flex-wrap">
          <div style="min-width: 240px">
            <div class="text-xs text-gray-500 mb-1">网关（Modbus TCP）</div>
            <el-select
              v-model="form.gatewayId"
              class="w-full"
              filterable
              placeholder="选择网关"
            >
              <el-option
                v-for="opt in gatewayOptions"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
          </div>

          <div style="min-width: 240px">
            <div class="text-xs text-gray-500 mb-1">设备</div>
            <el-select
              v-model="form.deviceId"
              class="w-full"
              filterable
              placeholder="选择设备"
            >
              <el-option
                v-for="opt in deviceOptions"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
          </div>

          <div style="min-width: 160px">
            <div class="text-xs text-gray-500 mb-1">Unit ID</div>
            <el-input
              :model-value="deviceUnitId ?? ''"
              disabled
              placeholder="来自 addressConfig.unitId"
            />
          </div>
        </div>

        <div class="flex items-center gap-2">
          <el-button type="primary" :icon="iconAdd" @click="openAddDialog">
            +
          </el-button>
          <el-button
            type="success"
            :icon="iconSnapshot"
            :loading="loading"
            @click="readSnapshot"
          >
            截图/读取
          </el-button>
          <el-button
            :icon="iconClear"
            :disabled="rows.length === 0"
            @click="clearRows"
          >
            清空
          </el-button>
        </div>
      </div>
    </el-card>

    <!-- Table -->
    <el-card shadow="never">
      <el-table
        v-loading="loading"
        :data="rows"
        row-key="id"
        style="width: 100%"
      >
        <el-table-column label="序号" width="80" align="center">
          <template #default="{ $index }">
            <span class="font-mono">{{ $index + 1 }}</span>
          </template>
        </el-table-column>

        <el-table-column label="名称" min-width="180">
          <template #default="{ row }">
            <el-input v-model="row.name" placeholder="--" />
          </template>
        </el-table-column>

        <el-table-column label="区块" width="160">
          <template #default="{ row }">
            {{ blockByKey(row.blockKey).label }}
          </template>
        </el-table-column>

        <el-table-column label="地址" width="110" align="right">
          <template #default="{ row }">
            <span class="font-mono">{{ row.address }}</span>
          </template>
        </el-table-column>

        <el-table-column label="数量" width="90" align="right">
          <template #default="{ row }">
            <span class="font-mono">{{ row.quantity }}</span>
          </template>
        </el-table-column>

        <el-table-column label="位偏移" width="100" align="right">
          <template #default="{ row }">
            <span class="font-mono">
              {{
                blockByKey(row.blockKey).valueType === "bit"
                  ? row.bitOffset
                  : "-"
              }}
            </span>
          </template>
        </el-table-column>

        <el-table-column label="位数" width="90" align="right">
          <template #default="{ row }">
            <span class="font-mono">
              {{
                blockByKey(row.blockKey).valueType === "bit"
                  ? row.bitCount
                  : "-"
              }}
            </span>
          </template>
        </el-table-column>

        <el-table-column label="指令" width="140" align="center">
          <template #default="{ row }">
            <span class="font-mono">{{ fcLabel(row.blockKey) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="数值" min-width="180">
          <template #default="{ row }">
            <template v-if="blockByKey(row.blockKey).valueType === 'bit'">
              <template v-if="blockByKey(row.blockKey).writable">
                <div class="flex items-center gap-2">
                  <el-switch v-model="row.valueBit" />
                  <span class="font-mono text-xs text-gray-500">{{
                    row.valueBit ? 1 : 0
                  }}</span>
                </div>
              </template>
              <template v-else>
                <span class="font-mono">{{ row.valueBit ? 1 : 0 }}</span>
              </template>
            </template>
            <template v-else>
              <template v-if="blockByKey(row.blockKey).writable">
                <el-input-number
                  v-model="row.valueRegister"
                  :min="0"
                  :max="65535"
                  class="w-full"
                />
              </template>
              <template v-else>
                <span class="font-mono">{{ row.valueRegister }}</span>
              </template>
            </template>
          </template>
        </el-table-column>

        <el-table-column label="写" width="120" align="center">
          <template #default="{ row }">
            <el-button
              v-if="blockByKey(row.blockKey).writable"
              type="primary"
              size="small"
              :loading="row.writing"
              :disabled="row.writing || loading"
              @click="writeRow(row)"
            >
              写
            </el-button>
            <span v-else class="text-gray-400">-</span>
          </template>
        </el-table-column>

        <template #empty>
          <el-empty description="暂无数据项，点击 + 添加" :image-size="120" />
        </template>
      </el-table>
    </el-card>

    <!-- Add dialog -->
    <el-dialog
      v-model="addDialogVisible"
      title="新增数据项配置"
      width="520px"
      append-to-body
      destroy-on-close
    >
      <el-form
        ref="addFormRef"
        :model="addForm"
        :rules="addRules"
        label-position="top"
        class="mt-2"
      >
        <el-form-item label="数据条数" prop="count">
          <el-input-number
            v-model="addForm.count"
            :min="1"
            :max="countMax"
            class="w-full"
          />
          <div class="mt-1 text-xs text-gray-500">
            范围 1-{{ countMax }}；将按地址递增生成行
          </div>
        </el-form-item>

        <el-form-item label="区块" prop="blockKey">
          <el-select v-model="addForm.blockKey" class="w-full">
            <el-option
              v-for="b in BLOCKS"
              :key="b.key"
              :label="b.label"
              :value="b.key"
            />
          </el-select>
        </el-form-item>

        <el-form-item label="起始数据地址（0-based）" prop="startAddress">
          <el-input-number
            v-model="addForm.startAddress"
            :min="0"
            :max="65535"
            class="w-full"
          />
        </el-form-item>
      </el-form>

      <template #footer>
        <div class="flex justify-end gap-3">
          <el-button @click="addDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="confirmAdd">确定</el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.ems-page {
  max-width: 1600px;
  padding: var(--space-6);
  margin: 0 auto;
}

.animate-fade-in-up {
  animation: fade-in-up 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
}

@keyframes fade-in-up {
  from {
    opacity: 0;
    transform: translateY(15px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
