import { http } from "@/utils/http";
import type { ApiResponse } from "@/api/ems/types";

export type RbacUserDto = {
  userId: string;
  username: string;
  status: string;
  roles: string[];
};

export type CreateRbacUserRequest = {
  username: string;
  password: string;
  status?: string;
  roles?: string[];
};

export type UpdateRbacUserRequest = {
  password?: string;
  status?: string;
};

export type SetUserRolesRequest = {
  roles: string[];
};

export const listRbacUsers = () => {
  return http.request<ApiResponse<RbacUserDto[]>>("get", "/rbac/users");
};

export const createRbacUser = (data: CreateRbacUserRequest) => {
  return http.request<ApiResponse<RbacUserDto>>("post", "/rbac/users", { data });
};

export const updateRbacUser = (userId: string, data: UpdateRbacUserRequest) => {
  return http.request<ApiResponse<RbacUserDto>>("put", `/rbac/users/${userId}`, {
    data
  });
};

export const setRbacUserRoles = (userId: string, roles: string[]) => {
  return http.request<ApiResponse<RbacUserDto>>(
    "put",
    `/rbac/users/${userId}/roles`,
    { data: { roles } satisfies SetUserRolesRequest }
  );
};

