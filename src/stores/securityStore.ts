/**
 * Security scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { setDomain, saveCache, loadCache } from "./domainStatusStore";
import type { SecurityScanResult, CleanResult } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const securityResult = ref<SecurityScanResult | null>(null);
export const securityScanning = ref(false);
export const securityScanned = ref(false);
export const securityError = ref("");

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanSecurity() {
  if (securityScanning.value) return;
  securityScanning.value = true;
  securityError.value = "";
  securityResult.value = null;
  securityScanned.value = false;
  setDomain("security", { status: "scanning", error: "" });
  try {
    securityResult.value = await invoke<SecurityScanResult>("scan_security");
    securityScanned.value = true;
    void saveCache("security", securityResult.value);
    setDomain("security", {
      status: "done",
      itemCount: securityResult.value?.summary?.total_findings ?? 0,
      totalSize: 0,
    });
  } catch (e) {
    securityError.value = String(e);
    setDomain("security", { status: "error", error: String(e) });
  } finally {
    securityScanning.value = false;
  }
}

export async function disableLaunchItem(path: string): Promise<CleanResult> {
  return await invoke<CleanResult>("disable_launch_item", { path });
}

export async function removeLaunchItem(path: string): Promise<CleanResult> {
  return await invoke<CleanResult>("remove_launch_item", { path });
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreSecurityCache(): Promise<void> {
  const cached = await loadCache<SecurityScanResult>("security");
  if (cached) {
    securityResult.value = cached;
    securityScanned.value = true;
  }
}
