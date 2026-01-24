import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type RealtimeValueDto = {
  projectId: string;
  pointId: string;
  tsMs: number;
  value: string;
  quality?: string;
};

export const getRealtime = (projectId: string, pointId: string) => {
  return http.request<ApiResponse<RealtimeValueDto[]>>(
    "get",
    `/projects/${projectId}/realtime`,
    { params: { pointId } }
  );
};
