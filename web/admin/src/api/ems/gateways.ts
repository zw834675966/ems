import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type GatewayDto = {
  gatewayId: string;
  projectId: string;
  name: string;
  status: string;
  online?: boolean;
  lastSeenAtMs?: number;
  /** 协议类型: mqtt | modbus_tcp | tcp_server | tcp_client */
  protocolType: string;
  /** 协议配置 JSON 字符串 */
  protocolConfig?: string;
  lastError?: string;
};

export type TestGatewayResponse = {
  success: boolean;
  latencyMs?: number;
  error?: string;
  testedAtMs: number;
};

export type CreateGatewayRequest = {
  name: string;
  status?: string;
  protocolType?: string;
  protocolConfig?: string;
};

export const listGateways = (projectId: string) => {
  return http.request<ApiResponse<GatewayDto[]>>(
    "get",
    `/projects/${projectId}/gateways`
  );
};

export const createGateway = (
  projectId: string,
  data: CreateGatewayRequest
) => {
  return http.request<ApiResponse<GatewayDto>>(
    "post",
    `/projects/${projectId}/gateways`,
    { data }
  );
};
export const updateGateway = (
  projectId: string,
  gatewayId: string,
  data: Partial<CreateGatewayRequest>
) => {
  return http.request<ApiResponse<GatewayDto>>(
    "put",
    `/projects/${projectId}/gateways/${gatewayId}`,
    { data }
  );
};

export const deleteGateway = (projectId: string, gatewayId: string) => {
  return http.request<ApiResponse<void>>(
    "delete",
    `/projects/${projectId}/gateways/${gatewayId}`
  );
};

export const testGateway = (projectId: string, gatewayId: string) => {
  return http.request<ApiResponse<TestGatewayResponse>>(
    "post",
    `/projects/${projectId}/gateways/${gatewayId}/test`
  );
};
