import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { defineComponent, h, provide, inject } from "vue";

const testGateway = vi.fn();

let crudState: {
  loading: any;
  error: any;
  filteredData: any;
  searchValue: any;
  pagination: any;
  fetchList: any;
  handleDelete: any;
  onPageSizeChange: any;
  onCurrentPageChange: any;
};

const createCrudState = () => {
  const { ref } = require("vue");
  return {
    loading: ref(false),
    error: ref(""),
    filteredData: ref<any[]>([]),
    searchValue: ref(""),
    pagination: ref({ total: 0, pageSize: 10, currentPage: 1 }),
    fetchList: vi.fn().mockResolvedValue(undefined),
    handleDelete: vi.fn(),
    onPageSizeChange: vi.fn(),
    onCurrentPageChange: vi.fn()
  };
};

vi.mock("@/components/ReCrud/useCrud", () => {
  if (!crudState) {
    crudState = createCrudState();
  }
  return {
    useCrud: () => crudState
  };
});

vi.mock("@/api/ems/gateways", () => ({
  listGateways: vi.fn(),
  createGateway: vi.fn(),
  updateGateway: vi.fn(),
  deleteGateway: vi.fn(),
  testGateway: (...args: any[]) => testGateway(...args)
}));

vi.mock("@/components/ReIcon/src/hooks", () => ({
  useRenderIcon: () => () => null
}));

vi.mock("@/config/constants", () => ({
  PROTOCOL_OPTIONS: []
}));

vi.mock("@/components/EmsProjectSelector/index.vue", () => {
  const { defineComponent, h } = require("vue");
  return {
    default: defineComponent({
      name: "EmsProjectSelector",
      props: { modelValue: { type: String, default: "" } },
      emits: ["update:modelValue", "change"],
      setup() {
        return () => h("div");
      }
    })
  };
});

vi.mock("element-plus", () => ({
  ElMessage: {
    warning: vi.fn(),
    success: vi.fn(),
    error: vi.fn()
  }
}));

const SimpleStub = defineComponent({
  name: "SimpleStub",
  setup(_, { slots }) {
    return () => h("div", slots.default?.());
  }
});

const ElButtonStub = defineComponent({
  name: "ElButtonStub",
  props: {
    disabled: Boolean,
    loading: Boolean,
    type: String,
    link: Boolean
  },
  emits: ["click"],
  setup(props, { slots, emit }) {
    return () =>
      h(
        "button",
        { disabled: props.disabled, onClick: () => emit("click") },
        slots.default?.()
      );
  }
});

const RowProvider = defineComponent({
  name: "RowProvider",
  props: {
    row: { type: Object, required: true }
  },
  setup(props, { slots }) {
    provide("ems-row", props.row);
    return () => h("div", slots.default?.());
  }
});

const ElTableStub = defineComponent({
  name: "ElTableStub",
  props: {
    data: { type: Array, default: () => [] }
  },
  setup(props, { slots }) {
    return () =>
      h("div", [
        (props.data as any[]).map((row, index) =>
          h(
            RowProvider,
            { row, key: index },
            { default: () => slots.default?.() }
          )
        ),
        props.data.length === 0 ? slots.empty?.() : null
      ]);
  }
});

const ElTableColumnStub = defineComponent({
  name: "ElTableColumnStub",
  setup(_, { slots }) {
    const row = inject<any>("ems-row");
    return () => h("div", slots.default?.({ row }));
  }
});

describe("EmsGateways view", () => {
  const globalStubs = {
    "el-input": SimpleStub,
    "el-card": SimpleStub,
    "el-alert": SimpleStub,
    "el-skeleton": SimpleStub,
    "el-tag": SimpleStub,
    "el-tooltip": SimpleStub,
    "el-pagination": SimpleStub,
    "el-dialog": SimpleStub,
    "el-form": SimpleStub,
    "el-form-item": SimpleStub,
    "el-select": SimpleStub,
    "el-option": SimpleStub,
    "el-empty": SimpleStub,
    "el-table": ElTableStub,
    "el-table-column": ElTableColumnStub,
    "el-button": ElButtonStub,
    "el-row": SimpleStub,
    "el-col": SimpleStub,
    "el-input-number": SimpleStub,
    EmsProjectSelector: SimpleStub
  };

  beforeEach(() => {
    vi.clearAllMocks();
    if (!crudState) {
      crudState = createCrudState();
    }
    crudState.filteredData.value = [];
  });

  it("未选择项目时点击添加网关提示警告", async () => {
    const GatewaysView = (await import("../gateways/index.vue")).default;
    const wrapper = mount(GatewaysView, {
      global: {
        stubs: globalStubs,
        directives: {
          loading: () => {}
        }
      }
    });

    const addButton = wrapper
      .findAll("button")
      .find(btn => btn.text().includes("添加网关"));
    expect(addButton).toBeTruthy();
    await addButton?.trigger("click");

    const { ElMessage } = await import("element-plus");
    expect(ElMessage.warning).toHaveBeenCalledWith("请先选择项目");
  });

  it("测试网关成功后提示并刷新列表", async () => {
    crudState.filteredData.value = [
      {
        gatewayId: "gateway-1",
        name: "Gateway One",
        status: "online",
        protocolType: "mqtt"
      }
    ];
    testGateway.mockResolvedValue({
      success: true,
      data: { success: true, latencyMs: 12 }
    });

    const GatewaysView = (await import("../gateways/index.vue")).default;
    const wrapper = mount(GatewaysView, {
      global: {
        stubs: globalStubs,
        directives: {
          loading: () => {}
        }
      }
    });

    const testButton = wrapper
      .findAll("button")
      .find(btn => btn.text().trim() === "测试");
    expect(testButton).toBeTruthy();
    await testButton?.trigger("click");
    await flushPromises();

    expect(testGateway).toHaveBeenCalledWith("", "gateway-1");
    expect(crudState.fetchList).toHaveBeenCalled();
    const { ElMessage } = await import("element-plus");
    expect(ElMessage.success).toHaveBeenCalledWith("测试成功: 延迟 12ms");
  });
});
