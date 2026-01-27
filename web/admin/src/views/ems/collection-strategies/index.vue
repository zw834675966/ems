<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  listStrategies,
  upsertStrategies,
  deleteStrategy,
  batchUpdateEnabled,
  testPoint,
  type CollectionStrategyDto,
  type UpsertStrategyRequest
} from "@/api/ems/collection-strategy";
import { listDevices, type DeviceDto } from "@/api/ems/devices";
import { listGateways, type GatewayDto } from "@/api/ems/gateways";
import { modbusSnapshot } from "@/api/ems/modbus";
import { listPointMappings, type PointMappingDto } from "@/api/ems/pointMappings";
import { listPoints, type PointDto } from "@/api/ems/points";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import dayjs from "dayjs";

defineOptions({
  name: "EmsCollectionStrategy"
});

// ============================================================================
// 状态定义
// ============================================================================

const loading = ref(false);
const saving = ref(false);
const projectId = ref("");
const points = ref<PointDto[]>([]);
const strategies = ref<CollectionStrategyDto[]>([]);
const devices = ref<DeviceDto[]>([]);
const gateways = ref<GatewayDto[]>([]);
const pointMappings = ref<PointMappingDto[]>([]);
const selectedPointIds = ref<string[]>([]);
const testingPoints = ref<Set<string>>(new Set());
const writingPoints = ref<Set<string>>(new Set());

const writeInputs = ref<Record<string, string>>({});

const refreshTimers = ref<number[]>([]);

const scheduleRefreshAfterTrigger = async () => {
  // 采集写入与状态更新是异步的（collector flush 约 1s），做一次延迟刷新。
  for (const t of refreshTimers.value) {
    window.clearTimeout(t);
  }
  refreshTimers.value = [];

  // 1) 快速刷新，覆盖大多数场景
  refreshTimers.value.push(
    window.setTimeout(() => {
      loadData();
    }, 1200)
  );
  // 2) 兜底刷新，覆盖首轮未落库/未更新策略状态的场景
  refreshTimers.value.push(
    window.setTimeout(() => {
      loadData();
    }, 3000)
  );
};

// 编辑表单
const editForm = ref({
  intervalValue: 1000,
  intervalUnit: "ms" as "ms" | "s" | "min",
  writeToDb: true,
  enabled: true
});

// ============================================================================
// 计算属性
// ============================================================================

/** 合并点位和策略数据，生成表格数据 */
const tableData = computed(() => {
  return points.value.map(point => {
    const strategy = strategies.value.find(s => s.pointId === point.pointId);
    return {
      ...point,
      strategy,
      hasStrategy: !!strategy,
      enabled: strategy?.enabled ?? false,
      intervalValue: strategy?.intervalValue ?? 1000,
      intervalUnit: strategy?.intervalUnit ?? "ms",
      writeToDb: strategy?.writeToDb ?? false,
      lastValue: strategy?.lastValue,
      lastError: strategy?.lastError,
      lastCollectedAtMs: strategy?.lastCollectedAtMs
    };
  });
});

const devicesById = computed(() => {
  const map = new Map<string, DeviceDto>();
  for (const d of devices.value) map.set(d.deviceId, d);
  return map;
});

const gatewaysById = computed(() => {
  const map = new Map<string, GatewayDto>();
  for (const g of gateways.value) map.set(g.gatewayId, g);
  return map;
});

type WritableMapping = {
  address: number;
  writeFc: number; // 5/6
  dataType?: string;
  registerCount?: number;
};

const safeJsonParse = (text?: string) => {
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
};

const findWritableModbusMapping = (pointId: string): WritableMapping | null => {
  const mapping = pointMappings.value.find(
    m => m.pointId === pointId && m.sourceType === "modbus" && m.writable
  );
  if (!mapping?.protocolDetail) return null;

  const detail = safeJsonParse(mapping.protocolDetail);
  const fc = Number(detail?.functionCode);
  const address = Number(detail?.registerAddress);
  const registerCount = Number(detail?.registerCount);
  const dataType = typeof detail?.dataType === "string" ? detail.dataType : undefined;

  if (!Number.isFinite(fc) || !Number.isFinite(address)) return null;

  const writeFc = fc === 1 || fc === 2 ? 5 : fc === 3 || fc === 4 ? 6 : 0;
  if (!writeFc) return null;

  return {
    address,
    writeFc,
    dataType,
    registerCount: Number.isFinite(registerCount) ? registerCount : undefined
  };
};

const resolveWriteContext = (row: any) => {
  const device = devicesById.value.get(row.deviceId);
  if (!device) return null;
  const gateway = gatewaysById.value.get(device.gatewayId);
  if (!gateway || gateway.protocolType !== "modbus_tcp") return null;
  const addressCfg = safeJsonParse(device.addressConfig);
  const unitId = Number(addressCfg?.unitId);
  if (!Number.isFinite(unitId) || unitId <= 0) return null;

  const mapping = findWritableModbusMapping(row.pointId);
  if (!mapping) return null;
  if (mapping.registerCount && mapping.registerCount !== 1) return null;

  return {
    gatewayId: gateway.gatewayId,
    deviceId: device.deviceId,
    unitId,
    mapping
  };
};

const coerceInputToU16 = (raw: string, dataType: string | undefined, writeFc: number) => {
  const text = (raw ?? "").trim();
  if (writeFc === 5) {
    // 线圈：空/0/false => 0，其余视为 true
    if (!text) return 0;
    const lowered = text.toLowerCase();
    if (lowered === "0" || lowered === "false" || lowered === "off" || lowered === "no") return 0;
    return 1;
  }

  // 寄存器：仅支持单寄存器写入（u16）
  const n = Number(text);
  if (!Number.isFinite(n)) throw new Error("请输入数值");

  const isIntegerType =
    typeof dataType === "string" &&
    (dataType.includes("int") || dataType.includes("uint") || dataType.includes("i") || dataType.includes("u"));

  const coerced = isIntegerType ? Math.trunc(n) : n;
  if (!Number.isFinite(coerced)) throw new Error("数值无效");
  if (coerced < 0 || coerced > 65535) throw new Error("寄存器值范围为 0-65535");
  return Math.trunc(coerced);
};

const formatLastValue = (raw?: string, dataType?: string) => {
  if (raw == null || `${raw}`.trim() === "") return "";
  const text = `${raw}`.trim();
  const dt = (dataType ?? "").toLowerCase();

  if (dt.includes("bool")) {
    const lowered = text.toLowerCase();
    if (lowered === "0" || lowered === "false") return "false";
    return "true";
  }

  if (dt.includes("int") || dt.includes("uint")) {
    const n = Number(text);
    if (!Number.isFinite(n)) return text;
    return String(Math.trunc(n));
  }

  return text;
};

/** 是否有选中点位 */
const hasSelection = computed(() => selectedPointIds.value.length > 0);

const handleSelectRow = (selection: any[]) => {
  selectedPointIds.value = selection.map(row => row.pointId);
};

// ============================================================================
// 数据加载
// ============================================================================

const loadData = async () => {
  if (!projectId.value) {
    points.value = [];
    strategies.value = [];
    devices.value = [];
    gateways.value = [];
    pointMappings.value = [];
    return;
  }

  loading.value = true;
  try {
    const [pointsRes, strategiesRes, devicesRes, gatewaysRes, mappingsRes] =
      await Promise.all([
        listPoints(projectId.value),
        listStrategies(projectId.value),
        listDevices(projectId.value),
        listGateways(projectId.value),
        listPointMappings(projectId.value)
      ]);

    if (pointsRes.success) {
      points.value = pointsRes.data ?? [];
    }
    if (strategiesRes.success) {
      strategies.value = strategiesRes.data ?? [];
    }
    if (devicesRes.success) {
      devices.value = devicesRes.data ?? [];
    }
    if (gatewaysRes.success) {
      gateways.value = gatewaysRes.data ?? [];
    }
    if (mappingsRes.success) {
      pointMappings.value = mappingsRes.data ?? [];
    }
  } catch (err) {
    ElMessage.error("加载数据失败");
  } finally {
    loading.value = false;
  }
};

const canWritePoint = (row: any) => {
  return !!resolveWriteContext(row);
};

const handleWrite = async (row: any) => {
  const pid = projectId.value.trim();
  if (!pid) return;

  const ctx = resolveWriteContext(row);
  if (!ctx) {
    ElMessage.warning("该点位未配置可写 Modbus 映射或设备 Unit ID");
    return;
  }

  const input = writeInputs.value[row.pointId] ?? "";
  let value: number;
  try {
    // 优先使用映射的 dataType（更贴近协议），否则退回点位 dataType
    const dt = ctx.mapping.dataType ?? row.dataType;
    value = coerceInputToU16(input, dt, ctx.mapping.writeFc);
  } catch (e: any) {
    ElMessage.error(e?.message ?? "写入值不合法");
    return;
  }

  writingPoints.value.add(row.pointId);
  try {
    const res = await modbusSnapshot(pid, {
      gatewayId: ctx.gatewayId,
      deviceId: ctx.deviceId,
      unitId: ctx.unitId,
      reads: [],
      writes: [
        {
          functionCode: ctx.mapping.writeFc,
          address: ctx.mapping.address,
          value
        }
      ],
      lockTimeoutMs: 500
    });
    if (!res.success) {
      ElMessage.error(res.error?.message || "写入失败");
      return;
    }
    const wr = res.data?.writeResults?.[0];
    if (!wr?.ok) {
      ElMessage.error(`${wr?.error?.code ?? "ERROR"}: ${wr?.error?.message ?? "写入失败"}`);
      return;
    }
    ElMessage.success("写入成功");
    scheduleRefreshAfterTrigger();
  } catch {
    ElMessage.error("写入异常");
  } finally {
    writingPoints.value.delete(row.pointId);
  }
};

// ============================================================================
// 测试连接
// ============================================================================

const handleTest = async (pointId: string) => {
  testingPoints.value.add(pointId);
  try {
    const res = await testPoint(projectId.value, pointId);
    if (res.success && res.data) {
      const result = res.data;
      if (result.success) {
        ElMessage.success(`测试成功: ${result.value} (${result.latencyMs}ms)`);
        scheduleRefreshAfterTrigger();
      } else {
        ElMessage.error(`测试失败: ${result.error}`);
      }
    } else {
      ElMessage.error("测试请求失败");
    }
  } catch {
    ElMessage.error("测试请求异常");
  } finally {
    testingPoints.value.delete(pointId);
  }
};

// ============================================================================
// 策略保存
// ============================================================================

const handleSaveSelected = async () => {
  if (!hasSelection.value) {
    ElMessage.warning("请先选择点位");
    return;
  }

  saving.value = true;
  try {
    const strategyRequests: UpsertStrategyRequest[] =
      selectedPointIds.value.map(pointId => ({
        pointId,
        enabled: editForm.value.enabled,
        intervalValue: editForm.value.intervalValue,
        intervalUnit: editForm.value.intervalUnit,
        writeToDb: editForm.value.writeToDb
      }));

    const res = await upsertStrategies(projectId.value, {
      strategies: strategyRequests
    });
    if (res.success) {
      ElMessage.success(`已保存 ${strategyRequests.length} 个策略`);
      await loadData();
      selectedPointIds.value = [];
    } else {
      ElMessage.error(res.error?.message || "保存失败");
    }
  } catch {
    ElMessage.error("保存失败");
  } finally {
    saving.value = false;
  }
};

// ============================================================================
// 删除策略
// ============================================================================

const handleDelete = async (strategyId: string) => {
  try {
    await ElMessageBox.confirm("确定删除该采集策略？", "提示", {
      type: "warning"
    });
    const res = await deleteStrategy(projectId.value, strategyId);
    if (res.success) {
      ElMessage.success("删除成功");
      await loadData();
    } else {
      ElMessage.error("删除失败");
    }
  } catch {}
};

// ============================================================================
// 批量启用/禁用
// ============================================================================

const handleBatchEnable = async (enabled: boolean) => {
  const strategyIds = strategies.value
    .filter(s => selectedPointIds.value.includes(s.pointId))
    .map(s => s.strategyId);

  if (strategyIds.length === 0) {
    ElMessage.warning("所选点位尚无策略配置");
    return;
  }

  try {
    const res = await batchUpdateEnabled(projectId.value, {
      strategyIds,
      enabled
    });
    if (res.success) {
      ElMessage.success(
        `已${enabled ? "启用" : "禁用"} ${res.data?.updated ?? 0} 个策略`
      );
      await loadData();
    }
  } catch {
    ElMessage.error("操作失败");
  }
};

// ============================================================================
// 生命周期
// ============================================================================

watch(
  () => projectId.value,
  () => {
    selectedPointIds.value = [];
    loadData();
  }
);

onMounted(() => {
  if (projectId.value) {
    loadData();
  }
});

onBeforeUnmount(() => {
  for (const t of refreshTimers.value) {
    window.clearTimeout(t);
  }
});
</script>

<template>
  <div class="ems-page animate-fade-in-up">
    <!-- Header -->
    <el-card shadow="never" class="mb-4">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div>
            <div class="text-base font-medium">采集策略配置</div>
            <div class="text-xs text-gray-400">
              批量配置点位采集频率与存储规则
            </div>
          </div>
          <EmsProjectSelector v-model="projectId" />
        </div>
      </template>

      <!-- Batch Config Form -->
      <div class="config-panel">
        <div class="flex items-center gap-4 flex-wrap">
          <div
            class="flex items-center gap-3 bg-white dark:bg-gray-800 p-1.5 rounded-md border border-gray-200 dark:border-gray-700"
          >
            <span class="text-sm text-gray-500 pl-2">采集间隔</span>
            <div class="flex items-center">
              <el-input-number
                v-model="editForm.intervalValue"
                :min="1"
                :max="86400000"
                controls-position="right"
                size="default"
                class="!w-32"
              />
              <el-select
                v-model="editForm.intervalUnit"
                size="default"
                class="!w-24 ml-[-1px]"
              >
                <el-option label="毫秒" value="ms" />
                <el-option label="秒" value="s" />
                <el-option label="分钟" value="min" />
              </el-select>
            </div>
          </div>

          <el-divider direction="vertical" />

          <div class="flex items-center gap-2">
            <span class="text-xs text-gray-500">写入数据库:</span>
            <el-switch v-model="editForm.writeToDb" size="small" />
          </div>

          <div class="flex items-center gap-2">
            <span class="text-xs text-gray-500">启用采集:</span>
            <el-switch v-model="editForm.enabled" size="small" />
          </div>

          <el-divider direction="vertical" />

          <el-button
            type="primary"
            size="small"
            :disabled="!hasSelection"
            :loading="saving"
            @click="handleSaveSelected"
          >
            保存配置 ({{ selectedPointIds.length }})
          </el-button>

          <el-button-group size="small">
            <el-button
              :disabled="!hasSelection"
              @click="handleBatchEnable(true)"
            >
              批量启用
            </el-button>
            <el-button
              :disabled="!hasSelection"
              @click="handleBatchEnable(false)"
            >
              批量禁用
            </el-button>
          </el-button-group>
        </div>
      </div>
    </el-card>

    <!-- Points Table -->
    <el-card shadow="never">
      <el-table
        v-loading="loading"
        :data="tableData"
        style="width: 100%"
        row-key="pointId"
        @selection-change="handleSelectRow"
      >
        <el-table-column type="selection" width="50" />

        <el-table-column label="点位" min-width="140" show-overflow-tooltip>
          <template #default="{ row }">
            <div class="group cursor-default">
              <div class="font-medium truncate">{{ row.key }}</div>
              <div
                class="text-xs text-gray-400 truncate opacity-0 group-hover:opacity-100 transition-opacity duration-200"
              >
                {{ row.pointId }}
              </div>
            </div>
          </template>
        </el-table-column>

        <el-table-column
          label="设备"
          min-width="140"
          prop="deviceId"
          show-overflow-tooltip
        >
          <template #default="{ row }">
            <code class="text-xs">{{ row.deviceId }}</code>
          </template>
        </el-table-column>

        <el-table-column label="数据类型" width="100" show-overflow-tooltip>
          <template #default="{ row }">
            <el-tag size="small" type="info">{{ row.dataType }}</el-tag>
          </template>
        </el-table-column>

        <el-table-column label="策略状态" width="100">
          <template #default="{ row }">
            <el-tag
              v-if="row.hasStrategy"
              :type="row.enabled ? 'success' : 'warning'"
              size="small"
            >
              {{ row.enabled ? "已启用" : "已禁用" }}
            </el-tag>
            <span v-else class="text-gray-300 text-xs">未配置</span>
          </template>
        </el-table-column>

        <el-table-column label="采集间隔" width="120">
          <template #default="{ row }">
            <span v-if="row.hasStrategy" class="text-sm">
              {{ row.intervalValue }}{{ row.intervalUnit }}
            </span>
            <span v-else class="text-gray-300">-</span>
          </template>
        </el-table-column>

        <el-table-column label="写入" width="240" align="center">
          <template #default="{ row }">
            <div class="flex items-center justify-center gap-2">
              <el-tag
                v-if="row.hasStrategy && row.writeToDb"
                size="small"
                type="success"
              >
                DB
              </el-tag>
              <template v-if="canWritePoint(row)">
                <el-input
                  v-model="writeInputs[row.pointId]"
                  size="small"
                  class="!w-24"
                  placeholder="写入值"
                  @keyup.enter="handleWrite(row)"
                />
                <el-button
                  type="primary"
                  size="small"
                  :loading="writingPoints.has(row.pointId)"
                  @click="handleWrite(row)"
                >
                  写入
                </el-button>
              </template>
              <span v-else class="text-gray-300">-</span>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="最新值" min-width="120">
          <template #default="{ row }">
            <span
              v-if="row.lastValue"
              class="font-mono text-sm"
              :title="row.lastValue"
            >
              {{ formatLastValue(row.lastValue, row.dataType) }}
            </span>
            <span v-else class="text-gray-300">-</span>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="160" fixed="right" align="center">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              :loading="testingPoints.has(row.pointId)"
              @click="handleTest(row.pointId)"
            >
              测试采集
            </el-button>
            <el-button
              v-if="row.hasStrategy"
              type="danger"
              link
              size="small"
              @click="handleDelete(row.strategy.strategyId)"
            >
              移除策略
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<style scoped>
.ems-page {
  max-width: 1600px;
  padding: var(--space-6);
  margin: 0 auto;
}

.config-panel {
  padding: 12px 16px;
  background: var(--color-gray-50);
  border-radius: 8px;
}

:deep(.dark) .config-panel {
  background: #2c2c2e;
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
