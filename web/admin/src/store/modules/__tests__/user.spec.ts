import { beforeEach, describe, expect, it, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";

const mocks = vi.hoisted(() => ({
  routerPush: vi.fn(),
  resetRouter: vi.fn(),
  setToken: vi.fn(),
  removeToken: vi.fn()
}));

vi.mock("../../utils", () => ({
  store: {},
  router: { push: mocks.routerPush },
  resetRouter: mocks.resetRouter,
  routerArrays: [],
  storageLocal: () => ({
    getItem: vi.fn().mockReturnValue(null)
  })
}));

vi.mock("../multiTags", () => ({
  useMultiTagsStoreHook: () => ({
    handleTags: vi.fn(),
    multiTags: []
  })
}));

const getLogin = vi.fn();
const refreshTokenApi = vi.fn();

vi.mock("@/api/user", () => ({
  getLogin: (...args: any[]) => getLogin(...args),
  refreshTokenApi: (...args: any[]) => refreshTokenApi(...args)
}));

vi.mock("@/utils/auth", () => ({
  setToken: (...args: any[]) => mocks.setToken(...args),
  removeToken: (...args: any[]) => mocks.removeToken(...args),
  userKey: "ems-user"
}));

import { useUserStore } from "../user";

describe("useUserStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it("logs in and stores token on success", async () => {
    getLogin.mockResolvedValue({
      success: true,
      data: {
        accessToken: "access",
        refreshToken: "refresh",
        refreshJti: "jti",
        expiresAt: 123
      }
    });
    const store = useUserStore();
    await store.loginByUsername({ username: "admin", password: "admin123" });
    expect(mocks.setToken).toHaveBeenCalledWith(
      expect.objectContaining({ accessToken: "access" })
    );
  });

  it("rejects login when API fails", async () => {
    getLogin.mockRejectedValue(new Error("boom"));
    const store = useUserStore();
    await expect(
      store.loginByUsername({ username: "admin", password: "bad" })
    ).rejects.toThrow("boom");
  });

  it("clears state on logout", () => {
    const store = useUserStore();
    store.username = "admin";
    store.roles = ["admin"];
    store.permissions = ["project.read"];
    store.logOut();
    expect(store.username).toBe("");
    expect(store.roles).toEqual([]);
    expect(store.permissions).toEqual([]);
    expect(mocks.removeToken).toHaveBeenCalled();
    expect(mocks.resetRouter).toHaveBeenCalled();
    expect(mocks.routerPush).toHaveBeenCalledWith("/login");
  });
});
