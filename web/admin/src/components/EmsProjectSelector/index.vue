<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import { listProjects, type ProjectDto } from "@/api/ems/projects";
import { clearEmsProjectId, loadEmsProjectId, saveEmsProjectId } from "@/utils/emsProject";

const props = defineProps<{
  modelValue: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "change", value: string): void;
}>();

const loading = ref(false);
const error = ref("");
const projects = ref<ProjectDto[]>([]);

const projectId = computed({
  get: () => props.modelValue,
  set: (value: string) => emit("update:modelValue", value)
});

const options = computed(() =>
  projects.value.map(item => ({
    label: `${item.name} (${item.projectId})`,
    value: item.projectId
  }))
);

const load = async () => {
  error.value = "";
  loading.value = true;
  try {
    const res = await listProjects();
    if (!res.success) {
      error.value = res.error?.message ?? "加载项目失败";
      projects.value = [];
      return;
    }
    projects.value = res.data ?? [];
  } catch (e) {
    error.value = "加载项目失败";
    projects.value = [];
  } finally {
    loading.value = false;
  }
};

const pickDefault = () => {
  const saved = loadEmsProjectId();
  if (saved && projects.value.some(p => p.projectId === saved)) {
    projectId.value = saved;
    return;
  }
  if (projectId.value && projects.value.some(p => p.projectId === projectId.value)) {
    saveEmsProjectId(projectId.value);
    return;
  }
  const first = projects.value[0]?.projectId ?? "";
  if (first) {
    projectId.value = first;
  }
};

const handleChange = (value: string) => {
  saveEmsProjectId(value);
  emit("change", value);
};

const copyProjectId = async () => {
  const value = projectId.value;
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    ElMessage.success("已复制 projectId");
  } catch (e) {
    ElMessage.info("复制失败，请手动复制");
  }
};

const clearSelection = () => {
  clearEmsProjectId();
  projectId.value = "";
  emit("change", "");
};

watch(
  () => projectId.value,
  value => {
    if (value) saveEmsProjectId(value);
  }
);

onMounted(async () => {
  await load();
  pickDefault();
});
</script>

<template>
  <div class="flex items-center gap-2 flex-wrap">
    <el-select
      v-model="projectId"
      class="min-w-[320px]!"
      placeholder="选择项目"
      filterable
      :disabled="disabled"
      :loading="loading"
      @change="handleChange"
    >
      <el-option
        v-for="opt in options"
        :key="opt.value"
        :label="opt.label"
        :value="opt.value"
      />
    </el-select>
    <el-button :loading="loading" @click="load">刷新项目</el-button>
    <el-button :disabled="!projectId" @click="copyProjectId">复制 projectId</el-button>
    <el-button :disabled="!projectId" type="danger" plain @click="clearSelection">
      清空
    </el-button>
    <span v-if="error" class="text-red-500 text-sm">{{ error }}</span>
  </div>
</template>

