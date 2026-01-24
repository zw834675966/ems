<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import { getRealtime, type RealtimeValueDto } from "@/api/ems/realtime";
import { listPoints, type PointDto } from "@/api/ems/points";
import { listDevices, type DeviceDto } from "@/api/ems/devices";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";
import dayjs from "dayjs";

defineOptions({
  name: "EmsRealtime"
});

const loading = ref(false);
const projectId = ref("");
const points = ref<PointDto[]>([]);
const devices = ref<DeviceDto[]>([]);
const realtimeValues = ref<Map<string, RealtimeValueDto>>(new Map());
const lastRefreshed = ref(0);
const autoRefresh = ref(true);
let timer: ReturnType<typeof setInterval> | null = null;

// ============================================================================
// 计算属性
// ============================================================================

const tableData = computed(() => {
  return points.value.map(point => {
    const val = realtimeValues.value.get(point.pointId);
    const device = devices.value.find(d => d.deviceId === point.deviceId);
    return {
      ...point,
      deviceName: device?.name ?? point.deviceId,
      value: val?.value,
      quality: val?.quality,
      tsMs: val?.tsMs,
      // 如果没有实时值，显示 -
      hasValue: !!val
    };
  });
});

// ============================================================================
// 数据加载
// ============================================================================

const loadMetadata = async () => {
  if (!projectId.value) {
    points.value = [];
    devices.value = [];
    return;
  }
  loading.value = true;
  try {
    const [pointsRes, devicesRes] = await Promise.all([
      listPoints(projectId.value),
      listDevices(projectId.value)
    ]);
    if (pointsRes.success) {
      points.value = pointsRes.data ?? [];
    }
    if (devicesRes.success) {
      devices.value = devicesRes.data ?? [];
    }
  } catch {
    ElMessage.error("加载元数据失败");
  } finally {
    loading.value = false;
  }
};

const refreshValues = async () => {
  if (!projectId.value) return;
  try {
     // 不传 pointId 获取项目下所有点位实时值
     // @ts-ignore: getRealtime 定义可能需要更新以支持可选 pointId
    const res = await getRealtime(projectId.value, ""); 
    if (res.success && res.data) {
      const map = new Map<string, RealtimeValueDto>();
      res.data.forEach(item => {
        map.set(item.pointId, item);
      });
      realtimeValues.value = map;
      lastRefreshed.value = Date.now();
    }
  } catch (e) {
    console.error("Auto refresh failed", e);
  }
};

const toggleAutoRefresh = (val: boolean) => {
  if (val) {
    refreshValues();
    timer = setInterval(refreshValues, 3000); // 3秒刷新一次
  } else {
    if (timer) clearInterval(timer);
    timer = null;
  }
};

// ============================================================================
// 生命周期
// ============================================================================

watch(() => projectId.value, async () => {
  realtimeValues.value.clear();
  await loadMetadata();
  refreshValues();
});

onMounted(() => {
  if (projectId.value) {
    loadMetadata().then(() => {
      if (autoRefresh.value) toggleAutoRefresh(true);
    });
  }
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <div class="ems-page animate-fade-in-up">
    <!-- Header -->
    <el-card shadow="never" class="mb-4">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div>
            <div class="text-base font-medium">实时数据监控</div>
            <div class="text-xs text-gray-400">实时查看所有点位的最新采集值与质量状态</div>
          </div>
          <EmsProjectSelector v-model="projectId" />
        </div>
      </template>
      
      <div class="flex items-center justify-between">
        <div class="text-sm text-gray-500">
          共监控 <span class="font-bold text-primary">{{ points.length }}</span> 个点位
          <span v-if="lastRefreshed" class="ml-4 text-xs">
            最后更新: {{ dayjs(lastRefreshed).format('HH:mm:ss') }}
          </span>
        </div>
        <div class="flex items-center gap-3">
           <span class="text-sm text-gray-600">自动刷新(3s)</span>
           <el-switch v-model="autoRefresh" @change="toggleAutoRefresh" />
           <el-divider direction="vertical" />
           <el-button :icon="useRenderIcon('ep:refresh')" circle @click="refreshValues" />
        </div>
      </div>
    </el-card>

    <!-- Data Table -->
    <el-card shadow="never">
      <el-table
        v-loading="loading"
        :data="tableData"
        style="width: 100%"
        row-key="pointId"
        height="calc(100vh - 280px)" 
      >
        <el-table-column label="点位名称" min-width="150" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="font-medium">{{ row.key }}</span>
            <div class="text-xs text-gray-400">{{ row.pointId }}</div>
          </template>
        </el-table-column>

        <el-table-column label="所属设备" min-width="150" show-overflow-tooltip>
          <template #default="{ row }">
             {{ row.deviceName }}
          </template>
        </el-table-column>

        <el-table-column label="实时值" min-width="140">
          <template #default="{ row }">
            <div v-if="row.hasValue" class="flex items-baseline gap-1">
              <span class="text-lg font-bold font-mono text-primary">{{ row.value }}</span>
              <span class="text-xs text-gray-500">{{ row.unit }}</span>
            </div>
            <span v-else class="text-gray-300">-</span>
          </template>
        </el-table-column>

        <el-table-column label="质量" width="100" align="center">
          <template #default="{ row }">
            <template v-if="row.hasValue">
               <el-tag v-if="row.quality === 'good' || !row.quality" type="success" size="small" effect="dark">Good</el-tag>
               <el-tag v-else type="warning" size="small" effect="dark">{{ row.quality }}</el-tag>
            </template>
            <span v-else class="text-gray-300">-</span>
          </template>
        </el-table-column>

        <el-table-column label="更新时间" width="180" align="right">
          <template #default="{ row }">
            <span v-if="row.tsMs" class="font-mono text-xs text-gray-500">
               {{ dayjs(row.tsMs).format('YYYY-MM-DD HH:mm:ss.SSS') }}
            </span>
            <span v-else class="text-gray-300">-</span>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<style scoped>
.ems-page {
  padding: var(--space-6);
  max-width: 1600px;
  margin: 0 auto;
  height: calc(100vh - 100px); /* 适应全屏高度 */
}

.animate-fade-in-up {
  animation: fade-in-up 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
}

@keyframes fade-in-up {
  from { opacity: 0; transform: translateY(15px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
