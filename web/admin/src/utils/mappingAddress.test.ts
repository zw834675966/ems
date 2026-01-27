import { describe, expect, it } from "vitest";

import {
  formatMappingAddressLine,
  mappingAddressFormLabel,
  mappingAddressPlaceholder,
  mappingSourceTypeLabel
} from "./mappingAddress";

describe("mappingAddress", () => {
  it("formats mqtt address", () => {
    expect(formatMappingAddressLine("mqtt", "topic/xxx")).toBe("MQTT: topic/xxx");
  });

  it("formats modbus address", () => {
    expect(formatMappingAddressLine("modbus", "unit/1/fc/3/addr/0")).toBe(
      "Modbus: unit/1/fc/3/addr/0"
    );
  });

  it("infers source type from address when missing/unknown", () => {
    expect(mappingSourceTypeLabel(undefined, "unit/1/fc/3/addr/0")).toBe("Modbus");
    expect(mappingSourceTypeLabel("unknown", "dev/1/tag/2")).toBe("TCP");
  });

  it("provides form labels/placeholders", () => {
    expect(mappingAddressFormLabel("mqtt")).toBe("topic (MQTT)");
    expect(mappingAddressFormLabel("modbus")).toBe("address (Modbus)");
    expect(mappingAddressFormLabel("tcp")).toBe("address (TCP)");

    expect(mappingAddressPlaceholder("mqtt")).toBe("topic/xxx");
    expect(mappingAddressPlaceholder("modbus")).toBe("unit/<unitId>/fc/<fc>/addr/<addr>");
    expect(mappingAddressPlaceholder("tcp")).toBe("dev/<devId>/tag/<tag>");
  });
});

