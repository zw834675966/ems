import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type MeasurementValueDto = {
  projectId: string;
  pointId: string;
  tsMs: number;
  value: string;
  quality?: string;
};

export type MeasurementsQuery = {
  pointId: string;
  from?: number;
  to?: number;
  limit?: number;
  cursorTsMs?: number;
  order?: "asc" | "desc";
};

export const listMeasurements = (
  projectId: string,
  query: MeasurementsQuery
) => {
  return http.request<ApiResponse<MeasurementValueDto[]>>(
    "get",
    `/projects/${projectId}/measurements`,
    { params: query }
  );
};
