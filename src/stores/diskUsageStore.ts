/**
 * Disk usage store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { saveCache } from "./domainStatusStore";
import type { DiskUsage } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const diskUsage = ref<DiskUsage | null>(null);
export const diskUsageLoading = ref(false);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadDiskUsage() {
  if (diskUsageLoading.value) return;
  diskUsageLoading.value = true;
  try {
    diskUsage.value = await invoke<DiskUsage>("get_disk_usage");
    void saveCache("disk-usage", diskUsage.value);
  } catch (e) {
    console.warn('[disk-usage] load failed:', e);
  } finally {
    diskUsageLoading.value = false;
  }
}
