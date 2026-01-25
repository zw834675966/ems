<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from "vue";
import {
  listMeasurements,
  type MeasurementValueDto
} from "@/api/ems/measurements";
import { listPoints, type PointDto } from "@/api/ems/points";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import * as echarts from "echarts";
import dayjs from "dayjs";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";

defineOptions({
  name: "EmsMeasurements"
});

const loading = ref(false);
const error = ref("");
const projectId = ref("");
const timeRange = ref<[Date, Date] | null>(null);
const pointId = ref("");
const limit = ref(100);

// 分页状态
const nextCursor = ref<number | undefined>(undefined);
const hasMore = ref(false);

const items = ref<MeasurementValueDto[]>([]);
const points = ref<PointDto[]>([]);

const chartRef = ref<HTMLElement | null>(null);
let chartInstance: echarts.ECharts | null = null;

const pointOptions = computed(() =>
  points.value.map(item => ({
    label: `${item.key} (${item.pointId})`,
    value: item.pointId,
    unit: item.unit
  }))
);

const currentPoint = computed(() =>
  points.value.find(p => p.pointId === pointId.value)
);

const shortcuts = [
  {
    text: "最近 1 小时",
    value: () => {
      const end = new Date();
      const start = new Date();
      start.setTime(start.getTime() - 3600 * 1000);
      return [start, end];
    }
  },
  {
    text: "最近 24 小时",
    value: () => {
      const end = new Date();
      const start = new Date();
      start.setTime(start.getTime() - 3600 * 1000 * 24);
      return [start, end];
    }
  },
  {
    text: "最近 7 天",
    value: () => {
      const end = new Date();
      const start = new Date();
      start.setTime(start.getTime() - 3600 * 1000 * 24 * 7);
      return [start, end];
    }
  }
];

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

const initChart = () => {
  if (chartRef.value) {
    chartInstance = echarts.init(chartRef.value);
    window.addEventListener("resize", handleResize);
  }
};

const handleResize = () => {
  chartInstance?.resize();
};

const updateChart = () => {
  if (!chartInstance) return;

  const data = items.value.slice().sort((a, b) => a.tsMs - b.tsMs);
  const xAxisData = data.map(item => dayjs(item.tsMs).format("MM-DD HH:mm:ss"));
  const seriesData = data.map(item => parseFloat(item.value));

  const option: echarts.EChartsOption = {
    tooltip: {
      trigger: "axis",
      backgroundColor: "rgba(255, 255, 255, 0.9)",
      borderWidth: 0,
      textStyle: { color: "#333" }
    },
    grid: {
      top: "10%",
      left: "3%",
      right: "4%",
      bottom: "3%",
      containLabel: true
    },
    xAxis: {
      type: "category",
      boundaryGap: false,
      data: xAxisData,
      axisLine: { lineStyle: { color: "#eee" } },
      axisLabel: { color: "#999", fontSize: 10 }
    },
    yAxis: {
      type: "value",
      axisLine: { show: false },
      splitLine: { lineStyle: { type: "dashed", color: "#f0f0f0" } },
      axisLabel: { color: "#999" }
    },
    series: [
      {
        name: currentPoint.value?.key || "数值",
        type: "line",
        smooth: true,
        showSymbol: false,
        data: seriesData,
        itemStyle: { color: "#d4a853" },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: "rgba(212, 168, 83, 0.2)" },
            { offset: 1, color: "rgba(212, 168, 83, 0)" }
          ])
        },
        lineStyle: { width: 3 }
      }
    ]
  };

  chartInstance.setOption(option);
};

const fetchList = async (isLoadMore = false) => {
  error.value = "";
  const pid = projectId.value.trim();
  const id = pointId.value.trim();
  if (!pid || !id) return;

  loading.value = true;
  try {
    const params: any = {
      pointId: id,
      limit: limit.value || 100,
      order: "desc"
    };

    if (timeRange.value) {
      params.from = timeRange.value[0].getTime();
      params.to = timeRange.value[1].getTime();
    }

    if (isLoadMore && nextCursor.value) {
      params.cursorTsMs = nextCursor.value;
    }

    const res = await listMeasurements(pid, params);
    if (!res.success) {
      error.value = res.error?.message ?? "查询失败";
      return;
    }

    const newItems = res.data ?? [];
    if (isLoadMore) {
      items.value = [...items.value, ...newItems];
    } else {
      items.value = newItems;
    }

    // 更新游标 (取最后一条的时间戳 - 1?? 或者直接取最后一条时间戳，取决于后端实现。
    // 通常 cursor pagination 如果是 desc，取最后一条 tsMs)
    if (newItems.length > 0) {
      nextCursor.value = newItems[newItems.length - 1].tsMs;
      // 简单判断: 如果返回数量等于 limit，说明可能还有下一页
      hasMore.value = newItems.length >= (limit.value || 100);
    } else {
      hasMore.value = false;
    }

    nextTick(() => {
      updateChart();
    });
  } catch (err) {
    error.value = "请求失败";
  } finally {
    loading.value = false;
  }
};

const handleSearch = () => {
  nextCursor.value = undefined;
  hasMore.value = false;
  fetchList(false);
};

const handleLoadMore = () => {
  fetchList(true);
};

watch(
  () => projectId.value,
  async () => {
    pointId.value = "";
    items.value = [];
    timeRange.value = null;
    await refreshPoints();
  }
);

watch(
  () => pointId.value,
  () => {
    if (pointId.value) handleSearch();
  }
);

onMounted(() => {
  initChart();
  if (projectId.value) refreshPoints();
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  chartInstance?.dispose();
});
</script>

<template>
  <div class="ems-page animate-fade-in-up">
    <el-card shadow="never" class="mb-4!">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div>
            <div class="text-base font-medium">历史数据查询</div>
            <div class="text-xs text-gray-400">
              查询点位的历史趋势与数据明细
            </div>
          </div>
          <EmsProjectSelector v-model="projectId" />
        </div>
      </template>

      <div class="flex gap-4 flex-wrap">
        <div class="flex-1 min-w-[240px]">
          <label class="text-xs text-gray-400 block mb-1">监控点位</label>
          <el-select
            v-model="pointId"
            class="w-full"
            filterable
            placeholder="选择点位"
            :loading="loading"
          >
            <el-option
              v-for="opt in pointOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </div>

        <div class="flex-1 min-w-[380px]">
          <label class="text-xs text-gray-400 block mb-1">时间范围</label>
          <el-date-picker
            v-model="timeRange"
            type="datetimerange"
            range-separator="至"
            start-placeholder="开始时间"
            end-placeholder="结束时间"
            :shortcuts="shortcuts"
            class="w-full!"
          />
        </div>

        <div class="flex items-end gap-2">
          <el-button type="primary" :loading="loading" @click="handleSearch">
            查询数据
          </el-button>
        </div>
      </div>
      <div v-if="error" class="mt-4">
        <el-alert type="error" :closable="false" show-icon>{{
          error
        }}</el-alert>
      </div>
    </el-card>

    <div v-if="items.length > 0" class="flex flex-col gap-6">
      <el-card shadow="never" class="chart-card">
        <template #header>
          <div class="flex justify-between items-center">
            <span
              class="text-xs font-bold uppercase tracking-wider text-gray-400"
              >趋势图表</span
            >
            <el-radio-group v-model="limit" size="small" @change="handleSearch">
              <el-radio-button :value="100">100条</el-radio-button>
              <el-radio-button :value="500">500条</el-radio-button>
              <el-radio-button :value="1000">1000条</el-radio-button>
            </el-radio-group>
          </div>
        </template>
        <div ref="chartRef" class="w-full h-[400px]" />
      </el-card>

      <el-card shadow="never">
        <template #header>
          <span class="text-xs font-bold uppercase tracking-wider text-gray-400"
            >数据明细 ({{ items.length }})</span
          >
        </template>
        <el-table
          v-loading="loading"
          :data="items"
          max-height="600"
          row-class-name="animate-fade-in-up"
        >
          <el-table-column prop="tsMs" label="采样时间" min-width="180">
            <template #default="{ row }">
              <span class="text-xs font-mono">
                {{ dayjs(row.tsMs).format("YYYY-MM-DD HH:mm:ss.SSS") }}
              </span>
            </template>
          </el-table-column>
          <el-table-column prop="value" label="采集值" min-width="120">
            <template #default="{ row }">
              <span class="font-bold text-amber-600">{{ row.value }}</span>
              <span class="ml-1 text-[10px] text-gray-400">{{
                currentPoint?.unit
              }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="quality" label="质量" width="100">
            <template #default="{ row }">
              <el-tag
                size="small"
                :type="
                  row.quality === 'good' || !row.quality ? 'success' : 'warning'
                "
                effect="plain"
              >
                {{ row.quality || "Good" }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>

        <!-- Load More Button -->
        <div
          v-if="hasMore"
          class="p-4 flex justify-center border-t border-gray-100"
        >
          <el-button :loading="loading" round @click="handleLoadMore"
            >加载更多历史数据</el-button
          >
        </div>
        <div v-else class="p-4 text-center text-xs text-gray-300">
          没有更多数据了
        </div>
      </el-card>
    </div>

    <div
      v-else-if="!loading"
      class="mt-20 flex flex-col items-center opacity-30"
    >
      <el-empty description="暂无历史数据，请调整查询条件" />
    </div>
  </div>
</template>

<style scoped>
.ems-page {
  max-width: 1600px;
  padding: var(--space-6);
  margin: 0 auto;
}

.chart-card {
  background: var(--color-white);
  border: 1px solid var(--color-cream-100);
  border-radius: 20px;

  :deep(.el-card__header) {
    border-bottom: 1px solid rgb(0 0 0 / 3%);
  }
}

:deep(.dark) {
  .chart-card {
    background: #1c1c1e;
    border-color: #2c2c2e;
  }
}
</style>
