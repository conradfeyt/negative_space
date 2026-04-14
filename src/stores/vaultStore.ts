/**
 * Vault store — secure storage for sensitive files.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { VaultEntry, VaultSummary, CompressResult, RestoreResult, MoveResult, StorageConfig } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const vaultSummary = ref<VaultSummary | null>(null);
export const vaultEntries = ref<VaultEntry[]>([]);
export const vaultError = ref("");

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

export async function compressToVault(paths: string[]): Promise<CompressResult> {
  try {
    const result = await invoke<CompressResult>("compress_to_vault", { paths });
    await loadVaultSummary();
    return result;
  } catch (e) {
    return { success: false, files_compressed: 0, total_original_size: 0, total_compressed_size: 0, total_savings: 0, errors: [String(e)] };
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

export async function deleteVaultEntry(entryId: string): Promise<void> {
  try {
    await invoke<void>("delete_vault_entry", { entryId });
    await loadVaultSummary();
  } catch (e) {
    vaultError.value = String(e);
  }
}

// ---------------------------------------------------------------------------
// Move files to directory (fire-and-forget)
// ---------------------------------------------------------------------------

export async function moveFilesToDirectory(
  paths: string[],
  targetDir: string,
): Promise<MoveResult> {
  try {
    return await invoke<MoveResult>("move_files_to_directory", {
      paths,
      targetDir,
    });
  } catch (e) {
    return { success: false, files_moved: 0, errors: [String(e)] };
  }
}

// ---------------------------------------------------------------------------
// Storage configuration
// ---------------------------------------------------------------------------

export const storageConfig = ref<StorageConfig>({ archive_dir: null, vault_dir: null });

export async function loadStorageConfig(): Promise<void> {
  try {
    storageConfig.value = await invoke<StorageConfig>("get_storage_config");
  } catch (e) {
    vaultError.value = String(e);
  }
}

export async function setStorageConfig(config: StorageConfig): Promise<void> {
  await invoke<void>("set_storage_config", { config });
  storageConfig.value = config;
}
