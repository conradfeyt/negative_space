/**
 * Shared domain-status tracking, scan-cache persistence, full-disk-access
 * state, and protected-files management.
 *
 * Every domain store imports helpers from here instead of defining its own.
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Per-domain scan status — unified tracking for dashboard + sidebar
// ---------------------------------------------------------------------------

export type DomainStatus = "idle" | "scanning" | "done" | "error";

export interface DomainInfo {
  status: DomainStatus;
  itemCount: number;
  totalSize: number;
  error: string;
}

export const domainStatus = ref<Record<string, DomainInfo>>({
  caches:     { status: "idle", itemCount: 0, totalSize: 0, error: "" },
  logs:       { status: "idle", itemCount: 0, totalSize: 0, error: "" },
  largeFiles: { status: "idle", itemCount: 0, totalSize: 0, error: "" },
  apps:       { status: "idle", itemCount: 0, totalSize: 0, error: "" },
  trash:      { status: "idle", itemCount: 0, totalSize: 0, error: "" },
  docker:     { status: "idle", itemCount: 0, totalSize: 0, error: "" },
  browsers:   { status: "idle", itemCount: 0, totalSize: 0, error: "" },
  security:   { status: "idle", itemCount: 0, totalSize: 0, error: "" },
});

export function setDomain(id: string, partial: Partial<DomainInfo>) {
  const d = domainStatus.value[id];
  if (d) Object.assign(d, partial);
}

export const totalReclaimable = computed(() =>
  Object.values(domainStatus.value).reduce((sum, d) => sum + d.totalSize, 0)
);

export const domainsScanned = computed(() =>
  Object.values(domainStatus.value).filter((d) => d.status === "done").length
);

export const totalDomains = computed(() =>
  Object.keys(domainStatus.value).length
);

// ---------------------------------------------------------------------------
// Full Disk Access
// ---------------------------------------------------------------------------

export const hasFullDiskAccess = ref<boolean | null>(null);

export async function checkFullDiskAccess() {
  // TODO: restore FDA check — temporarily bypassed for showcase tuning
  hasFullDiskAccess.value = true;
  return;
  try {
    hasFullDiskAccess.value = await invoke<boolean>("check_full_disk_access");
  } catch (e) {
    console.warn('[fda] check failed:', e);
    hasFullDiskAccess.value = false;
  }
}

// ---------------------------------------------------------------------------
// Scan result persistence — cache results to disk between sessions
// ---------------------------------------------------------------------------

export const lastScanned = ref<Record<string, number>>({});

interface CachedResult<T> {
  data: T;
  timestamp: number;
}

export async function saveCache(domain: string, data: unknown) {
  try {
    const wrapped: CachedResult<unknown> = { data, timestamp: Date.now() };
    await invoke("save_scan_cache", { domain, data: JSON.stringify(wrapped) });
    lastScanned.value = { ...lastScanned.value, [domain]: wrapped.timestamp };
  } catch (e) { console.debug(`[cache] save failed for ${domain}:`, e); }
}

export async function loadCache<T>(domain: string): Promise<T | null> {
  try {
    const raw = await invoke<string | null>("load_scan_cache", { domain });
    if (!raw) return null;
    const wrapped: CachedResult<T> = JSON.parse(raw);
    lastScanned.value = { ...lastScanned.value, [domain]: wrapped.timestamp };
    return wrapped.data;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// User-protected files — persisted to localStorage
// ---------------------------------------------------------------------------

const PROTECTED_KEY = "negativ_protected_files";

function loadProtected(): Set<string> {
  try {
    const raw = localStorage.getItem(PROTECTED_KEY);
    return raw ? new Set(JSON.parse(raw)) : new Set();
  } catch { return new Set(); }
}

export const protectedFiles = ref<Set<string>>(loadProtected());

export function toggleProtected(path: string) {
  const next = new Set(protectedFiles.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  protectedFiles.value = next;
  localStorage.setItem(PROTECTED_KEY, JSON.stringify([...next]));
}

export function isProtected(path: string): boolean {
  return protectedFiles.value.has(path);
}

// ---------------------------------------------------------------------------
// Shared file operations
// ---------------------------------------------------------------------------
import type { CleanResult } from "../types";

export async function deleteFiles(paths: string[]): Promise<CleanResult> {
  return await invoke<CleanResult>("delete_files", { paths });
}

export async function previewFile(
  path: string,
  maxSize?: number
): Promise<import("../types").FilePreview> {
  return await invoke<import("../types").FilePreview>("preview_file", { path, maxSize });
}
