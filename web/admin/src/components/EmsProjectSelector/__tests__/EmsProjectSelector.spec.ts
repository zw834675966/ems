import { describe, expect, it, vi, beforeEach } from "vitest";
import { defineComponent } from "vue";
import { mount, flushPromises } from "@vue/test-utils";
import EmsProjectSelector from "../index.vue";

const listProjects = vi.fn();
const saveEmsProjectId = vi.fn();
const loadEmsProjectId = vi.fn();
const clearEmsProjectId = vi.fn();

vi.mock("@/api/ems/projects", () => ({
  listProjects: (...args: any[]) => listProjects(...args)
}));

vi.mock("@/utils/emsProject", () => ({
  saveEmsProjectId: (...args: any[]) => saveEmsProjectId(...args),
  loadEmsProjectId: (...args: any[]) => loadEmsProjectId(...args),
  clearEmsProjectId: (...args: any[]) => clearEmsProjectId(...args)
}));

vi.mock("element-plus", () => ({
  ElMessage: {
    success: vi.fn(),
    info: vi.fn()
  }
}));

const ElSelectStub = defineComponent({
  props: {
    modelValue: {
      type: String,
      default: ""
    },
    disabled: Boolean
  },
  emits: ["update:modelValue", "change"],
  template: `
    <select
      data-test="project-select"
      :value="modelValue"
      :disabled="disabled"
      @change="onChange"
    >
      <slot />
    </select>
  `,
  methods: {
    onChange(event: Event) {
      const value = (event.target as HTMLSelectElement).value;
      this.$emit("update:modelValue", value);
      this.$emit("change", value);
    }
  }
});

const ElOptionStub = defineComponent({
  props: {
    label: String,
    value: String
  },
  template: `<option :value="value">{{ label }}</option>`
});

const ElButtonStub = defineComponent({
  props: {
    disabled: Boolean
  },
  emits: ["click"],
  template: `<button :disabled="disabled" @click="$emit('click')"><slot /></button>`
});

describe("EmsProjectSelector", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    loadEmsProjectId.mockReturnValue("");
    (navigator as any).clipboard = {
      writeText: vi.fn().mockResolvedValue(null)
    };
  });

  it("loads projects and selects default on success", async () => {
    listProjects.mockResolvedValue({
      success: true,
      data: [
        { projectId: "project-1", name: "Project 1" },
        { projectId: "project-2", name: "Project 2" }
      ]
    });

    const wrapper = mount(EmsProjectSelector, {
      props: { modelValue: "" },
      global: {
        stubs: {
          "el-select": ElSelectStub,
          "el-option": ElOptionStub,
          "el-button": ElButtonStub
        }
      }
    });

    await flushPromises();
    expect(listProjects).toHaveBeenCalled();
    expect(wrapper.emitted("update:modelValue")?.[0]).toEqual(["project-1"]);

    const select = wrapper.get<HTMLSelectElement>(
      '[data-test="project-select"]'
    );
    await select.setValue("project-2");
    expect(saveEmsProjectId).toHaveBeenCalledWith("project-2");
  });

  it("renders error on failure and blocks copy", async () => {
    listProjects.mockResolvedValue({
      success: false,
      error: { message: "Forbidden" }
    });

    const wrapper = mount(EmsProjectSelector, {
      props: { modelValue: "" },
      global: {
        stubs: {
          "el-select": ElSelectStub,
          "el-option": ElOptionStub,
          "el-button": ElButtonStub
        }
      }
    });

    await flushPromises();
    expect(wrapper.text()).toContain("Forbidden");
    const buttons = wrapper.findAll("button");
    const copyButton = buttons.find(btn =>
      btn.text().includes("复制 projectId")
    );
    expect(copyButton?.attributes("disabled")).toBeDefined();
  });
});
