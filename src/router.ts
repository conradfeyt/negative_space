import { createRouter, createWebHashHistory } from "vue-router";
import Dashboard from "./views/Dashboard.vue";
import LargeFiles from "./views/LargeFiles.vue";
import Caches from "./views/Caches.vue";
import Logs from "./views/Logs.vue";
import Docker from "./views/Docker.vue";
import Apps from "./views/Apps.vue";
import Trash from "./views/Trash.vue";
import Browsers from "./views/Browsers.vue";
import Duplicates from "./views/Duplicates.vue";
import Vault from "./views/Vault.vue";
import SpaceMap from "./views/SpaceMap.vue";
import Cpu from "./views/Cpu.vue";
import Memory from "./views/Memory.vue";
import Packages from "./views/Packages.vue";
import Thermal from "./views/Thermal.vue";
import Maintenance from "./views/Maintenance.vue";
import Security from "./views/Security.vue";
import Settings from "./views/Settings.vue";
import IconTest from "./views/IconTest.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/dashboard" },
    { path: "/dashboard", name: "dashboard", component: Dashboard },
    { path: "/large-files", name: "large-files", component: LargeFiles },
    { path: "/caches", name: "caches", component: Caches },
    { path: "/logs", name: "logs", component: Logs },
    { path: "/docker", name: "docker", component: Docker },
    { path: "/apps", name: "apps", component: Apps },
    { path: "/trash", name: "trash", component: Trash },
    { path: "/browsers", name: "browsers", component: Browsers },
    { path: "/duplicates", name: "duplicates", component: Duplicates },
    { path: "/vault", name: "vault", component: Vault },
    { path: "/space-map", name: "space-map", component: SpaceMap },
    { path: "/cpu", name: "cpu", component: Cpu },
    { path: "/memory", name: "memory", component: Memory },
    { path: "/packages", name: "packages", component: Packages },
    { path: "/thermal", name: "thermal", component: Thermal },
    { path: "/maintenance", name: "maintenance", component: Maintenance },
    { path: "/security", name: "security", component: Security },
    { path: "/settings", name: "settings", component: Settings },
    { path: "/icon-test", name: "icon-test", component: IconTest },
  ],
});

export default router;
