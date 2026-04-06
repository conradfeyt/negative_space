<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize, getFileExtension } from "../utils";
import {
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
  deleteFiles,
  previewFile,
} from "../stores/scanStore";
import FdaWarningBanner from "../components/FdaWarningBanner.vue";
import StatCard from "../components/StatCard.vue";
import EmptyState from "../components/EmptyState.vue";
import type { DuplicateGroup, SimilarGroup, FilePreview } from "../types";
import {
  useDuplicateFilters,
  extCardColor,
  isImageFile,
  KIND_LABELS,
  type FileKind,
} from "../composables/useDuplicateFilters";

const PREVIEW_FILES_PER_GROUP = 10;

// Tab state
const activeTab = ref<"exact" | "similar">("exact");

// Similar images state
const similarThreshold = ref(10);
const similarMinSizeMb = ref(0);
const similarSelected = ref<Set<string>>(new Set());

async function scanSimilar() {
  similarSelected.value = new Set();
  await scanSimilarImages(similarThreshold.value, similarMinSizeMb.value);
}

function toggleSimilarFile(path: string) {
  const next = new Set(similarSelected.value);
  if (next.has(path)) next.delete(path); else next.add(path);
  similarSelected.value = next;
}

function selectSimilarDuplicates(group: SimilarGroup) {
  const next = new Set(similarSelected.value);
  group.files.forEach((f, i) => {
    if (i !== group.representative_idx) next.add(f.path);
  });
  similarSelected.value = next;
}

async function deleteSimilarSelected() {
  if (similarSelected.value.size === 0) return;
  cleaning.value = true;
  cleanError.value = "";
  successMsg.value = "";
  try {
    const result = await deleteFiles(Array.from(similarSelected.value));
    successMsg.value = `Deleted ${result.deleted_count} file(s), freed ${formatSize(result.freed_bytes)}`;
    similarSelected.value = new Set();
    await scanSimilar();
  } catch (e) {
    cleanError.value = String(e);
  } finally {
    cleaning.value = false;
  }
}

// ── File kind filtering (composable) ──────────────────────────────────────

const {
  activeKindFilter,
  kindCounts,
  filteredGroups,
} = useDuplicateFilters(duplicateResult);


// File icon cache (same pattern as LargeFiles)
const fileIconCache = ref<Record<string, string>>({});

async function loadFileIcon(ext: string) {
  if (fileIconCache.value[ext] || ext === "") return;
  fileIconCache.value[ext] = "";
  try {
    const base64 = await invoke<string>("render_sf_symbol", { name: ext, size: 64, mode: "uttype", style: "plain" });
    if (base64) fileIconCache.value[ext] = base64;
  } catch { /* non-critical */ }
}

function getFileIcon(name: string): string {
  const ext = getFileExtension(name);
  if (!fileIconCache.value[ext] && ext) loadFileIcon(ext);
  return fileIconCache.value[ext] || "";
}

// Preload common icons
for (const ext of ["png", "jpg", "pdf", "zip", "mp4", "mp3", "doc", "js", "py"]) loadFileIcon(ext);

// isImageFile and extCardColor are imported from useDuplicateFilters

function getExtLabel(name: string): string {
  const ext = name.split(".").pop()?.toUpperCase() ?? "";
  return ext ? `.${ext}` : "?";
}


// Group thumbnail cache: group hash -> base64 JPEG
// Thumbnails are generated during the Rust scan and included in group.thumbnail.
// No frontend async loading needed — just render what's already there.

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

async function scan() {
  successMsg.value = "";
  cleanError.value = "";
  selected.value = new Set();
  await scanDuplicates("~", minSizeMb.value);
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
  <section class="duplicates-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Duplicate Finder</h2>
          <p class="text-muted">
            {{ activeTab === 'exact' ? 'Find identical files wasting disk space' : 'Find visually similar images' }}
          </p>
        </div>
        <div class="tab-switcher" role="tablist">
          <button class="tab-btn" :class="{ active: activeTab === 'exact' }" role="tab" :aria-selected="activeTab === 'exact'" @click="activeTab = 'exact'">Exact Duplicates</button>
          <button class="tab-btn" :class="{ active: activeTab === 'similar' }" role="tab" :aria-selected="activeTab === 'similar'" @click="activeTab = 'similar'">Similar Images</button>
        </div>
        <div v-if="activeTab === 'exact'" class="scan-controls">
          <div class="min-size-control">
            <label for="dup-min-size" class="text-muted">Min size</label>
            <select id="dup-min-size" v-model.number="minSizeMb" class="size-select">
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
            <span v-if="duplicateScanning" class="spinner-sm"></span>
            {{ duplicateScanning ? "Scanning..." : "Find Duplicates" }}
          </button>
        </div>
        <div v-if="activeTab === 'similar'" class="scan-controls">
          <div class="min-size-control">
            <label for="dup-threshold" class="text-muted">Threshold</label>
            <select id="dup-threshold" v-model.number="similarThreshold" class="size-select">
              <option :value="5">Strict (5)</option>
              <option :value="10">Normal (10)</option>
              <option :value="15">Loose (15)</option>
              <option :value="20">Very loose (20)</option>
            </select>
          </div>
          <button
            class="btn-primary scan-btn"
            :disabled="similarScanning"
            @click="scanSimilar"
          >
            <span v-if="similarScanning" class="spinner-sm"></span>
            {{ similarScanning ? "Scanning..." : "Find Similar" }}
          </button>
        </div>
      </div>
    </div>

    <!-- FDA warning -->
    <FdaWarningBanner
      title="Limited scan -- Full Disk Access required for full coverage"
      text="Without Full Disk Access, only developer tools, package managers, and project directories are scanned. Desktop, Documents, Downloads, and most of ~/Library are skipped."
    />

    <!-- ══════ EXACT DUPLICATES TAB ══════ -->
    <template v-if="activeTab === 'exact'">

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
    <EmptyState
      v-else-if="duplicateScanned && (!duplicateResult || duplicateResult.groups.length === 0)"
      title="No duplicate files found"
      :description="duplicateResult ? `Scanned ${duplicateResult.files_scanned.toLocaleString()} files` : 'Run a scan to find exact duplicate files.'"
    />

    <!-- Results -->
    <template v-else-if="duplicateResult && duplicateResult.groups.length > 0">
      <!-- Scan stats -->
      <div class="stats-bar">
        <StatCard :value="duplicateResult.files_scanned.toLocaleString()" label="files scanned" />
        <StatCard :value="String(duplicateResult.groups.length)" label="duplicate groups" />
        <StatCard :value="String(duplicateResult.total_duplicate_files)" label="duplicate files" />
        <StatCard :value="formatSize(duplicateResult.total_wasted_bytes)" label="wasted space" highlight />
      </div>

      <!-- Skipped paths -->
      <div
        v-if="duplicateResult.skipped_paths.length > 0"
        class="skipped-note text-muted"
      >
        Skipped (no FDA):
        {{ duplicateResult.skipped_paths.join(", ") }}
      </div>

      <!-- Kind filter pills -->
      <div class="kind-filter-bar">
        <button
          v-for="kind in (['all', 'images', 'documents', 'audio', 'video', 'archives', 'code', 'other'] as FileKind[])"
          :key="kind"
          class="kind-pill"
          :class="{ active: activeKindFilter === kind, empty: kind !== 'all' && !kindCounts[kind] }"
          @click="activeKindFilter = kind"
          :disabled="kind !== 'all' && !kindCounts[kind]"
        >
          {{ KIND_LABELS[kind] }}
          <span v-if="kind !== 'all' && kindCounts[kind]" class="kind-count">{{ kindCounts[kind].groups }}</span>
          <span v-if="kind === 'all'" class="kind-count">{{ duplicateResult!.groups.length }}</span>
        </button>
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
            <span v-if="cleaning" class="spinner-sm"></span>
            {{ cleaning ? "Deleting..." : "Delete Selected" }}
          </button>
        </div>
      </div>

      <!-- Duplicate groups — card gallery layout -->
      <div class="group-list">
        <div
          v-for="group in filteredGroups"
          :key="group.hash"
          class="group-card"
        >
          <!-- Group header -->
          <div class="group-header">
            <div class="group-info">
              <span class="group-name">
                {{ group.files[0].name }}
                <span class="text-muted group-count">({{ group.files.length }} copies)</span>
              </span>
              <span class="group-meta text-muted">
                {{ formatSize(group.size) }} each &mdash; {{ formatSize(group.wasted_bytes) }} wasted
              </span>
            </div>
            <button
              v-if="!allDuplicatesSelected(group)"
              class="btn-sm btn-secondary"
              @click="selectDuplicates(group)"
            >Select duplicates</button>
            <button v-else class="btn-sm btn-secondary" @click="deselectGroup(group)">Deselect all</button>
          </div>

          <!-- Card strip (max 10 visible cards to keep DOM light) -->
          <div class="card-strip-container">
            <div class="card-strip">
              <div
                v-for="(file, idx) in group.files.slice(0, PREVIEW_FILES_PER_GROUP)"
                :key="file.path"
                class="file-card"
                :class="{
                  'file-card--selected': selected.has(file.path),
                  'file-card--keep': idx === 0,
                }"
                @click="loadPreview(file.path, $event)"
              >
                <label class="card-checkbox" @click.stop>
                  <input type="checkbox" :checked="selected.has(file.path)" @change="toggleFile(file.path)" />
                </label>
                <span v-if="idx === 0" class="card-badge-keep">Keep</span>

                <div v-if="isImageFile(file.name)" class="card-face card-face--thumb">
                  <img v-if="group.thumbnail" :src="'data:image/jpeg;base64,' + group.thumbnail" alt="" class="card-thumb-img" />
                  <img v-else-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="card-placeholder-icon" />
                  <span v-else class="card-loading-dot"><span class="spinner-sm"></span></span>
                </div>

                <div v-else class="card-face card-face--ext" :style="{ background: `linear-gradient(135deg, ${extCardColor(file.name)}, color-mix(in srgb, ${extCardColor(file.name)} 80%, black))` }">
                  <span class="card-ext-label">{{ getExtLabel(file.name) }}</span>
                </div>

                <div class="card-meta">
                  <div class="card-filename" :title="file.name">{{ file.name }}</div>
                  <div class="card-dir text-muted" :title="file.parent_dir">{{ shortPath(file.parent_dir) }}</div>
                  <div class="card-date text-muted mono">{{ file.modified }}</div>
                </div>
              </div>

              <!-- Overflow indicator -->
              <div v-if="group.files.length > 10" class="card-overflow" @click="selectDuplicates(group)">
                <span class="card-overflow-count">+{{ group.files.length - 10 }}</span>
                <span class="card-overflow-label">more copies</span>
                <span class="card-overflow-action">Select all duplicates</span>
              </div>
            </div>
          </div>

          <!-- Inline preview panel -->
          <div v-if="previewPath && group.files.some(f => f.path === previewPath)" class="preview-panel">
            <div class="preview-header">
              <span class="preview-title">{{ (previewData as any)?.file_name || '...' }}</span>
              <button class="btn-preview-close" @click="closePreview">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
              </button>
            </div>
            <div v-if="previewLoading" class="preview-loading"><span class="spinner"></span><span>Generating preview...</span></div>
            <div v-else-if="previewData?.kind === 'Image'" class="preview-body preview-image">
              <img :src="'data:image/png;base64,' + (previewData as any).data" :alt="(previewData as any).file_name" class="preview-thumb" />
              <div class="preview-meta">
                <span>{{ (previewData as any).file_type.toUpperCase() }}</span>
                <span>{{ (previewData as any).width }} x {{ (previewData as any).height }}</span>
                <span>{{ formatSize((previewData as any).file_size) }}</span>
              </div>
            </div>
            <div v-else-if="previewData?.kind === 'Text'" class="preview-body preview-text">
              <div class="preview-text-meta">
                <span class="badge badge-accent" v-if="(previewData as any).file_type">{{ extLabel((previewData as any).file_type) }}</span>
                <span class="text-muted">{{ (previewData as any).total_lines }} lines<template v-if="(previewData as any).truncated"> (first 100)</template></span>
                <span class="text-muted">{{ formatSize((previewData as any).file_size) }}</span>
              </div>
              <pre class="preview-code"><code>{{ (previewData as any).content }}</code></pre>
            </div>
            <div v-else-if="previewData?.kind === 'Metadata'" class="preview-body preview-metadata">
              <div class="metadata-info">
                <span class="metadata-type">{{ (previewData as any).file_type }}</span>
                <span class="metadata-size">{{ formatSize((previewData as any).file_size) }}</span>
                <span class="metadata-mime text-muted">{{ (previewData as any).mime_guess }}</span>
              </div>
            </div>
            <div v-else-if="previewData?.kind === 'Error'" class="preview-body preview-error">
              <span class="text-muted">{{ (previewData as any).message }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>
    </template><!-- /exact tab -->

    <!-- ══════ SIMILAR IMAGES TAB ══════ -->
    <template v-if="activeTab === 'similar'">

      <!-- Messages -->
      <div v-if="similarError" class="error-message">{{ similarError }}</div>
      <div v-if="cleanError" class="error-message">{{ cleanError }}</div>
      <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

      <!-- Progress -->
      <div v-if="similarScanning" class="similar-progress">
        <div class="similar-progress-header">
          <span class="spinner"></span>
          <span v-if="!similarProgress || similarProgress.phase === 'discovery'">
            Discovering images…
          </span>
          <span v-else-if="similarProgress.phase === 'hashing'">
            Hashing images — {{ similarProgress.images_processed }} / {{ similarProgress.total_images }}
          </span>
          <span v-else-if="similarProgress.phase === 'thumbnails'">
            Generating thumbnails…
          </span>
          <span v-else>
            Comparing hashes…
          </span>
        </div>
        <div v-if="similarProgress && similarProgress.phase === 'hashing' && similarProgress.total_images > 0" class="similar-progress-bar-wrap">
          <div class="similar-progress-bar" role="progressbar" :aria-valuenow="Math.round(similarProgress.images_processed / similarProgress.total_images * 100)" aria-valuemin="0" aria-valuemax="100">
            <div class="similar-progress-fill" :style="{ width: Math.round(similarProgress.images_processed / similarProgress.total_images * 100) + '%' }"></div>
          </div>
          <span class="similar-progress-pct text-muted">{{ Math.round(similarProgress.images_processed / similarProgress.total_images * 100) }}%</span>
        </div>
        <div v-if="similarProgress && similarProgress.current_file" class="similar-progress-file text-muted">
          {{ shortPath(similarProgress.current_file) }}
        </div>
      </div>

      <!-- Empty state -->
      <EmptyState
        v-else-if="similarScanned && (!similarResult || similarResult.groups.length === 0)"
        title="No similar images found"
        :description="similarResult ? `Scanned ${similarResult.images_scanned.toLocaleString()} images${similarResult.images_skipped > 0 ? ` (${similarResult.images_skipped} skipped)` : ''}` : 'Run a scan to find visually similar images.'"
      />

      <!-- Results -->
      <template v-else-if="similarResult && similarResult.groups.length > 0">
        <div class="stats-bar">
          <StatCard :value="similarResult.images_scanned.toLocaleString()" label="images scanned" />
          <StatCard :value="String(similarResult.groups.length)" label="similar groups" />
          <StatCard :value="String(similarResult.total_similar_files)" label="similar files" />
          <StatCard :value="formatSize(similarResult.total_wasted_bytes)" label="reclaimable" highlight />
        </div>

        <!-- Batch actions -->
        <div class="batch-bar" v-if="similarResult.groups.length > 0">
          <button class="btn-secondary" @click="similarResult.groups.forEach(g => selectSimilarDuplicates(g))">
            Select all duplicates
          </button>
          <button
            class="btn-danger"
            :disabled="similarSelected.size === 0 || cleaning"
            @click="deleteSimilarSelected"
          >
            <span v-if="cleaning" class="spinner-sm"></span>
            {{ cleaning ? "Deleting..." : `Delete ${similarSelected.size} selected` }}
          </button>
        </div>

        <!-- Groups — card gallery (same layout as exact duplicates) -->
        <div class="group-list">
          <div v-for="group in similarResult.groups" :key="group.id" class="group-card">
            <div class="group-header">
              <div class="group-info">
                <span class="group-name">
                  {{ group.files.length }} similar images
                  <span class="text-muted group-count">(distance {{ group.avg_distance.toFixed(1) }})</span>
                </span>
                <span class="group-meta text-muted">{{ formatSize(group.wasted_bytes) }} reclaimable</span>
              </div>
              <button class="btn-sm btn-secondary" @click="selectSimilarDuplicates(group)">Select duplicates</button>
            </div>

            <div class="card-strip-container">
              <div class="card-strip">
                <div
                  v-for="(file, idx) in group.files.slice(0, PREVIEW_FILES_PER_GROUP)"
                  :key="file.path"
                  class="file-card"
                  :class="{
                    'file-card--selected': similarSelected.has(file.path),
                    'file-card--keep': idx === group.representative_idx,
                  }"
                  @click="toggleSimilarFile(file.path)"
                >
                  <label class="card-checkbox" @click.stop>
                    <input type="checkbox" :checked="similarSelected.has(file.path)" @change="toggleSimilarFile(file.path)" />
                  </label>
                  <span v-if="idx === group.representative_idx" class="card-badge-keep">Keep</span>

                  <div class="card-face card-face--thumb">
                    <img v-if="file.thumbnail" :src="'data:image/jpeg;base64,' + file.thumbnail" alt="" class="card-thumb-img" />
                    <img v-else-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="card-placeholder-icon" />
                    <span v-else class="card-loading-dot"><span class="spinner-sm"></span></span>
                  </div>

                  <div class="card-meta">
                    <div class="card-filename" :title="file.name">{{ file.name }}</div>
                    <div class="card-dir text-muted" :title="file.parent_dir">{{ shortPath(file.parent_dir) }}</div>
                    <div class="card-date text-muted mono">{{ file.modified }}</div>
                  </div>
                </div>

                <div v-if="group.files.length > 10" class="card-overflow" @click="selectSimilarDuplicates(group)">
                  <span class="card-overflow-count">+{{ group.files.length - 10 }}</span>
                  <span class="card-overflow-label">more images</span>
                  <span class="card-overflow-action">Select all duplicates</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>

    </template><!-- /similar tab -->
  </section>
</template>

<style scoped>
.duplicates-view {
  max-width: 1440px;
}

.tab-switcher {
  display: flex;
  gap: 2px;
  background: rgba(0, 0, 0, 0.06);
  border-radius: 8px;
  padding: 2px;
}

.tab-btn {
  padding: 5px 14px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.tab-btn.active {
  background: rgba(255, 255, 255, 0.7);
  color: var(--text);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.tab-btn:hover:not(.active) {
  color: var(--text);
}

.badge-keep {
  display: inline-block;
  font-size: 9px;
  font-weight: 600;
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--accent-glow);
  color: var(--accent);
  margin-left: 6px;
  vertical-align: middle;
}

.similar-progress {
  padding: var(--sp-4);
  background: rgba(255, 255, 255, 0.5);
  border-radius: var(--radius-lg);
  margin-bottom: var(--sp-3);
}

.similar-progress-header {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: 13px;
  font-weight: 500;
  margin-bottom: var(--sp-2);
}

.similar-progress-bar-wrap {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  margin-bottom: var(--sp-1);
}

.similar-progress-bar {
  flex: 1;
  height: 6px;
  background: rgba(0, 0, 0, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.similar-progress-fill {
  height: 100%;
  background: var(--accent, rgba(59, 199, 232, 0.8));
  border-radius: 3px;
  transition: width 0.3s ease;
}

.similar-progress-pct {
  font-size: 11px;
  font-weight: 500;
  min-width: 30px;
  text-align: right;
}

.similar-progress-file {
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 500px;
}

.kind-filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: var(--sp-3);
}

.kind-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 12px;
  border: none;
  border-radius: 14px;
  background: rgba(0, 0, 0, 0.05);
  color: var(--text);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, opacity 0.15s;
}

.kind-pill:hover:not(:disabled) {
  background: rgba(0, 0, 0, 0.1);
}

.kind-pill.active {
  background: var(--accent-glow);
  color: var(--accent-deep);
}

.kind-pill.empty {
  opacity: 0.35;
  cursor: default;
}

.kind-count {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 5px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.06);
  color: var(--muted);
}

.kind-pill.active .kind-count {
  background: var(--accent-glow);
  color: var(--accent-deep);
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

/* Stats bar */
.stats-bar {
  display: flex;
  gap: var(--sp-4);
  margin-bottom: var(--sp-4);
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
  gap: var(--sp-4);
}

.group-card {
  background: rgba(255, 255, 255, 0.45);
  border-radius: var(--radius-lg, 16px);
  border: 1px solid rgba(0, 0, 0, 0.06);
  overflow: hidden;
}

.group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
}

.group-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.group-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.group-count {
  font-weight: 400;
  font-size: 12px;
}

.group-meta {
  font-size: 11px;
}

/* ── Card strip ────────────────────────────────────────── */

.card-strip-container {
  overflow-x: auto;
  scroll-snap-type: x mandatory;
  -webkit-overflow-scrolling: touch;
  padding: 0 16px 14px;
}

.card-strip-container::-webkit-scrollbar {
  height: 4px;
}
.card-strip-container::-webkit-scrollbar-track {
  background: transparent;
}
.card-strip-container::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.12);
  border-radius: 2px;
}

.card-strip {
  display: flex;
  gap: 10px;
  flex-wrap: nowrap;
}

/* ── File card ─────────────────────────────────────────── */

.file-card {
  position: relative;
  flex-shrink: 0;
  width: 140px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.6);
  border: 2px solid transparent;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s, transform 0.1s;
  scroll-snap-align: start;
  overflow: hidden;
}

.file-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.file-card--keep {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent-glow);
}

.file-card--selected {
  border-color: rgba(217, 75, 75, 0.6);
  box-shadow: 0 0 0 2px rgba(217, 75, 75, 0.15);
}

/* Checkbox overlay */
.card-checkbox {
  position: absolute;
  top: 6px;
  left: 6px;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  background: rgba(255, 255, 255, 0.8);
  border-radius: 5px;
  cursor: pointer;
  backdrop-filter: blur(4px);
}

.card-checkbox input[type="checkbox"] {
  margin: 0;
  cursor: pointer;
}

/* Keep badge */
.card-badge-keep {
  position: absolute;
  top: 6px;
  right: 6px;
  z-index: 2;
  font-size: 9px;
  font-weight: 700;
  padding: 2px 7px;
  border-radius: 6px;
  background: rgba(59, 199, 232, 0.85);
  color: white;
  letter-spacing: 0.03em;
  backdrop-filter: blur(4px);
}

/* ── Card face (thumbnail/icon area) ───────────────────── */

.card-face {
  width: 140px;
  height: 140px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.card-face--thumb {
  background: rgba(0, 0, 0, 0.03);
}

.card-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.card-placeholder-icon {
  width: 48px;
  height: 48px;
  opacity: 0.5;
}

.card-loading-dot {
  opacity: 0.3;
}

.card-face--ext {
  border-radius: 0;
}

/* Overflow indicator */
.card-overflow {
  flex-shrink: 0;
  width: 140px;
  min-height: 200px;
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.04);
  border: 2px dashed rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.card-overflow:hover {
  background: rgba(0, 0, 0, 0.07);
  border-color: rgba(59, 199, 232, 0.4);
}

.card-overflow-count {
  font-size: 24px;
  font-weight: 700;
  color: var(--text);
  opacity: 0.5;
}

.card-overflow-label {
  font-size: 11px;
  color: var(--muted);
}

.card-overflow-action {
  font-size: 10px;
  color: var(--accent, rgba(59, 199, 232, 1));
  font-weight: 500;
  margin-top: 4px;
}

.card-ext-label {
  font-size: 22px;
  font-weight: 700;
  color: rgba(255, 255, 255, 0.9);
  letter-spacing: -0.02em;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

/* ── Card meta ─────────────────────────────────────────── */

.card-meta {
  padding: 8px 10px;
}

.card-filename {
  font-size: 11px;
  font-weight: 500;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-dir {
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 1px;
}

.card-date {
  font-size: 10px;
  margin-top: 2px;
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
  background: var(--preview-bg);
  color: var(--preview-text);
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
