/**
 * Disk Map (SpaceMap) store — scan, cache, enrichment.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { hasFullDiskAccess } from "./domainStatusStore";
import type { DiskMapResult, DiskNode, CacheMetadata } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const diskMapResult = ref<DiskMapResult | null>(null);
export const diskMapLoading = ref(false);
export const diskMapLoaded = ref(false);
export const diskMapError = ref("");

// Disk Map Cache
export const diskMapCaches = ref<CacheMetadata[]>([]);
export const diskMapCacheLoading = ref(false);
export const diskMapActiveCacheId = ref("");

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadDiskMap() {
  if (diskMapLoading.value) return;
  diskMapLoading.value = true;
  diskMapError.value = "";
  diskMapResult.value = null;
  diskMapLoaded.value = false;
  diskMapActiveCacheId.value = "";
  try {
    diskMapResult.value = await invoke<DiskMapResult>("get_disk_map", {
      hasFda: hasFullDiskAccess.value === true,
    });
    diskMapLoaded.value = true;
    saveDiskMapCache().catch(e => console.warn('[disk-map] cache save failed:', e));
  } catch (e) {
    diskMapError.value = String(e);
  } finally {
    diskMapLoading.value = false;
  }
}

export async function expandDiskNode(path: string): Promise<DiskNode | null> {
  try {
    return await invoke<DiskNode>("expand_disk_node", {
      path,
      hasFda: hasFullDiskAccess.value === true,
    });
  } catch (_) {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Disk Map Cache functions
// ---------------------------------------------------------------------------

export async function saveDiskMapCache(): Promise<string | null> {
  if (!diskMapResult.value) return null;
  try {
    const data = JSON.stringify(diskMapResult.value);
    const id = await invoke<string>("save_disk_map_cache", { data });
    diskMapActiveCacheId.value = id;
    await listDiskMapCaches();
    return id;
  } catch (e) {
    console.warn('[disk-map] cache save failed:', e);
    return null;
  }
}

export async function listDiskMapCaches(): Promise<CacheMetadata[]> {
  try {
    diskMapCaches.value = await invoke<CacheMetadata[]>("list_disk_map_caches");
    return diskMapCaches.value;
  } catch (e) {
    console.warn('[disk-map] list caches failed:', e);
    diskMapCaches.value = [];
    return [];
  }
}

export async function loadDiskMapCache(id: string): Promise<boolean> {
  diskMapCacheLoading.value = true;
  try {
    const json = await invoke<string>("load_disk_map_cache", { id });
    diskMapResult.value = JSON.parse(json) as DiskMapResult;
    diskMapLoaded.value = true;
    diskMapActiveCacheId.value = id;
    diskMapError.value = "";
    return true;
  } catch (e) {
    diskMapError.value = String(e);
    return false;
  } finally {
    diskMapCacheLoading.value = false;
  }
}

export async function deleteDiskMapCache(id: string): Promise<boolean> {
  try {
    await invoke<void>("delete_disk_map_cache", { id });
    if (diskMapActiveCacheId.value === id) {
      diskMapActiveCacheId.value = "";
    }
    await listDiskMapCaches();
    return true;
  } catch (e) {
    console.warn('[disk-map] delete cache failed:', e);
    return false;
  }
}

export async function loadMostRecentCache(): Promise<boolean> {
  try {
    const caches = await listDiskMapCaches();
    if (caches.length === 0) return false;
    return await loadDiskMapCache(caches[0].id);
  } catch (e) {
    console.warn('[disk-map] load most recent cache failed:', e);
    return false;
  }
}

// ---------------------------------------------------------------------------
// Disk Map Recency Enrichment
// ---------------------------------------------------------------------------

export async function enrichDiskNodes(batchSize = 50): Promise<void> {
  if (!diskMapResult.value) return;

  const paths: string[] = [];
  function collectPaths(node: DiskNode) {
    if (node.path) paths.push(node.path);
    for (const child of node.children) {
      collectPaths(child);
    }
  }
  collectPaths(diskMapResult.value.root);

  for (let i = 0; i < paths.length; i += batchSize) {
    const batch = paths.slice(i, i + batchSize);
    try {
      const result = await invoke<Record<string, number>>("enrich_disk_nodes", {
        paths: batch,
      });

      function patchNode(node: DiskNode) {
        if (node.path && result[node.path] !== undefined) {
          node.modified = result[node.path];
        }
        for (const child of node.children) {
          patchNode(child);
        }
      }
      patchNode(diskMapResult.value!.root);
    } catch (e) {
      console.warn('[disk-map] recency enrichment failed:', e);
    }
  }
}
