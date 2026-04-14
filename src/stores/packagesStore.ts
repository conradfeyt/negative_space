/**
 * Packages & Runtimes scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { saveCache, loadCache } from "./domainStatusStore";
import type { PackageScanResult, CustomProbe, CommandRecord } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const packagesResult = ref<PackageScanResult | null>(null);
export const packagesScanning = ref(false);
export const packagesScanned = ref(false);
export const packagesError = ref("");

// Custom probes
export const customProbes = ref<CustomProbe[]>([]);
export const customProbesLoaded = ref(false);

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
// Custom probe CRUD
// ---------------------------------------------------------------------------

export async function loadCustomProbes(): Promise<void> {
  try {
    customProbes.value = await invoke<CustomProbe[]>("get_custom_probes");
    customProbesLoaded.value = true;
  } catch (e) {
    console.warn("[packages] Failed to load custom probes:", e);
  }
}

export async function saveCustomProbes(probes: CustomProbe[]): Promise<void> {
  await invoke("save_custom_probes", { probes });
  customProbes.value = probes;
}

export async function deleteCustomProbe(id: string): Promise<void> {
  await invoke("delete_custom_probe", { id });
  customProbes.value = customProbes.value.filter((p) => p.id !== id);
}

export async function testProbeCommand(
  program: string,
  args: string[]
): Promise<CommandRecord> {
  return invoke<CommandRecord>("test_probe_command", { program, args });
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
