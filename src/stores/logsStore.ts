/**
 * Logs scan store.
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { setDomain, hasFullDiskAccess, saveCache, loadCache } from "./domainStatusStore";
import type { LogEntry } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const logs = ref<LogEntry[]>([]);
export const logsScanning = ref(false);
export const logsScanned = ref(false);
export const logsError = ref("");

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------

export const totalLogSize = computed(() =>
  logs.value.reduce((sum, l) => sum + l.size, 0)
);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanLogs() {
  if (logsScanning.value) return;
  logsScanning.value = true;
  logsError.value = "";
  logs.value = [];
  logsScanned.value = false;
  setDomain("logs", { status: "scanning", error: "" });
  try {
    logs.value = await invoke<LogEntry[]>("scan_logs", {
      hasFda: hasFullDiskAccess.value === true,
    });
    logsScanned.value = true;
    void saveCache("logs", logs.value);
    setDomain("logs", {
      status: "done",
      itemCount: logs.value.length,
      totalSize: logs.value.reduce((s, l) => s + l.size, 0),
    });
  } catch (e) {
    logsError.value = String(e);
    setDomain("logs", { status: "error", error: String(e) });
  } finally {
    logsScanning.value = false;
  }
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreLogsCache(): Promise<void> {
  const cached = await loadCache<LogEntry[]>("logs");
  if (cached) {
    logs.value = cached;
    logsScanned.value = true;
    setDomain("logs", {
      status: "done",
      itemCount: cached.length,
      totalSize: cached.reduce((s, l) => s + l.size, 0),
    });
  }
}
