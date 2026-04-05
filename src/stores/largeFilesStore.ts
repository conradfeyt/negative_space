/**
 * Large files scan store.
 *
 * Streaming large-file scan — files appear in the UI immediately as discovered.
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { fileDiskSize } from "../utils";
import { getDisabledPaths } from "../composables/useScanSettings";
import { setDomain, hasFullDiskAccess, saveCache, loadCache } from "./domainStatusStore";
import type {
  FileInfo,
  LargeFileFound,
  LargeFileScanProgress,
  LargeFileScanDone,
} from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const largeFiles = ref<FileInfo[]>([]);
export const largeFilesScanning = ref(false);
export const largeFilesScanned = ref(false);
export const largeFilesError = ref("");
export const largeFilesSkipped = ref<string[]>([]);
export const largeFilesCurrentDir = ref("");

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------

export const totalLargeFileSize = computed(() =>
  largeFiles.value.reduce((sum, f) => sum + fileDiskSize(f), 0)
);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function scanLargeFiles(path = "~", minSizeMb = 100) {
  if (largeFilesScanning.value) return;
  largeFilesScanning.value = true;
  largeFilesError.value = "";
  largeFiles.value = [];
  largeFilesSkipped.value = [];
  largeFilesScanned.value = false;
  largeFilesCurrentDir.value = "";
  setDomain("largeFiles", { status: "scanning", error: "" });

  const unlisteners: UnlistenFn[] = [];

  try {
    unlisteners.push(
      await listen<LargeFileFound>("large-file-found", (event) => {
        largeFiles.value = [...largeFiles.value, event.payload.file];
      })
    );

    unlisteners.push(
      await listen<LargeFileScanProgress>("large-file-progress", (event) => {
        largeFilesCurrentDir.value = event.payload.current_dir;
      })
    );

    unlisteners.push(
      await listen<LargeFileScanDone>("large-file-done", (event) => {
        largeFilesSkipped.value = event.payload.skipped_paths;
        largeFilesScanned.value = true;
        largeFilesCurrentDir.value = "";
        void saveCache("large-files", largeFiles.value);

        const files = largeFiles.value;
        setDomain("largeFiles", {
          status: "done",
          itemCount: files.length,
          totalSize: files.reduce((s, f) => s + fileDiskSize(f), 0),
        });
      })
    );

    await invoke<void>("scan_large_files_stream", {
      path,
      minSizeMb,
      skipPaths: getDisabledPaths(),
      hasFda: hasFullDiskAccess.value === true,
    });
  } catch (e) {
    largeFilesError.value = String(e);
    setDomain("largeFiles", { status: "error", error: String(e) });
  } finally {
    for (const unlisten of unlisteners) {
      unlisten();
    }
    largeFilesScanning.value = false;
    largeFilesCurrentDir.value = "";
  }
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreLargeFilesCache(): Promise<void> {
  const cached = await loadCache<FileInfo[]>("large-files");
  if (cached) {
    largeFiles.value = cached;
    largeFilesScanned.value = true;
    setDomain("largeFiles", {
      status: "done",
      itemCount: cached.length,
      totalSize: cached.reduce((s, f) => s + fileDiskSize(f), 0),
    });
  }
}
