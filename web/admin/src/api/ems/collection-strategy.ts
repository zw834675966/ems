import { http } from "@/utils/http";
import type { ApiResponse } from "./types";
import type { PointDto } from "./points";

// ============================================================================
// 采集策略相关类型定义
// ============================================================================

/** 采集策略 DTO */
export type CollectionStrategyDto = {
  strategyId: string;
  projectId: string;
  pointId: string;
  enabled: boolean;
  intervalValue: number;
  intervalUnit: "ms" | "s" | "min";
  writeToDb: boolean;
  lastCollectedAtMs?: number;
  lastValue?: string;
  lastError?: string;
};

/** 创建/更新采集策略请求 */
export type UpsertStrategyRequest = {
  pointId: string;
  enabled: boolean;
  intervalValue: number;
  intervalUnit: "ms" | "s" | "min";
  writeToDb: boolean;
};

/** 批量创建/更新采集策略请求 */
export type BatchUpsertStrategiesRequest = {
  strategies: UpsertStrategyRequest[];
};

/** 批量更新启用状态请求 */
export type BatchUpdateEnabledRequest = {
  strategyIds: string[];
  enabled: boolean;
};

/** 点位测试结果 */
export type PointTestResultDto = {
  success: boolean;
  pointId: string;
  value?: string;
  error?: string;
  latencyMs: number;
};

// ============================================================================
// API 函数
// ============================================================================

/**
 * 批量获取多项目点位
 * @param projectIds 项目 ID 列表（逗号分隔）
 */
export const listPointsBatch = (projectIds: string[]) => {
  return http.request<ApiResponse<PointDto[]>>("get", `/points/batch`, {
    params: { projectIds: projectIds.join(",") }
  });
};

/**
 * 列出项目的采集策略
 */
export const listStrategies = (projectId: string) => {
  return http.request<ApiResponse<CollectionStrategyDto[]>>(
    "get",
    `/projects/${projectId}/collection-strategies`
  );
};

/**
 * 批量创建/更新采集策略
 */
export const upsertStrategies = (
  projectId: string,
  data: BatchUpsertStrategiesRequest
) => {
  return http.request<ApiResponse<CollectionStrategyDto[]>>(
    "post",
    `/projects/${projectId}/collection-strategies`,
    { data }
  );
};

/**
 * 删除采集策略
 */
export const deleteStrategy = (projectId: string, strategyId: string) => {
  return http.request<ApiResponse<void>>(
    "delete",
    `/projects/${projectId}/collection-strategies/${strategyId}`
  );
};

/**
 * 批量更新启用状态
 */
export const batchUpdateEnabled = (
  projectId: string,
  data: BatchUpdateEnabledRequest
) => {
  return http.request<ApiResponse<{ updated: number }>>(
    "post",
    `/projects/${projectId}/collection-strategies/batch-enabled`,
    { data }
  );
};

/**
 * 测试点位连接
 */
export const testPoint = (projectId: string, pointId: string) => {
  return http.request<ApiResponse<PointTestResultDto>>(
    "post",
    `/projects/${projectId}/points/${pointId}/test`
  );
};
