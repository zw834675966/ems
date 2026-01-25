<script setup lang="ts">
import {
  listProjects,
  createProject,
  updateProject,
  deleteProject,
  type ProjectDto
} from "@/api/ems/projects";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";
import { useCrud } from "@/components/ReCrud/useCrud";
import { ref, reactive, nextTick } from "vue";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import { TIMEZONE_OPTIONS } from "@/config/constants";

defineOptions({
  name: "EmsProjects"
});

const {
  loading,
  error,
  filteredData,
  searchValue,
  pagination,
  fetchList,
  handleDelete,
  onPageSizeChange,
  onCurrentPageChange
} = useCrud(listProjects, deleteProject, { idKey: "projectId" });

const dialogVisible = ref(false);
const isEdit = ref(false);
const currentId = ref("");
const formRef = ref<FormInstance>();
const form = reactive({
  name: "",
  timezone: "Asia/Shanghai"
});

// 表单校验规则
const rules = reactive<FormRules>({
  name: [
    { required: true, message: "请输入项目名称", trigger: "blur" },
    {
      min: 2,
      max: 50,
      message: "项目名称长度应为 2-50 个字符",
      trigger: "blur"
    }
  ]
});

const resetForm = () => {
  form.name = "";
  form.timezone = "Asia/Shanghai";
  nextTick(() => {
    formRef.value?.clearValidate();
  });
};

const handleAdd = () => {
  isEdit.value = false;
  currentId.value = "";
  resetForm();
  dialogVisible.value = true;
};

const handleEdit = (row: ProjectDto) => {
  isEdit.value = true;
  currentId.value = row.projectId;
  form.name = row.name;
  form.timezone = row.timezone || "Asia/Shanghai";
  dialogVisible.value = true;
  nextTick(() => {
    formRef.value?.clearValidate();
  });
};

const submit = async () => {
  if (!formRef.value) return;

  // 使用 Element Plus 表单校验
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  loading.value = true;
  try {
    const name = form.name.trim();
    const timezone = form.timezone.trim();
    let res;
    if (isEdit.value) {
      res = await updateProject(currentId.value, {
        name,
        timezone: timezone ? timezone : undefined
      });
    } else {
      res = await createProject({
        name,
        timezone: timezone ? timezone : undefined
      });
    }

    if (!res.success) {
      ElMessage.error(
        res.error?.message ?? (isEdit.value ? "更新失败" : "创建失败")
      );
      return;
    }
    ElMessage.success(isEdit.value ? "更新成功" : "创建成功");
    dialogVisible.value = false;
    await fetchList();
  } catch (err) {
    ElMessage.error("请求失败");
  } finally {
    loading.value = false;
  }
};
</script>

<template>
  <div class="apple-container animate-fade-in-up">
    <div class="flex-b mb-8">
      <div>
        <h1>项目管理</h1>
        <p class="text-secondary">管理您的能源监控项目资源</p>
      </div>
      <div class="flex gap-4">
        <el-input
          v-model="searchValue"
          placeholder="搜索项目..."
          class="w-[240px]"
          clearable
          :prefix-icon="useRenderIcon('ep:search')"
        />
        <el-button
          :icon="useRenderIcon('ep:refresh')"
          :loading="loading"
          @click="fetchList"
        >
          刷新
        </el-button>
        <el-button
          type="primary"
          :icon="useRenderIcon('ri:add-circle-line')"
          @click="handleAdd"
        >
          添加项目
        </el-button>
      </div>
    </div>

    <el-card class="apple-shadow" shadow="never">
      <div v-if="error" class="mb-4">
        <el-alert type="error" :closable="false" show-icon>
          {{ error }}
        </el-alert>
      </div>

      <el-skeleton
        :rows="5"
        animated
        :loading="loading && filteredData.length === 0"
      >
        <template #default>
          <el-table
            v-loading="loading"
            :data="filteredData"
            row-class-name="animate-fade-in-up"
          >
            <el-table-column
              prop="name"
              label="项目名称"
              min-width="180"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                <span class="font-bold text-primary">{{ row.name }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="projectId" label="项目 ID" min-width="200">
              <template #default="{ row }">
                <code class="text-xs opacity-60">{{ row.projectId }}</code>
              </template>
            </el-table-column>
            <el-table-column prop="timezone" label="时区" width="180">
              <template #default="{ row }">
                {{
                  TIMEZONE_OPTIONS.find(t => t.value === row.timezone)?.label ||
                  row.timezone ||
                  "UTC"
                }}
              </template>
            </el-table-column>
            <el-table-column label="状态" width="120" align="center">
              <template #default>
                <el-tag type="success">正常</el-tag>
              </template>
            </el-table-column>
            <el-table-column
              label="操作"
              width="140"
              fixed="right"
              align="center"
            >
              <template #default="{ row }">
                <el-button
                  link
                  type="primary"
                  :icon="useRenderIcon('ep:edit-pen')"
                  @click="handleEdit(row)"
                />
                <el-button
                  link
                  type="danger"
                  :icon="useRenderIcon('ep:delete')"
                  @click="handleDelete(row)"
                />
              </template>
            </el-table-column>

            <template #empty>
              <el-empty description="暂无项目数据" :image-size="120">
                <el-button type="primary" @click="handleAdd"
                  >创建第一个项目</el-button
                >
              </el-empty>
            </template>
          </el-table>
        </template>
      </el-skeleton>

      <div class="mt-8 flex justify-end">
        <el-pagination
          v-model:current-page="pagination.currentPage"
          v-model:page-size="pagination.pageSize"
          :total="pagination.total"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="onPageSizeChange"
          @current-change="onCurrentPageChange"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑项目' : '添加项目'"
      width="520px"
      append-to-body
      destroy-on-close
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-position="top"
        class="mt-4"
      >
        <el-form-item label="项目名称" prop="name">
          <el-input
            v-model="form.name"
            placeholder="输入项目名称"
            clearable
            maxlength="50"
            show-word-limit
          />
        </el-form-item>

        <el-form-item label="时区">
          <el-select
            v-model="form.timezone"
            placeholder="选择时区"
            style="width: 100%"
          >
            <el-option
              v-for="item in TIMEZONE_OPTIONS"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="flex justify-end gap-3">
          <el-button @click="dialogVisible = false">取消</el-button>
          <el-button type="primary" :loading="loading" @click="submit">
            {{ isEdit ? "保存" : "创建" }}
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
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

.animate-fade-in-up {
  animation: fade-in-up 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
}

/* 视图局部样式已简化，主要依赖全局设计体系 */
</style>
