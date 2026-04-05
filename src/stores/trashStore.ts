/**
 * Trash scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { setDomain, saveCache, loadCache } from "./domainStatusStore";
import type { TrashInfo, CleanResult } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const trashInfo = ref<TrashInfo | null>(null);
export const trashLoading = ref(false);
export const trashError = ref("");

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadTrashInfo() {
  if (trashLoading.value) return;
  trashLoading.value = true;
  trashError.value = "";
  setDomain("trash", { status: "scanning", error: "" });
  try {
    trashInfo.value = await invoke<TrashInfo>("get_trash_info");
    void saveCache("trash", trashInfo.value);
    setDomain("trash", {
      status: "done",
      itemCount: trashInfo.value?.item_count ?? 0,
      totalSize: trashInfo.value?.size ?? 0,
    });
  } catch (e) {
    trashError.value = String(e);
    setDomain("trash", { status: "error", error: String(e) });
  } finally {
    trashLoading.value = false;
  }
}

export async function emptyTrash(): Promise<CleanResult> {
  const result = await invoke<CleanResult>("empty_trash");
  await loadTrashInfo();
  return result;
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreTrashCache(): Promise<void> {
  const cached = await loadCache<TrashInfo>("trash");
  if (cached) {
    trashInfo.value = cached;
    setDomain("trash", {
      status: "done",
      itemCount: cached.item_count,
      totalSize: cached.size,
    });
  }
}
