import { http } from "@/utils/http";

// DTO compatible interfaces
export interface SystemLogDto {
  logId: string;
  tenantId: string;
  projectId?: string;
  category: "operation" | "error" | "warning";
  level: "info" | "warn" | "error";
  title: string;
  message: string;
  source?: string;
  resource?: string;
  actor?: string;
  metadata?: any;
  isRead: boolean;
  createdAtMs: number;
}

export interface UnreadStatsDto {
  operation: number;
  error: number;
  warning: number;
  total: number;
}

export interface SystemLogQuery {
  from?: number;
  to?: number;
  category?: "operation" | "error" | "warning";
  level?: "info" | "warn" | "error";
  unreadOnly?: boolean;
  limit?: number;
}

export const useSystemLogsApi = (projectId: string) => {
  return {
    list: (params?: SystemLogQuery) => {
      // 映射前端 query 到后端要求的字段
      // 注意: DTO 中是 camelCase (fromMs, toMs)，但 SystemLogQuery 定义通常是 query param
      // 查看后端定义: pub struct SystemLogQuery { pub from: Option<i64>, ... }
      // 前端这里定义的 SystemLogQuery 属性名应该与后端 Deserialize 的结构一致
      // 后端: #[serde(rename_all = "camelCase")] pub struct SystemLogQuery { pub from: ... }
      // 所以前端传 params 应该是 { from: ..., to: ... }
      return http.request<SystemLogDto[]>("get", `/projects/${projectId}/system-logs`, { params });
    },
    getUnreadCount: (category?: "operation" | "error" | "warning") => {
      return http.request<UnreadStatsDto>("get", `/projects/${projectId}/system-logs/unread`, { params: { category } });
    },
    markAsRead: (logIds: string[]) => {
      return http.request<number>("post", `/projects/${projectId}/system-logs/read`, { data: { logIds } });
    }
  };
};
