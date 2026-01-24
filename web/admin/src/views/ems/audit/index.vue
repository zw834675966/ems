<script setup lang="ts">
import { ref } from "vue";
import { listAuditLogs, type AuditLogDto } from "@/api/ems/audit";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";

defineOptions({
  name: "EmsAudit"
});

const loading = ref(false);
const error = ref("");
const projectId = ref("");
const form = ref({
  limit: 100,
  from: "",
  to: ""
});
const items = ref<AuditLogDto[]>([]);

const fetchList = async () => {
  error.value = "";
  items.value = [];
  const pid = projectId.value.trim();
  if (!pid) {
    error.value = "请选择项目";
    return;
  }
  const from = form.value.from.trim();
  const to = form.value.to.trim();
  const limit = Number(form.value.limit) || 100;
  loading.value = true;
  try {
    const res = await listAuditLogs(pid, {
      limit,
      from: from ? Number(from) : undefined,
      to: to ? Number(to) : undefined
    });
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
</script>

<template>
  <div class="ems-page">
    <el-card shadow="never" class="mb-4!">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div>
            <div class="text-base font-medium">审计日志</div>
            <div class="text-xs text-gray-500">控制链路与关键操作审计</div>
          </div>
          <EmsProjectSelector v-model="projectId" />
        </div>
      </template>
      <el-form :model="form" label-width="120px">
        <el-form-item label="from(tsMs)">
          <el-input v-model="form.from" placeholder="可选，毫秒时间戳" />
        </el-form-item>
        <el-form-item label="to(tsMs)">
          <el-input v-model="form.to" placeholder="可选，毫秒时间戳" />
        </el-form-item>
        <el-form-item label="limit">
          <el-input v-model="form.limit" placeholder="默认 100" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="loading" @click="fetchList">
            加载
          </el-button>
          <span v-if="error" class="ml-2 text-red-500">{{ error }}</span>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="never">
      <template #header>
        <div>结果</div>
      </template>
      <el-table :data="items" border>
        <el-table-column prop="tsMs" label="tsMs" min-width="160" />
        <el-table-column prop="actor" label="actor" min-width="120" />
        <el-table-column prop="action" label="action" min-width="200" />
        <el-table-column prop="resource" label="resource" min-width="200" />
        <el-table-column prop="result" label="result" min-width="120" />
        <el-table-column prop="detail" label="detail" min-width="220" />
      </el-table>
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
