import { defineFakeRoute } from "vite-plugin-fake-server/client";

// 模拟刷新token接口
export default defineFakeRoute([
  {
    url: "/refresh-token",
    method: "post",
    response: ({ body }) => {
      if (body.refreshToken) {
        return {
          success: true,
          data: {
            accessToken: "eyJhbGciOiJIUzUxMiJ9.newAdmin",
            refreshToken: "eyJhbGciOiJIUzUxMiJ9.newAdminRefresh",
            // `expires` 使用 Unix ms 时间戳，方便与后端保持一致（每次都应该递增）。
            expires: Date.UTC(2030, 9, 30, 23, 59, 59)
          }
        };
      } else {
        return {
          success: false,
          data: {}
        };
      }
    }
  }
]);
