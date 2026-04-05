/**
 * Browser scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { setDomain, hasFullDiskAccess, saveCache, loadCache } from "./domainStatusStore";
import type { BrowserScanResult, BrowserCleanResult } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const browserResult = ref<BrowserScanResult | null>(null);
export const browserScanning = ref(false);
export const browserScanned = ref(false);
export const browserError = ref("");

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanBrowsers() {
  if (browserScanning.value) return;
  browserScanning.value = true;
  browserError.value = "";
  browserResult.value = null;
  browserScanned.value = false;
  setDomain("browsers", { status: "scanning", error: "" });
  try {
    browserResult.value = await invoke<BrowserScanResult>("scan_browsers", {
      hasFda: hasFullDiskAccess.value === true,
    });
    browserScanned.value = true;
    void saveCache("browsers", browserResult.value);
    const totalSize = browserResult.value?.browsers?.reduce(
      (sum, b) => sum + b.total_size, 0
    ) ?? 0;
    const itemCount = browserResult.value?.browsers?.length ?? 0;
    setDomain("browsers", { status: "done", itemCount, totalSize });
  } catch (e) {
    browserError.value = String(e);
    setDomain("browsers", { status: "error", error: String(e) });
  } finally {
    browserScanning.value = false;
  }
}

export async function cleanBrowserData(
  paths: string[]
): Promise<BrowserCleanResult> {
  return await invoke<BrowserCleanResult>("clean_browser_data", { paths });
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreBrowsersCache(): Promise<void> {
  const cached = await loadCache<BrowserScanResult>("browsers");
  if (cached) {
    browserResult.value = cached;
    browserScanned.value = true;
    setDomain("browsers", {
      status: "done",
      itemCount: cached.browsers?.length ?? 0,
      totalSize: cached.total_size ?? 0,
    });
  }
}
