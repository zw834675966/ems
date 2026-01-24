import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type DeviceDto = {
  deviceId: string;
  projectId: string;
  gatewayId: string;
  name: string;
  model?: string;
  online?: boolean;
  lastSeenAtMs?: number;
  lastError?: string;
  /** 设备所在房间 ID */
  roomId?: string;
  /** 协议地址配置 JSON 字符串 */
  addressConfig?: string;
};

export type CreateDeviceRequest = {
  gatewayId: string;
  name: string;
  model?: string;
  roomId?: string;
  addressConfig?: string;
};

export const listDevices = (projectId: string) => {
  return http.request<ApiResponse<DeviceDto[]>>(
    "get",
    `/projects/${projectId}/devices`
  );
};

export const createDevice = (projectId: string, data: CreateDeviceRequest) => {
  return http.request<ApiResponse<DeviceDto>>(
    "post",
    `/projects/${projectId}/devices`,
    { data }
  );
};
export const updateDevice = (
  projectId: string,
  deviceId: string,
  data: Partial<CreateDeviceRequest>
) => {
  return http.request<ApiResponse<DeviceDto>>(
    "put",
    `/projects/${projectId}/devices/${deviceId}`,
    { data }
  );
};

export const deleteDevice = (projectId: string, deviceId: string) => {
  return http.request<ApiResponse<void>>(
    "delete",
    `/projects/${projectId}/devices/${deviceId}`
  );
};
