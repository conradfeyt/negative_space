/**
 * Global scan store — thin re-export facade.
 *
 * All domain logic has been extracted to individual stores. This file
 * re-exports everything so existing `import { ... } from "./stores/scanStore"`
 * statements continue to work without modification.
 */
import { ref } from "vue";
import { loadCache } from "./domainStatusStore";
import type { DiskUsage } from "../types";

// ---------------------------------------------------------------------------
// Re-export: shared infrastructure (domainStatusStore)
// ---------------------------------------------------------------------------
export {
  // Types
  type DomainStatus,
  type DomainInfo,
  // Domain status refs & helpers
  domainStatus,
  setDomain,
  totalReclaimable,
  domainsScanned,
  totalDomains,
  // Full Disk Access
  hasFullDiskAccess,
  checkFullDiskAccess,
  // Cache persistence
  lastScanned,
  saveCache,
  loadCache,
  // Protected files
  protectedFiles,
  toggleProtected,
  isProtected,
  // Shared file operations
  deleteFiles,
  previewFile,
} from "./domainStatusStore";

// ---------------------------------------------------------------------------
// Re-export: disk usage
// ---------------------------------------------------------------------------
export {
  diskUsage,
  diskUsageLoading,
  loadDiskUsage,
} from "./diskUsageStore";

// ---------------------------------------------------------------------------
// Re-export: intelligence / AI
// ---------------------------------------------------------------------------
export {
  intelligenceAvailable,
  aiAvailable,
  fileClassifications,
  scanSummary,
  checkIntelligence,
  classifyFiles,
  generateScanSummary,
} from "./intelligenceStore";

// ---------------------------------------------------------------------------
// Re-export: large files
// ---------------------------------------------------------------------------
export {
  largeFiles,
  largeFilesScanning,
  largeFilesScanned,
  largeFilesError,
  largeFilesSkipped,
  largeFilesCurrentDir,
  totalLargeFileSize,
  scanLargeFiles,
} from "./largeFilesStore";
// (restoreLargeFilesCache is internal — called from restoreAllCaches)

// ---------------------------------------------------------------------------
// Re-export: caches
// ---------------------------------------------------------------------------
export {
  caches,
  cachesScanning,
  cachesScanned,
  cachesError,
  totalCacheSize,
  scanCaches,
} from "./cachesStore";

// ---------------------------------------------------------------------------
// Re-export: logs
// ---------------------------------------------------------------------------
export {
  logs,
  logsScanning,
  logsScanned,
  logsError,
  totalLogSize,
  scanLogs,
} from "./logsStore";

// ---------------------------------------------------------------------------
// Re-export: apps
// ---------------------------------------------------------------------------
export {
  apps,
  appsScanning,
  appsScanned,
  appsError,
  scanApps,
  uninstallApp,
} from "./appsStore";

// ---------------------------------------------------------------------------
// Re-export: browsers
// ---------------------------------------------------------------------------
export {
  browserResult,
  browserScanning,
  browserScanned,
  browserError,
  scanBrowsers,
  cleanBrowserData,
} from "./browserStore";

// ---------------------------------------------------------------------------
// Re-export: duplicates + similar images
// ---------------------------------------------------------------------------
export {
  duplicateResult,
  duplicateScanning,
  duplicateScanned,
  duplicateError,
  scanDuplicates,
  similarResult,
  similarScanning,
  similarScanned,
  similarError,
  similarProgress,
  scanSimilarImages,
} from "./duplicatesStore";

// ---------------------------------------------------------------------------
// Re-export: disk map
// ---------------------------------------------------------------------------
export {
  diskMapResult,
  diskMapLoading,
  diskMapLoaded,
  diskMapError,
  diskMapCaches,
  diskMapCacheLoading,
  diskMapActiveCacheId,
  loadDiskMap,
  expandDiskNode,
  saveDiskMapCache,
  listDiskMapCaches,
  loadDiskMapCache,
  deleteDiskMapCache,
  loadMostRecentCache,
  enrichDiskNodes,
} from "./diskMapStore";

// ---------------------------------------------------------------------------
// Re-export: system (vitals, thermal, memory)
// ---------------------------------------------------------------------------
export {
  vitalsResult,
  vitalsScanning,
  vitalsScanned,
  vitalsError,
  scanVitals,
  quitProcess,
  forceQuitProcess,
  quitProcessGroup,
  thermalResult,
  thermalScanning,
  thermalScanned,
  thermalError,
  scanThermal,
  memoryResult,
  memoryScanning,
  memoryScanned,
  memoryError,
  scanMemory,
} from "./systemStore";

// ---------------------------------------------------------------------------
// Re-export: docker
// ---------------------------------------------------------------------------
export {
  dockerInstalled,
  dockerInfo,
  dockerLoading,
  dockerError,
  checkDockerInstalled,
  loadDockerInfo,
  cleanDocker,
} from "./dockerStore";

// ---------------------------------------------------------------------------
// Re-export: trash
// ---------------------------------------------------------------------------
export {
  trashInfo,
  trashLoading,
  trashError,
  loadTrashInfo,
  emptyTrash,
} from "./trashStore";

// ---------------------------------------------------------------------------
// Re-export: security
// ---------------------------------------------------------------------------
export {
  securityResult,
  securityScanning,
  securityScanned,
  securityError,
  scanSecurity,
  disableLaunchItem,
  removeLaunchItem,
} from "./securityStore";

// ---------------------------------------------------------------------------
// Re-export: maintenance
// ---------------------------------------------------------------------------
export {
  maintenanceTasks,
  maintenanceLoaded,
  loadMaintenanceTasks,
  runMaintenanceTask,
} from "./maintenanceStore";

// ---------------------------------------------------------------------------
// Re-export: packages
// ---------------------------------------------------------------------------
export {
  packagesResult,
  packagesScanning,
  packagesScanned,
  packagesError,
  scanPackages,
} from "./packagesStore";

// ---------------------------------------------------------------------------
// Re-export: vault
// ---------------------------------------------------------------------------
export {
  vaultSummary,
  vaultEntries,
  vaultCandidates,
  vaultScanning,
  vaultCompressing,
  vaultError,
  loadVaultSummary,
  scanVaultCandidates,
  compressToVault,
  restoreFromVault,
  compressDirectoryToVault,
  collectVaultDirectory,
  deleteVaultEntry,
} from "./vaultStore";

// ---------------------------------------------------------------------------
// Scan All — orchestrates across domain stores
// ---------------------------------------------------------------------------
import { loadDiskUsage } from "./diskUsageStore";
import { scanCaches } from "./cachesStore";
import { scanLogs } from "./logsStore";
import { scanLargeFiles } from "./largeFilesStore";
import { scanApps } from "./appsStore";
import { scanBrowsers } from "./browserStore";
import { loadTrashInfo } from "./trashStore";
import { loadDockerInfo } from "./dockerStore";

export const scanAllRunning = ref(false);
export const scanAllStep = ref("");
export const scanAllDone = ref(false);
export const lastScanTime = ref<number | null>(null);

export async function scanAll() {
  if (scanAllRunning.value) return;
  scanAllRunning.value = true;
  scanAllDone.value = false;
  scanAllStep.value = "";

  try {
    scanAllStep.value = "disk_usage";
    await loadDiskUsage();

    scanAllStep.value = "caches";
    await scanCaches();

    scanAllStep.value = "logs";
    await scanLogs();

    scanAllStep.value = "large_files";
    await scanLargeFiles();

    scanAllStep.value = "apps";
    await scanApps();

    scanAllStep.value = "browsers";
    await scanBrowsers();

    scanAllStep.value = "trash";
    await loadTrashInfo();

    scanAllStep.value = "docker";
    await loadDockerInfo();

    scanAllDone.value = true;
    scanAllStep.value = "";
    lastScanTime.value = Date.now();
  } catch (e) {
    console.warn('[scanAll] unexpected error:', e);
  } finally {
    scanAllRunning.value = false;
    scanAllStep.value = "";
  }
}

// ---------------------------------------------------------------------------
// Restore all caches — called on app startup
// ---------------------------------------------------------------------------
import { restoreLargeFilesCache } from "./largeFilesStore";
import { restoreCachesCache } from "./cachesStore";
import { restoreLogsCache } from "./logsStore";
import { restoreAppsCache } from "./appsStore";
import { restoreTrashCache } from "./trashStore";
import { restoreDockerCache } from "./dockerStore";
import { restoreSecurityCache } from "./securityStore";
import { restoreBrowsersCache } from "./browserStore";
import { restoreDuplicatesCache, restoreSimilarCache } from "./duplicatesStore";
import { restorePackagesCache } from "./packagesStore";
import { listDiskMapCaches, loadDiskMapCache } from "./diskMapStore";
import { diskUsage } from "./diskUsageStore";

export async function restoreAllCaches() {
  // Restore disk usage cache
  const cachedDisk = await loadCache<DiskUsage>("disk-usage");
  if (cachedDisk) diskUsage.value = cachedDisk;

  // Restore all domain caches in parallel
  await Promise.all([
    restoreLargeFilesCache(),
    restoreCachesCache(),
    restoreLogsCache(),
    restoreAppsCache(),
    restoreTrashCache(),
    restoreDockerCache(),
    restoreSecurityCache(),
    restoreBrowsersCache(),
    restoreDuplicatesCache(),
    restoreSimilarCache(),
    restorePackagesCache(),
  ]);

  // Restore most recent disk map cache (for dashboard waffle chart)
  try {
    const caches = await listDiskMapCaches();
    if (caches.length > 0) {
      await loadDiskMapCache(caches[0].id);
    }
  } catch (e) { console.warn('[disk-map] cache restore failed:', e); }
}
