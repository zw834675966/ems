<script setup lang="ts">
import { onMounted, ref, reactive, nextTick } from "vue";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import dayjs from "dayjs";
import {
  listGateways,
  createGateway,
  updateGateway,
  deleteGateway,
  testGateway,
  type GatewayDto
} from "@/api/ems/gateways";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";
import { useCrud } from "@/components/ReCrud/useCrud";
import { PROTOCOL_OPTIONS } from "@/config/constants";
import {
  buildProtocolConfig,
  parseProtocolConfig,
  getProtocolLabel
} from "@/utils/protocol";

defineOptions({
  name: "EmsGateways"
});

const projectId = ref("");

// 使用 useCrud，增加 watchSource 支持自动化加载
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
  () => listGateways(projectId.value),
  id => deleteGateway(projectId.value, id),
  {
    immediate: false,
    idKey: "gatewayId",
    watchSource: projectId
  }
);

const dialogVisible = ref(false);
const isEdit = ref(false);
const currentId = ref("");
const formRef = ref<FormInstance>();
const testingGateways = ref<Set<string>>(new Set());

const form = reactive({
  name: "",
  status: "online",
  protocolType: "mqtt",
  modbusHost: "",
  modbusPort: 502,
  modbusPollInterval: 1000,
  tcpServerPort: 9000,
  tcpClientHost: "",
  tcpClientPort: 8080,
  tcpClientPollIntervalMs: 1000
});

// 表单校验规则
const rules = reactive<FormRules>({
  name: [
    { required: true, message: "请输入网关名称", trigger: "blur" },
    {
      min: 2,
      max: 50,
      message: "网关名称长度应为 2-50 个字符",
      trigger: "blur"
    }
  ],
  modbusHost: [
    { required: true, message: "请输入 Modbus 服务器地址", trigger: "blur" }
  ],
  tcpClientHost: [
    { required: true, message: "请输入 TCP 服务器地址", trigger: "blur" }
  ]
});

const resetForm = () => {
  form.name = "";
  form.status = "online";
  form.protocolType = "mqtt";
  form.modbusHost = "";
  form.modbusPort = 502;
  form.modbusPollInterval = 1000;
  form.tcpServerPort = 9000;
  form.tcpClientHost = "";
  form.tcpClientPort = 8080;
  form.tcpClientPollIntervalMs = 1000;
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

const handleEdit = (row: GatewayDto) => {
  isEdit.value = true;
  currentId.value = row.gatewayId;
  resetForm();
  form.name = row.name;
  form.status = row.status;
  form.protocolType = row.protocolType || "mqtt";
  parseProtocolConfig(row.protocolType, row.protocolConfig, form);
  dialogVisible.value = true;
  nextTick(() => {
    formRef.value?.clearValidate();
  });
};

const handleTest = async (row: GatewayDto) => {
  testingGateways.value.add(row.gatewayId);
  try {
    const res = await testGateway(projectId.value, row.gatewayId);
    if (res.success && res.data) {
      const result = res.data;
      if (result.success) {
        ElMessage.success(`测试成功: 延迟 ${result.latencyMs}ms`);
      } else {
        ElMessage.error(`测试失败: ${result.error}`);
      }
      // 刷新列表以更新状态
      await fetchList();
    } else {
      ElMessage.error("测试请求失败");
    }
  } catch {
    ElMessage.error("测试请求异常");
  } finally {
    testingGateways.value.delete(row.gatewayId);
  }
};

const submit = async () => {
  if (!formRef.value) return;

  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  // 额外的协议特定校验
  if (form.protocolType === "modbus_tcp" && !form.modbusHost.trim()) {
    ElMessage.warning("请输入 Modbus 服务器地址");
    return;
  }
  if (form.protocolType === "tcp_client" && !form.tcpClientHost.trim()) {
    ElMessage.warning("请输入 TCP 服务器地址");
    return;
  }

  const pid = projectId.value.trim();

  loading.value = true;
  try {
    const name = form.name.trim();
    const status = form.status.trim();
    const protocolConfig = buildProtocolConfig(form.protocolType, form);

    let res;
    if (isEdit.value) {
      res = await updateGateway(pid, currentId.value, {
        name,
        status: status || undefined,
        protocolType: form.protocolType,
        protocolConfig
      });
    } else {
      res = await createGateway(pid, {
        name,
        status: status || undefined,
        protocolType: form.protocolType,
        protocolConfig
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

onMounted(() => {
  if (projectId.value) {
    fetchList();
  }
});
</script>

<template>
  <div class="apple-container animate-fade-in-up">
    <div class="flex-b mb-8 flex-wrap gap-4">
      <div>
        <h1>网关管理</h1>
        <p class="text-secondary">管理项目内的物理网关及其连接状态和协议配置</p>
      </div>
      <div class="flex items-center gap-4 flex-wrap">
        <el-input
          v-model="searchValue"
          placeholder="搜索网关..."
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
          添加网关
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
              label="网关名称"
              min-width="180"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                <span class="font-bold text-primary">{{ row.name }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="protocolType" label="协议类型" width="140">
              <template #default="{ row }">
                <el-tag
                  :type="row.protocolType === 'mqtt' ? 'primary' : 'warning'"
                >
                  {{ getProtocolLabel(row.protocolType) }}
                </el-tag>
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
            <el-table-column prop="gatewayId" label="网关 ID" min-width="180">
              <template #default="{ row }">
                <code class="text-xs opacity-60">{{ row.gatewayId }}</code>
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
              width="180"
              fixed="right"
              align="center"
            >
              <template #default="{ row }">
                <el-button
                  link
                  type="success"
                  :icon="useRenderIcon('ep:connection')"
                  :loading="testingGateways.has(row.gatewayId)"
                  @click="handleTest(row)"
                >
                  测试
                </el-button>
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
                description="请先选择项目，或暂无网关数据"
                :image-size="120"
              >
                <el-button type="primary" @click="handleAdd"
                  >添加网关</el-button
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
      :title="isEdit ? '编辑网关' : '添加网关'"
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
        <el-form-item label="网关名称" prop="name">
          <el-input
            v-model="form.name"
            placeholder="例如：1号楼中心网关"
            maxlength="50"
            show-word-limit
          />
        </el-form-item>

        <el-form-item label="协议类型" required>
          <el-select v-model="form.protocolType" class="w-full">
            <el-option
              v-for="opt in PROTOCOL_OPTIONS"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>

        <!-- Modbus TCP 配置 -->
        <template v-if="form.protocolType === 'modbus_tcp'">
          <div class="bg-secondary p-4 rounded-lg mb-6">
            <h4 class="text-sm mb-4">Modbus TCP 配置</h4>
            <el-row :gutter="12">
              <el-col :span="16">
                <el-form-item label="服务器地址" required>
                  <el-input
                    v-model="form.modbusHost"
                    placeholder="192.168.1.100"
                  />
                </el-form-item>
              </el-col>
              <el-col :span="8">
                <el-form-item label="端口">
                  <el-input-number
                    v-model="form.modbusPort"
                    :min="1"
                    :max="65535"
                    class="w-full"
                  />
                </el-form-item>
              </el-col>
            </el-row>
            <el-form-item label="轮询间隔 (ms)" class="mb-0">
              <el-input-number
                v-model="form.modbusPollInterval"
                :min="100"
                :max="60000"
                :step="100"
                class="w-full"
              />
            </el-form-item>
          </div>
        </template>

        <!-- TCP Server 配置 -->
        <template v-if="form.protocolType === 'tcp_server'">
          <div class="bg-secondary p-4 rounded-lg mb-6">
            <h4 class="text-sm mb-4">TCP Server 配置</h4>
            <el-form-item label="监听端口" required class="mb-0">
              <el-input-number
                v-model="form.tcpServerPort"
                :min="1"
                :max="65535"
                class="w-full"
              />
            </el-form-item>
          </div>
        </template>

        <!-- TCP Client 配置 -->
        <template v-if="form.protocolType === 'tcp_client'">
          <div class="bg-secondary p-4 rounded-lg mb-6">
            <h4 class="text-sm mb-4">TCP Client 配置</h4>
            <el-row :gutter="12">
              <el-col :span="16">
                <el-form-item label="服务器地址" required class="mb-0">
                  <el-input
                    v-model="form.tcpClientHost"
                    placeholder="192.168.1.100"
                  />
                </el-form-item>
              </el-col>
              <el-col :span="8">
                <el-form-item label="端口" class="mb-0">
                  <el-input-number
                    v-model="form.tcpClientPort"
                    :min="1"
                    :max="65535"
                    class="w-full"
                  />
                </el-form-item>
              </el-col>
            </el-row>
            <el-form-item label="轮询间隔 (ms)" class="mt-4 mb-0">
              <el-input-number
                v-model="form.tcpClientPollIntervalMs"
                :min="100"
                :max="60000"
                :step="100"
                class="w-full"
              />
            </el-form-item>
          </div>
        </template>

        <el-form-item label="配置状态">
          <el-select v-model="form.status" class="w-full">
            <el-option label="Online (激活)" value="online" />
            <el-option label="Offline (禁用)" value="offline" />
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
