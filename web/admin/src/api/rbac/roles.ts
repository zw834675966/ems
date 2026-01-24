import { http } from "@/utils/http";
import type { ApiResponse } from "@/api/ems/types";

export type RbacRoleDto = {
  roleCode: string;
  name: string;
  permissions: string[];
};

export type CreateRbacRoleRequest = {
  roleCode: string;
  name: string;
  permissions?: string[];
};

export type SetRolePermissionsRequest = {
  permissions: string[];
};

export const listRbacRoles = () => {
  return http.request<ApiResponse<RbacRoleDto[]>>("get", "/rbac/roles");
};

export const createRbacRole = (data: CreateRbacRoleRequest) => {
  return http.request<ApiResponse<RbacRoleDto>>("post", "/rbac/roles", { data });
};

export const deleteRbacRole = (roleCode: string) => {
  return http.request<ApiResponse<null>>("delete", `/rbac/roles/${roleCode}`);
};

export const setRbacRolePermissions = (roleCode: string, permissions: string[]) => {
  return http.request<ApiResponse<RbacRoleDto>>(
    "put",
    `/rbac/roles/${roleCode}/permissions`,
    { data: { permissions } satisfies SetRolePermissionsRequest }
  );
};

