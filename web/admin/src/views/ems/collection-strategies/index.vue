<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
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
const selectedPointIds = ref<string[]>([]);
const testingPoints = ref<Set<string>>(new Set());

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
    return;
  }

  loading.value = true;
  try {
    const [pointsRes, strategiesRes] = await Promise.all([
      listPoints(projectId.value),
      listStrategies(projectId.value)
    ]);

    if (pointsRes.success) {
      points.value = pointsRes.data ?? [];
    }
    if (strategiesRes.success) {
      strategies.value = strategiesRes.data ?? [];
    }
  } catch (err) {
    ElMessage.error("加载数据失败");
  } finally {
    loading.value = false;
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

        <el-table-column label="写入DB" width="80" align="center">
          <template #default="{ row }">
            <el-icon
              v-if="row.hasStrategy && row.writeToDb"
              class="text-green-500"
            >
              <i class="ri-checkbox-circle-fill" />
            </el-icon>
            <span v-else class="text-gray-300">-</span>
          </template>
        </el-table-column>

        <el-table-column label="最新值" min-width="120">
          <template #default="{ row }">
            <span v-if="row.lastValue" class="font-mono text-sm">{{
              row.lastValue
            }}</span>
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
