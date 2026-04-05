/**
 * Apps scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { setDomain, hasFullDiskAccess, saveCache, loadCache } from "./domainStatusStore";
import type { AppInfo, CleanResult } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const apps = ref<AppInfo[]>([]);
export const appsScanning = ref(false);
export const appsScanned = ref(false);
export const appsError = ref("");

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanApps() {
  if (appsScanning.value) return;
  appsScanning.value = true;
  appsError.value = "";
  apps.value = [];
  appsScanned.value = false;
  setDomain("apps", { status: "scanning", error: "" });
  try {
    apps.value = await invoke<AppInfo[]>("scan_apps", {
      hasFda: hasFullDiskAccess.value === true,
    });
    appsScanned.value = true;
    void saveCache("apps", apps.value);
    setDomain("apps", {
      status: "done",
      itemCount: apps.value.length,
      totalSize: apps.value.reduce((s, a) => s + a.footprint, 0),
    });
  } catch (e) {
    appsError.value = String(e);
    setDomain("apps", { status: "error", error: String(e) });
  } finally {
    appsScanning.value = false;
  }
}

export async function uninstallApp(
  appPath: string,
  removeLeftovers: boolean
): Promise<CleanResult> {
  return await invoke<CleanResult>("uninstall_app", {
    appPath,
    removeLeftovers,
  });
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreAppsCache(): Promise<void> {
  const cached = await loadCache<AppInfo[]>("apps");
  if (cached) {
    apps.value = cached;
    appsScanned.value = true;
    setDomain("apps", {
      status: "done",
      itemCount: cached.length,
      totalSize: cached.reduce((s, a) => s + a.footprint, 0),
    });
  }
}
