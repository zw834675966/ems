<script setup lang="ts">
import { computed, onMounted, ref, reactive, nextTick } from "vue";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import {
  listPoints,
  createPoint,
  updatePoint,
  deletePoint,
  type PointDto
} from "@/api/ems/points";
import { listDevices, type DeviceDto } from "@/api/ems/devices";
import { listGateways, type GatewayDto } from "@/api/ems/gateways";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";
import { useCrud } from "@/components/ReCrud/useCrud";

defineOptions({
  name: "EmsPoints"
});

const projectId = ref("");

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
} = useCrud(
  () => listPoints(projectId.value),
  (id) => deletePoint(projectId.value, id),
  {
    immediate: false,
    idKey: "pointId",
    watchSource: projectId,
    onFetchSuccess: () => {
        refreshDevices();
        refreshGateways();
    }
  }
);

const devices = ref<DeviceDto[]>([]);
const gateways = ref<GatewayDto[]>([]);

const dialogVisible = ref(false);
const isEdit = ref(false);
const currentId = ref("");
const formRef = ref<FormInstance>();
const form = reactive({
  deviceId: "",
  key: "",
  dataType: "float",
  unit: ""
});

// 表单校验规则
const rules = reactive<FormRules>({
  deviceId: [
    { required: true, message: "请选择设备", trigger: "change" }
  ],
  key: [
    { required: true, message: "请输入点位 Key", trigger: "blur" },
    { min: 1, max: 50, message: "点位 Key 长度应为 1-50 个字符", trigger: "blur" }
  ],
  dataType: [
    { required: true, message: "请选择数据类型", trigger: "change" }
  ]
});

const deviceOptions = computed(() =>
  devices.value.map(item => ({
    label: `${item.name} (${item.deviceId})`,
    value: item.deviceId
  }))
);

// 当前选中设备对应的网关
const selectedDevice = computed(() =>
  devices.value.find(d => d.deviceId === form.deviceId)
);

const selectedGateway = computed(() =>
  gateways.value.find(g => g.gatewayId === selectedDevice.value?.gatewayId)
);

const isModbusTcp = computed(() =>
  selectedGateway.value?.protocolType === "modbus_tcp"
);

const refreshDevices = async () => {
  const pid = projectId.value.trim();
  devices.value = [];
  if (!pid) return;
  try {
    const res = await listDevices(pid);
    if (!res.success) return;
    devices.value = res.data ?? [];
  } catch {}
};

const refreshGateways = async () => {
  const pid = projectId.value.trim();
  gateways.value = [];
  if (!pid) return;
  try {
    const res = await listGateways(pid);
    if (!res.success) return;
    gateways.value = res.data ?? [];
  } catch {}
};

const resetForm = () => {
  form.deviceId = "";
  form.key = "";
  form.dataType = "float";
  form.unit = "";
  nextTick(() => {
    formRef.value?.clearValidate();
  });
};

const handleAdd = () => {
  if (!projectId.value) {
    ElMessage.warning("请先选择项目");
    return;
  }
  isEdit.value = false;
  currentId.value = "";
  resetForm();
  dialogVisible.value = true;
};

const handleEdit = (row: PointDto) => {
  isEdit.value = true;
  currentId.value = row.pointId;
  resetForm();
  form.deviceId = row.deviceId;
  form.key = row.key;
  form.dataType = row.dataType;
  form.unit = row.unit || "";

  dialogVisible.value = true;
  nextTick(() => {
    formRef.value?.clearValidate();
  });
};

const submit = async () => {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  const pid = projectId.value.trim();
  const deviceId = form.deviceId.trim();
  const key = form.key.trim();
  const dataType = form.dataType.trim();

  loading.value = true;
  try {
    const unit = form.unit.trim();
    let res;
    if (isEdit.value) {
      res = await updatePoint(pid, currentId.value, {
        deviceId,
        key,
        dataType,
        unit: unit ? unit : undefined
      });
    } else {
      res = await createPoint(pid, {
        deviceId,
        key,
        dataType,
        unit: unit ? unit : undefined
      });
    }

    if (!res.success) {
      ElMessage.error(res.error?.message ?? (isEdit.value ? "更新失败" : "创建失败"));
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

onMounted(() => {
  if (projectId.value) {
    refreshDevices();
    refreshGateways();
    fetchList();
  }
});
</script>

<template>
  <div class="apple-container animate-fade-in-up">
    <div class="flex-b mb-8 flex-wrap gap-4">
      <div>
        <h1>点位管理</h1>
        <p class="text-secondary">管理项目内的采集点位（Point）及其数据元信息</p>
      </div>
      <div class="flex items-center gap-4 flex-wrap">
        <el-input
          v-model="searchValue"
          placeholder="搜索点位 Key..."
          class="w-[200px]"
          clearable
          :prefix-icon="useRenderIcon('ep:search')"
        />
        <EmsProjectSelector v-model="projectId" />
        <el-button
          type="primary"
          :icon="useRenderIcon('ri:add-circle-line')"
          @click="handleAdd"
        >
          添加点位
        </el-button>
      </div>
    </div>

    <el-card class="apple-shadow" shadow="never">
      <div v-if="error" class="mb-4">
        <el-alert type="error" :closable="false" show-icon>{{ error }}</el-alert>
      </div>

      <el-skeleton :rows="5" animated :loading="loading && filteredData.length === 0">
        <template #default>
          <el-table
            v-loading="loading"
            :data="filteredData"
            row-class-name="animate-fade-in-up"
          >
            <el-table-column prop="key" label="点位 Key" min-width="180" show-overflow-tooltip>
              <template #default="{ row }">
                <span class="font-bold text-primary">{{ row.key }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="dataType" label="数据类型" width="140">
              <template #default="{ row }">
                <el-tag size="small">{{ row.dataType }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="unit" label="单位" width="100">
              <template #default="{ row }">
                <span class="text-secondary">{{ row.unit || '-' }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="pointId" label="点位 ID" min-width="180">
              <template #default="{ row }">
                <code class="text-xs opacity-60">{{ row.pointId }}</code>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="140" fixed="right" align="center">
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
              <el-empty description="请先选择项目，或暂无点位数据" :image-size="120">
                <el-button type="primary" @click="handleAdd">添加点位</el-button>
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
      :title="isEdit ? '编辑点位' : '添加点位'"
      width="560px"
      append-to-body
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top" class="mt-4">
        <el-form-item label="所属设备" prop="deviceId">
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
        </el-form-item>
        <el-form-item label="点位 Key" prop="key">
          <el-input v-model="form.key" placeholder="例如：voltage_a, temperature" maxlength="50" show-word-limit />
        </el-form-item>
        
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="数据类型" prop="dataType">
              <el-select v-model="form.dataType" class="w-full">
                <el-option label="Float" value="float" />
                <el-option label="Integer" value="int" />
                <el-option label="Boolean" value="bool" />
                <el-option label="String" value="string" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="显示单位">
              <el-input v-model="form.unit" placeholder="例如：V, ℃, kW" />
            </el-form-item>
            </el-col>
        </el-row>
        
        <template v-if="isModbusTcp">
          <el-alert
            type="info"
            :closable="false"
            show-icon
            class="mb-6"
            title="Modbus/TCP 点位协议细节请在“点位映射”页面配置（sourceType=modbus）。此处仅维护点位元信息（dataType/unit）。"
          />
        </template>
      </el-form>
      <template #footer>
        <div class="flex justify-end gap-3">
          <el-button @click="dialogVisible = false">取消</el-button>
          <el-button type="primary" :loading="loading" @click="submit">
            {{ isEdit ? '保存' : '创建' }}
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.animate-fade-in-up {
  animation: fade-in-up 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
}

@keyframes fade-in-up {
  from { opacity: 0; transform: translateY(15px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
