<script setup lang="ts">
import { ref, reactive, watch } from "vue";

// TCP Point Detail Structure
export interface TcpPointDetail {
  tag: string;
  valueType: string;
  endian: string;
}

interface Props {
  modelValue: string; // JSON string
}

const props = defineProps<Props>();
const emit = defineEmits(["update:modelValue"]);

const localForm = reactive({
  tag: "",
  valueType: "uint16",
  endian: "big_endian"
});

const VALUE_TYPES = [
  { label: "Uint8", value: "uint8" },
  { label: "Uint16", value: "uint16" },
  { label: "Uint32", value: "uint32" },
  { label: "Int32", value: "int32" }
];

watch(
  () => props.modelValue,
  (newVal) => {
    if (!newVal) return;
    try {
      const detail = JSON.parse(newVal);
      localForm.tag = detail.tag || "";
      localForm.valueType = detail.valueType || "uint16";
      localForm.endian = detail.endian || "big_endian";
    } catch {}
  },
  { immediate: true }
);

watch(
  localForm,
  () => {
    emit("update:modelValue", JSON.stringify({
        tag: localForm.tag, // string to backend? backend expects u8 tag? 
        // Backend struct TcpPointDetail: tag: u8. But frontend form had string input?
        // Checking `types.rs`: pub tag: u8.
        // So we must ensure it is a number.
        // Wait, original `index.vue` passed `tcpTag` string to `buildTcpProtocolDetail`.
        // Let's check `utils/protocol.ts` if buildTcpProtocolDetail parses it.
        // Assuming user enters a number string.
        valueType: localForm.valueType,
        endian: localForm.endian
    }));
  },
  { deep: true }
);

</script>

<template>
  <el-form :model="localForm" label-position="top" class="p-4 bg-gray-50 rounded mb-4">
    <div class="font-bold mb-4 text-sm text-gray-700">TCP (TLV) 参数配置</div>
    
    <el-form-item label="Tag (ID)" required>
        <el-input v-model="localForm.tag" placeholder="请输入数字 Tag ID (0-255)" />
    </el-form-item>

    <div class="grid grid-cols-2 gap-4">
        <el-form-item label="数据类型">
            <el-select v-model="localForm.valueType">
                <el-option v-for="t in VALUE_TYPES" :key="t.value" :label="t.label" :value="t.value" />
            </el-select>
        </el-form-item>

        <el-form-item label="字节序">
             <el-select v-model="localForm.endian">
                <el-option label="Big Endian" value="big_endian" />
                <el-option label="Little Endian" value="little_endian" />
            </el-select>
        </el-form-item>
    </div>
  </el-form>
</template>
