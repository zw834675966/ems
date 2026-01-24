import { http } from "@/utils/http";

export type AsyncRouteMeta = {
  title: string;
  icon: string;
  rank: number;
  roles?: string[];
  auths?: string[];
};

export type AsyncRoute = {
  path: string;
  name: string;
  component: string;
  meta: AsyncRouteMeta;
  children?: AsyncRoute[];
};

type Result = {
  success: boolean;
  data: AsyncRoute[];
};

export const getAsyncRoutes = () => {
  return http.request<Result>("get", "/get-async-routes");
};
