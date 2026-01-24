import { beforeEach, describe, expect, it, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";

vi.mock("../../utils", () => ({
  store: {},
  ascending: (routes: any[]) => routes,
  filterTree: (routes: any[]) => routes,
  filterNoPermissionTree: (routes: any[]) => routes,
  formatFlatteningRoutes: (routes: any[]) => routes,
  getKeyList: (list: any[], key: string) => list.map(item => item[key]),
  constantMenus: [{ path: "/static", name: "static" }]
}));

vi.mock("../multiTags", () => ({
  useMultiTagsStoreHook: () => ({
    multiTags: [{ name: "cached" }]
  })
}));

import { usePermissionStore } from "../permission";

describe("usePermissionStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("builds whole menus and flattening routes", () => {
    const store = usePermissionStore();
    store.handleWholeMenus([{ path: "/dynamic", name: "dynamic" }]);
    expect(store.wholeMenus).toHaveLength(2);
    expect(store.flatteningRoutes).toHaveLength(2);
  });

  it("manages cache list and clears stale entries", () => {
    const store = usePermissionStore();
    store.cacheOperate({ mode: "add", name: "cached" });
    store.cacheOperate({ mode: "add", name: "stale" });
    expect(store.cachePageList).toEqual(["cached", "stale"]);

    store.cacheOperate({ mode: "delete", name: "stale" });
    expect(store.cachePageList).toEqual(["cached"]);
  });
});
