import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type ModbusReadItem = {
  functionCode: number; // 1/2/3/4
  address: number; // 0-based
  quantity: number;
};

export type ModbusWriteItem = {
  functionCode: number; // 5/6
  address: number; // 0-based
  value: number; // FC05: 0/1, FC06: 0..65535
};

export type ModbusSnapshotRequest = {
  gatewayId: string;
  deviceId?: string;
  unitId?: number;
  reads: ModbusReadItem[];
  writes: ModbusWriteItem[];
  lockTimeoutMs?: number;
  connectTimeoutMs?: number;
  requestTimeoutMs?: number;
  maxRetries?: number;
  retryIntervalMs?: number;
};

export type ModbusSnapshotResponse = {
  capturedAtMs: number;
  latencyMs: number;
  target: {
    gatewayId: string;
    deviceId?: string;
    unitId: number;
    host: string;
    port: number;
  };
  results: Array<{
    functionCode: number;
    address: number;
    quantity: number;
    raw: { bits?: boolean[]; registers?: number[] };
    error?: { code: string; message: string } | null;
  }>;
  writeResults: Array<{
    functionCode: number;
    address: number;
    value: number;
    ok: boolean;
    error?: { code: string; message: string };
  }>;
};

export const modbusSnapshot = (
  projectId: string,
  data: ModbusSnapshotRequest
) => {
  return http.request<ApiResponse<ModbusSnapshotResponse>>(
    "post",
    `/projects/${projectId}/modbus/snapshot`,
    { data }
  );
};
