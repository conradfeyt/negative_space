/**
 * Packages & Runtimes scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { saveCache, loadCache } from "./domainStatusStore";
import type { PackageScanResult } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const packagesResult = ref<PackageScanResult | null>(null);
export const packagesScanning = ref(false);
export const packagesScanned = ref(false);
export const packagesError = ref("");

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanPackages() {
  if (packagesScanning.value) return;
  packagesScanning.value = true;
  packagesError.value = "";
  packagesResult.value = null;
  packagesScanned.value = false;
  try {
    packagesResult.value = await invoke<PackageScanResult>("scan_packages");
    packagesScanned.value = true;
    void saveCache("packages", packagesResult.value);
  } catch (e) {
    packagesError.value = String(e);
  } finally {
    packagesScanning.value = false;
  }
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restorePackagesCache(): Promise<void> {
  const cached = await loadCache<PackageScanResult>("packages");
  if (cached) {
    packagesResult.value = cached;
    packagesScanned.value = true;
  }
}
