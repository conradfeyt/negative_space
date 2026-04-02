<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize } from "../utils";
import {
  duplicateResult,
  duplicateScanning,
  duplicateScanned,
  duplicateError,
  scanDuplicates,
  deleteFiles,
  hasFullDiskAccess,
  checkFullDiskAccess,
  previewFile,
} from "../stores/scanStore";
import type { DuplicateGroup, FilePreview } from "../types";

// Track which groups are expanded
const expandedGroups = ref<Set<string>>(new Set());

// Track selected files for deletion: Set of file paths
const selected = ref<Set<string>>(new Set());

// Min size filter
const minSizeMb = ref(1);

// Cleaning state
const cleaning = ref(false);
const successMsg = ref("");
const cleanError = ref("");

// Preview state
const previewData = ref<FilePreview | null>(null);
const previewLoading = ref(false);
const previewPath = ref<string | null>(null);

async function openFdaSettings() {
  try {
    await invoke("open_full_disk_access_settings");
  } catch (_) {}
}

async function recheckFda() {
  await checkFullDiskAccess();
}

async function scan() {
  successMsg.value = "";
  cleanError.value = "";
  selected.value = new Set();
  expandedGroups.value = new Set();
  await scanDuplicates("~", minSizeMb.value);
}

function toggleGroup(hash: string) {
  const next = new Set(expandedGroups.value);
  if (next.has(hash)) {
    next.delete(hash);
  } else {
    next.add(hash);
  }
  expandedGroups.value = next;
}

function toggleFile(path: string) {
  const next = new Set(selected.value);
  if (next.has(path)) {
    next.delete(path);
  } else {
    next.add(path);
  }
  selected.value = next;
}

/** Select all files in a group EXCEPT the first one (keep one copy) */
function selectDuplicates(group: DuplicateGroup) {
  const next = new Set(selected.value);
  // Skip the first file (the "original"), select the rest
  for (let i = 1; i < group.files.length; i++) {
    next.add(group.files[i].path);
  }
  selected.value = next;
}

/** Deselect all files in a group */
function deselectGroup(group: DuplicateGroup) {
  const next = new Set(selected.value);
  for (const f of group.files) {
    next.delete(f.path);
  }
  selected.value = next;
}

/** Check if all duplicates (all except first) are selected */
function allDuplicatesSelected(group: DuplicateGroup): boolean {
  if (group.files.length < 2) return false;
  for (let i = 1; i < group.files.length; i++) {
    if (!selected.value.has(group.files[i].path)) return false;
  }
  return true;
}

const totalSelected = computed(() => {
  if (!duplicateResult.value) return 0;
  let total = 0;
  for (const group of duplicateResult.value.groups) {
    for (const f of group.files) {
      if (selected.value.has(f.path)) {
        total += f.size;
      }
    }
  }
  return total;
});

async function deleteSelected() {
  if (selected.value.size === 0) return;
  cleaning.value = true;
  cleanError.value = "";
  successMsg.value = "";
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      successMsg.value = `Deleted ${result.deleted_count} duplicate(s), freed ${formatSize(result.freed_bytes)}`;
    }
    if (result.errors.length > 0) {
      cleanError.value = result.errors.join("; ");
    }
    // Re-scan to show updated results
    selected.value = new Set();
    expandedGroups.value = new Set();
    await scanDuplicates("~", minSizeMb.value);
  } catch (e) {
    cleanError.value = String(e);
  } finally {
    cleaning.value = false;
  }
}

/** Load a preview for a file */
async function loadPreview(path: string, event: Event) {
  event.stopPropagation();
  // If clicking the same file, toggle off
  if (previewPath.value === path) {
    closePreview();
    return;
  }
  previewPath.value = path;
  previewLoading.value = true;
  previewData.value = null;
  try {
    previewData.value = await previewFile(path);
  } catch (_) {
    previewData.value = {
      kind: "Error",
      message: "Failed to generate preview",
      file_name: path.split("/").pop() || path,
    };
  } finally {
    previewLoading.value = false;
  }
}

function closePreview() {
  previewData.value = null;
  previewPath.value = null;
  previewLoading.value = false;
}

/** Get a syntax hint for code highlighting styling (just extension label) */
function extLabel(fileType: string): string {
  if (!fileType) return "";
  return fileType.toUpperCase();
}

/** Shorten a path for display: replace home dir with ~ */
function shortPath(p: string): string {
  const home = p.match(/^\/Users\/[^/]+/)?.[0];
  if (home) return p.replace(home, "~");
  return p;
}
</script>

<template>
  <div class="duplicates-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Duplicate Finder</h2>
          <p class="text-muted">
            Find identical files wasting disk space
          </p>
        </div>
        <div class="scan-controls">
          <div class="min-size-control">
            <label class="text-muted">Min size</label>
            <select v-model.number="minSizeMb" class="size-select">
              <option :value="0">1 KB</option>
              <option :value="1">1 MB</option>
              <option :value="5">5 MB</option>
              <option :value="10">10 MB</option>
              <option :value="50">50 MB</option>
              <option :value="100">100 MB</option>
            </select>
          </div>
          <button
            class="btn-primary scan-btn"
            :disabled="duplicateScanning"
            @click="scan"
          >
            <span v-if="duplicateScanning" class="spinner spinner-sm"></span>
            {{ duplicateScanning ? "Scanning..." : "Find Duplicates" }}
          </button>
        </div>
      </div>
    </div>

    <!-- FDA warning -->
    <div v-if="hasFullDiskAccess === false" class="fda-warning-banner">
      <span class="fda-warning-dot"></span>
      <div class="fda-warning-body">
        <div class="fda-warning-title">
          Limited scan -- Full Disk Access required for full coverage
        </div>
        <div class="fda-warning-text">
          Without Full Disk Access, only developer tools, package managers, and
          project directories are scanned. Desktop, Documents, Downloads, and
          most of ~/Library are skipped.
        </div>
        <div class="fda-warning-actions">
          <button class="btn-fda btn-fda-primary" @click="openFdaSettings">
            Open System Settings
          </button>
          <button class="btn-fda btn-fda-secondary" @click="recheckFda">
            Re-check
          </button>
        </div>
      </div>
    </div>

    <!-- Messages -->
    <div v-if="duplicateError" class="error-message">{{ duplicateError }}</div>
    <div v-if="cleanError" class="error-message">{{ cleanError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <!-- Loading -->
    <div v-if="duplicateScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning for duplicates... this may take a while</span>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="duplicateScanned && (!duplicateResult || duplicateResult.groups.length === 0)"
      class="card empty-state"
    >
      <p class="text-muted">No duplicate files found</p>
      <p v-if="duplicateResult" class="text-muted scan-stats">
        Scanned {{ duplicateResult.files_scanned.toLocaleString() }} files
      </p>
    </div>

    <!-- Results -->
    <template v-else-if="duplicateResult && duplicateResult.groups.length > 0">
      <!-- Scan stats -->
      <div class="stats-bar">
        <div class="stat">
          <span class="stat-value">
            {{ duplicateResult.files_scanned.toLocaleString() }}
          </span>
          <span class="stat-label">files scanned</span>
        </div>
        <div class="stat">
          <span class="stat-value">
            {{ duplicateResult.groups.length }}
          </span>
          <span class="stat-label">duplicate groups</span>
        </div>
        <div class="stat">
          <span class="stat-value">
            {{ duplicateResult.total_duplicate_files }}
          </span>
          <span class="stat-label">duplicate files</span>
        </div>
        <div class="stat stat-highlight">
          <span class="stat-value">
            {{ formatSize(duplicateResult.total_wasted_bytes) }}
          </span>
          <span class="stat-label">wasted space</span>
        </div>
      </div>

      <!-- Skipped paths -->
      <div
        v-if="duplicateResult.skipped_paths.length > 0"
        class="skipped-note text-muted"
      >
        Skipped (no FDA):
        {{ duplicateResult.skipped_paths.join(", ") }}
      </div>

      <!-- Action bar -->
      <div class="summary-bar">
        <div class="results-actions">
          <span v-if="selected.size > 0" class="selected-info">
            {{ selected.size }} file(s) selected ({{ formatSize(totalSelected) }})
          </span>
          <button
            class="btn-danger"
            :disabled="selected.size === 0 || cleaning"
            @click="deleteSelected"
          >
            <span v-if="cleaning" class="spinner spinner-sm"></span>
            {{ cleaning ? "Deleting..." : "Delete Selected" }}
          </button>
        </div>
      </div>

      <!-- Duplicate groups -->
      <div class="group-list">
        <div
          v-for="group in duplicateResult.groups"
          :key="group.hash"
          class="card-flush group-card"
        >
          <!-- Group header -->
          <div class="group-header" @click="toggleGroup(group.hash)">
            <div class="group-header-left">
              <span
                class="expand-chevron"
                :class="{ expanded: expandedGroups.has(group.hash) }"
              ><svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg></span>
              <div class="group-info">
                <span class="group-name">
                  {{ group.files[0].name }}
                  <span class="text-muted group-count">
                    ({{ group.files.length }} copies)
                  </span>
                </span>
                <span class="group-meta text-muted">
                  {{ formatSize(group.size) }} each --
                  {{ formatSize(group.wasted_bytes) }} wasted
                </span>
              </div>
            </div>
            <div class="group-header-right" @click.stop>
              <button
                v-if="!allDuplicatesSelected(group)"
                class="btn-sm btn-secondary"
                @click="selectDuplicates(group)"
                title="Select all copies except the first (keeps one)"
              >
                Select duplicates
              </button>
              <button
                v-else
                class="btn-sm btn-secondary"
                @click="deselectGroup(group)"
              >
                Deselect all
              </button>
            </div>
          </div>

          <!-- File list (expanded) -->
          <div v-if="expandedGroups.has(group.hash)" class="group-files">
            <div
              v-for="(file, idx) in group.files"
              :key="file.path"
              :class="[
                'file-row',
                { selected: selected.has(file.path), 'is-original': idx === 0 },
              ]"
              @click="toggleFile(file.path)"
            >
              <div class="file-row-left">
                <input
                  type="checkbox"
                  :checked="selected.has(file.path)"
                  @change.stop="toggleFile(file.path)"
                  @click.stop
                />
                <div class="file-info">
                  <div class="file-name-row">
                    <span class="file-name">{{ file.name }}</span>
                    <span v-if="idx === 0" class="badge badge-accent">
                      Keep
                    </span>
                  </div>
                  <span class="file-path mono truncate" :title="file.path">
                    {{ shortPath(file.parent_dir) }}
                  </span>
                </div>
              </div>
              <div class="file-row-right">
                <button
                  :class="['btn-preview', { active: previewPath === file.path }]"
                  @click="loadPreview(file.path, $event)"
                  title="Preview this file"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                    <circle cx="12" cy="12" r="3"/>
                  </svg>
                </button>
                <span class="file-modified text-muted">{{ file.modified }}</span>
              </div>
            </div>

            <!-- Inline preview panel (shows below the file list within the group) -->
            <div
              v-if="previewPath && group.files.some(f => f.path === previewPath)"
              class="preview-panel"
            >
              <div class="preview-header">
                <span class="preview-title">
                  {{ previewData?.kind !== 'Error' ? (previewData as any)?.file_name || '...' : (previewData as any)?.file_name || '...' }}
                </span>
                <button class="btn-preview-close" @click="closePreview">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                    <path d="M18 6 6 18M6 6l12 12"/>
                  </svg>
                </button>
              </div>

              <!-- Loading state -->
              <div v-if="previewLoading" class="preview-loading">
                <span class="spinner"></span>
                <span>Generating preview...</span>
              </div>

              <!-- Image preview -->
              <div v-else-if="previewData?.kind === 'Image'" class="preview-body preview-image">
                <img
                  :src="'data:image/png;base64,' + (previewData as any).data"
                  :alt="(previewData as any).file_name"
                  class="preview-thumb"
                />
                <div class="preview-meta">
                  <span>{{ (previewData as any).file_type.toUpperCase() }}</span>
                  <span>{{ (previewData as any).width }} x {{ (previewData as any).height }}</span>
                  <span>{{ formatSize((previewData as any).file_size) }}</span>
                </div>
              </div>

              <!-- Text preview -->
              <div v-else-if="previewData?.kind === 'Text'" class="preview-body preview-text">
                <div class="preview-text-meta">
                  <span class="badge badge-accent" v-if="(previewData as any).file_type">
                    {{ extLabel((previewData as any).file_type) }}
                  </span>
                  <span class="text-muted">
                    {{ (previewData as any).total_lines }} lines
                    <template v-if="(previewData as any).truncated"> (showing first 100)</template>
                  </span>
                  <span class="text-muted">{{ formatSize((previewData as any).file_size) }}</span>
                </div>
                <pre class="preview-code"><code>{{ (previewData as any).content }}</code></pre>
              </div>

              <!-- Metadata only -->
              <div v-else-if="previewData?.kind === 'Metadata'" class="preview-body preview-metadata">
                <div class="metadata-icon">
                  <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                    <polyline points="14 2 14 8 20 8"/>
                  </svg>
                </div>
                <div class="metadata-info">
                  <span class="metadata-type">{{ (previewData as any).file_type }}</span>
                  <span class="metadata-size">{{ formatSize((previewData as any).file_size) }}</span>
                  <span class="metadata-mime text-muted">{{ (previewData as any).mime_guess }}</span>
                </div>
              </div>

              <!-- Error -->
              <div v-else-if="previewData?.kind === 'Error'" class="preview-body preview-error">
                <span class="text-muted">{{ (previewData as any).message }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.duplicates-view {
  max-width: 1440px;
}

.scan-controls {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.min-size-control {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.min-size-control label {
  font-size: 12px;
  white-space: nowrap;
}

.size-select {
  font-size: 12px;
  padding: 6px var(--sp-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface);
  color: var(--text);
  cursor: pointer;
}

.scan-stats {
  font-size: 12px;
  margin-top: var(--sp-2);
}

/* Stats bar */
.stats-bar {
  display: flex;
  gap: var(--sp-4);
  margin-bottom: var(--sp-4);
}

.stat {
  flex: 1;
  background: var(--glass);
  border-radius: var(--radius-md);
  padding: 14px var(--sp-4);
  box-shadow: var(--shadow-sm);
  text-align: center;
}

.stat-highlight {
  border: 1px solid var(--accent);
  background: rgba(59, 199, 232, 0.20);
}

.stat-value {
  display: block;
  font-size: 18px;
  font-weight: 700;
  color: var(--text);
}

.stat-highlight .stat-value {
  color: var(--accent);
}

.stat-label {
  display: block;
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.skipped-note {
  font-size: 12px;
  margin-bottom: var(--sp-3);
  padding: 0 var(--sp-1);
}

/* Group list */
.group-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}

.group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px var(--sp-5);
  cursor: pointer;
  transition: background 0.15s;
}

.group-header:hover {
  background: var(--surface-alt);
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  min-width: 0;
  flex: 1;
}

.group-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.group-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.group-count {
  font-weight: 400;
  font-size: 13px;
}

.group-meta {
  font-size: 12px;
}

.group-header-right {
  flex-shrink: 0;
  margin-left: var(--sp-3);
}

/* File rows */
.group-files {
  border-top: 1px solid var(--border-divider);
}

.file-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-3) var(--sp-5) var(--sp-3) 44px;
  cursor: pointer;
  transition: background 0.1s;
  border-bottom: 1px solid var(--border-divider);
}

.file-row:last-child {
  border-bottom: none;
}

.file-row:hover {
  background: var(--surface-alt);
}

.file-row.selected {
  background: rgba(255, 69, 58, 0.12);
}

.file-row.is-original {
  background: rgba(0, 180, 216, 0.10);
}

.file-row-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  min-width: 0;
  flex: 1;
}

.file-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.file-name-row {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.file-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.file-path {
  font-size: 11px;
  color: var(--muted);
  max-width: 400px;
}

.file-row-right {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
  margin-left: var(--sp-3);
}

.file-modified {
  font-size: 11px;
}

/* Preview button */
.btn-preview {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  border: 1px solid transparent;
  padding: 0;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
  flex-shrink: 0;
}

.btn-preview:hover {
  background: var(--accent-light);
  color: var(--accent);
  border-color: var(--accent);
}

.btn-preview.active {
  background: var(--accent);
  color: #ffffff;
  border-color: var(--accent);
}

/* Preview panel */
.preview-panel {
  border-top: 2px solid var(--accent);
  background: transparent;
  animation: slideDown 0.2s ease;
}

.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-3) var(--sp-5);
  border-bottom: 1px solid var(--border-divider);
}

.preview-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-preview-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  padding: 0;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  flex-shrink: 0;
}

.btn-preview-close:hover {
  background: var(--surface-hover);
  color: var(--text);
}

.preview-loading {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-6) var(--sp-5);
  color: var(--muted);
  font-size: 13px;
}

.preview-body {
  padding: var(--sp-4) var(--sp-5);
}

/* Image preview */
.preview-image {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-3);
}

.preview-thumb {
  max-width: 100%;
  max-height: 300px;
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  background: repeating-conic-gradient(var(--border-subtle) 0% 25%, transparent 0% 50%) 50% / 16px 16px;
  object-fit: contain;
}

.preview-meta {
  display: flex;
  gap: var(--sp-4);
  font-size: 12px;
  color: var(--muted);
}

/* Text preview */
.preview-text {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}

.preview-text-meta {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  font-size: 12px;
}

.preview-code {
  background: #1e1e2e;
  color: #cdd6f4;
  border-radius: var(--radius-md);
  padding: 14px var(--sp-4);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  overflow-x: auto;
  overflow-y: auto;
  max-height: 300px;
  white-space: pre;
  tab-size: 4;
  margin: 0;
}

.preview-code code {
  font-family: inherit;
}

/* Metadata preview */
.preview-metadata {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
}

.metadata-icon {
  color: var(--muted);
  flex-shrink: 0;
}

.metadata-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.metadata-type {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.metadata-size {
  font-size: 13px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
}

.metadata-mime {
  font-size: 11px;
}

/* Error preview */
.preview-error {
  text-align: center;
  padding: var(--sp-6);
  font-size: 13px;
}
</style>
