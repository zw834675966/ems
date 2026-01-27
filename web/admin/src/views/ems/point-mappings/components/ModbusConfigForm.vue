<script setup lang="ts">
import { ref, reactive, watch, computed, onMounted } from "vue";
import { type FormRules, type FormInstance } from "element-plus";

// Modbus Point Detail Structure matching protocol definition
export interface ModbusPointDetail {
  functionCode: number;
  registerAddress: number;
  registerCount: number;
  dataType: string; // "int16", "float32", etc.
  endian: string; // "big_endian", "little_endian"
  wordOrder?: string; // "ABCD", "CDAB", etc.
}

interface Props {
  modelValue: string; // JSON string of ModbusPointDetail
  writable: boolean; // From parent form
}

const props = defineProps<Props>();
const emit = defineEmits(["update:modelValue", "update:writable"]);

const formRef = ref<FormInstance>();

// --- Internal State ---

// User Intent: High level selection
type ModbusIntent = "coil" | "discrete" | "holding" | "input";
const intent = ref<ModbusIntent>("holding");

// Address Mode: standard (0-based) or plc (1-based)
const addressMode = ref<"protocol" | "plc">("plc");
// Display Address (might be 1-based)
const displayAddress = ref<number | null>(null);

// Internal form data
const localForm = reactive({
  functionCode: 3,
  registerAddress: 0,
  registerCount: 1,
  dataType: "int16",
  endian: "big_endian",
  wordOrder: undefined as string | undefined,
});

// Constants
const DATA_TYPES = [
  { label: "布尔量 (Bool)", value: "bool" },
  { label: "16位有符号整数 (Int16)", value: "int16" },
  { label: "16位无符号整数 (Uint16)", value: "uint16" },
  { label: "32位有符号整数 (Int32)", value: "int32" },
  { label: "32位无符号整数 (Uint32)", value: "uint32" },
  { label: "32位浮点数 (Float32)", value: "float32" },
  { label: "64位浮点数 (Float64)", value: "float64" },
];

const INTENTS = [
  { label: "控制开关 (Coil - RW)", value: "coil", fc: 1, writable: true, type: "bool" },
  { label: "只读状态 (Discrete Input - RO)", value: "discrete", fc: 2, writable: false, type: "bool" },
  { label: "控制参数/设定值 (Holding Register - RW)", value: "holding", fc: 3, writable: true, type: "num" },
  { label: "传感器/采集值 (Input Register - RO)", value: "input", fc: 4, writable: false, type: "num" },
];

const WORD_ORDERS = [
  { label: "ABCD (Big Endian)", value: "ABCD" },
  { label: "CDAB (Little Endian Swap)", value: "CDAB" },
  { label: "BADC (Big Endian Swap)", value: "BADC" },
  { label: "DCBA (Little Endian)", value: "DCBA" },
];

// --- Logic & Helpers ---

// Handle Intent Change
const applyIntent = (newIntent: ModbusIntent) => {
  const def = INTENTS.find(i => i.value === newIntent);
  if (!def) return;
  
  localForm.functionCode = def.fc;
  emit("update:writable", def.writable); // Sync writable to parent
  
  // Set default data type if switching between bool/num
  if (def.type === "bool") {
      localForm.dataType = "bool";
      localForm.registerCount = 1;
  } else if (localForm.dataType === "bool") {
      // Switching from bool to num, default to int16
      localForm.dataType = "int16";
      localForm.registerCount = 1;
  }
};

const handleIntentChange = (val: ModbusIntent) => {
    intent.value = val;
    applyIntent(val);
};

// Address Handling Functions
const updateDisplayAddress = () => {
    if (addressMode.value === "plc") {
        let base = 0;
        switch (localForm.functionCode) {
            case 1: base = 0; break; // Coils usually 00001
            case 2: base = 10000; break; // Discrete 10001
            case 3: base = 40000; break; // Holding 40001
            case 4: base = 30000; break; // Input 30001
        }
        // Ideally PLC address logic is complex (0xxxx, 1xxxx), 
        // here we simply simplify to: Display = Register + 1 (if mode is PLC)
        // Or strictly follow 1-based logic: 40001 -> Reg 0.
        // Let's implement simple 1-based logic.
        displayAddress.value = localForm.registerAddress + 1;
    } else {
        displayAddress.value = localForm.registerAddress;
    }
};

const handleDisplayAddressChange = (val: number | null) => {
    if (val === null) return;
    if (addressMode.value === "plc") {
        localForm.registerAddress = Math.max(0, val - 1);
    } else {
        localForm.registerAddress = Math.max(0, val);
    }
};

// Reverse Inference
function inferIntentFromValues() {
    const fc = localForm.functionCode;
    if (fc === 1 || fc === 5 || fc === 15) intent.value = "coil";
    else if (fc === 2) intent.value = "discrete";
    else if (fc === 3 || fc === 6 || fc === 16) intent.value = "holding";
    else if (fc === 4) intent.value = "input";
}

// --- Watchers ---

// 1. Sync from Props to Local
watch(
  () => props.modelValue,
  (newVal) => {
    if (!newVal) {
        // Init default
        applyIntent("holding");
        return;
    }
    try {
      const detail = JSON.parse(newVal) as ModbusPointDetail;
      localForm.functionCode = detail.functionCode || 3;
      localForm.registerAddress = detail.registerAddress || 0;
      localForm.registerCount = detail.registerCount || 1;
      localForm.dataType = detail.dataType || "int16";
      localForm.endian = detail.endian || "big_endian";
      localForm.wordOrder = detail.wordOrder;
      
      // Reverse infer intent
      inferIntentFromValues();
      // Update display address
      updateDisplayAddress();
    } catch (e) {
      console.warn("Failed to parse Modbus detail", e);
    }
  },
  { immediate: true }
);

// 2. Sync Local to Props (Emission)
watch(
  localForm,
  () => {
    const detail: ModbusPointDetail = {
      functionCode: localForm.functionCode,
      registerAddress: localForm.registerAddress,
      registerCount: localForm.registerCount,
      dataType: localForm.dataType,
      endian: localForm.endian,
      wordOrder: localForm.wordOrder,
    };
    emit("update:modelValue", JSON.stringify(detail));
  },
  { deep: true }
);

// 3. Handle Intent Change (Moved to top)

// 4. Handle DataType Change (Auto-calc count)
watch(() => localForm.dataType, (type) => {
    switch (type) {
        case "bool":
        case "int16":
        case "uint16":
            localForm.registerCount = 1;
            break;
        case "int32":
        case "uint32":
        case "float32":
            localForm.registerCount = 2;
            if (!localForm.wordOrder) localForm.wordOrder = "ABCD";
            break;
        case "float64":
            localForm.registerCount = 4;
            if (!localForm.wordOrder) localForm.wordOrder = "ABCD";
            break;
    }
});

// 5. Address Handling (Funcs moved to top)

watch([() => localForm.registerAddress, addressMode], () => {
    updateDisplayAddress();
});





// Validation logic
const is32Bit = computed(() => ["int32", "uint32", "float32", "float64"].includes(localForm.dataType));

</script>

<template>
  <el-form ref="formRef" :model="localForm" label-position="top" class="p-4 bg-gray-50 rounded mb-4">
    <div class="font-bold mb-4 text-sm text-gray-700">Modbus 参数配置</div>
    
    <el-form-item label="数据用途 (Intent)">
        <el-radio-group :model-value="intent" @update:modelValue="handleIntentChange">
            <el-radio-button v-for="item in INTENTS" :key="item.value" :label="item.value">
                {{ item.label }}
            </el-radio-button>
        </el-radio-group>
        <div class="text-xs text-gray-400 mt-1">
            自动关联功能码：FC {{ localForm.functionCode }}，读写模式：{{ props.writable ? '可写' : '只读' }}
        </div>
    </el-form-item>

    <div class="grid grid-cols-2 gap-4">
        <el-form-item label="数据类型">
            <el-select v-model="localForm.dataType">
                <el-option
                    v-for="type in DATA_TYPES"
                    :key="type.value"
                    :label="type.label"
                    :value="type.value"
                    :disabled="intent === 'coil' || intent === 'discrete' ? type.value !== 'bool' : type.value === 'bool'"
                />
            </el-select>
        </el-form-item>

        <el-form-item v-if="localForm.functionCode !== 1 && localForm.functionCode !== 2" label="寄存器数量">
             <el-input-number v-model="localForm.registerCount" :min="1" :max="125" controls-position="right" />
        </el-form-item>
    </div>

    <div class="grid grid-cols-2 gap-4">
        <el-form-item label="地址模式">
            <el-radio-group v-model="addressMode" @change="updateDisplayAddress">
                <el-radio label="plc">PLC 地址 (1-based)</el-radio>
                <el-radio label="protocol">协议地址 (0-based)</el-radio>
            </el-radio-group>
        </el-form-item>

        <el-form-item label="起始地址">
             <el-input-number 
                :model-value="displayAddress" 
                @update:modelValue="handleDisplayAddressChange"
                :min="addressMode === 'plc' ? 1 : 0" 
                :max="65535" 
                class="w-full"
                controls-position="right" 
             />
             <div class="text-xs text-gray-400 mt-1">
                 协议实际地址: {{ localForm.registerAddress }} (0x{{ localForm.registerAddress.toString(16).toUpperCase() }})
             </div>
        </el-form-item>
    </div>

    <div class="grid grid-cols-2 gap-4">
        <el-form-item label="字节序 (Endian)">
            <el-select v-model="localForm.endian">
                <el-option label="Big Endian (默认)" value="big_endian" />
                <el-option label="Little Endian" value="little_endian" />
            </el-select>
        </el-form-item>
        
        <el-form-item v-if="is32Bit" label="字序 (Word Order)" requiredProp>
             <el-select v-model="localForm.wordOrder" placeholder="32位数据必选">
                <el-option v-for="wo in WORD_ORDERS" :key="wo.value" :label="wo.label" :value="wo.value" />
            </el-select>
        </el-form-item>
    </div>
  </el-form>
</template>
