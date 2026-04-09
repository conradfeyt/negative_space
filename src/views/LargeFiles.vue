<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save, open as openDialog } from "@tauri-apps/plugin-dialog";
import type { FileInfo } from "../types";
import { formatSize, fileDiskSize, timeAgo, revealInFinder, getFileExtension } from "../utils";
import {
  largeFiles,
  largeFilesScanning,
  largeFilesScanned,
  largeFilesError,
  largeFilesCurrentDir,
  scanLargeFiles,
  removeDeletedFiles,
  deleteFiles,
  fileClassifications,
  classifyFiles,
  vaultEntries,
  setVaultEntries,
  toggleProtected,
  isProtected,
} from "../stores/scanStore";
import FdaWarningBanner from "../components/FdaWarningBanner.vue";
import EmptyState from "../components/EmptyState.vue";
import Modal from "../components/Modal.vue";
import Checkbox from "../components/Checkbox.vue";
import TabBar from "../components/TabBar.vue";
import ScanBar from "../components/ScanBar.vue";
import type { TabOption } from "../components/TabBar.vue";
import {
  useFileGrouping,
  collectFiles,
  parentFolder,
} from "../composables/useFileGrouping";

const selected = ref<Set<string>>(new Set());
const deleting = ref(false);
const successMsg = ref("");
const minSizeMb = ref(100);
const scanPath = ref("~");

// Native icons for known directories
const knownDirIcons = ref<Record<string, string>>({});
async function loadKnownDirIcons() {
  const iconDefs: Record<string, { name: string; mode: string; style: string }> = {
    "~": { name: "house", mode: "sf", style: "plain" },
    "/": { name: "externaldrive", mode: "sf", style: "plain" },
    "~/Documents": { name: "doc", mode: "sf", style: "plain" },
    "~/Desktop": { name: "menubar.dock.rectangle", mode: "sf", style: "plain" },
    "~/Downloads": { name: "arrow.down.circle", mode: "sf", style: "plain" },
    "~/Library/Mobile Documents/com~apple~CloudDocs": { name: "icloud", mode: "sf", style: "plain" },
    "/Applications": { name: "square.grid.2x2", mode: "sf", style: "plain" },
    "~/Movies": { name: "film", mode: "sf", style: "plain" },
    "~/Pictures": { name: "photo", mode: "sf", style: "plain" },
    "~/Music": { name: "music.note", mode: "sf", style: "plain" },
  };
  for (const [key, def] of Object.entries(iconDefs)) {
    try {
      const b64 = await invoke<string>("render_sf_symbol", { name: def.name, size: 32, mode: def.mode, style: def.style });
      if (b64) knownDirIcons.value[key] = b64;
    } catch { /* non-critical */ }
  }
}
loadKnownDirIcons();

const scanPathIcon = computed(() => {
  const shortened = scanPath.value.replace(/^\/Users\/[^/]+/, "~");
  return knownDirIcons.value[shortened] ?? "";
});

const knownDirs: Record<string, string> = {
  "~": "Home",
  "/": "Macintosh HD",
  "~/Documents": "Documents",
  "~/Desktop": "Desktop",
  "~/Downloads": "Downloads",
  "~/Library/Mobile Documents/com~apple~CloudDocs": "iCloud Drive",
  "/Applications": "Applications",
  "~/Movies": "Movies",
  "~/Pictures": "Pictures",
  "~/Music": "Music",
};

const scanPathDisplay = computed(() => {
  const shortened = scanPath.value.replace(/^\/Users\/[^/]+/, "~");
  return knownDirs[shortened] ?? shortened;
});

async function pickScanFolder() {
  const folder = await openDialog({
    directory: true,
    multiple: false,
    title: "Choose folder to scan",
    defaultPath: scanPath.value === "~" ? undefined : scanPath.value,
  });
  if (folder) scanPath.value = folder as string;
}

/** Track which groups are collapsed */
const collapsedGroups = ref<Set<string>>(new Set(["vaulted"]));

// File grouping composable — sortMode, activeGroups, vaultedFiles, etc.
const {
  sortMode,
  activeGroups,
  vaultedFiles,
  vaultedTotalSize,
  totalLargeFileSize,
} = useFileGrouping({
  files: largeFiles,
  getClassification: (path: string) => fileClassifications.value.get(path),
  isVaulted,
});

const sortOptions = computed<TabOption[]>(() => {
  const opts: TabOption[] = [
    { value: "size", label: "Size" },
    { value: "directory", label: "Directory" },
  ];
  if (fileClassifications.value.size > 0) {
    opts.push({ value: "safety", label: "Safety" });
  }
  opts.push({ value: "type", label: "Type" });
  return opts;
});

async function scan() {
  successMsg.value = "";
  selected.value = new Set();
  collapsedGroups.value = new Set(["vaulted"]);
  await scanLargeFiles(scanPath.value, minSizeMb.value);
}

// Classify files when scan completes
watch(largeFilesScanned, (scanned) => {
  if (scanned && largeFiles.value.length > 0) {
    const files = largeFiles.value.map(f => ({
      path: f.path,
      name: f.name,
      size: f.apparent_size,
      file_type: getFileExtension(f.name),
    }));
    classifyFiles(files);
  }
});

// Also classify on mount if scan results already exist (from cache)
try {
  if (largeFilesScanned.value && largeFiles.value.length > 0 && fileClassifications.value.size === 0) {
    const files = largeFiles.value.map(f => ({
      path: f.path,
      name: f.name,
      size: f.apparent_size,
      file_type: getFileExtension(f.name),
      modified: f.modified || null,
    }));
    classifyFiles(files);
  }
} catch { /* non-critical */ }

function getClassification(path: string) {
  return fileClassifications.value.get(path);
}

function safetyBadgeClass(safety: string): string {
  switch (safety) {
    case "safe":
    case "safe_stale": return "badge-success";
    case "safe_rebuild":
    case "probably_safe": return "badge-accent";
    case "risky": return "badge-danger";
    case "vaulted": return "badge-info";
    default: return "badge-neutral";
  }
}

function safetyLabel(safety: string): string {
  switch (safety) {
    case "safe": return "Safe to delete";
    case "safe_stale": return "Safe — older than 90 days";
    case "safe_rebuild": return "Safe — recently modified";
    case "probably_safe": return "Likely safe";
    case "risky": return "Caution";
    case "vaulted": return "Vaulted — do not delete";
    default: return "";
  }
}

async function exportSelected() {
  if (selected.value.size === 0) return;
  const selectedPaths = selected.value;

  function mapFile(f: FileInfo) {
    const c = getClassification(f.path);
    return {
      path: f.path,
      name: f.name,
      size: diskSize(f),
      size_formatted: formatSize(diskSize(f)),
      apparent_size: f.apparent_size,
      is_sparse: f.is_sparse,
      modified: f.modified,
      safety: c?.safety ?? "unknown",
      explanation: c?.explanation ?? null,
    };
  }

  // Export using the same grouping as the current view
  const groups: Record<string, { label: string; files: ReturnType<typeof mapFile>[] }> = {};
  let totalFiles = 0;
  let totalSize = 0;

  for (const group of activeGroups.value) {
    const selectedInGroup = group.flatFiles.filter(f => selectedPaths.has(f.path));
    if (selectedInGroup.length === 0) continue;
    const mapped = selectedInGroup.map(mapFile);
    groups[group.id] = { label: group.label, files: mapped };
    totalFiles += mapped.length;
    totalSize += mapped.reduce((s, f) => s + f.size, 0);
  }

  const exportData = {
    exported: new Date().toISOString(),
    view_mode: sortMode.value,
    total_files: totalFiles,
    total_size: totalSize,
    total_size_formatted: formatSize(totalSize),
    groups,
  };

  const json = JSON.stringify(exportData, null, 2);
  const path = await save({
    defaultPath: `negativ-large-files-${sortMode.value}-${new Date().toISOString().slice(0, 10)}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  try {
    await invoke("export_disk_map", { data: json, path });
    successMsg.value = `Exported ${totalFiles} file(s) to ${path}`;
  } catch (e) {
    largeFilesError.value = `Export failed: ${e}`;
  }
}

function protectSelected() {
  for (const path of selected.value) {
    toggleProtected(path);
  }
  selected.value = new Set();
}

const showProtectedWarning = ref(false);

async function deleteSelected() {
  if (selected.value.size === 0) return;
  // Check if any protected files are selected
  const protectedSelected = Array.from(selected.value).filter(p => isProtected(p));
  if (protectedSelected.length > 0) {
    showProtectedWarning.value = true;
    return;
  }
  deleting.value = true;
  successMsg.value = "";
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      successMsg.value = `Deleted ${result.deleted_count} file(s), freed ${formatSize(result.freed_bytes)}`;
      removeDeletedFiles(selected.value);
      // Update the on-disk cache so deleted files don't reappear on next launch
      invoke("save_scan_cache", { domain: "large-files", data: JSON.stringify(largeFiles.value) }).catch(e => console.warn('[large-files] cache save failed:', e));
      selected.value = new Set();
    }
    if (result.errors.length > 0) {
      largeFilesError.value = result.errors.join("; ");
    }
  } catch (e) {
    largeFilesError.value = String(e);
  } finally {
    deleting.value = false;
  }
}


function isVaulted(path: string): boolean {
  return getClassification(path)?.safety === "vaulted";
}

function isLocked(path: string): boolean {
  return isVaulted(path);
}

// Map vault archive hashes back to original names
const vaultNameMap = computed(() => {
  const map = new Map<string, string>();
  for (const entry of vaultEntries.value) {
    const originalName = entry.original_path.split("/").pop() ?? entry.vault_filename;
    // Key by multiple formats to handle .zst / .tar.zst variants
    const hash = entry.vault_filename.split(".")[0];
    map.set(hash, originalName);
    map.set(entry.vault_filename, originalName);
    map.set(entry.id, originalName);
    // Also key by blake3_hash if available
    if (entry.blake3_hash) map.set(entry.blake3_hash, originalName);
  }
  return map;
});

function vaultDisplayName(filename: string): string {
  // Try full filename, then hash prefix, then strip .tar.zst/.zst
  return vaultNameMap.value.get(filename)
    ?? vaultNameMap.value.get(filename.split(".")[0])
    ?? vaultNameMap.value.get(filename.replace(/\.tar\.zst$/, ".zst"))
    ?? filename;
}

// File type icons — cached by extension
const fileIconCache = ref<Record<string, string>>({});

async function loadFileIcon(ext: string) {
  if (fileIconCache.value[ext] || ext === "") return;
  fileIconCache.value[ext] = ""; // mark as loading
  try {
    // Use a dummy path with the extension — NSWorkspace returns the icon for that file type
    const base64 = await invoke<string>("render_sf_symbol", {
      name: ext,
      size: 64,
      mode: "uttype",
      style: "plain",
    });
    if (base64) fileIconCache.value[ext] = base64;
  } catch { /* non-critical */ }
}

function getFileIcon(name: string): string {
  const ext = getFileExtension(name);
  if (!fileIconCache.value[ext] && ext) loadFileIcon(ext);
  return fileIconCache.value[ext] || "";
}

// Native macOS folder icon for reveal-in-finder buttons
const nativeFolderIcon = ref("");
invoke<string>("render_sf_symbol", { name: "public.folder", size: 32, mode: "uttype", style: "plain" })
  .then(b64 => { if (b64) nativeFolderIcon.value = b64; })
  .catch(e => console.warn('[large-files] folder icon load failed:', e));

// Preload common extensions on mount
const commonExts = ["png", "jpg", "log", "dmg", "qcow2", "jar", "a", "rlib", "so", "dylib", "img", "raw", "db", "f3d", "zip", "zst", "bin", "dill", "fst", "dat", "pack", "idx"];
for (const ext of commonExts) loadFileIcon(ext);

// Load vault manifest for name resolution
invoke<any[]>("get_vault_entries").then((entries) => {
  setVaultEntries(entries);
}).catch(e => console.warn('[large-files] vault entries load failed:', e));

function toggleSelect(path: string) {
  if (isLocked(path)) return;
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  selected.value = next;
}

function toggleAll() {
  if (allSelected.value) selected.value = new Set();
  else selected.value = new Set(largeFiles.value.filter(f => !isLocked(f.path)).map((f) => f.path));
}

function toggleGroup(groupId: string) {
  const next = new Set(collapsedGroups.value);
  if (next.has(groupId)) next.delete(groupId);
  else next.add(groupId);
  collapsedGroups.value = next;
}

function toggleGroupSelect(files: FileInfo[]) {
  const selectableFiles = files.filter(f => !isLocked(f.path));
  const paths = selectableFiles.map((f) => f.path);
  const allGroupSelected = paths.every((p) => selected.value.has(p));
  const next = new Set(selected.value);
  if (allGroupSelected) {
    paths.forEach((p) => next.delete(p));
  } else {
    paths.forEach((p) => next.add(p));
  }
  selected.value = next;
}

const selectableCount = computed(() => largeFiles.value.filter(f => !isLocked(f.path)).length);
const allSelected = computed(
  () => selectableCount.value > 0 && selected.value.size >= selectableCount.value
);

const totalSelected = computed(() =>
  largeFiles.value
    .filter((f) => selected.value.has(f.path))
    .reduce((sum, f) => sum + diskSize(f), 0)
);

function isSparse(file: FileInfo): boolean {
  return file.is_sparse && file.actual_size < file.apparent_size * 0.8;
}

const diskSize = fileDiskSize;

// File categorization, path helpers, directory tree builder, grouping computeds,
// and collectFiles are all in useFileGrouping composable.


// Sticky bar detection
const stickyBarRef = ref<HTMLElement | null>(null);
const stickyBarSentinel = ref<HTMLElement | null>(null);
const isStuck = ref(false);
let stickyObserver: IntersectionObserver | null = null;

onMounted(() => {
  if (stickyBarSentinel.value) {
    stickyObserver = new IntersectionObserver(
      ([entry]) => { isStuck.value = !entry.isIntersecting; },
      { threshold: 0 }
    );
    stickyObserver.observe(stickyBarSentinel.value);
  }
});

onUnmounted(() => {
  stickyObserver?.disconnect();
});

function isGroupAllSelected(files: FileInfo[]): boolean {
  return files.length > 0 && files.every((f) => selected.value.has(f.path));
}

function isGroupPartialSelected(files: FileInfo[]): boolean {
  const selCount = files.filter((f) => selected.value.has(f.path)).length;
  return selCount > 0 && selCount < files.length;
}
</script>

<template>
  <div class="large-files">
    <div class="view-header lf-header">
      <div>
        <h2>Large Files</h2>
        <p class="text-muted">Find and remove large files taking up space</p>
      </div>
      <ScanBar :scanning="largeFilesScanning" @scan="scan">
        <label for="lf-min-size" class="sr-only">Minimum size (MB)</label>
        <input id="lf-min-size" v-model.number="minSizeMb" type="number" min="1" max="10000" class="scan-bar-size" title="Minimum size (MB)" @blur="minSizeMb = Math.max(1, Math.min(10000, minSizeMb || 1))" />
        <span class="scan-bar-unit">MB</span>
        <span class="scan-bar-divider"></span>
        <button class="scan-bar-folder" @click="pickScanFolder" title="Choose scan folder">
          <img v-if="scanPathIcon" :src="scanPathIcon" alt="" class="scan-bar-folder-icon" />
          <img v-else-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" />
          <svg v-else width="16" height="16" viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
          <span>{{ scanPathDisplay }}</span>
        </button>
      </ScanBar>
    </div>

    <!-- FDA warning -->
    <FdaWarningBanner
      text="Without Full Disk Access, Desktop, Documents, Downloads, and other protected folders are skipped to avoid macOS permission prompts."
    />

    <div v-if="largeFilesError" class="error-message">{{ largeFilesError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <!-- Protected files warning modal -->
    <Modal :visible="showProtectedWarning" title="Protected files selected" @close="showProtectedWarning = false">
      <template #icon>
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
        </svg>
      </template>
      <p>Some of the selected files are protected. Unprotect or deselect them before deleting.</p>
      <template #actions>
        <button class="btn-primary btn-sm" @click="showProtectedWarning = false">OK</button>
      </template>
    </Modal>

    <!-- Scanning progress -->
    <div v-if="largeFilesScanning" class="scan-progress-bar">
      <div class="scan-progress-left">
        <span class="spinner-xs"></span>
        <span class="scan-progress-label">Scanning...</span>
      </div>
      <div class="scan-progress-dir mono truncate">{{ largeFilesCurrentDir }}</div>
    </div>

    <!-- Empty state -->
    <EmptyState
      v-if="!largeFilesScanning && largeFilesScanned && largeFiles.length === 0"
      :title="`No files found larger than ${minSizeMb} MB`"
      description="Try lowering the minimum size threshold or scanning a different directory."
    />

    <!-- Sort tabs -->
    <div v-if="largeFiles.length > 0" class="sort-row">
      <TabBar
        :options="sortOptions"
        v-model="sortMode"
      />
    </div>

    <!-- Results -->
    <div v-if="largeFiles.length > 0" class="results-container">

      <!-- Sentinel for sticky detection -->
      <div ref="stickyBarSentinel" class="sticky-sentinel"></div>
      <!-- Sticky action bar -->
      <div ref="stickyBarRef" class="results-sticky-bar" :class="{ 'is-stuck': isStuck }">
        <div class="summary-left">
          <Checkbox :model-value="allSelected" @change="toggleAll" />
          <span class="results-count">{{ largeFiles.length }} file(s)</span>
          <span class="results-total-size">&mdash; {{ formatSize(totalLargeFileSize) }}</span>
          <span v-if="selected.size > 0" class="selected-info">
            ({{ selected.size }} selected, {{ formatSize(totalSelected) }})
          </span>
        </div>
        <div class="results-actions">
          <button class="btn-secondary btn-sm protect-action-btn" :disabled="selected.size === 0" @click="protectSelected" title="Toggle protection on selected files">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
            Protect
          </button>
          <button class="btn-secondary btn-sm export-btn" :disabled="selected.size === 0" @click="exportSelected">Export</button>
          <button class="btn-danger btn-sm" :disabled="selected.size === 0 || deleting" @click="deleteSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Deleting..." : "Delete Selected" }}
          </button>
        </div>
      </div>

      <!-- Vaulted archives — locked section at top -->
      <div v-if="vaultedFiles.length > 0" class="card-flush file-group vault-group">
        <div class="group-header vault-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has('vaulted')" @click="toggleGroup('vaulted')" @keydown.enter="toggleGroup('vaulted')" @keydown.space.prevent="toggleGroup('vaulted')">
          <div class="group-header-left">
            <span class="expand-chevron" :class="{ expanded: !collapsedGroups.has('vaulted') }">
              <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
            </span>
            <div class="group-title-block">
              <span class="group-title vault-title">Vaulted Archives</span>
              <span class="group-meta text-muted">
                {{ vaultedFiles.length }} file(s) &middot; {{ formatSize(vaultedTotalSize) }}
              </span>
            </div>
          </div>
        </div>
        <div v-if="!collapsedGroups.has('vaulted')" class="group-description vault-description">
          Archived by Negativ_ Vault. Manage from the Vault view — do not delete directly.
        </div>
        <div v-if="!collapsedGroups.has('vaulted')" class="vault-file-list">
          <div v-for="file in vaultedFiles" :key="file.path" class="vault-file-row">
            <span class="vault-badge">Vaulted</span>
            <span class="vault-file-name">{{ vaultDisplayName(file.name) }}</span>
            <span class="vault-file-size mono">{{ formatSize(diskSize(file)) }}</span>
            <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
              <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
            </button>
          </div>
        </div>
      </div>

      <!-- Category groups -->
      <div v-for="group in activeGroups" :key="group.id" class="card-flush file-group">

        <!-- Category header -->
        <div class="group-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has(group.id)" @click="toggleGroup(group.id)" @keydown.enter="toggleGroup(group.id)" @keydown.space.prevent="toggleGroup(group.id)">
          <div class="group-header-left">
            <span class="expand-chevron" :class="{ expanded: !collapsedGroups.has(group.id) }">
              <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
            </span>
            <div class="group-title-block">
              <span class="group-title">{{ group.label }}</span>
              <span class="group-meta text-muted">
                {{ group.totalFiles }} file(s) &middot; {{ formatSize(group.totalSize) }}
              </span>
            </div>
          </div>
          <Checkbox
            :model-value="isGroupAllSelected(group.flatFiles)"
            :indeterminate="isGroupPartialSelected(group.flatFiles)"
            @change="toggleGroupSelect(group.flatFiles)"
          />
        </div>

        <div v-if="!collapsedGroups.has(group.id)" class="group-description text-muted">
          {{ group.description }}
        </div>

        <!-- ===== SIZE MODE: flat file list ===== -->
        <template v-if="!collapsedGroups.has(group.id) && sortMode !== 'directory'">
          <div class="file-list">
            <div
              v-for="file in group.flatFiles"
              :key="file.path"
              class="file-row"
              :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
              @click="toggleSelect(file.path)"
            >
              <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <span v-if="isProtected(file.path)" class="icon-shield-badge" title="Click to unprotect" @click.stop="toggleProtected(file.path)">
                  <svg class="shield-normal" viewBox="0 0 24 24" fill="var(--protect-green)" stroke="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                  <svg class="shield-hover" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="4.5" y1="4.5" x2="19.5" y2="19.5" stroke="var(--danger)" stroke-width="2.5"/></svg>
                </span>
              </div>
              <div class="file-row-info">
                <div class="file-row-name">
                  <span class="file-name">{{ file.name }}</span>
                  <span v-if="isSparse(file)" class="badge badge-warning pill sparse-badge">Sparse</span>
                </div>
                <div class="file-row-path">{{ parentFolder(file.path) }}</div>
              </div>
              <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="badge safety-pill" :class="safetyBadgeClass(getClassification(file.path)?.safety ?? 'unknown')" :data-tooltip="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
              <div class="file-row-size mono">
                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
              </div>
              <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
              </button>
              <div class="file-row-check">
                <Checkbox v-if="!isLocked(file.path)" :model-value="selected.has(file.path)" @change="toggleSelect(file.path)" />
              </div>
            </div>
          </div>
        </template>

        <!-- ===== DIRECTORY MODE: recursive tree ===== -->
        <template v-if="!collapsedGroups.has(group.id) && sortMode === 'directory'">
          <!-- Render top-level children of the tree root (root itself is a virtual node) -->
          <template v-for="child in group.tree.children" :key="child.key">
            <div class="dir-tree" :style="{ '--depth': 0 }">
              <!-- Recursive directory node rendering -->
              <div class="dir-node">
                <div class="dir-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has(child.key)" @click="toggleGroup(child.key)" @keydown.enter="toggleGroup(child.key)" @keydown.space.prevent="toggleGroup(child.key)">
                  <div class="dir-header-left">
                    <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(child.key) }">
                      <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                    </span>
                    <span class="dir-header-icon">
                      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                      </svg>
                    </span>
                    <span class="dir-path mono">{{ child.path || child.name }}</span>
                  </div>
                  <div class="dir-header-right">
                    <span class="dir-meta text-muted">{{ child.totalFiles }} file(s)</span>
                    <span class="dir-size mono">{{ formatSize(child.totalSize) }}</span>
                    <Checkbox
                      :model-value="isGroupAllSelected(collectFiles(child))"
                      :indeterminate="isGroupPartialSelected(collectFiles(child))"
                      @change="toggleGroupSelect(collectFiles(child))"
                    />
                  </div>
                </div>

                <template v-if="!collapsedGroups.has(child.key)">
                  <!-- Sub-directories (depth 1) -->
                  <template v-for="d1 in child.children" :key="d1.key">
                    <div class="dir-tree" :style="{ '--depth': 1 }">
                      <div class="dir-node">
                        <div class="dir-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has(d1.key)" @click="toggleGroup(d1.key)" @keydown.enter="toggleGroup(d1.key)" @keydown.space.prevent="toggleGroup(d1.key)">
                          <div class="dir-header-left">
                            <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(d1.key) }">
                              <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                            </span>
                            <span class="dir-header-icon">
                              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                              </svg>
                            </span>
                            <span class="dir-path mono">{{ d1.name }}</span>
                          </div>
                          <div class="dir-header-right">
                            <span class="dir-meta text-muted">{{ d1.totalFiles }} file(s)</span>
                            <span class="dir-size mono">{{ formatSize(d1.totalSize) }}</span>
                            <Checkbox
                              :model-value="isGroupAllSelected(collectFiles(d1))"
                              :indeterminate="isGroupPartialSelected(collectFiles(d1))"
                              @change="toggleGroupSelect(collectFiles(d1))"
                            />
                          </div>
                        </div>

                        <template v-if="!collapsedGroups.has(d1.key)">
                          <!-- Sub-directories (depth 2) -->
                          <template v-for="d2 in d1.children" :key="d2.key">
                            <div class="dir-tree" :style="{ '--depth': 2 }">
                              <div class="dir-node">
                                <div class="dir-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has(d2.key)" @click="toggleGroup(d2.key)" @keydown.enter="toggleGroup(d2.key)" @keydown.space.prevent="toggleGroup(d2.key)">
                                  <div class="dir-header-left">
                                    <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(d2.key) }">
                                      <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                                    </span>
                                    <span class="dir-header-icon">
                                      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                                      </svg>
                                    </span>
                                    <span class="dir-path mono">{{ d2.name }}</span>
                                  </div>
                                  <div class="dir-header-right">
                                    <span class="dir-meta text-muted">{{ d2.totalFiles }} file(s)</span>
                                    <span class="dir-size mono">{{ formatSize(d2.totalSize) }}</span>
                                    <Checkbox
                                      :model-value="isGroupAllSelected(collectFiles(d2))"
                                      :indeterminate="isGroupPartialSelected(collectFiles(d2))"
                                      @change="toggleGroupSelect(collectFiles(d2))"
                                    />
                                  </div>
                                </div>

                                <template v-if="!collapsedGroups.has(d2.key)">
                                  <!-- Deeper levels: just show files flat -->
                                  <template v-for="d3 in d2.children" :key="d3.key">
                                    <div class="dir-tree" :style="{ '--depth': 3 }">
                                      <div class="dir-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has(d3.key)" @click="toggleGroup(d3.key)" @keydown.enter="toggleGroup(d3.key)" @keydown.space.prevent="toggleGroup(d3.key)">
                                        <div class="dir-header-left">
                                          <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(d3.key) }">
                                            <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                                          </span>
                                          <span class="dir-header-icon">
                                            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                              <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                                            </svg>
                                          </span>
                                          <span class="dir-path mono">{{ d3.name }}</span>
                                        </div>
                                        <div class="dir-header-right">
                                          <span class="dir-meta text-muted">{{ d3.totalFiles }} file(s)</span>
                                          <span class="dir-size mono">{{ formatSize(d3.totalSize) }}</span>
                                          <Checkbox
                                            :model-value="isGroupAllSelected(collectFiles(d3))"
                                            :indeterminate="isGroupPartialSelected(collectFiles(d3))"
                                            @change="toggleGroupSelect(collectFiles(d3))"
                                          />
                                        </div>
                                      </div>
                                      <!-- Files inside depth-3+ dirs shown flat -->
                                      <div v-if="!collapsedGroups.has(d3.key)" class="file-list file-list-indented">
                                        <div
                                          v-for="file in collectFiles(d3)"
                                          :key="file.path"
                                          class="file-row file-row--tree"
                                          :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                                          @click="toggleSelect(file.path)"
                                        >
                                          <div class="file-row-check">
                                            <Checkbox v-if="!isLocked(file.path)" :model-value="selected.has(file.path)" @change="toggleSelect(file.path)" />
                                          </div>
                                          <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="var(--protect-green)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              </div>
              <div class="file-row-info">
                                            <div class="file-row-name">
                                              <span class="file-name">{{ file.name }}</span>
                                              <span v-if="isSparse(file)" class="badge badge-warning pill sparse-badge">Sparse</span>
                                            </div>
                                          </div>
                                          <div class="file-row-size mono">
                                            <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                                            <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                                          </div>
                                          <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="badge safety-pill" :class="safetyBadgeClass(getClassification(file.path)?.safety ?? 'unknown')" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                                          <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                            <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
                                          </button>
                                        </div>
                                      </div>
                                    </div>
                                  </template>

                                  <!-- Files at depth 2 -->
                                  <div v-if="d2.files.length > 0" class="file-list file-list-indented">
                                    <div
                                      v-for="file in d2.files"
                                      :key="file.path"
                                      class="file-row file-row--tree"
                                      :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                                      @click="toggleSelect(file.path)"
                                    >
                                      <div class="file-row-check">
                                        <Checkbox v-if="!isLocked(file.path)" :model-value="selected.has(file.path)" @change="toggleSelect(file.path)" />
                                      </div>
                                      <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="var(--protect-green)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              </div>
              <div class="file-row-info">
                                        <div class="file-row-name">
                                          <span class="file-name">{{ file.name }}</span>
                                          <span v-if="isSparse(file)" class="badge badge-warning pill sparse-badge">Sparse</span>
                                        </div>
                                      </div>
                                      <div class="file-row-size mono">
                                        <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                                        <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                                      </div>
                                      <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="badge safety-pill" :class="safetyBadgeClass(getClassification(file.path)?.safety ?? 'unknown')" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                                      <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                        <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
                                      </button>
                                    </div>
                                  </div>
                                </template>
                              </div>
                            </div>
                          </template>

                          <!-- Files at depth 1 -->
                          <div v-if="d1.files.length > 0" class="file-list file-list-indented">
                            <div
                              v-for="file in d1.files"
                              :key="file.path"
                              class="file-row file-row--tree"
                              :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                              @click="toggleSelect(file.path)"
                            >
                              <div class="file-row-check">
                                <Checkbox v-if="!isLocked(file.path)" :model-value="selected.has(file.path)" @change="toggleSelect(file.path)" />
                              </div>
                              <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="var(--protect-green)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              </div>
              <div class="file-row-info">
                                <div class="file-row-name">
                                  <span class="file-name">{{ file.name }}</span>
                                  <span v-if="isSparse(file)" class="badge badge-warning pill sparse-badge">Sparse</span>
                                </div>
                              </div>
                              <div class="file-row-size mono">
                                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                              </div>
                              <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="badge safety-pill" :class="safetyBadgeClass(getClassification(file.path)?.safety ?? 'unknown')" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                              <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
                              </button>
                            </div>
                          </div>
                        </template>
                      </div>
                    </div>
                  </template>

                  <!-- Files at depth 0 (directly under top-level dir) -->
                  <div v-if="child.files.length > 0" class="file-list file-list-indented">
                    <div
                      v-for="file in child.files"
                      :key="file.path"
                      class="file-row file-row--tree"
                      :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                      @click="toggleSelect(file.path)"
                    >
                      <div class="file-row-check">
                        <Checkbox v-if="!isLocked(file.path)" :model-value="selected.has(file.path)" @change="toggleSelect(file.path)" />
                      </div>
                      <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="var(--protect-green)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              </div>
              <div class="file-row-info">
                        <div class="file-row-name">
                          <span class="file-name">{{ file.name }}</span>
                          <span v-if="isSparse(file)" class="badge badge-warning pill sparse-badge">Sparse</span>
                        </div>
                      </div>
                      <div class="file-row-size mono">
                        <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                        <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                      </div>
                      <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="badge safety-pill" :class="safetyBadgeClass(getClassification(file.path)?.safety ?? 'unknown')" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                      <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                        <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
                      </button>
                    </div>
                  </div>
                </template>
              </div>
            </div>
          </template>

          <!-- Files at tree root level (shouldn't normally happen, but handle gracefully) -->
          <div v-if="group.tree.files.length > 0" class="file-list">
            <div
              v-for="file in group.tree.files"
              :key="file.path"
              class="file-row"
              :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
              @click="toggleSelect(file.path)"
            >
              <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <span v-if="isProtected(file.path)" class="icon-shield-badge" title="Click to unprotect" @click.stop="toggleProtected(file.path)">
                  <svg class="shield-normal" viewBox="0 0 24 24" fill="var(--protect-green)" stroke="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                  <svg class="shield-hover" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="4.5" y1="4.5" x2="19.5" y2="19.5" stroke="var(--danger)" stroke-width="2.5"/></svg>
                </span>
              </div>
              <div class="file-row-info">
                <div class="file-row-name">
                  <span class="file-name">{{ file.name }}</span>
                  <span v-if="isSparse(file)" class="badge badge-warning pill sparse-badge">Sparse</span>
                </div>
                <div class="file-row-path">{{ parentFolder(file.path) }}</div>
              </div>
              <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="badge safety-pill" :class="safetyBadgeClass(getClassification(file.path)?.safety ?? 'unknown')" :data-tooltip="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
              <div class="file-row-size mono">
                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
              </div>
              <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
              </button>
              <div class="file-row-check">
                <Checkbox v-if="!isLocked(file.path)" :model-value="selected.has(file.path)" @change="toggleSelect(file.path)" />
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.large-files { max-width: 1440px; }
.controls-card { margin-bottom: var(--sp-6); }
.controls-row { display: flex; align-items: flex-end; gap: var(--sp-4); }
.control-group { display: flex; flex-direction: column; gap: var(--sp-1); }
.control-label { font-size: 12px; font-weight: 400; color: var(--muted); }
/* ---- Scan bar slot content ---- */
.scan-bar-size {
  width: 80px;
  border: none !important;
  background: transparent !important;
  box-shadow: none !important;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  padding: 4px 0;
  outline: none;
  font-family: var(--font-mono);
  text-align: right;
  border-radius: 0;
  -moz-appearance: textfield;
}

.scan-bar-size::-webkit-inner-spin-button,
.scan-bar-size::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.scan-bar-unit {
  font-size: 12px;
  color: var(--muted);
  margin-right: 8px;
  flex-shrink: 0;
}

.scan-bar-divider {
  width: 1px;
  height: 18px;
  background: rgba(0, 0, 0, 0.12);
  margin: 0 8px;
  flex-shrink: 0;
}

.scan-bar-folder {
  display: flex;
  align-items: center;
  gap: 5px;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 14px;
  transition: background 0.15s;
  flex: 1;
  min-width: 0;
}

.scan-bar-folder:hover {
  background: rgba(0, 0, 0, 0.06);
}

.scan-bar-folder-icon {
  height: 16px;
  width: auto;
  flex-shrink: 0;
}

/* ---- Scanning progress bar ---- */
.scan-progress-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
  padding: 10px var(--sp-5);
  margin-bottom: var(--sp-5);
  border-radius: var(--radius-sm);
  background: var(--info-tint);
  border: 1px solid rgba(20, 138, 160, 0.10);
  animation: fadeIn 0.2s ease;
}

.scan-progress-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.scan-progress-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--info-text);
}

.scan-progress-dir {
  font-size: 11px;
  color: var(--text-secondary);
  text-align: right;
  min-width: 0;
}

/* ---- Sort row ---- */
.sort-row {
  margin-bottom: var(--sp-4);
}

/* ---- Sticky action bar ---- */
.sticky-sentinel {
  height: 0;
  margin: 0;
  padding: 0;
}

.results-sticky-bar {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  margin-bottom: var(--sp-4);
  transition: background 0.2s, border-color 0.2s;
  border-bottom: 1px solid transparent;
}

.results-sticky-bar.is-stuck {
  margin-left: -40px;
  margin-right: -40px;
  width: calc(100% + 80px);
  padding: 10px 40px;
  background: rgba(235, 232, 238, 0.92);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom-color: rgba(0, 0, 0, 0.06);
}

.summary-left {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.results-count {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.results-total-size {
  font-size: 13px;
  color: var(--muted);
}

.results-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}


/* ---- Sort toggle ---- */

/* ---- Category group ---- */
.file-group {
  margin-bottom: var(--sp-4);
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px 10px 16px;
  cursor: pointer;
  transition: background 0.15s ease;
  user-select: none;
}

.group-header:hover {
  background: rgba(255, 255, 255, 0.2);
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.group-title-block {
  display: flex;
  align-items: baseline;
  gap: var(--sp-3);
}

.group-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: -0.2px;
}

.group-meta {
  font-size: 12px;
}


.group-description {
  font-size: 11px;
  padding: 0 16px 8px 16px;
  color: rgba(0, 0, 0, 0.85);
}

/* ---- Directory tree ---- */
.dir-tree {
  padding-left: calc(var(--depth, 0) * 20px);
}

.dir-node {
  margin-bottom: 1px;
}

.dir-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 14px 6px 12px;
  cursor: pointer;
  transition: background 0.15s ease;
  user-select: none;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
}

.dir-header:hover {
  background: rgba(255, 255, 255, 0.15);
}

.dir-header-left {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  min-width: 0;
}

.dir-header-icon {
  color: var(--muted);
  flex-shrink: 0;
  display: flex;
}

.dir-path {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.dir-meta {
  justify-self: end;
}

.dir-size {
  justify-self: end;
}


.dir-meta {
  font-size: 11px;
  white-space: nowrap;
}

.dir-size {
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
}

.expand-chevron--sm svg {
  width: 10px;
  height: 10px;
}

/* ---- File list ---- */
.file-list {
  display: flex;
  flex-direction: column;
}

.file-row {
  display: grid;
  grid-template-columns: 32px 1fr 160px 100px 24px 28px;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  cursor: pointer;
  transition: background 0.12s ease;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.file-row:last-child {
  border-bottom: none;
}

.file-row--tree {
  grid-template-columns: 32px 1fr 100px 24px 28px;
  padding-left: 24px;
}

.file-row--tree .file-icon-wrap {
  grid-column: 1;
}

.file-row--tree .file-row-info {
  grid-column: 2;
}

.file-row--tree .file-row-size {
  grid-column: 3;
}

.file-row--tree .safety-pill,
.file-row--tree .safety-pill-placeholder {
  display: none;
}

.file-row--tree .btn-reveal {
  grid-column: 4;
}

.file-row--tree .file-row-check {
  grid-column: 5;
  grid-row: 1;
}

.file-row:hover {
  background: rgba(255, 255, 255, 0.2);
}

.file-row--selected {
  background: rgba(2, 117, 244, 0.06);
}

.file-row--selected:hover {
  background: rgba(2, 117, 244, 0.10);
}

.file-row-check {
  display: flex;
  align-items: center;
  justify-content: center;
  grid-column: 6;
}

.file-row-info {
  min-width: 0;
  grid-column: 2;
}

.file-icon-wrap {
  position: relative;
  grid-column: 1;
  width: 32px;
  height: 32px;
}

.file-row-icon {
  border-radius: 3px;
}

.file-row-icon-placeholder {
  width: 32px;
  height: 32px;
}

.icon-shield-badge {
  position: absolute;
  top: -3px;
  right: -5px;
  width: 14px;
  height: 14px;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.2));
  cursor: pointer;
  transition: transform 0.1s;
}

.icon-shield-badge svg {
  width: 100%;
  height: 100%;
}

.icon-shield-badge .shield-hover {
  display: none;
}

.icon-shield-badge:hover {
  transform: scale(1.3);
}

.icon-shield-badge:hover .shield-normal {
  display: none;
}

.icon-shield-badge:hover .shield-hover {
  display: block;
}

.file-row--protected .file-name,
.file-row--protected .file-row-path {
  color: var(--green);
}

.protect-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.protect-action-btn svg {
  color: var(--protect-green);
}

.file-row-name {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.file-name {
  font-family: var(--font-sans); font-size: 13px;
  font-weight: 500;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sparse-badge {
  flex-shrink: 0;
}

/* Header with inline controls */
.lf-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}


/* Vault section */
.vault-group {
  padding: 4px 0;
  margin-bottom: var(--sp-4);
}

.vault-title {
  color: var(--vault-text);
}

.vault-description {
  color: var(--vault-muted);
  font-style: italic;
  font-size: 11px;
}

.vault-file-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 0;
}

.vault-file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  border-bottom: 1px solid var(--vault-tint);
}

.vault-file-row:hover {
  background: var(--vault-tint);
  opacity: 0.8;
}

.vault-badge {
  color: var(--vault);
  border-color: var(--vault-border);
  font-size: 9px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 6px;
  border: 1px solid;
  flex-shrink: 0;
  white-space: nowrap;
}

.vault-file-name {
  font-size: 12px;
  color: var(--vault-muted);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vault-file-size {
  font-size: 12px;
  color: var(--vault-muted);
  flex-shrink: 0;
}

.safety-pill {
  grid-column: 3;
  justify-self: end;
  position: relative;
  cursor: default;
}

.safety-pill[data-tooltip]:not([data-tooltip=""])::after {
  content: attr(data-tooltip);
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  background: white;
  color: rgba(0, 0, 0, 0.8);
  font-size: 11px;
  font-weight: 400;
  padding: 8px 12px;
  border-radius: 10px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12), 0 0 0 0.5px rgba(0, 0, 0, 0.06);
  white-space: normal;
  min-width: 200px;
  max-width: 320px;
  line-height: 1.4;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.1s ease;
  z-index: 100;
}

.safety-pill[data-tooltip]:not([data-tooltip=""]):hover::after {
  opacity: 1;
}

.safety-pill[data-tooltip]:not([data-tooltip=""])::before {
  content: '';
  position: absolute;
  bottom: calc(100% + 2px);
  left: 50%;
  transform: translateX(-50%);
  border: 6px solid transparent;
  border-top-color: white;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.1s ease;
  z-index: 101;
}

.safety-pill[data-tooltip]:not([data-tooltip=""]):hover::before {
  opacity: 1;
}

.safety-pill-placeholder {
  grid-column: 3;
}

.file-row-explanation {
  font-size: 11px;
  margin-top: 2px;
  line-height: 1.3;
  color: rgba(0, 0, 0, 0.45);
}

.file-row-path {
  font-family: var(--font-sans); font-size: 10px;
  margin-top: 1px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgba(0, 0, 0, 0.85);
}

.file-row-size {
  grid-column: 4;
  text-align: right;
  white-space: nowrap;
  font-size: 13px;
}

.size-value {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.sparse-logical {
  display: block;
  font-size: 10px;
  margin-top: 1px;
}

.file-time-ago {
  display: block;
  font-size: 10px;
  margin-top: 1px;
}

/* ---- Protect toggle button ---- */
.protect-btn {
  grid-column: 6;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--vault);
  opacity: 0;
  cursor: pointer;
  flex-shrink: 0;
  transition: opacity 0.15s, color 0.15s;
}

.file-row:hover .protect-btn {
  opacity: 0.7;
}

.protect-btn svg {
  width: 16px;
  height: 16px;
}

.protect-btn:hover {
  opacity: 0.8;
}

.protect-btn--active {
  color: var(--vault);
  opacity: 0.85;
}

.protect-btn--active:hover {
  opacity: 1;
}

/* ---- Reveal in Finder button (LargeFiles-specific grid + hover-reveal) ---- */
.btn-reveal {
  grid-column: 5;
  opacity: 0;
}

.file-row:hover .btn-reveal {
  opacity: 0.8;
}

.file-row .btn-reveal:hover {
  opacity: 1;
}

.file-list-indented { padding-left: var(--sp-5); }

/* Screen-reader-only: visually hidden but accessible to assistive tech */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>

