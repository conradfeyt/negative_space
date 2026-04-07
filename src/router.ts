import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/dashboard" },
  { path: "/dashboard", name: "dashboard", component: () => import("./views/Dashboard.vue") },
  { path: "/large-files", name: "large-files", component: () => import("./views/LargeFiles.vue") },
  { path: "/caches", name: "caches", component: () => import("./views/Caches.vue") },
  { path: "/logs", name: "logs", component: () => import("./views/Logs.vue") },
  { path: "/docker", name: "docker", component: () => import("./views/Docker.vue") },
  { path: "/apps", name: "apps", component: () => import("./views/Apps.vue") },
  { path: "/trash", name: "trash", component: () => import("./views/Trash.vue") },
  { path: "/browsers", name: "browsers", component: () => import("./views/Browsers.vue") },
  { path: "/duplicates", name: "duplicates", component: () => import("./views/Duplicates.vue") },
  { path: "/vault", name: "vault", component: () => import("./views/Vault.vue") },
  { path: "/space-map", name: "space-map", component: () => import("./views/SpaceMap.vue") },
  { path: "/cpu", name: "cpu", component: () => import("./views/Cpu.vue") },
  { path: "/memory", name: "memory", component: () => import("./views/Memory.vue") },
  { path: "/packages", name: "packages", component: () => import("./views/Packages.vue") },
  { path: "/thermal", name: "thermal", component: () => import("./views/Thermal.vue") },
  { path: "/maintenance", name: "maintenance", component: () => import("./views/Maintenance.vue") },
  { path: "/security", name: "security", component: () => import("./views/Security.vue") },
  { path: "/settings", name: "settings", component: () => import("./views/Settings.vue") },
];

routes.push({ path: "/icon-test", name: "icon-test", component: () => import("./views/IconTest.vue") });
routes.push({ path: "/showcase", name: "showcase", component: () => import("./views/Showcase.vue") });

routes.push({ path: "/:pathMatch(.*)*", redirect: "/dashboard" });

const router = createRouter({
  history: createWebHashHistory(),
  scrollBehavior(_to, _from, savedPosition) {
    return savedPosition || { top: 0 };
  },
  routes,
});

export default router;
