import { http } from "@/utils/http";
import type { ApiResponse } from "@/api/ems/types";

export type PermissionDto = {
  permissionCode: string;
  description: string;
};

export const listRbacPermissions = () => {
  return http.request<ApiResponse<PermissionDto[]>>("get", "/rbac/permissions");
};

