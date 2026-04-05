/**
 * Duplicates + Similar Images scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { hasFullDiskAccess, saveCache, loadCache } from "./domainStatusStore";
import { getDisabledPaths } from "../composables/useScanSettings";
import type { DuplicateScanResult, SimilarScanResult, SimilarScanProgress } from "../types";

// ---------------------------------------------------------------------------
// Duplicates state
// ---------------------------------------------------------------------------

export const duplicateResult = ref<DuplicateScanResult | null>(null);
export const duplicateScanning = ref(false);
export const duplicateScanned = ref(false);
export const duplicateError = ref("");

// ---------------------------------------------------------------------------
// Similar Images state
// ---------------------------------------------------------------------------

export const similarResult = ref<SimilarScanResult | null>(null);
export const similarScanning = ref(false);
export const similarScanned = ref(false);
export const similarError = ref("");
export const similarProgress = ref<SimilarScanProgress | null>(null);

// ---------------------------------------------------------------------------
// Duplicates actions
// ---------------------------------------------------------------------------

export async function scanDuplicates(path = "~", minSizeMb = 1) {
  if (duplicateScanning.value) return;
  duplicateScanning.value = true;
  duplicateError.value = "";
  duplicateResult.value = null;
  duplicateScanned.value = false;
  try {
    duplicateResult.value = await invoke<DuplicateScanResult>(
      "scan_duplicates",
      {
        path,
        minSizeMb,
        hasFda: hasFullDiskAccess.value === true,
        skipPaths: getDisabledPaths(),
      }
    );
    duplicateScanned.value = true;
    void saveCache("duplicates", duplicateResult.value);
  } catch (e) {
    duplicateError.value = String(e);
  } finally {
    duplicateScanning.value = false;
  }
}

// ---------------------------------------------------------------------------
// Similar Images actions
// ---------------------------------------------------------------------------

export async function scanSimilarImages(threshold = 10, minSizeMb = 0) {
  if (similarScanning.value) return;
  similarScanning.value = true;
  similarError.value = "";
  similarResult.value = null;
  similarScanned.value = false;
  similarProgress.value = null;

  let unlisten: UnlistenFn | null = null;
  try {
    unlisten = await listen<SimilarScanProgress>("similar-scan-progress", (event) => {
      similarProgress.value = event.payload;
    });

    similarResult.value = await invoke<SimilarScanResult>(
      "scan_similar_images",
      {
        threshold,
        minSizeMb,
        hasFda: hasFullDiskAccess.value === true,
        skipPaths: getDisabledPaths(),
      }
    );
    similarScanned.value = true;
    void saveCache("similar-images", similarResult.value);
  } catch (e) {
    similarError.value = String(e);
  } finally {
    similarScanning.value = false;
    similarProgress.value = null;
    if (unlisten) unlisten();
  }
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreDuplicatesCache(): Promise<void> {
  const cached = await loadCache<DuplicateScanResult>("duplicates");
  if (cached) {
    duplicateResult.value = cached;
    duplicateScanned.value = true;
  }
}

export async function restoreSimilarCache(): Promise<void> {
  const cached = await loadCache<SimilarScanResult>("similar-images");
  if (cached) {
    similarResult.value = cached;
    similarScanned.value = true;
  }
}
