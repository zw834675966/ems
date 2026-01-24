import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type ProjectDto = {
  projectId: string;
  name: string;
  timezone: string;
};

export type CreateProjectRequest = {
  name: string;
  timezone?: string;
};

export const listProjects = () => {
  return http.request<ApiResponse<ProjectDto[]>>("get", "/projects");
};

export const createProject = (data: CreateProjectRequest) => {
  return http.request<ApiResponse<ProjectDto>>("post", "/projects", { data });
};

export const updateProject = (projectId: string, data: Partial<CreateProjectRequest>) => {
  return http.request<ApiResponse<ProjectDto>>("put", `/projects/${projectId}`, {
    data
  });
};

export const deleteProject = (projectId: string) => {
  return http.request<ApiResponse<void>>("delete", `/projects/${projectId}`);
};
