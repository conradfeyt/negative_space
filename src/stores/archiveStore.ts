/**
 * Archive store — compress large files to save disk space.
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

export const archiveSummary = ref<VaultSummary | null>(null);
export const archiveEntries = ref<VaultEntry[]>([]);
export const archiveCandidates = ref<CompressionCandidate[]>([]);
export const archiveScanning = ref(false);
export const archiveCompressing = ref(false);
export const archiveError = ref("");

// ---------------------------------------------------------------------------
// Mutations
// ---------------------------------------------------------------------------

export function setArchiveEntries(entries: VaultEntry[]) {
  archiveEntries.value = entries;
}

export function removeArchiveCandidates(paths: string[]) {
  archiveCandidates.value = archiveCandidates.value.filter(c => !paths.includes(c.path));
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadArchiveSummary() {
  try {
    archiveSummary.value = await invoke<VaultSummary>("get_archive_summary");
    archiveEntries.value = await invoke<VaultEntry[]>("get_archive_entries");
  } catch (e) {
    archiveError.value = String(e);
  }
}

export async function scanArchiveCandidates(path = "~", minSizeMb = 10, minAgeDays = 30) {
  archiveScanning.value = true;
  archiveError.value = "";
  try {
    const fda = hasFullDiskAccess.value ?? false;
    archiveCandidates.value = await invoke<CompressionCandidate[]>("scan_archive_candidates", {
      path, minSizeMb, minAgeDays, fda,
    });
  } catch (e) {
    archiveError.value = String(e);
  } finally {
    archiveScanning.value = false;
  }
}

export async function compressToArchive(paths: string[]): Promise<CompressResult> {
  archiveCompressing.value = true;
  archiveError.value = "";
  try {
    const result = await invoke<CompressResult>("compress_to_archive", { paths });
    await loadArchiveSummary();
    return result;
  } catch (e) {
    archiveError.value = String(e);
    return { success: false, files_compressed: 0, total_original_size: 0, total_compressed_size: 0, total_savings: 0, errors: [String(e)] };
  } finally {
    archiveCompressing.value = false;
  }
}

export async function restoreFromArchive(entryId: string): Promise<RestoreResult> {
  try {
    const result = await invoke<RestoreResult>("restore_from_archive", { entryId });
    await loadArchiveSummary();
    return result;
  } catch (e) {
    return { success: false, restored_path: "", errors: [String(e)] };
  }
}

export async function compressDirectoryToArchive(path: string): Promise<CompressResult> {
  archiveCompressing.value = true;
  archiveError.value = "";
  try {
    const result = await invoke<CompressResult>("compress_directory_to_archive", { path });
    await loadArchiveSummary();
    return result;
  } catch (e) {
    archiveError.value = String(e);
    return { success: false, files_compressed: 0, total_original_size: 0, total_compressed_size: 0, total_savings: 0, errors: [String(e)] };
  } finally {
    archiveCompressing.value = false;
  }
}

export async function collectArchiveDirectory(path: string) {
  archiveScanning.value = true;
  archiveError.value = "";
  try {
    archiveCandidates.value = await invoke<CompressionCandidate[]>("collect_archive_directory", { path });
  } catch (e) {
    archiveError.value = String(e);
  } finally {
    archiveScanning.value = false;
  }
}

export async function deleteArchiveEntry(entryId: string): Promise<void> {
  try {
    await invoke<void>("delete_archive_entry", { entryId });
    await loadArchiveSummary();
  } catch (e) {
    archiveError.value = String(e);
  }
}
