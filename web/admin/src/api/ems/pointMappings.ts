import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type PointMappingDto = {
  sourceId: string;
  projectId: string;
  pointId: string;
  sourceType: string;
  address: string;
  writable: boolean;
  scale?: number;
  offset?: number;
  /** 协议细节配置 JSON 字符串 */
  protocolDetail?: string;
};

export type CreatePointMappingRequest = {
  pointId: string;
  sourceType: string;
  address: string;
  writable?: boolean;
  scale?: number;
  offset?: number;
  protocolDetail?: string;
};

export const listPointMappings = (projectId: string) => {
  return http.request<ApiResponse<PointMappingDto[]>>(
    "get",
    `/projects/${projectId}/point-mappings`
  );
};

export const createPointMapping = (
  projectId: string,
  data: CreatePointMappingRequest
) => {
  return http.request<ApiResponse<PointMappingDto>>(
    "post",
    `/projects/${projectId}/point-mappings`,
    { data }
  );
};

export const updatePointMapping = (
  projectId: string,
  sourceId: string,
  data: Partial<CreatePointMappingRequest>
) => {
  return http.request<ApiResponse<PointMappingDto>>(
    "put",
    `/projects/${projectId}/point-mappings/${sourceId}`,
    { data }
  );
};

export const deletePointMapping = (projectId: string, sourceId: string) => {
  return http.request<ApiResponse<void>>(
    "delete",
    `/projects/${projectId}/point-mappings/${sourceId}`
  );
};
