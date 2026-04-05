/**
 * Apple Intelligence / AI classification store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { domainStatus } from "./domainStatusStore";
import type { FileClassification, ScanSummaryOutput } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const intelligenceAvailable = ref(false);
export const aiAvailable = ref(false);
export const fileClassifications = ref<Map<string, FileClassification>>(new Map());
export const scanSummary = ref<ScanSummaryOutput | null>(null);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function checkIntelligence() {
  try {
    intelligenceAvailable.value = await invoke<boolean>("check_intelligence_available");
    aiAvailable.value = await invoke<boolean>("check_ai_available");
  } catch {
    intelligenceAvailable.value = false;
    aiAvailable.value = false;
  }
}

export async function classifyFiles(files: { path: string; name: string; size: number; file_type: string; modified?: string | null }[]) {
  try {
    const results = await invoke<FileClassification[]>("classify_files_ai", { files });
    const map = new Map(fileClassifications.value);
    for (const r of results) {
      map.set(r.path, r);
    }
    fileClassifications.value = map;
  } catch {
    // Non-critical — classification is optional
  }
}

export async function generateScanSummary() {
  if (!intelligenceAvailable.value) return;
  try {
    const domains = Object.entries(domainStatus.value).map(([key, info]) => ({
      domain: key,
      item_count: info.itemCount,
      total_size: info.totalSize,
    }));
    const total = domains.reduce((s, d) => s + d.total_size, 0);
    scanSummary.value = await invoke<ScanSummaryOutput>("generate_scan_summary_ai", {
      input: { domains, total_reclaimable: total },
    });
  } catch {
    // Non-critical
  }
}
