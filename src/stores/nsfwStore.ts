/**
 * Sensitive Content (NSFW) scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { hasFullDiskAccess, saveCache, loadCache } from "./domainStatusStore";
import { getDisabledPaths } from "../composables/useScanSettings";
import type { NsfwScanResult, NsfwScanProgress } from "../types";

const EXCLUDED_LABELS_KEY = "negativ_nsfw_excluded_labels";
const LABEL_WEIGHTS_KEY = "negativ_nsfw_label_weights";

export const ALL_LABELS = [
  "FEMALE_BREAST_EXPOSED", "BUTTOCKS_EXPOSED", "FEMALE_GENITALIA_EXPOSED",
  "MALE_GENITALIA_EXPOSED", "ANUS_EXPOSED", "MALE_BREAST_EXPOSED",
  "BELLY_EXPOSED", "ARMPITS_EXPOSED", "FEMALE_GENITALIA_COVERED",
  "FEMALE_BREAST_COVERED", "BUTTOCKS_COVERED", "BELLY_COVERED",
  "ANUS_COVERED", "ARMPITS_COVERED", "FEET_COVERED", "FEET_EXPOSED",
  "FACE_FEMALE", "FACE_MALE",
] as const;

export const EXPOSED_LABELS = new Set([
  "NSFW_SCORE",
  "FEMALE_BREAST_EXPOSED", "BUTTOCKS_EXPOSED", "FEMALE_GENITALIA_EXPOSED",
  "MALE_GENITALIA_EXPOSED", "ANUS_EXPOSED", "MALE_BREAST_EXPOSED",
  "BELLY_EXPOSED", "ARMPITS_EXPOSED",
]);

function loadExcludedLabels(): Set<string> {
  try {
    const raw = localStorage.getItem(EXCLUDED_LABELS_KEY);
    if (raw) return new Set(JSON.parse(raw));
  } catch { /* ignore */ }
  return new Set();
}

function loadLabelWeights(): Record<string, number> {
  try {
    const raw = localStorage.getItem(LABEL_WEIGHTS_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return {};
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const nsfwResult = ref<NsfwScanResult | null>(null);
export const nsfwScanning = ref(false);
export const nsfwScanned = ref(false);
export const nsfwError = ref("");
export const nsfwProgress = ref<NsfwScanProgress | null>(null);
export const excludedLabels = ref<Set<string>>(loadExcludedLabels());
export const labelWeights = ref<Record<string, number>>(loadLabelWeights());

export function toggleExcludedLabel(label: string) {
  const next = new Set(excludedLabels.value);
  if (next.has(label)) next.delete(label);
  else next.add(label);
  excludedLabels.value = next;
  localStorage.setItem(EXCLUDED_LABELS_KEY, JSON.stringify([...next]));
}

export function isLabelExcluded(label: string): boolean {
  return excludedLabels.value.has(label);
}

export function setLabelWeight(label: string, weight: number) {
  const next = { ...labelWeights.value };
  if (weight === 1.0) delete next[label];
  else next[label] = weight;
  labelWeights.value = next;
  localStorage.setItem(LABEL_WEIGHTS_KEY, JSON.stringify(next));
}

export function getLabelWeight(label: string): number {
  return labelWeights.value[label] ?? 1.0;
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanNsfw(threshold = 0.5, minSizeMb = 0, path = "~") {
  if (nsfwScanning.value) return;
  nsfwScanning.value = true;
  nsfwError.value = "";
  nsfwResult.value = null;
  nsfwScanned.value = false;
  nsfwProgress.value = null;

  let unlisten: UnlistenFn | null = null;
  try {
    unlisten = await listen<NsfwScanProgress>("nsfw-scan-progress", (event) => {
      nsfwProgress.value = event.payload;
    });

    nsfwResult.value = await invoke<NsfwScanResult>("scan_nsfw", {
      path,
      threshold,
      minSizeMb,
      hasFda: hasFullDiskAccess.value === true,
      skipPaths: getDisabledPaths(),
    });
    nsfwScanned.value = true;
    void saveCache("nsfw", nsfwResult.value);
  } catch (e) {
    nsfwError.value = String(e);
  } finally {
    nsfwScanning.value = false;
    nsfwProgress.value = null;
    if (unlisten) unlisten();
  }
}

export async function stopNsfwScan() {
  try {
    await invoke("cancel_nsfw_scan");
  } catch (e) {
    console.warn("[nsfw] cancel failed:", e);
  }
}

export async function dismissNsfwPaths(paths: string[]) {
  try {
    await invoke("dismiss_nsfw_paths", { paths });
    if (nsfwResult.value) {
      nsfwResult.value = {
        ...nsfwResult.value,
        flagged: nsfwResult.value.flagged.filter((f) => !paths.includes(f.path)),
      };
      void saveCache("nsfw", nsfwResult.value);
    }
  } catch (e) {
    console.warn("[nsfw] dismiss failed:", e);
  }
}

export async function clearNsfwDismissed() {
  try {
    await invoke("clear_nsfw_dismissed");
  } catch (e) {
    console.warn("[nsfw] clear dismissed failed:", e);
  }
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreNsfwCache(): Promise<void> {
  const cached = await loadCache<NsfwScanResult>("nsfw");
  if (cached) {
    nsfwResult.value = cached;
    nsfwScanned.value = true;
  }
}
