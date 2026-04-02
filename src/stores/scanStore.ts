/**
 * Global scan store — holds all scan state so it persists across view navigation.
 *
 * Scans run in the background. The user can start a scan, navigate to other views,
 * and come back to see results. No more spinning beach ball locking up the UI.
 *
 * Uses Vue's reactivity system with module-level refs (singleton pattern).
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DiskUsage,
  FileInfo,
  LargeFileFound,
  LargeFileScanProgress,
  LargeFileScanDone,
  CacheEntry,
  LogEntry,
  DockerInfo,
  AppInfo,
  TrashInfo,
  CleanResult,
  SecurityScanResult,
  BrowserScanResult,
  BrowserCleanResult,
  MaintenanceTask,
  MaintenanceResult,
  DuplicateScanResult,
  DiskMapResult,
  DiskNode,
  CacheMetadata,
  MemoryScanResult,
  FilePreview,
  PackageScanResult,
  VitalsResult,
  ThermalScanResult,
  FileClassification,
  ScanSummaryOutput,
  VaultEntry,
  VaultSummary,
  CompressionCandidate,
  CompressResult,
  RestoreResult,
} from "../types";
import { getDisabledPaths } from "../composables/useScanSettings";

// ---------------------------------------------------------------------------
// Apple Intelligence
// ---------------------------------------------------------------------------

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

export const intelligenceAvailable = ref(false);
export const aiAvailable = ref(false);
export const fileClassifications = ref<Map<string, FileClassification>>(new Map());
export const scanSummary = ref<ScanSummaryOutput | null>(null);

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

// ---------------------------------------------------------------------------
// Scan result persistence — cache results to disk between sessions
// ---------------------------------------------------------------------------

/** Timestamps of last successful scan per domain */
export const lastScanned = ref<Record<string, number>>({});

interface CachedResult<T> {
  data: T;
  timestamp: number;
}

async function saveCache(domain: string, data: unknown) {
  try {
    const wrapped: CachedResult<unknown> = { data, timestamp: Date.now() };
    await invoke("save_scan_cache", { domain, data: JSON.stringify(wrapped) });
    lastScanned.value = { ...lastScanned.value, [domain]: wrapped.timestamp };
  } catch {
    // Cache save failures are non-critical
  }
}

async function loadCache<T>(domain: string): Promise<T | null> {
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

/** Restore all cached scan results on app startup. */
export async function restoreAllCaches() {
  const [
    cachedDisk, cachedLargeFiles, cachedCaches, cachedLogs, cachedApps,
    cachedTrash, cachedDocker, cachedSecurity, cachedBrowsers,
    cachedDuplicates, cachedPackages,
  ] = await Promise.all([
    loadCache<DiskUsage>("disk-usage"),
    loadCache<FileInfo[]>("large-files"),
    loadCache<CacheEntry[]>("caches"),
    loadCache<LogEntry[]>("logs"),
    loadCache<AppInfo[]>("apps"),
    loadCache<TrashInfo>("trash"),
    loadCache<DockerInfo>("docker"),
    loadCache<SecurityScanResult>("security"),
    loadCache<BrowserScanResult>("browsers"),
    loadCache<DuplicateScanResult>("duplicates"),
    loadCache<PackageScanResult>("packages"),
  ]);

  if (cachedDisk) diskUsage.value = cachedDisk;
  if (cachedLargeFiles) {
    largeFiles.value = cachedLargeFiles;
    largeFilesScanned.value = true;
    setDomain("largeFiles", { status: "done", itemCount: cachedLargeFiles.length, totalSize: cachedLargeFiles.reduce((s, f) => s + (f.is_sparse && f.actual_size < f.apparent_size * 0.8 ? f.actual_size : f.apparent_size), 0) });
  }
  if (cachedCaches) {
    caches.value = cachedCaches;
    cachesScanned.value = true;
    setDomain("caches", { status: "done", itemCount: cachedCaches.length, totalSize: cachedCaches.reduce((s, c) => s + c.size, 0) });
  }
  if (cachedLogs) {
    logs.value = cachedLogs;
    logsScanned.value = true;
    setDomain("logs", { status: "done", itemCount: cachedLogs.length, totalSize: cachedLogs.reduce((s, l) => s + l.size, 0) });
  }
  if (cachedApps) {
    apps.value = cachedApps;
    appsScanned.value = true;
    setDomain("apps", { status: "done", itemCount: cachedApps.length, totalSize: cachedApps.reduce((s, a) => s + a.footprint, 0) });
  }
  if (cachedTrash) {
    trashInfo.value = cachedTrash;
    setDomain("trash", { status: "done", itemCount: cachedTrash.item_count, totalSize: cachedTrash.size });
  }
  if (cachedDocker) {
    dockerInfo.value = cachedDocker;
    setDomain("docker", { status: "done", itemCount: cachedDocker.images?.length ?? 0, totalSize: 0 });
  }
  if (cachedSecurity) { securityResult.value = cachedSecurity; securityScanned.value = true; }
  if (cachedBrowsers) {
    browserResult.value = cachedBrowsers;
    browserScanned.value = true;
    setDomain("browsers", { status: "done", itemCount: cachedBrowsers.browsers?.length ?? 0, totalSize: cachedBrowsers.total_size ?? 0 });
  }
  if (cachedDuplicates) { duplicateResult.value = cachedDuplicates; duplicateScanned.value = true; }
  if (cachedPackages) { packagesResult.value = cachedPackages; packagesScanned.value = true; }

  // Restore most recent disk map cache (for dashboard waffle chart)
  try {
    const caches = await listDiskMapCaches();
    if (caches.length > 0) {
      await loadDiskMapCache(caches[0].id);
    }
  } catch (_) { /* non-critical */ }
}

// ---------------------------------------------------------------------------
// State — module-level refs are shared across all components that import this
// ---------------------------------------------------------------------------

// Disk usage
export const diskUsage = ref<DiskUsage | null>(null);
export const diskUsageLoading = ref(false);

// Large files
export const largeFiles = ref<FileInfo[]>([]);
export const largeFilesScanning = ref(false);
export const largeFilesScanned = ref(false);
export const largeFilesError = ref("");
export const largeFilesSkipped = ref<string[]>([]);
/** The directory the scanner is currently walking (updated in real time) */
export const largeFilesCurrentDir = ref("");

// Caches
export const caches = ref<CacheEntry[]>([]);
export const cachesScanning = ref(false);
export const cachesScanned = ref(false);
export const cachesError = ref("");

// Logs
export const logs = ref<LogEntry[]>([]);
export const logsScanning = ref(false);
export const logsScanned = ref(false);
export const logsError = ref("");

// Docker
export const dockerInstalled = ref<boolean | null>(null); // null = not yet checked
export const dockerInfo = ref<DockerInfo | null>(null);
export const dockerLoading = ref(false);
export const dockerError = ref("");

// Apps
export const apps = ref<AppInfo[]>([]);
export const appsScanning = ref(false);
export const appsScanned = ref(false);
export const appsError = ref("");

// Trash
export const trashInfo = ref<TrashInfo | null>(null);
export const trashLoading = ref(false);
export const trashError = ref("");

// Security
export const securityResult = ref<SecurityScanResult | null>(null);
export const securityScanning = ref(false);
export const securityScanned = ref(false);
export const securityError = ref("");

// Browsers
export const browserResult = ref<BrowserScanResult | null>(null);
export const browserScanning = ref(false);
export const browserScanned = ref(false);
export const browserError = ref("");

// Maintenance
export const maintenanceTasks = ref<MaintenanceTask[]>([]);
export const maintenanceLoaded = ref(false);

// Duplicates
export const duplicateResult = ref<DuplicateScanResult | null>(null);
export const duplicateScanning = ref(false);
export const duplicateScanned = ref(false);
export const duplicateError = ref("");

// Disk Map
export const diskMapResult = ref<DiskMapResult | null>(null);
export const diskMapLoading = ref(false);
export const diskMapLoaded = ref(false);
export const diskMapError = ref("");

// Disk Map Cache
export const diskMapCaches = ref<CacheMetadata[]>([]);
export const diskMapCacheLoading = ref(false);
/** ID of the currently loaded cache (empty string = fresh scan, not from cache) */
export const diskMapActiveCacheId = ref("");

// Memory
export const memoryResult = ref<MemoryScanResult | null>(null);
export const memoryScanning = ref(false);
export const memoryScanned = ref(false);
export const memoryError = ref("");

// Packages & Runtimes
export const packagesResult = ref<PackageScanResult | null>(null);
export const packagesScanning = ref(false);
export const packagesScanned = ref(false);
export const packagesError = ref("");

// System Vitals
export const vitalsResult = ref<VitalsResult | null>(null);
export const vitalsScanning = ref(false);
export const vitalsScanned = ref(false);
export const vitalsError = ref("");

// Hardware Thermal Sensors (SMC)
export const thermalResult = ref<ThermalScanResult | null>(null);
export const thermalScanning = ref(false);
export const thermalScanned = ref(false);
export const thermalError = ref("");

// Scan All progress
export const scanAllRunning = ref(false);
export const scanAllStep = ref("");
export const scanAllDone = ref(false);
export const lastScanTime = ref<number | null>(null);

// ---------------------------------------------------------------------------
// Per-domain scan status — unified tracking for dashboard + sidebar
// ---------------------------------------------------------------------------
// Status: 'idle' | 'scanning' | 'done' | 'error'
export type DomainStatus = "idle" | "scanning" | "done" | "error";

export interface DomainInfo {
  status: DomainStatus;
  itemCount: number;    // number of items found
  totalSize: number;    // bytes reclaimable / relevant
  error: string;
}

// One entry per scannable domain
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

// Helper to update a domain's status
function setDomain(id: string, partial: Partial<DomainInfo>) {
  const d = domainStatus.value[id];
  if (d) Object.assign(d, partial);
}

// Computed: total reclaimable across all scanned domains
export const totalReclaimable = computed(() =>
  Object.values(domainStatus.value).reduce((sum, d) => sum + d.totalSize, 0)
);

// Computed: number of domains that have completed scanning
export const domainsScanned = computed(() =>
  Object.values(domainStatus.value).filter((d) => d.status === "done").length
);

// Computed: total scannable domains
export const totalDomains = computed(() =>
  Object.keys(domainStatus.value).length
);

// Full Disk Access
export const hasFullDiskAccess = ref<boolean | null>(null);

// ---------------------------------------------------------------------------
// Computed summaries
// ---------------------------------------------------------------------------

export const totalCacheSize = computed(() =>
  caches.value.reduce((sum, c) => sum + c.size, 0)
);

export const totalLogSize = computed(() =>
  logs.value.reduce((sum, l) => sum + l.size, 0)
);

export const totalLargeFileSize = computed(() =>
  largeFiles.value.reduce((sum, f) => {
    const sparse = f.is_sparse && f.actual_size < f.apparent_size * 0.8;
    return sum + (sparse ? f.actual_size : f.apparent_size);
  }, 0)
);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadDiskUsage() {
  if (diskUsageLoading.value) return;
  diskUsageLoading.value = true;
  try {
    diskUsage.value = await invoke<DiskUsage>("get_disk_usage");
    void saveCache("disk-usage", diskUsage.value);
  } catch (_) {
    // non-critical
  } finally {
    diskUsageLoading.value = false;
  }
}

/** Helper: compute actual disk size for a file (respects sparse files). */
function fileDiskSize(f: FileInfo): number {
  const sparse = f.is_sparse && f.actual_size < f.apparent_size * 0.8;
  return sparse ? f.actual_size : f.apparent_size;
}

/**
 * Streaming large-file scan. Files appear in the UI immediately as they're
 * discovered, and a "currently scanning" indicator shows the active directory.
 *
 * Uses Tauri events instead of a single invoke → result round-trip.
 */
export async function scanLargeFiles(path = "~", minSizeMb = 100) {
  if (largeFilesScanning.value) return;
  largeFilesScanning.value = true;
  largeFilesError.value = "";
  largeFiles.value = [];
  largeFilesSkipped.value = [];
  largeFilesScanned.value = false;
  largeFilesCurrentDir.value = "";
  setDomain("largeFiles", { status: "scanning", error: "" });

  // Set up event listeners BEFORE invoking the command so we don't miss
  // any events that fire immediately.
  const unlisteners: UnlistenFn[] = [];

  try {
    // Listen for individual file discoveries.
    unlisteners.push(
      await listen<LargeFileFound>("large-file-found", (event) => {
        largeFiles.value = [...largeFiles.value, event.payload.file];
      })
    );

    // Listen for progress updates (current directory).
    unlisteners.push(
      await listen<LargeFileScanProgress>("large-file-progress", (event) => {
        largeFilesCurrentDir.value = event.payload.current_dir;
      })
    );

    // Listen for scan completion.
    unlisteners.push(
      await listen<LargeFileScanDone>("large-file-done", (event) => {
        largeFilesSkipped.value = event.payload.skipped_paths;
        largeFilesScanned.value = true;
        largeFilesCurrentDir.value = "";
        void saveCache("large-files", largeFiles.value);

        // Compute final domain stats from the accumulated files.
        const files = largeFiles.value;
        setDomain("largeFiles", {
          status: "done",
          itemCount: files.length,
          totalSize: files.reduce((s, f) => s + fileDiskSize(f), 0),
        });
      })
    );

    // Now invoke the streaming command. It returns () — all data comes via events.
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
    // Clean up all event listeners.
    for (const unlisten of unlisteners) {
      unlisten();
    }
    largeFilesScanning.value = false;
    largeFilesCurrentDir.value = "";
  }
}

export async function scanCaches() {
  if (cachesScanning.value) return;
  cachesScanning.value = true;
  cachesError.value = "";
  caches.value = [];
  cachesScanned.value = false;
  setDomain("caches", { status: "scanning", error: "" });
  try {
    caches.value = await invoke<CacheEntry[]>("scan_caches", {
      hasFda: hasFullDiskAccess.value === true,
    });
    cachesScanned.value = true;
    void saveCache("caches", caches.value);
    setDomain("caches", {
      status: "done",
      itemCount: caches.value.length,
      totalSize: caches.value.reduce((s, c) => s + c.size, 0),
    });
  } catch (e) {
    cachesError.value = String(e);
    setDomain("caches", { status: "error", error: String(e) });
  } finally {
    cachesScanning.value = false;
  }
}

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
    // total_reclaimable comes as a human-readable string from Docker CLI;
    // parse it to bytes if possible, otherwise default to 0
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

export async function scanApps() {
  if (appsScanning.value) return;
  appsScanning.value = true;
  appsError.value = "";
  apps.value = [];
  appsScanned.value = false;
  setDomain("apps", { status: "scanning", error: "" });
  try {
    apps.value = await invoke<AppInfo[]>("scan_apps", {
      hasFda: hasFullDiskAccess.value === true,
    });
    appsScanned.value = true;
    void saveCache("apps", apps.value);
    setDomain("apps", {
      status: "done",
      itemCount: apps.value.length,
      totalSize: apps.value.reduce((s, a) => s + a.footprint, 0),
    });
  } catch (e) {
    appsError.value = String(e);
    setDomain("apps", { status: "error", error: String(e) });
  } finally {
    appsScanning.value = false;
  }
}

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

export async function checkFullDiskAccess() {
  try {
    hasFullDiskAccess.value = await invoke<boolean>("check_full_disk_access");
  } catch (_) {
    hasFullDiskAccess.value = false;
  }
}

/**
 * Scan All — runs each scan sequentially so we can show progress steps.
 * This runs in the background; the user can navigate away and come back.
 *
 * Covers all 8 scannable domains plus disk usage:
 *   caches, logs, large files, apps, browsers, security, trash, docker
 *
 * Excluded: Duplicates (slow, needs user config), Space Map (auto-loads),
 * Memory (live polling), Maintenance (task list, not a scan).
 */
export async function scanAll() {
  if (scanAllRunning.value) return;
  scanAllRunning.value = true;
  scanAllDone.value = false;
  scanAllStep.value = "";

  try {
    // --- Disk usage (not a domain, but feeds the dashboard bar) ---
    scanAllStep.value = "disk_usage";
    await loadDiskUsage();

    // --- Core cleanup domains ---
    scanAllStep.value = "caches";
    await scanCaches();

    scanAllStep.value = "logs";
    await scanLogs();

    scanAllStep.value = "large_files";
    await scanLargeFiles();

    // --- App & browser data ---
    scanAllStep.value = "apps";
    await scanApps();

    scanAllStep.value = "browsers";
    await scanBrowsers();

    // --- System domains ---
    scanAllStep.value = "trash";
    await loadTrashInfo();

    scanAllStep.value = "docker";
    await loadDockerInfo();

    // Security scan excluded from quick scan — too slow, run manually from Security view

    scanAllDone.value = true;
    scanAllStep.value = "";
    lastScanTime.value = Date.now();
  } catch (_) {
    // errors captured per-scan
  } finally {
    scanAllRunning.value = false;
    scanAllStep.value = "";
  }
}

// ---------------------------------------------------------------------------
// Delete / Clean actions
// ---------------------------------------------------------------------------

export async function deleteFiles(paths: string[]): Promise<CleanResult> {
  const result = await invoke<CleanResult>("delete_files", { paths });
  return result;
}

export async function cleanDocker(pruneAll: boolean): Promise<CleanResult> {
  const result = await invoke<CleanResult>("clean_docker", { pruneAll });
  return result;
}

export async function emptyTrash(): Promise<CleanResult> {
  const result = await invoke<CleanResult>("empty_trash");
  // Refresh trash info after emptying
  await loadTrashInfo();
  return result;
}

export async function uninstallApp(
  appPath: string,
  removeLeftovers: boolean
): Promise<CleanResult> {
  const result = await invoke<CleanResult>("uninstall_app", {
    appPath,
    removeLeftovers,
  });
  return result;
}

export async function scanSecurity() {
  if (securityScanning.value) return;
  securityScanning.value = true;
  securityError.value = "";
  securityResult.value = null;
  securityScanned.value = false;
  setDomain("security", { status: "scanning", error: "" });
  try {
    securityResult.value = await invoke<SecurityScanResult>("scan_security");
    securityScanned.value = true;
    void saveCache("security", securityResult.value);
    setDomain("security", {
      status: "done",
      itemCount: securityResult.value?.summary?.total_findings ?? 0,
      totalSize: 0,
    });
  } catch (e) {
    securityError.value = String(e);
    setDomain("security", { status: "error", error: String(e) });
  } finally {
    securityScanning.value = false;
  }
}

export async function scanBrowsers() {
  if (browserScanning.value) return;
  browserScanning.value = true;
  browserError.value = "";
  browserResult.value = null;
  browserScanned.value = false;
  setDomain("browsers", { status: "scanning", error: "" });
  try {
    browserResult.value = await invoke<BrowserScanResult>("scan_browsers", {
      hasFda: hasFullDiskAccess.value === true,
    });
    browserScanned.value = true;
    void saveCache("browsers", browserResult.value);
    const totalSize = browserResult.value?.browsers?.reduce(
      (sum, b) => sum + b.total_size, 0
    ) ?? 0;
    const itemCount = browserResult.value?.browsers?.length ?? 0;
    setDomain("browsers", { status: "done", itemCount, totalSize });
  } catch (e) {
    browserError.value = String(e);
    setDomain("browsers", { status: "error", error: String(e) });
  } finally {
    browserScanning.value = false;
  }
}

export async function cleanBrowserData(
  paths: string[]
): Promise<BrowserCleanResult> {
  return await invoke<BrowserCleanResult>("clean_browser_data", { paths });
}

export async function scanDuplicates(path = "~", minSizeMb = 1) {
  if (duplicateScanning.value) return;
  duplicateScanning.value = true;
  duplicateError.value = "";
  duplicateResult.value = null;
  duplicateScanned.value = false;
  try {
    duplicateResult.value = await invoke<DuplicateScanResult>(
      "scan_duplicates",
      {
        path,
        minSizeMb,
        hasFda: hasFullDiskAccess.value === true,
        skipPaths: getDisabledPaths(),
      }
    );
    duplicateScanned.value = true;
    void saveCache("duplicates", duplicateResult.value);
  } catch (e) {
    duplicateError.value = String(e);
  } finally {
    duplicateScanning.value = false;
  }
}

export async function loadDiskMap() {
  if (diskMapLoading.value) return;
  diskMapLoading.value = true;
  diskMapError.value = "";
  diskMapResult.value = null;
  diskMapLoaded.value = false;
  diskMapActiveCacheId.value = "";
  try {
    diskMapResult.value = await invoke<DiskMapResult>("get_disk_map", {
      hasFda: hasFullDiskAccess.value === true,
    });
    diskMapLoaded.value = true;

    // Auto-save scan result to cache (fire-and-forget).
    saveDiskMapCache().catch(() => {});
  } catch (e) {
    diskMapError.value = String(e);
  } finally {
    diskMapLoading.value = false;
  }
}

export async function expandDiskNode(path: string): Promise<DiskNode | null> {
  try {
    return await invoke<DiskNode>("expand_disk_node", {
      path,
      hasFda: hasFullDiskAccess.value === true,
    });
  } catch (_) {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Disk Map Cache functions
// ---------------------------------------------------------------------------

/** Save the current disk map result to the app cache. */
export async function saveDiskMapCache(): Promise<string | null> {
  if (!diskMapResult.value) return null;
  try {
    const data = JSON.stringify(diskMapResult.value);
    const id = await invoke<string>("save_disk_map_cache", { data });
    diskMapActiveCacheId.value = id;
    // Refresh the cache list so the UI shows the new entry.
    await listDiskMapCaches();
    return id;
  } catch (_) {
    return null;
  }
}

/** List all available disk map caches. */
export async function listDiskMapCaches(): Promise<CacheMetadata[]> {
  try {
    diskMapCaches.value = await invoke<CacheMetadata[]>("list_disk_map_caches");
    return diskMapCaches.value;
  } catch (_) {
    diskMapCaches.value = [];
    return [];
  }
}

/** Load a specific cached disk map by ID. */
export async function loadDiskMapCache(id: string): Promise<boolean> {
  diskMapCacheLoading.value = true;
  try {
    const json = await invoke<string>("load_disk_map_cache", { id });
    diskMapResult.value = JSON.parse(json) as DiskMapResult;
    diskMapLoaded.value = true;
    diskMapActiveCacheId.value = id;
    diskMapError.value = "";
    return true;
  } catch (e) {
    diskMapError.value = String(e);
    return false;
  } finally {
    diskMapCacheLoading.value = false;
  }
}

/** Delete a specific cached disk map by ID. */
export async function deleteDiskMapCache(id: string): Promise<boolean> {
  try {
    await invoke<void>("delete_disk_map_cache", { id });
    // If the deleted cache was the active one, clear the active ID.
    if (diskMapActiveCacheId.value === id) {
      diskMapActiveCacheId.value = "";
    }
    await listDiskMapCaches();
    return true;
  } catch (_) {
    return false;
  }
}

/** Load the most recent cache on app startup (silent, no error shown). */
export async function loadMostRecentCache(): Promise<boolean> {
  try {
    const caches = await listDiskMapCaches();
    if (caches.length === 0) return false;
    return await loadDiskMapCache(caches[0].id);
  } catch (_) {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Disk Map Recency Enrichment
// ---------------------------------------------------------------------------

/**
 * Enrich disk nodes with modification timestamps (async, batched).
 *
 * Collects all non-synthetic paths from the current disk map result,
 * sends them to the Rust backend in batches, and patches the `modified`
 * field on each matching DiskNode. The sunburst can reactively update
 * colors as data arrives.
 *
 * @param batchSize Number of paths to send per backend call (default 50).
 */
export async function enrichDiskNodes(batchSize = 50): Promise<void> {
  if (!diskMapResult.value) return;

  // Collect all paths from the tree.
  const paths: string[] = [];
  function collectPaths(node: DiskNode) {
    if (node.path) paths.push(node.path);
    for (const child of node.children) {
      collectPaths(child);
    }
  }
  collectPaths(diskMapResult.value.root);

  // Send in batches to avoid overwhelming the subprocess calls.
  for (let i = 0; i < paths.length; i += batchSize) {
    const batch = paths.slice(i, i + batchSize);
    try {
      const result = await invoke<Record<string, number>>("enrich_disk_nodes", {
        paths: batch,
      });

      // Patch the modification times into the tree.
      function patchNode(node: DiskNode) {
        if (node.path && result[node.path] !== undefined) {
          node.modified = result[node.path];
        }
        for (const child of node.children) {
          patchNode(child);
        }
      }
      patchNode(diskMapResult.value!.root);
    } catch (_) {
      // Non-critical — enrichment is best-effort.
    }
  }
}

export async function loadMaintenanceTasks() {
  try {
    const result = await invoke<{ tasks: MaintenanceTask[] }>(
      "get_maintenance_tasks"
    );
    maintenanceTasks.value = result.tasks;
    maintenanceLoaded.value = true;
  } catch (e) {
    // non-critical
  }
}

export async function runMaintenanceTask(
  taskId: string
): Promise<MaintenanceResult> {
  // Mark the task as running in local state
  const idx = maintenanceTasks.value.findIndex((t) => t.id === taskId);
  if (idx !== -1) {
    maintenanceTasks.value[idx].status = "running";
    maintenanceTasks.value[idx].message = "";
  }

  try {
    const result = await invoke<MaintenanceResult>("run_maintenance_task", {
      taskId,
    });
    // Update local state with result
    if (idx !== -1) {
      maintenanceTasks.value[idx].status = result.success
        ? "success"
        : "error";
      maintenanceTasks.value[idx].message = result.message;
    }
    return result;
  } catch (e) {
    if (idx !== -1) {
      maintenanceTasks.value[idx].status = "error";
      maintenanceTasks.value[idx].message = String(e);
    }
    return {
      task_id: taskId,
      success: false,
      message: String(e),
    };
  }
}

/**
 * Scan memory. If `live` is true, this is a background refresh — we don't
 * null the existing result (avoids flash) and we don't set the scanning flag
 * (avoids spinner on every tick). If false, it's a fresh/manual scan.
 */
export async function scanMemory(live = false) {
  if (memoryScanning.value) return;
  if (!live) {
    memoryScanning.value = true;
    memoryError.value = "";
    memoryResult.value = null;
    memoryScanned.value = false;
  }
  try {
    memoryResult.value = await invoke<MemoryScanResult>("scan_memory");
    memoryScanned.value = true;
  } catch (e) {
    if (!live) memoryError.value = String(e);
  } finally {
    memoryScanning.value = false;
  }
}

export async function scanPackages() {
  if (packagesScanning.value) return;
  packagesScanning.value = true;
  packagesError.value = "";
  packagesResult.value = null;
  packagesScanned.value = false;
  try {
    packagesResult.value = await invoke<PackageScanResult>("scan_packages");
    packagesScanned.value = true;
    void saveCache("packages", packagesResult.value);
  } catch (e) {
    packagesError.value = String(e);
  } finally {
    packagesScanning.value = false;
  }
}

export async function previewFile(
  path: string,
  maxSize?: number
): Promise<FilePreview> {
  return await invoke<FilePreview>("preview_file", { path, maxSize });
}

export async function disableLaunchItem(path: string): Promise<CleanResult> {
  return await invoke<CleanResult>("disable_launch_item", { path });
}

export async function removeLaunchItem(path: string): Promise<CleanResult> {
  return await invoke<CleanResult>("remove_launch_item", { path });
}

// ---------------------------------------------------------------------------
// System Vitals
// ---------------------------------------------------------------------------

/**
 * Scan system vitals. If `live` is true, this is a background refresh —
 * we don't null the existing result (avoids flash) and don't show spinner.
 */
export async function scanVitals(live = false) {
  if (vitalsScanning.value) return;
  if (!live) {
    vitalsScanning.value = true;
    vitalsError.value = "";
    vitalsResult.value = null;
    vitalsScanned.value = false;
  }
  try {
    vitalsResult.value = await invoke<VitalsResult>("scan_vitals");
    vitalsScanned.value = true;
  } catch (e) {
    if (!live) vitalsError.value = String(e);
  } finally {
    vitalsScanning.value = false;
  }
}

export async function quitProcess(pid: number): Promise<string> {
  return await invoke<string>("quit_process", { pid });
}

export async function forceQuitProcess(pid: number): Promise<string> {
  return await invoke<string>("force_quit_process", { pid });
}

export async function quitProcessGroup(pids: number[]): Promise<string> {
  return await invoke<string>("quit_process_group", { pids });
}

// ---------------------------------------------------------------------------
// Hardware Thermal Sensors (SMC)
// ---------------------------------------------------------------------------

/**
 * Read all hardware temperature sensors and fan speeds from the SMC.
 * If `live` is true, this is a background refresh — don't null existing data.
 */
export async function scanThermal(live = false) {
  if (thermalScanning.value) return;
  if (!live) {
    thermalScanning.value = true;
    thermalError.value = "";
    thermalResult.value = null;
    thermalScanned.value = false;
  }
  try {
    thermalResult.value = await invoke<ThermalScanResult>("scan_thermal");
    thermalScanned.value = true;
  } catch (e) {
    if (!live) thermalError.value = String(e);
  } finally {
    thermalScanning.value = false;
  }
}

// ---------------------------------------------------------------------------
// Vault
// ---------------------------------------------------------------------------

export const vaultSummary = ref<VaultSummary | null>(null);
export const vaultEntries = ref<VaultEntry[]>([]);
export const vaultCandidates = ref<CompressionCandidate[]>([]);
export const vaultScanning = ref(false);
export const vaultCompressing = ref(false);
export const vaultError = ref("");

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
    // Refresh vault state after compression
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
