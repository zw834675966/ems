import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type AuditLogDto = {
  auditId: string;
  projectId?: string;
  actor: string;
  action: string;
  resource: string;
  result: string;
  detail?: string;
  tsMs: number;
};

export type AuditLogQuery = {
  from?: number;
  to?: number;
  limit?: number;
};

export const listAuditLogs = (projectId: string, query: AuditLogQuery) => {
  return http.request<ApiResponse<AuditLogDto[]>>(
    "get",
    `/projects/${projectId}/audit`,
    { params: query }
  );
};

