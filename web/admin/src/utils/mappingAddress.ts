export type MappingSourceType = "mqtt" | "modbus" | "tcp";

export const inferMappingSourceTypeFromAddress = (
  address?: string
): MappingSourceType | undefined => {
  if (!address) return undefined;
  if (address.startsWith("unit/")) return "modbus";
  if (address.startsWith("dev/")) return "tcp";
  return "mqtt";
};

export const normalizeMappingSourceType = (
  sourceType?: string,
  address?: string
): MappingSourceType | undefined => {
  const st = (sourceType ?? "").toLowerCase();
  if (st === "mqtt" || st === "modbus" || st === "tcp") return st;
  return inferMappingSourceTypeFromAddress(address);
};

export const mappingSourceTypeLabel = (
  sourceType?: string,
  address?: string
): string => {
  const normalized = normalizeMappingSourceType(sourceType, address);
  if (normalized === "mqtt") return "MQTT";
  if (normalized === "modbus") return "Modbus";
  if (normalized === "tcp") return "TCP";
  return sourceType || "ADDR";
};

export const formatMappingAddressLine = (
  sourceType?: string,
  address?: string
): string | null => {
  if (!address) return null;
  return `${mappingSourceTypeLabel(sourceType, address)}: ${address}`;
};

export const mappingAddressPlaceholder = (sourceType: string): string => {
  if (sourceType === "modbus") return "unit/<unitId>/fc/<fc>/addr/<addr>";
  if (sourceType === "tcp") return "dev/<devId>/tag/<tag>";
  return "topic/xxx";
};

export const mappingAddressFormLabel = (sourceType: string): string => {
  if (sourceType === "modbus") return "address (Modbus)";
  if (sourceType === "tcp") return "address (TCP)";
  return "topic (MQTT)";
};

