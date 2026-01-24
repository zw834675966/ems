import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type PointDto = {
  pointId: string;
  projectId: string;
  deviceId: string;
  key: string;
  dataType: string;
  unit?: string;
  protocolDetail?: string;
};

export type CreatePointRequest = {
  deviceId: string;
  key: string;
  dataType: string;
  unit?: string;
  protocolDetail?: string;
};

export const listPoints = (projectId: string) => {
  return http.request<ApiResponse<PointDto[]>>(
    "get",
    `/projects/${projectId}/points`
  );
};

export const createPoint = (projectId: string, data: CreatePointRequest) => {
  return http.request<ApiResponse<PointDto>>(
    "post",
    `/projects/${projectId}/points`,
    { data }
  );
};
export const updatePoint = (
  projectId: string,
  pointId: string,
  data: Partial<CreatePointRequest>
) => {
  return http.request<ApiResponse<PointDto>>(
    "put",
    `/projects/${projectId}/points/${pointId}`,
    { data }
  );
};

export const deletePoint = (projectId: string, pointId: string) => {
  return http.request<ApiResponse<void>>(
    "delete",
    `/projects/${projectId}/points/${pointId}`
  );
};
