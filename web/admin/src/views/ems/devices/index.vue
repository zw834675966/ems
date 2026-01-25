<script setup lang="ts">
import { computed, onMounted, ref, reactive, nextTick } from "vue";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import dayjs from "dayjs";
import {
  listDevices,
  createDevice,
  updateDevice,
  deleteDevice,
  type DeviceDto
} from "@/api/ems/devices";
import { listGateways, type GatewayDto } from "@/api/ems/gateways";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";
import { useCrud } from "@/components/ReCrud/useCrud";
import {
  buildAddressConfig,
  parseAddressConfig,
  getProtocolLabel
} from "@/utils/protocol";

defineOptions({
  name: "EmsDevices"
});

const projectId = ref("");

// 使用 useCrud
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
  () => listDevices(projectId.value),
  id => deleteDevice(projectId.value, id),
  {
    immediate: false,
    idKey: "deviceId",
    watchSource: projectId,
    onFetchSuccess: () => {
      refreshGateways();
    }
  }
);

const gateways = ref<GatewayDto[]>([]);

const dialogVisible = ref(false);
const isEdit = ref(false);
const currentId = ref("");
const formRef = ref<FormInstance>();
const form = reactive({
  gatewayId: "",
  name: "",
  model: "",
  unitId: 1,
  devId: "",
  devType: ""
});

// 表单校验规则
const rules = reactive<FormRules>({
  gatewayId: [{ required: true, message: "请选择网关", trigger: "change" }],
  name: [
    { required: true, message: "请输入设备名称", trigger: "blur" },
    {
      min: 2,
      max: 50,
      message: "设备名称长度应为 2-50 个字符",
      trigger: "blur"
    }
  ]
});

// 获取当前选中网关的协议类型
const selectedGateway = computed(() =>
  gateways.value.find(g => g.gatewayId === form.gatewayId)
);

const isModbusGateway = computed(
  () => selectedGateway.value?.protocolType === "modbus_tcp"
);

const isTcpGateway = computed(
  () =>
    selectedGateway.value?.protocolType === "tcp_server" ||
    selectedGateway.value?.protocolType === "tcp_client"
);

const gatewayOptions = computed(() =>
  gateways.value.map(item => ({
    label: `${item.name} (${getProtocolLabel(item.protocolType)})`,
    value: item.gatewayId
  }))
);

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
  form.gatewayId = "";
  form.name = "";
  form.model = "";
  form.unitId = 1;
  form.devId = "";
  form.devType = "";
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

const handleEdit = (row: DeviceDto) => {
  isEdit.value = true;
  currentId.value = row.deviceId;
  resetForm();
  form.gatewayId = row.gatewayId;
  form.name = row.name;
  form.model = row.model || "";
  parseAddressConfig(row.addressConfig, form);
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
  const name = form.name.trim();
  const gatewayId = form.gatewayId.trim();
  if (isTcpGateway.value && !form.devId.trim()) {
    ElMessage.warning("请填写 DEV_ID");
    return;
  }

  loading.value = true;
  try {
    const model = form.model.trim();
    const addressConfig = buildAddressConfig(
      selectedGateway.value?.protocolType,
      form
    );

    let res;
    if (isEdit.value) {
      res = await updateDevice(pid, currentId.value, {
        gatewayId,
        name,
        model: model || undefined,
        addressConfig
      });
    } else {
      res = await createDevice(pid, {
        gatewayId,
        name,
        model: model || undefined,
        addressConfig
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

const getAddressLabel = (addressConfig?: string) => {
  if (!addressConfig) return "-";
  try {
    const config = JSON.parse(addressConfig);
    if (config.unitId !== undefined || config.slave_id !== undefined) {
      return `#${config.unitId ?? config.slave_id}`;
    }
    if (config.devId !== undefined) {
      return `dev:${config.devId}`;
    }
    return "-";
  } catch {
    return "-";
  }
};

onMounted(() => {
  if (projectId.value) {
    refreshGateways();
    fetchList();
  }
});
</script>

<template>
  <div class="apple-container animate-fade-in-up">
    <div class="flex-b mb-8 flex-wrap gap-4">
      <div>
        <h1>设备管理</h1>
        <p class="text-secondary">管理项目内的物理设备及其在线状态和协议地址</p>
      </div>
      <div class="flex items-center gap-4 flex-wrap">
        <el-input
          v-model="searchValue"
          placeholder="搜索设备..."
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
          添加设备
        </el-button>
      </div>
    </div>

    <el-card class="apple-shadow" shadow="never">
      <div v-if="error" class="mb-4">
        <el-alert type="error" :closable="false" show-icon>{{
          error
        }}</el-alert>
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
              label="设备名称"
              min-width="180"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                <span class="font-bold text-primary">{{ row.name }}</span>
              </template>
            </el-table-column>
            <el-table-column
              prop="addressConfig"
              label="从站地址"
              width="120"
              align="center"
            >
              <template #default="{ row }">
                <el-tag v-if="row.addressConfig" type="info" class="w-12">
                  {{ getAddressLabel(row.addressConfig) }}
                </el-tag>
                <span v-else class="text-gray-300">-</span>
              </template>
            </el-table-column>
            <el-table-column
              prop="online"
              label="实时状态"
              width="120"
              align="center"
            >
              <template #default="{ row }">
                <el-tooltip
                  v-if="row.lastError"
                  :content="row.lastError"
                  placement="top"
                  effect="dark"
                >
                  <div class="flex-c gap-2 cursor-help">
                    <span
                      class="status-dot"
                      :class="row.online ? 'is-online' : 'is-error'"
                    />
                    <span
                      :class="
                        row.online
                          ? 'text-green-500 font-medium'
                          : 'text-red-500 font-medium'
                      "
                    >
                      {{ row.online ? "在线" : "异常" }}
                    </span>
                  </div>
                </el-tooltip>
                <div v-else class="flex-c gap-2">
                  <span
                    class="status-dot"
                    :class="{ 'is-online': row.online }"
                  />
                  <span
                    :class="
                      row.online
                        ? 'text-green-500 font-medium'
                        : 'text-gray-400'
                    "
                  >
                    {{ row.online ? "在线" : "离线" }}
                  </span>
                </div>
              </template>
            </el-table-column>
            <el-table-column prop="model" label="型号" min-width="120">
              <template #default="{ row }">
                <span class="text-secondary">{{ row.model || "-" }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="deviceId" label="设备 ID" min-width="180">
              <template #default="{ row }">
                <code class="text-xs opacity-60">{{ row.deviceId }}</code>
              </template>
            </el-table-column>
            <el-table-column prop="lastSeenAtMs" label="最后在线" width="180">
              <template #default="{ row }">
                <span class="text-xs text-secondary">
                  {{
                    row.lastSeenAtMs
                      ? dayjs(row.lastSeenAtMs).format("YYYY-MM-DD HH:mm:ss")
                      : "-"
                  }}
                </span>
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
              <el-empty
                description="请先选择项目，或暂无设备数据"
                :image-size="120"
              >
                <el-button type="primary" @click="handleAdd"
                  >添加设备</el-button
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
      :title="isEdit ? '编辑设备' : '添加设备'"
      width="560px"
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
        <el-form-item label="所属网关" prop="gatewayId">
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
          <div class="mt-1 text-xs text-secondary">
            仅显示当前项目下的运行网关
          </div>
        </el-form-item>

        <el-form-item label="设备名称" prop="name">
          <el-input
            v-model="form.name"
            placeholder="例如：1号变压器"
            maxlength="50"
            show-word-limit
          />
        </el-form-item>

        <el-form-item label="设备型号">
          <el-input v-model="form.model" placeholder="可选输入设备型号" />
        </el-form-item>

        <!-- Modbus 从站地址配置 -->
        <template v-if="isModbusGateway">
          <div class="bg-secondary p-4 rounded-lg mb-6 text-sm">
            <h4 class="text-sm mb-4">Modbus 地址配置</h4>
            <el-form-item label="从站地址 (Unit ID)" required class="mb-0">
              <el-input-number
                v-model="form.unitId"
                :min="1"
                :max="247"
                class="w-full"
              />
              <div class="mt-2 text-xs text-secondary">
                Modbus 从站地址范围：1-247（文档口径：unitId，兼容 slave_id）
              </div>
            </el-form-item>
          </div>
        </template>

        <template v-else-if="isTcpGateway">
          <div class="bg-secondary p-4 rounded-lg mb-6 text-sm">
            <h4 class="text-sm mb-4">TCP 设备地址配置</h4>
            <el-form-item label="DEV_ID (UInt16)" required class="mb-3">
              <el-input v-model="form.devId" placeholder="例如：1" />
              <div class="mt-2 text-xs text-secondary">
                对应 TCP.md 的 DEV_ID（设备唯一编号，建议与设备侧一致）
              </div>
            </el-form-item>
            <el-form-item label="DEV_TYPE (UInt8，可选)" class="mb-0">
              <el-input v-model="form.devType" placeholder="例如：1" />
            </el-form-item>
          </div>
        </template>
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
.status-dot {
  position: relative;
  width: 8px;
  height: 8px;
  background: var(--color-gray-200);
  border-radius: 50%;

  &.is-online {
    background: var(--color-green);
    box-shadow: 0 0 0 rgb(40 205 65 / 40%);
    animation: pulse 2s infinite;
  }

  &.is-error {
    background: var(--color-red);
    box-shadow: 0 0 0 rgb(255 59 48 / 40%);
    animation: pulse-error 2s infinite;
  }
}

@keyframes pulse {
  0% {
    box-shadow: 0 0 0 0 rgb(40 205 65 / 70%);
  }

  70% {
    box-shadow: 0 0 0 10px rgb(40 205 65 / 0%);
  }

  100% {
    box-shadow: 0 0 0 0 rgb(40 205 65 / 0%);
  }
}

@keyframes pulse-error {
  0% {
    box-shadow: 0 0 0 0 rgb(255 59 48 / 70%);
  }

  70% {
    box-shadow: 0 0 0 10px rgb(255 59 48 / 0%);
  }

  100% {
    box-shadow: 0 0 0 0 rgb(255 59 48 / 0%);
  }
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
