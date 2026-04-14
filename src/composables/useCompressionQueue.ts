/**
 * Composable that manages a compression queue for the Archive view.
 *
 * Handles staging folders/files, calculating sizes, and executing
 * compression in sequence. Store actions are injected as parameters
 * to keep the composable testable.
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { CompressResult } from "../types";
import { formatSize } from "../utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface QueueItem {
  path: string;
  name: string;
  size: number;
  isDirectory: boolean;
  calculating: boolean;
}

export interface CompressProgress {
  current: number;
  total: number;
  name: string;
}

export interface UseCompressionQueueOptions {
  compressDirectoryToArchive: (path: string) => Promise<CompressResult>;
  compressToArchive: (paths: string[]) => Promise<CompressResult>;
  onError: (message: string) => void;
  onSuccess: (message: string) => void;
  compressing: { value: boolean };
}

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export function useCompressionQueue(opts: UseCompressionQueueOptions) {
  const queue = ref<QueueItem[]>([]);
  const compressProgress = ref<CompressProgress | null>(null);

  const totalQueueSize = computed(() =>
    queue.value.reduce((sum, q) => sum + q.size, 0)
  );

  async function addFolderToQueue() {
    const folder = await open({
      directory: true,
      multiple: false,
      title: "Choose a folder to add to compression queue",
    });
    if (!folder || typeof folder !== "string") return;

    if (queue.value.some(q => q.path === folder)) {
      opts.onSuccess("This folder is already in the queue");
      return;
    }

    const name = folder.split("/").pop() || folder;
    const item: QueueItem = { path: folder, name, size: 0, isDirectory: true, calculating: true };
    queue.value.push(item);

    try {
      const size = await invoke<number>("get_directory_size", { path: folder });
      const idx = queue.value.findIndex(q => q.path === folder);
      if (idx !== -1) {
        queue.value[idx].size = size;
        queue.value[idx].calculating = false;
      }
    } catch {
      const idx = queue.value.findIndex(q => q.path === folder);
      if (idx !== -1) queue.value[idx].calculating = false;
    }
  }

  function removeFromQueue(path: string) {
    queue.value = queue.value.filter(q => q.path !== path);
  }

  function clearQueue() {
    queue.value = [];
  }

  async function compressQueue() {
    if (queue.value.length === 0) return;

    const items = [...queue.value];
    let totalSavings = 0;
    let compressed = 0;
    const errors: string[] = [];

    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      compressProgress.value = { current: i + 1, total: items.length, name: item.name };

      try {
        if (item.isDirectory) {
          const result = await opts.compressDirectoryToArchive(item.path);
          if (result.success || result.files_compressed > 0) {
            totalSavings += result.total_savings;
            compressed++;
            queue.value = queue.value.filter(q => q.path !== item.path);
          }
          if (result.errors.length) errors.push(...result.errors);
        } else {
          const result = await opts.compressToArchive([item.path]);
          if (result.success || result.files_compressed > 0) {
            totalSavings += result.total_savings;
            compressed++;
            queue.value = queue.value.filter(q => q.path !== item.path);
          }
          if (result.errors.length) errors.push(...result.errors);
        }
      } catch (e) {
        console.warn('[archive] compress failed for', item.path, e);
        errors.push(`${item.name}: ${String(e)}`);
      }
    }

    compressProgress.value = null;

    if (compressed > 0) {
      opts.onSuccess(`Archived ${compressed} item(s), saved ${formatSize(totalSavings)}`);
    }
    if (errors.length > 0) {
      opts.onError(errors.join("; "));
    }
  }

  return {
    queue,
    compressProgress,
    totalQueueSize,
    addFolderToQueue,
    removeFromQueue,
    clearQueue,
    compressQueue,
  };
}
