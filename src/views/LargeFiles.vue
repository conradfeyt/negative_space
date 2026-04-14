<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { FileInfo } from "../types";
import { formatSize, fileDiskSize, timeAgo, revealInFinder, getFileExtension } from "../utils";
import { showToast } from "../stores/toastStore";
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
  archiveEntries,
  setArchiveEntries,
  toggleProtected,
  isProtected,
} from "../stores/scanStore";
import EmptyState from "../components/EmptyState.vue";
import Modal from "../components/Modal.vue";
import Checkbox from "../components/Checkbox.vue";
import FileRow from "../components/FileRow.vue";
import DirTreeNode from "../components/DirTreeNode.vue";
import TabBar from "../components/TabBar.vue";
import ScanHeader from "../components/ScanHeader.vue";
import AppSelect from "../components/AppSelect.vue";
import StickyBar from "../components/StickyBar.vue";
import ChevronIcon from "../components/ChevronIcon.vue";
import CollapsibleSection from "../components/CollapsibleSection.vue";
import type { TabOption } from "../components/TabBar.vue";
import {
  useFileGrouping,
  parentFolder,
} from "../composables/useFileGrouping";
import { useScanLocations, useScanFolder } from "../composables/useScanFolder";

const { folders: scanFolders, addFolder, removeFolder } = useScanLocations("lf");
const { nativeFolderIcon } = useScanFolder("_lf_icons");

const selected = ref<Set<string>>(new Set());
const deleting = ref(false);
const minSizeMb = ref<number>(100);
const sizeOptions = [
  { value: 1, label: "1 MB" },
  { value: 5, label: "5 MB" },
  { value: 10, label: "10 MB" },
  { value: 50, label: "50 MB" },
  { value: 100, label: "100 MB" },
  { value: 500, label: "500 MB" },
  { value: 1000, label: "1 GB" },
];

/** Track which groups are collapsed */
const collapsedGroups = ref<Set<string>>(new Set(["archived"]));

const {
  sortMode,
  activeGroups,
  archivedFiles,
  archivedTotalSize,
  totalLargeFileSize,
} = useFileGrouping({
  files: largeFiles,
  getClassification: (path: string) => fileClassifications.value.get(path),
  isArchived,
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
  selected.value = new Set();
  collapsedGroups.value = new Set(["archived"]);
  await scanLargeFiles("~", Number(minSizeMb.value), scanFolders.value);
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

watch(largeFilesError, (err) => { if (err) showToast(err, "error"); });

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
    case "archived": return "badge-info";
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
    case "archived": return "Archived — do not delete";
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
    showToast(`Exported ${totalFiles} file(s) to ${path}`, "success");
  } catch (e) {
    showToast(`Export failed: ${e}`, "error");
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
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      showToast(`Deleted ${result.deleted_count} file(s), freed ${formatSize(result.freed_bytes)}`, "success");
      removeDeletedFiles(selected.value);
      // Update the on-disk cache so deleted files don't reappear on next launch
      invoke("save_scan_cache", { domain: "large-files", data: JSON.stringify(largeFiles.value) }).catch(e => console.warn('[large-files] cache save failed:', e));
      selected.value = new Set();
    }
    if (result.errors.length > 0) {
      showToast(result.errors.join("; "), "error");
    }
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    deleting.value = false;
  }
}


function isArchived(path: string): boolean {
  return getClassification(path)?.safety === "archived";
}

function isLocked(path: string): boolean {
  return isArchived(path);
}

const archiveNameMap = computed(() => {
  const map = new Map<string, string>();
  for (const entry of archiveEntries.value) {
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

function archiveDisplayName(filename: string): string {
  return archiveNameMap.value.get(filename)
    ?? archiveNameMap.value.get(filename.split(".")[0])
    ?? archiveNameMap.value.get(filename.replace(/\.tar\.zst$/, ".zst"))
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


// Preload common extensions on mount
const commonExts = ["png", "jpg", "log", "dmg", "qcow2", "jar", "a", "rlib", "so", "dylib", "img", "raw", "db", "f3d", "zip", "zst", "bin", "dill", "fst", "dat", "pack", "idx"];
for (const ext of commonExts) loadFileIcon(ext);

invoke<any[]>("get_archive_entries").then((entries) => {
  setArchiveEntries(entries);
}).catch(e => console.warn('[large-files] archive entries load failed:', e));

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
const partialSelected = computed(
  () => selected.value.size > 0 && selected.value.size < selectableCount.value
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



function isGroupAllSelected(files: FileInfo[]): boolean {
  return files.length > 0 && files.every((f) => selected.value.has(f.path));
}

function isGroupPartialSelected(files: FileInfo[]): boolean {
  const selCount = files.filter((f) => selected.value.has(f.path)).length;
  return selCount > 0 && selCount < files.length;
}

function fileSafetyLabel(path: string): string {
  return safetyLabel(getClassification(path)?.safety ?? "unknown");
}

function fileSafetyClass(path: string): string {
  return safetyBadgeClass(getClassification(path)?.safety ?? "unknown");
}

function fileSafetyTooltip(path: string): string {
  return getClassification(path)?.explanation || "";
}

function fileTimeAgo(file: FileInfo): string {
  return file.modified ? timeAgo(file.modified) : "";
}
</script>

<template>
  <div class="large-files">
    <ScanHeader
      title="Large Files"
      subtitle="Find and remove large files taking up space"
      :scanning="largeFilesScanning"
      :folders="scanFolders"
      @scan="scan"
      @add-folder="addFolder"
      @remove-folder="removeFolder"
    >
      <AppSelect v-model="minSizeMb" :options="sizeOptions" compact title="Minimum file size" />
    </ScanHeader>

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

      <StickyBar>
        <Checkbox :model-value="allSelected" :indeterminate="partialSelected" @change="toggleAll" />
        <span v-if="selected.size === 0" class="results-count">{{ largeFiles.length }} file(s) &mdash; {{ formatSize(totalLargeFileSize) }}</span>
        <span v-else-if="allSelected" class="results-count">{{ selected.size }} selected &mdash; {{ formatSize(totalSelected) }}</span>
        <span v-else class="results-count">{{ selected.size }} of {{ largeFiles.length }} selected &mdash; {{ formatSize(totalSelected) }}</span>
        <template #actions>
          <button class="btn-secondary btn-sm protect-action-btn" :disabled="selected.size === 0" @click="protectSelected" title="Toggle protection on selected files">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
            Protect
          </button>
          <button class="btn-secondary btn-sm export-btn" :disabled="selected.size === 0" @click="exportSelected">Export</button>
          <button class="btn-danger btn-sm" :disabled="selected.size === 0 || deleting" @click="deleteSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Deleting..." : "Delete Selected" }}
          </button>
        </template>
      </StickyBar>

      <div v-if="archivedFiles.length > 0" class="card-flush file-group archived-group">
        <CollapsibleSection
          :expanded="!collapsedGroups.has('archived')"
          variant="filled"
          @toggle="toggleGroup('archived')"
        >
          <template #header>
            <div class="group-header-left archived-header">
              <div class="group-title-block">
                <span class="group-title archived-title">Archived</span>
                <span class="group-meta text-muted">
                  {{ archivedFiles.length }} file(s) &middot; {{ formatSize(archivedTotalSize) }}
                </span>
              </div>
            </div>
          </template>
          <div class="group-description archived-description">
            Compressed by Negativ_ Archive. Manage from the Archive view — do not delete directly.
          </div>
          <div class="archived-file-list">
            <div v-for="file in archivedFiles" :key="file.path" class="archived-file-row">
              <span class="archived-badge">Archived</span>
              <span class="archived-file-name">{{ archiveDisplayName(file.name) }}</span>
              <span class="archived-file-size mono">{{ formatSize(diskSize(file)) }}</span>
              <button class="btn-reveal" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
              </button>
            </div>
          </div>
        </CollapsibleSection>
      </div>

      <!-- Category groups -->
      <div v-for="group in activeGroups" :key="group.id" class="card-flush file-group">

        <!-- Category header -->
        <div class="group-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has(group.id)" @click="toggleGroup(group.id)" @keydown.enter="toggleGroup(group.id)" @keydown.space.prevent="toggleGroup(group.id)">
          <div class="group-header-left">
            <ChevronIcon :expanded="!collapsedGroups.has(group.id)" variant="filled" />
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
            <FileRow
              v-for="file in group.flatFiles"
              :key="file.path"
              :file="file"
              :selected="selected.has(file.path)"
              :is-protected="isProtected(file.path)"
              :is-locked="isLocked(file.path)"
              :file-icon="getFileIcon(file.name)"
              :safety-label="fileSafetyLabel(file.path)"
              :safety-class="fileSafetyClass(file.path)"
              :safety-tooltip="fileSafetyTooltip(file.path)"
              :is-sparse="isSparse(file)"
              :disk-size="diskSize(file)"
              :parent-folder="parentFolder(file.path)"
              :time-ago="fileTimeAgo(file)"
              :is-tree="false"
              :native-folder-icon="nativeFolderIcon"
              @toggle="toggleSelect(file.path)"
              @reveal="revealInFinder(file.path)"
              @unprotect="toggleProtected(file.path)"
            />
          </div>
        </template>

        <!-- ===== DIRECTORY MODE: recursive tree ===== -->
        <template v-if="!collapsedGroups.has(group.id) && sortMode === 'directory'">
          <DirTreeNode
            v-for="child in group.tree.children"
            :key="child.key"
            :node="child"
            :depth="0"
            :collapsed-groups="collapsedGroups"
            :selected="selected"
            :get-file-icon="getFileIcon"
            :safety-label="fileSafetyLabel"
            :safety-class="fileSafetyClass"
            :safety-tooltip="fileSafetyTooltip"
            :is-protected="isProtected"
            :is-locked="isLocked"
            :is-sparse="isSparse"
            :disk-size="diskSize"
            :parent-folder="parentFolder"
            :time-ago="fileTimeAgo"
            :native-folder-icon="nativeFolderIcon"
            :is-group-all-selected="isGroupAllSelected"
            :is-group-partial-selected="isGroupPartialSelected"
            @toggle-group="toggleGroup"
            @toggle-group-select="toggleGroupSelect"
            @toggle-select="toggleSelect"
            @reveal="revealInFinder"
            @unprotect="toggleProtected"
          />

          <!-- Files at tree root level (shouldn't normally happen, but handle gracefully) -->
          <div v-if="group.tree.files.length > 0" class="file-list">
            <FileRow
              v-for="file in group.tree.files"
              :key="file.path"
              :file="file"
              :selected="selected.has(file.path)"
              :is-protected="isProtected(file.path)"
              :is-locked="isLocked(file.path)"
              :file-icon="getFileIcon(file.name)"
              :safety-label="fileSafetyLabel(file.path)"
              :safety-class="fileSafetyClass(file.path)"
              :safety-tooltip="fileSafetyTooltip(file.path)"
              :is-sparse="isSparse(file)"
              :disk-size="diskSize(file)"
              :parent-folder="parentFolder(file.path)"
              :time-ago="fileTimeAgo(file)"
              :is-tree="false"
              :native-folder-icon="nativeFolderIcon"
              @toggle="toggleSelect(file.path)"
              @reveal="revealInFinder(file.path)"
              @unprotect="toggleProtected(file.path)"
            />
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

/* ---- Sticky bar content ---- */
.results-count {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
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

.archived-group :deep(.collapsible-header) {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: 10px 14px 10px 16px;
  cursor: pointer;
  transition: background 0.15s ease;
  user-select: none;
}

.archived-group :deep(.collapsible-header:hover) {
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

/* ---- File list ---- */
.file-list {
  display: flex;
  flex-direction: column;
}


.protect-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.protect-action-btn svg {
  color: var(--protect-green);
}




.archived-group {
  padding: 4px 0;
  margin-bottom: var(--sp-4);
}

.archived-title {
  color: var(--archive-text);
}

.archived-description {
  color: var(--archive-muted);
  font-style: italic;
  font-size: 11px;
}

.archived-file-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 0;
}

.archived-file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  border-bottom: 1px solid var(--archive-tint);
}

.archived-file-row:hover {
  background: var(--archive-tint);
  opacity: 0.8;
}

.archived-badge {
  color: var(--archive);
  border-color: var(--archive-border);
  font-size: 9px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 6px;
  border: 1px solid;
  flex-shrink: 0;
  white-space: nowrap;
}

.archived-file-name {
  font-size: 12px;
  color: var(--archive-muted);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.archived-file-size {
  font-size: 12px;
  color: var(--archive-muted);
  flex-shrink: 0;
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

