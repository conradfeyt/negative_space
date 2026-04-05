/**
 * Vault store — compressed file storage.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { hasFullDiskAccess } from "./domainStatusStore";
import type {
  VaultEntry,
  VaultSummary,
  CompressionCandidate,
  CompressResult,
  RestoreResult,
} from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const vaultSummary = ref<VaultSummary | null>(null);
export const vaultEntries = ref<VaultEntry[]>([]);
export const vaultCandidates = ref<CompressionCandidate[]>([]);
export const vaultScanning = ref(false);
export const vaultCompressing = ref(false);
export const vaultError = ref("");

// ---------------------------------------------------------------------------
// Mutations (store-only — views should call these, not mutate refs directly)
// ---------------------------------------------------------------------------

export function setVaultEntries(entries: VaultEntry[]) {
  vaultEntries.value = entries;
}

export function removeCandidates(paths: string[]) {
  vaultCandidates.value = vaultCandidates.value.filter(c => !paths.includes(c.path));
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadVaultSummary() {
  try {
    vaultSummary.value = await invoke<VaultSummary>("get_vault_summary");
    vaultEntries.value = await invoke<VaultEntry[]>("get_vault_entries");
  } catch (e) {
    vaultError.value = String(e);
  }
}

export async function scanVaultCandidates(path = "~", minSizeMb = 10, minAgeDays = 30) {
  vaultScanning.value = true;
  vaultError.value = "";
  try {
    const fda = hasFullDiskAccess.value ?? false;
    vaultCandidates.value = await invoke<CompressionCandidate[]>("scan_vault_candidates", {
      path, minSizeMb, minAgeDays, fda,
    });
  } catch (e) {
    vaultError.value = String(e);
  } finally {
    vaultScanning.value = false;
  }
}

export async function compressToVault(paths: string[]): Promise<CompressResult> {
  vaultCompressing.value = true;
  vaultError.value = "";
  try {
    const result = await invoke<CompressResult>("compress_to_vault", { paths });
    await loadVaultSummary();
    return result;
  } catch (e) {
    vaultError.value = String(e);
    return { success: false, files_compressed: 0, total_original_size: 0, total_compressed_size: 0, total_savings: 0, errors: [String(e)] };
  } finally {
    vaultCompressing.value = false;
  }
}

export async function restoreFromVault(entryId: string): Promise<RestoreResult> {
  try {
    const result = await invoke<RestoreResult>("restore_from_vault", { entryId });
    await loadVaultSummary();
    return result;
  } catch (e) {
    return { success: false, restored_path: "", errors: [String(e)] };
  }
}

export async function compressDirectoryToVault(path: string): Promise<CompressResult> {
  vaultCompressing.value = true;
  vaultError.value = "";
  try {
    const result = await invoke<CompressResult>("compress_directory_to_vault", { path });
    await loadVaultSummary();
    return result;
  } catch (e) {
    vaultError.value = String(e);
    return { success: false, files_compressed: 0, total_original_size: 0, total_compressed_size: 0, total_savings: 0, errors: [String(e)] };
  } finally {
    vaultCompressing.value = false;
  }
}

export async function collectVaultDirectory(path: string) {
  vaultScanning.value = true;
  vaultError.value = "";
  try {
    vaultCandidates.value = await invoke<CompressionCandidate[]>("collect_vault_directory", { path });
  } catch (e) {
    vaultError.value = String(e);
  } finally {
    vaultScanning.value = false;
  }
}

export async function deleteVaultEntry(entryId: string): Promise<void> {
  try {
    await invoke<void>("delete_vault_entry", { entryId });
    await loadVaultSummary();
  } catch (e) {
    vaultError.value = String(e);
  }
}
