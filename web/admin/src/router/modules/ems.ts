import type { RouteConfigsTable } from "@/layout/types";

const Layout = () => import("@/layout/index.vue");

export default {
  path: "/ems",
  component: Layout,
  redirect: "/ems/projects",
  meta: {
    title: "能源管理",
    icon: "ep:monitor",
    rank: 10
  },
  children: [
    {
      path: "/ems/projects",
      name: "EmsProjects",
      component: () => import("@/views/ems/projects/index.vue"),
      meta: {
        title: "项目管理",
        icon: "ep:folder"
      }
    },
    {
      path: "/ems/gateways",
      name: "EmsGateways",
      component: () => import("@/views/ems/gateways/index.vue"),
      meta: {
        title: "网关管理",
        icon: "ep:connection"
      }
    },
    {
      path: "/ems/devices",
      name: "EmsDevices",
      component: () => import("@/views/ems/devices/index.vue"),
      meta: {
        title: "设备管理",
        icon: "ep:cpu"
      }
    },
    {
      path: "/ems/points",
      name: "EmsPoints",
      component: () => import("@/views/ems/points/index.vue"),
      meta: {
        title: "点位管理",
        icon: "ep:aim"
      }
    },
    {
      path: "/ems/point-mappings",
      name: "EmsPointMappings",
      component: () => import("@/views/ems/point-mappings/index.vue"),
      meta: {
        title: "点位映射",
        icon: "ep:connection"
      }
    },
    {
      path: "/ems/collection-strategies",
      name: "EmsCollectionStrategies",
      component: () => import("@/views/ems/collection-strategies/index.vue"),
      meta: {
        title: "采集策略",
        icon: "ep:timer"
      }
    },
    {
      path: "/ems/realtime",
      name: "EmsRealtime",
      component: () => import("@/views/ems/realtime/index.vue"),
      meta: {
        title: "实时监控",
        icon: "ep:data-line"
      }
    },
    {
      path: "/ems/measurements",
      name: "EmsMeasurements",
      component: () => import("@/views/ems/measurements/index.vue"),
      meta: {
        title: "历史数据",
        icon: "ep:trend-charts"
      }
    },
    {
      path: "/ems/modbus-snapshot",
      name: "EmsModbusSnapshot",
      component: () => import("@/views/ems/modbus-snapshot/index.vue"),
      meta: {
        title: "Modbus 截图/读取",
        icon: "ep:camera"
      }
    },
    {
      path: "/ems/commands",
      name: "EmsCommands",
      component: () => import("@/views/ems/commands/index.vue"),
      meta: {
        title: "指令下发",
        icon: "ep:promotion"
      }
    },
    {
      path: "/ems/audit",
      name: "EmsAudit",
      component: () => import("@/views/ems/audit/index.vue"),
      meta: {
        title: "审计日志",
        icon: "ep:document-copy"
      }
    },
    {
      path: "/ems/rbac",
      meta: {
        title: "权限管理",
        icon: "ep:lock"
      },
      children: [
        {
          path: "/ems/rbac/users",
          name: "EmsRbacUsers",
          component: () => import("@/views/ems/rbac/users/index.vue"),
          meta: {
            title: "用户管理"
          }
        },
        {
          path: "/ems/rbac/roles",
          name: "EmsRbacRoles",
          component: () => import("@/views/ems/rbac/roles/index.vue"),
          meta: {
            title: "角色管理"
          }
        }
      ]
    }
  ]
} satisfies RouteConfigsTable;
