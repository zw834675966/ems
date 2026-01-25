<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { TabItem, ListItem } from "./types";
import NoticeList from "./components/NoticeList.vue";
import BellIcon from "~icons/ep/bell";
import {
  useSystemLogsApi,
  SystemLogDto,
  UnreadStatsDto
} from "@/api/system-logs";
import dayjs from "dayjs";

const route = useRoute();
const noticesNum = ref(0);
/*
  Tab Mapping:
  Key 1: 错误 (Error) - Backend: 'error'
  Key 2: 操作日志 (Operation) - Backend: 'operation'
  Key 3: 警告 (Warning) - Backend: 'warning'
*/
const notices = ref<TabItem[]>([
  {
    key: "1",
    name: "错误",
    list: [],
    emptyText: "暂无错误日志"
  },
  {
    key: "2",
    name: "操作日志",
    list: [],
    emptyText: "暂无操作日志"
  },
  {
    key: "3",
    name: "警告",
    list: [],
    emptyText: "暂无警告"
  }
]);
const activeKey = ref("1");

const projectId = computed(() => {
  return (
    (route.params.projectId as string) ||
    (route.query.projectId as string) ||
    ""
  );
});

const { list: listSystemLogs, getUnreadCount } = useSystemLogsApi(
  projectId.value
);

const mapLogToListItem = (log: SystemLogDto): ListItem => {
  return {
    avatar: "",
    title: log.title,
    description: log.message,
    datetime: dayjs(log.createdAtMs).format("YYYY-MM-DD HH:mm:ss"),
    type: log.category,
    status:
      log.level === "error"
        ? "danger"
        : log.level === "warn"
          ? "warning"
          : "info",
    extra: log.level.toUpperCase()
  };
};

const fetchData = async () => {
  if (!projectId.value) {
    notices.value.forEach(tab => (tab.list = []));
    noticesNum.value = 0;
    return;
  }

  try {
    // Parallel fetch: logs and unread stats
    const [rawLogs, unreadStats] = await Promise.all([
      listSystemLogs({ limit: 50 }),
      getUnreadCount()
    ]);

    // Process Logs
    const errorLogs: ListItem[] = [];
    const operationLogs: ListItem[] = [];
    const warningLogs: ListItem[] = [];

    const logs = Array.isArray(rawLogs) ? rawLogs : (rawLogs as any).data || [];

    logs.forEach((log: SystemLogDto) => {
      const item = mapLogToListItem(log);
      if (log.category === "error") {
        errorLogs.push(item);
      } else if (log.category === "operation") {
        operationLogs.push(item);
      } else if (log.category === "warning") {
        warningLogs.push(item);
      }
    });

    notices.value[0].list = errorLogs;
    notices.value[1].list = operationLogs;
    notices.value[2].list = warningLogs;

    // Process Unread Stats for Badge
    // http.request for unreadStats returns UnreadStatsDto directly (or inside data if not unwrapped, but let's assume unwrapped based on utils)
    // Actually getUnreadCount in api/system-logs.ts calls http.request<UnreadStatsDto>.
    // If it returns { success, data }, we need to handle that.
    // Based on my previous fix for list, it seems I should check format.
    const stats: UnreadStatsDto = (unreadStats as any).data || unreadStats;
    if (stats && typeof stats.total === "number") {
      noticesNum.value = stats.total;
    } else {
      // Fallback to sum of lists if stats fail or structure different
      noticesNum.value =
        errorLogs.length + operationLogs.length + warningLogs.length;
    }
  } catch (error) {
    console.error("Failed to fetch system logs:", error);
  }
};

onMounted(() => {
  fetchData();
});

watch(
  () => projectId.value,
  () => {
    fetchData();
  }
);

const getLabel = computed(
  () => item =>
    item.name + (item.list.length > 0 ? `(${item.list.length})` : "")
);
</script>

<template>
  <el-dropdown trigger="click" placement="bottom-end">
    <span
      :class="[
        'dropdown-badge',
        'navbar-bg-hover',
        'select-none',
        Number(noticesNum) !== 0 && 'mr-[10px]'
      ]"
    >
      <!-- Error badge prioritized if there are errors -->
      <el-badge
        :value="Number(noticesNum) === 0 ? '' : noticesNum"
        :max="99"
        :type="notices[0].list.length > 0 ? 'danger' : 'primary'"
      >
        <span class="header-notice-icon">
          <IconifyIconOffline :icon="BellIcon" />
        </span>
      </el-badge>
    </span>
    <template #dropdown>
      <el-dropdown-menu>
        <el-tabs
          v-model="activeKey"
          :stretch="true"
          class="dropdown-tabs"
          :style="{ width: notices.length === 0 ? '200px' : '330px' }"
        >
          <el-empty
            v-if="notices.length === 0"
            description="暂无消息"
            :image-size="60"
          />
          <span v-else>
            <template v-for="item in notices" :key="item.key">
              <el-tab-pane :label="getLabel(item)" :name="`${item.key}`">
                <el-scrollbar max-height="330px">
                  <div class="noticeList-container">
                    <NoticeList :list="item.list" :emptyText="item.emptyText" />
                  </div>
                </el-scrollbar>
              </el-tab-pane>
            </template>
          </span>
        </el-tabs>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<style lang="scss" scoped>
.dropdown-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 48px;
  cursor: pointer;

  .header-notice-icon {
    font-size: 18px;
  }
}

.dropdown-tabs {
  .noticeList-container {
    padding: 15px 24px 0;
  }

  :deep(.el-tabs__header) {
    margin: 0;
  }

  :deep(.el-tabs__nav-wrap)::after {
    height: 1px;
  }

  :deep(.el-tabs__nav-wrap) {
    padding: 0 36px;
  }
}
</style>
