/**
 * Caches scan store.
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { setDomain, hasFullDiskAccess, saveCache, loadCache } from "./domainStatusStore";
import type { CacheEntry } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const caches = ref<CacheEntry[]>([]);
export const cachesScanning = ref(false);
export const cachesScanned = ref(false);
export const cachesError = ref("");

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------

export const totalCacheSize = computed(() =>
  caches.value.reduce((sum, c) => sum + c.size, 0)
);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanCaches() {
  if (cachesScanning.value) return;
  cachesScanning.value = true;
  cachesError.value = "";
  caches.value = [];
  cachesScanned.value = false;
  setDomain("caches", { status: "scanning", error: "" });
  try {
    caches.value = await invoke<CacheEntry[]>("scan_caches", {
      hasFda: hasFullDiskAccess.value === true,
    });
    cachesScanned.value = true;
    void saveCache("caches", caches.value);
    setDomain("caches", {
      status: "done",
      itemCount: caches.value.length,
      totalSize: caches.value.reduce((s, c) => s + c.size, 0),
    });
  } catch (e) {
    cachesError.value = String(e);
    setDomain("caches", { status: "error", error: String(e) });
  } finally {
    cachesScanning.value = false;
  }
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreCachesCache(): Promise<void> {
  const cached = await loadCache<CacheEntry[]>("caches");
  if (cached) {
    caches.value = cached;
    cachesScanned.value = true;
    setDomain("caches", {
      status: "done",
      itemCount: cached.length,
      totalSize: cached.reduce((s, c) => s + c.size, 0),
    });
  }
}
