import { http } from "@/utils/http";
import type { ApiResponse } from "./types";

export type CommandDto = {
  commandId: string;
  projectId: string;
  target: string;
  payload: unknown;
  status: string;
  issuedBy: string;
  issuedAtMs: number;
};

export type CommandReceiptDto = {
  receiptId: string;
  commandId: string;
  projectId: string;
  status: string;
  message?: string;
  tsMs: number;
};

export type CreateCommandRequest = {
  target: string;
  payload: unknown;
};

export const listCommands = (projectId: string, limit?: number) => {
  return http.request<ApiResponse<CommandDto[]>>(
    "get",
    `/projects/${projectId}/commands`,
    { params: { limit } }
  );
};

export const createCommand = (projectId: string, data: CreateCommandRequest) => {
  return http.request<ApiResponse<CommandDto>>(
    "post",
    `/projects/${projectId}/commands`,
    { data }
  );
};

export const listCommandReceipts = (projectId: string, commandId: string) => {
  return http.request<ApiResponse<CommandReceiptDto[]>>(
    "get",
    `/projects/${projectId}/commands/${commandId}/receipts`
  );
};

