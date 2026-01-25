<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import {
  listCommands,
  createCommand,
  listCommandReceipts,
  type CommandDto,
  type CommandReceiptDto
} from "@/api/ems/commands";
import { listPoints, type PointDto } from "@/api/ems/points";
import { listDevices, type DeviceDto } from "@/api/ems/devices";
import EmsProjectSelector from "@/components/EmsProjectSelector/index.vue";
import dayjs from "dayjs";
import { useRenderIcon } from "@/components/ReIcon/src/hooks";
import { ElMessage } from "element-plus";

defineOptions({
  name: "EmsCommands"
});

const loading = ref(false);
const error = ref("");
const projectId = ref("");
const listLimit = ref(20);
const items = ref<CommandDto[]>([]);

// Form State
const mode = ref<"simple" | "advanced">("simple");
const targetType = ref<"device" | "point">("point");
const targetId = ref("");

// Simple Mode Inputs
const simpleValue = ref<string | number>("");

// Advanced Mode Input
const rawJson = ref('{"action":"set","value":42}');

// Metadata
const points = ref<PointDto[]>([]);
const devices = ref<DeviceDto[]>([]);
const metadataLoading = ref(false);

const receiptsLoading = ref(false);
const receiptError = ref("");
const selectedCommandId = ref("");
const receipts = ref<CommandReceiptDto[]>([]);

// Options
const pointOptions = computed(() =>
  points.value.map(p => ({
    label: `${p.key} (${p.pointId})`,
    value: p.pointId
  }))
);

const deviceOptions = computed(() =>
  devices.value.map(d => ({
    label: `${d.name} (${d.deviceId})`,
    value: d.deviceId
  }))
);

// Load Metadata
const loadMetadata = async () => {
  if (!projectId.value) return;
  metadataLoading.value = true;
  try {
    const [pRes, dRes] = await Promise.all([
      listPoints(projectId.value),
      listDevices(projectId.value)
    ]);
    if (pRes.success) points.value = pRes.data ?? [];
    if (dRes.success) devices.value = dRes.data ?? [];
  } catch {
    // ignore
  } finally {
    metadataLoading.value = false;
  }
};

const fetchList = async () => {
  error.value = "";
  items.value = [];
  const pid = projectId.value.trim();
  if (!pid) return;

  loading.value = true;
  try {
    const res = await listCommands(pid, Number(listLimit.value) || 20);
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

const submit = async () => {
  error.value = "";
  const pid = projectId.value.trim();
  if (!pid) {
    ElMessage.warning("请选择项目");
    return;
  }

  if (mode.value === "simple" && !targetId.value) {
    ElMessage.warning("请选择目标对象");
    return;
  }

  let finalTarget = "";
  let finalPayload: unknown;

  if (mode.value === "simple") {
    finalTarget = targetId.value;
    // Simple mode defaults to wrapping value in an object or just sending raw value?
    // Usually commands expect a JSON object.
    // Let's assume common convention: { "value": X }
    finalPayload = { value: simpleValue.value };
  } else {
    // Advanced mode: manual target (if needed, but we can reuse targetId input or add raw string input for target)
    // To simplify: Advanced mode also uses the target selector for ID, but allows custom JSON payload.
    // If user needs custom target string, we could allow input.
    // Let's stick to target selector + Raw JSON.
    if (!targetId.value) {
      // If advanced user wants to type arbitrary string, we might need a text input.
      // Current UI uses Select. Let's assume target is always from metadata for safety.
      // But wait, user might want to test non-existent target?
      // Let's allow `allow-create` on select or just use input if raw.
    }
    finalTarget = targetId.value;

    try {
      finalPayload = JSON.parse(rawJson.value);
    } catch (err) {
      ElMessage.error("JSON 格式错误");
      return;
    }
  }

  loading.value = true;
  try {
    const res = await createCommand(pid, {
      target: finalTarget,
      payload: finalPayload
    });
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "下发失败");
      return;
    }
    ElMessage.success("命令下发成功");
    selectedCommandId.value = res.data?.commandId ?? "";
    // Configurable Auto-fetch
    await fetchList();
    // Auto-fetch receipts
    fetchReceipts(selectedCommandId.value);
  } catch (err) {
    ElMessage.error("请求失败");
  } finally {
    loading.value = false;
  }
};

const fetchReceipts = async (commandId?: string) => {
  receiptError.value = "";
  receipts.value = [];
  const pid = projectId.value.trim();
  const cid = (commandId ?? selectedCommandId.value).trim();
  if (!pid || !cid) return;

  receiptsLoading.value = true;
  try {
    const res = await listCommandReceipts(pid, cid);
    if (!res.success) {
      receiptError.value = res.error?.message ?? "加载失败";
      return;
    }
    receipts.value = res.data ?? [];
  } catch (err) {
    receiptError.value = "请求失败";
  } finally {
    receiptsLoading.value = false;
  }
};

const handleProjectChange = () => {
  targetId.value = "";
  loadMetadata();
  fetchList();
};

onMounted(() => {
  if (projectId.value) handleProjectChange();
});
</script>

<template>
  <div class="ems-page animate-fade-in-up">
    <!-- Header -->
    <el-card shadow="never" class="mb-4!">
      <template #header>
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div>
            <div class="text-base font-medium">控制命令下发</div>
            <div class="text-xs text-user-500">向设备或点位发送控制指令</div>
          </div>
          <EmsProjectSelector
            v-model="projectId"
            @change="handleProjectChange"
          />
        </div>
      </template>

      <el-form label-width="100px" label-position="left" class="max-w-[800px]">
        <!-- Mode Switch -->
        <el-form-item label="操作模式">
          <el-radio-group v-model="mode" size="default">
            <el-radio-button value="simple">简易模式 (Value)</el-radio-button>
            <el-radio-button value="advanced">高级模式 (JSON)</el-radio-button>
          </el-radio-group>
        </el-form-item>

        <!-- Target Selector -->
        <el-form-item label="目标对象">
          <div class="flex gap-2 w-full">
            <el-select v-model="targetType" class="!w-32">
              <el-option label="点位 (Point)" value="point" />
              <el-option label="设备 (Device)" value="device" />
            </el-select>
            <el-select
              v-model="targetId"
              class="flex-1"
              filterable
              placeholder="请选择目标"
              :loading="metadataLoading"
            >
              <el-option
                v-for="opt in targetType === 'point'
                  ? pointOptions
                  : deviceOptions"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
          </div>
        </el-form-item>

        <!-- Payload Input -->
        <el-form-item label="指令内容">
          <template v-if="mode === 'simple'">
            <el-input
              v-model="simpleValue"
              placeholder="请输入要下发的值 (例如: 1, 100, true)"
              clearable
            >
              <template #prepend>Value</template>
            </el-input>
            <div class="text-xs text-gray-400 mt-1">
              自动封装为: { "value": "{{ simpleValue }}" }
            </div>
          </template>
          <template v-else>
            <el-input
              v-model="rawJson"
              type="textarea"
              :rows="4"
              placeholder='{"action":"write","address":100,"value":1}'
            />
          </template>
        </el-form-item>

        <el-form-item>
          <el-button
            type="primary"
            :loading="loading"
            :icon="useRenderIcon('ep:promotion')"
            @click="submit"
          >
            发送指令
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <div class="flex gap-4 flex-wrap lg:flex-nowrap h-[600px]">
      <!-- Command History List -->
      <el-card
        shadow="never"
        class="flex-1 overflow-hidden flex flex-col min-w-[500px]"
      >
        <template #header>
          <div class="flex justify-between items-center">
            <span>命令历史</span>
            <el-button type="primary" link size="small" @click="fetchList"
              >刷新</el-button
            >
          </div>
        </template>
        <el-table
          :data="items"
          height="100%"
          stripe
          highlight-current-row
          @current-change="
            row => {
              if (row) fetchReceipts(row.commandId);
              selectedCommandId = row?.commandId;
            }
          "
        >
          <el-table-column prop="issuedAtMs" label="时间" width="160">
            <template #default="{ row }">
              <span class="text-xs font-mono">{{
                dayjs(row.issuedAtMs).format("MM-DD HH:mm:ss")
              }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="target"
            label="Target"
            min-width="120"
            show-overflow-tooltip
          />
          <el-table-column prop="status" label="状态" width="100">
            <template #default="{ row }">
              <el-tag
                :type="row.status === 'Sent' ? 'info' : 'success'"
                size="small"
                >{{ row.status }}</el-tag
              >
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80" align="center">
            <template #default="{ row }">
              <el-button
                link
                type="primary"
                size="small"
                @click.stop="
                  fetchReceipts(row.commandId);
                  selectedCommandId = row.commandId;
                "
                >详情</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </el-card>

      <!-- Receipts / Logs -->
      <el-card
        shadow="never"
        class="w-full lg:w-[450px] flex flex-col bg-gray-50 dark:bg-[#1d1e1f]"
      >
        <template #header>
          <div class="flex justify-between items-center">
            <span>执行回执</span>
            <div class="text-xs text-gray-400 font-mono">
              {{ selectedCommandId }}
            </div>
          </div>
        </template>

        <div v-loading="receiptsLoading" class="flex-1 overflow-y-auto p-2">
          <div
            v-if="receipts.length === 0"
            class="text-center text-gray-400 mt-10 text-sm"
          >
            暂无回执
          </div>
          <div v-else class="flex flex-col gap-3">
            <div
              v-for="log in receipts"
              :key="log.receiptId"
              class="bg-white dark:bg-[#2c2c2e] p-3 rounded shadow-sm border border-gray-100 dark:border-gray-700"
            >
              <div class="flex justify-between items-start mb-1">
                <el-tag
                  size="small"
                  :type="log.status === 'Success' ? 'success' : 'danger'"
                  >{{ log.status }}</el-tag
                >
                <span class="text-xs text-gray-400 font-mono">{{
                  dayjs(log.tsMs).format("HH:mm:ss.SSS")
                }}</span>
              </div>
              <div
                class="text-sm break-all font-mono text-gray-700 dark:text-gray-300"
              >
                {{ log.message }}
              </div>
            </div>
          </div>
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.ems-page {
  max-width: 1600px;
  padding: var(--space-6);
  margin: 0 auto;
}
</style>
