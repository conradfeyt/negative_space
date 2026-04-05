/**
 * Docker scan store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { setDomain, saveCache, loadCache } from "./domainStatusStore";
import type { DockerInfo, CleanResult } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const dockerInstalled = ref<boolean | null>(null);
export const dockerInfo = ref<DockerInfo | null>(null);
export const dockerLoading = ref(false);
export const dockerError = ref("");

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function checkDockerInstalled() {
  try {
    dockerInstalled.value = await invoke<boolean>("is_docker_installed");
  } catch {
    dockerInstalled.value = false;
  }
}

export async function loadDockerInfo() {
  if (dockerLoading.value) return;
  dockerLoading.value = true;
  dockerError.value = "";
  setDomain("docker", { status: "scanning", error: "" });
  try {
    dockerInfo.value = await invoke<DockerInfo>("get_docker_info");
    void saveCache("docker", dockerInfo.value);
    const reclaimStr = dockerInfo.value?.total_reclaimable ?? "0";
    const reclaimBytes = parseFloat(reclaimStr) || 0;
    setDomain("docker", {
      status: "done",
      itemCount: dockerInfo.value?.images?.length ?? 0,
      totalSize: reclaimBytes,
    });
  } catch (e) {
    dockerError.value = String(e);
    setDomain("docker", { status: "error", error: String(e) });
  } finally {
    dockerLoading.value = false;
  }
}

export async function cleanDocker(pruneAll: boolean): Promise<CleanResult> {
  return await invoke<CleanResult>("clean_docker", { pruneAll });
}

// ---------------------------------------------------------------------------
// Cache restore
// ---------------------------------------------------------------------------

export async function restoreDockerCache(): Promise<void> {
  const cached = await loadCache<DockerInfo>("docker");
  if (cached) {
    dockerInfo.value = cached;
    setDomain("docker", {
      status: "done",
      itemCount: cached.images?.length ?? 0,
      totalSize: 0,
    });
  }
}
