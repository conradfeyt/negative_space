<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize, getFileExtension } from "../utils";
import { showToast } from "../stores/toastStore";
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
import StatCard from "../components/StatCard.vue";
import Checkbox from "../components/Checkbox.vue";
import EmptyState from "../components/EmptyState.vue";
import TabBar from "../components/TabBar.vue";
import ScanHeader from "../components/ScanHeader.vue";
import AppSelect from "../components/AppSelect.vue";
import StickyBar from "../components/StickyBar.vue";
import ProgressBar from "../components/ProgressBar.vue";
import LoadingState from "../components/LoadingState.vue";
import KindFilterBar from "../components/KindFilterBar.vue";
import type { TabOption } from "../components/TabBar.vue";
import type { DuplicateGroup, SimilarGroup, FilePreview } from "../types";
import {
  useDuplicateFilters,
  extCardColor,
  isImageFile,
  KIND_LABELS,
  type FileKind,
} from "../composables/useDuplicateFilters";
import { useScanLocations } from "../composables/useScanFolder";

const PREVIEW_FILES_PER_GROUP = 10;

// Scan type & result view
const scanType = ref<string>("exact");
const scanTypeOptions = ref([
  { value: "exact", label: "Exact" },
  { value: "similar", label: "Similar" },
  { value: "both", label: "Both" },
]);

// Load SF Symbol icons for scan types
const SCAN_TYPE_ICONS: Record<string, string> = {
  exact: "equal.circle",
  similar: "photo.on.rectangle.angled",
  both: "square.stack.3d.up",
};

for (const [type, sfName] of Object.entries(SCAN_TYPE_ICONS)) {
  invoke<string>("render_sf_symbol", { name: sfName, size: 32, mode: "sf", style: "plain" })
    .then((b64) => {
      if (!b64) return;
      const opt = scanTypeOptions.value.find((o) => o.value === type);
      if (opt) (opt as any).icon = b64;
    })
    .catch(() => {});
}
const activeView = ref<"exact" | "similar">("exact");

// Settings popover
const showSettings = ref(false);

function onSettingsClickOutside(e: MouseEvent) {
  const el = (e.target as HTMLElement).closest('.settings-popover');
  if (!el) showSettings.value = false;
}

function onSettingsEsc(e: KeyboardEvent) {
  if (e.key === "Escape") showSettings.value = false;
}

watch(showSettings, (open) => {
  if (open) {
    setTimeout(() => {
      document.addEventListener("click", onSettingsClickOutside);
      document.addEventListener("keydown", onSettingsEsc);
    }, 0);
  } else {
    document.removeEventListener("click", onSettingsClickOutside);
    document.removeEventListener("keydown", onSettingsEsc);
  }
});

const minSizeStops = [0, 1, 5, 10, 50, 100];
const thresholdStops = [5, 10, 15, 20];

const minSizeLabel = computed(() => {
  if (minSizeMb.value === 0) return "1 KB";
  if (minSizeMb.value >= 1000) return `${minSizeMb.value / 1000} GB`;
  return `${minSizeMb.value} MB`;
});

const thresholdLabel = computed(() => {
  if (similarThreshold.value <= 5) return "Strict";
  if (similarThreshold.value <= 10) return "Normal";
  if (similarThreshold.value <= 15) return "Loose";
  return "Very Loose";
});

// ── Multi-directory picker ───────────────────────────────────────────────
const { folders: scanFolders, addFolder, removeFolder } = useScanLocations("dup");

const viewTabOptions: TabOption[] = [
  { value: "exact", label: "Exact Duplicates" },
  { value: "similar", label: "Similar Images" },
];

const hasExactResults = computed(() => !!duplicateResult.value && duplicateResult.value.groups.length > 0);
const hasSimilarResults = computed(() => !!similarResult.value && similarResult.value.groups.length > 0);
const showViewTabs = computed(() => hasExactResults.value && hasSimilarResults.value);

watch(scanType, (type) => {
  if (type === "exact") activeView.value = "exact";
  else if (type === "similar") activeView.value = "similar";
});

watch(duplicateError, (err) => { if (err) showToast(err, "error"); });
watch(similarError, (err) => { if (err) showToast(err, "error"); });

// Similar images state
const similarThreshold = ref(10);
const similarMinSizeMb = ref(0);
const similarSelected = ref<Set<string>>(new Set());

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
  try {
    const result = await deleteFiles(Array.from(similarSelected.value));
    showToast(`Deleted ${result.deleted_count} file(s), freed ${formatSize(result.freed_bytes)}`, "success");
    similarSelected.value = new Set();
    await scanSimilarImages(similarThreshold.value, similarMinSizeMb.value);
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    cleaning.value = false;
  }
}

// ── File kind filtering (composable) ──────────────────────────────────────

const {
  activeKinds,
  kindCounts,
  filteredGroups,
} = useDuplicateFilters(duplicateResult);

const ALL_KINDS: FileKind[] = ["all", "images", "documents", "audio", "video", "archives", "code", "other"];

const kindOptions = ALL_KINDS.filter((k) => k !== "all").map((k) => ({ key: k, label: KIND_LABELS[k] }));

const activeKindsList = computed({
  get: () => Array.from(activeKinds.value),
  set: (v: string[]) => {
    activeKinds.value = new Set(v as FileKind[]);
  },
});

const kindCountNumbers = computed(() => {
  const result: Record<string, number> = {};
  for (const [k, v] of Object.entries(kindCounts.value)) {
    result[k] = (v as { groups: number }).groups;
  }
  return result;
});

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

// Native filter icon
const filterIcon = ref("");
invoke<string>("render_sf_symbol", { name: "line.3.horizontal.decrease", size: 32, mode: "sf", style: "plain" })
  .then((b64) => { if (b64) filterIcon.value = b64; })
  .catch(() => {});

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

// Preview state
const previewData = ref<FilePreview | null>(null);
const previewLoading = ref(false);
const previewPath = ref<string | null>(null);

const scanLabel = "Scan";

const isScanning = computed(() => duplicateScanning.value || similarScanning.value);

const headerDescription = computed(() => {
  if (scanType.value === "exact") return "Find identical files wasting disk space";
  if (scanType.value === "similar") return "Find visually similar images";
  return "Find duplicate and similar files";
});

async function scan() {
  const folders = scanFolders.value;
  if (scanType.value === "exact" || scanType.value === "both") {
    selected.value = new Set();
    await scanDuplicates("~", minSizeMb.value, folders);
    activeView.value = "exact";
  }
  if (scanType.value === "similar" || scanType.value === "both") {
    similarSelected.value = new Set();
    await scanSimilarImages(similarThreshold.value, similarMinSizeMb.value, folders);
    if (scanType.value === "similar") activeView.value = "similar";
  }
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
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      showToast(`Deleted ${result.deleted_count} duplicate(s), freed ${formatSize(result.freed_bytes)}`, "success");
    }
    if (result.errors.length > 0) {
      showToast(result.errors.join("; "), "error");
    }
    // Re-scan to show updated results
    selected.value = new Set();
    await scanDuplicates("~", minSizeMb.value);
  } catch (e) {
    showToast(String(e), "error");
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
    <ScanHeader
      title="Duplicate Finder"
      :subtitle="headerDescription"
      :scanning="isScanning"
      :scan-label="scanLabel"
      :disabled="isScanning"
      :folders="scanFolders"
      @scan="scan"
      @add-folder="addFolder"
      @remove-folder="removeFolder"
    >
      <AppSelect v-model="scanType" :options="scanTypeOptions" compact />
      <span class="scan-bar-divider"></span>
      <button class="scan-bar-settings" @click.stop="showSettings = !showSettings" title="Scan settings">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="4" y1="21" x2="4" y2="14" /><line x1="4" y1="10" x2="4" y2="3" />
          <line x1="12" y1="21" x2="12" y2="12" /><line x1="12" y1="8" x2="12" y2="3" />
          <line x1="20" y1="21" x2="20" y2="16" /><line x1="20" y1="12" x2="20" y2="3" />
          <line x1="1" y1="14" x2="7" y2="14" />
          <line x1="9" y1="8" x2="15" y2="8" />
          <line x1="17" y1="16" x2="23" y2="16" />
        </svg>
      </button>
    </ScanHeader>

    <!-- Settings popover -->
    <Teleport to="body">
      <div v-if="showSettings" class="settings-popover" @click.stop>
        <div class="settings-popover-header">
          <div class="settings-popover-title">Scan settings</div>
          <button class="settings-popover-close" @click="showSettings = false">&times;</button>
        </div>

        <template v-if="scanType !== 'similar'">
          <div class="settings-row">
            <label class="settings-label">Minimum file size</label>
            <span class="settings-value">{{ minSizeLabel }}</span>
          </div>
          <input
            type="range"
            class="settings-slider"
            :value="minSizeStops.indexOf(minSizeMb)"
            min="0"
            :max="minSizeStops.length - 1"
            step="1"
            @input="minSizeMb = minSizeStops[Number(($event.target as HTMLInputElement).value)]"
          />
          <div class="settings-slider-labels">
            <span>1 KB</span>
            <span>100 MB</span>
          </div>
        </template>

        <template v-if="scanType !== 'exact'">
          <div class="settings-row">
            <label class="settings-label">Similarity</label>
            <span class="settings-value">{{ thresholdLabel }}</span>
          </div>
          <input
            type="range"
            class="settings-slider"
            :value="thresholdStops.indexOf(similarThreshold)"
            min="0"
            :max="thresholdStops.length - 1"
            step="1"
            @input="similarThreshold = thresholdStops[Number(($event.target as HTMLInputElement).value)]"
          />
          <div class="settings-slider-labels">
            <span>Strict</span>
            <span>Very Loose</span>
          </div>
        </template>
      </div>
    </Teleport>

    <!-- Result view filter (when both scan types have results) -->
    <div v-if="showViewTabs" class="view-filter-tabs">
      <TabBar :options="viewTabOptions" v-model="activeView" />
    </div>

    <!-- ══════ EXACT DUPLICATES ══════ -->
    <template v-if="activeView === 'exact'">

    <!-- Loading -->
    <LoadingState
      v-if="duplicateScanning"
      message="Scanning for duplicates... this may take a while"
    />

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

      <StickyBar>
        <KindFilterBar
          :options="kindOptions"
          v-model="activeKindsList"
          :counts="kindCountNumbers"
          :icon="filterIcon"
        />
        <span v-if="selected.size > 0" class="results-count">
          {{ selected.size }} file(s) selected &mdash; {{ formatSize(totalSelected) }}
        </span>
        <template #actions>
          <button
            class="btn-danger"
            :disabled="selected.size === 0 || cleaning"
            @click="deleteSelected"
          >
            <span v-if="cleaning" class="spinner-sm"></span>
            {{ cleaning ? "Deleting..." : "Delete Selected" }}
          </button>
        </template>
      </StickyBar>

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
                <Checkbox :model-value="selected.has(file.path)" @change="toggleFile(file.path)" />
                <span v-if="idx === 0" class="badge pill badge-accent card-badge-keep">Keep</span>

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
              <button class="btn-close" @click="closePreview">
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
    </template><!-- /exact -->

    <!-- ══════ SIMILAR IMAGES ══════ -->
    <template v-if="activeView === 'similar'">

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
          <ProgressBar
            :percent="Math.round((similarProgress.images_processed / similarProgress.total_images) * 100)"
            size="thin"
          />
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

        <StickyBar v-if="similarResult.groups.length > 0">
          <span v-if="similarSelected.size > 0" class="results-count">
            {{ similarSelected.size }} selected
          </span>
          <template #actions>
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
          </template>
        </StickyBar>

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
                  <Checkbox :model-value="similarSelected.has(file.path)" @change="toggleSimilarFile(file.path)" />
                  <span v-if="idx === group.representative_idx" class="badge pill badge-accent card-badge-keep">Keep</span>

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

    </template><!-- /similar -->
  </section>
</template>

<style scoped>
.duplicates-view {
  max-width: 1440px;
}

.results-count {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
}

.scan-bar-divider {
  width: 1px;
  height: 18px;
  background: var(--border);
  margin: 0 4px;
  flex-shrink: 0;
}

/* Settings button in ScanBar */
.scan-bar-settings {
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 8px;
  transition: background 0.15s;
  opacity: 0.7;
}

.scan-bar-settings:hover {
  background: rgba(0, 0, 0, 0.06);
  opacity: 1;
}

/* Result view filter tabs */
.view-filter-tabs {
  margin-bottom: var(--sp-3);
}

.similar-progress {
  padding: var(--sp-4);
  background: var(--glass);
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

.similar-progress-bar-wrap :deep(.progress-track) {
  flex: 1;
  min-width: 0;
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




/* Stats bar */
.stats-bar {
  display: flex;
  gap: var(--sp-4);
  margin-bottom: var(--sp-4);
}



/* Group list */
.group-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.group-card {
  background: var(--glass);
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
  margin: 0 16px 14px;
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
  background: var(--glass);
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

.file-card > .checkbox {
  position: absolute;
  top: 4px;
  left: 4px;
  z-index: 2;
}

/* Keep badge — positioning only; visual style from global .badge .pill .badge-accent */
.card-badge-keep {
  position: absolute;
  top: 6px;
  right: 6px;
  z-index: 2;
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
  border-color: rgba(2, 117, 244, 0.4);
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
  color: var(--accent, rgba(2, 117, 244, 1));
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

<style>
/* Settings popover — Teleported to body, needs global styles */
.settings-popover {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 9999;
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: var(--radius-lg, 12px);
  padding: 0 20px 16px;
  width: 260px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.18);
  -webkit-app-region: no-drag;
}

.settings-popover-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 0 8px;
}

.settings-popover-title {
  font-size: 13px;
  font-weight: 600;
}

.settings-popover-close {
  background: none;
  border: none;
  font-size: 18px;
  line-height: 1;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 0 2px;
  opacity: 0.6;
  transition: opacity 0.15s;
}

.settings-popover-close:hover {
  opacity: 1;
}

.settings-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 12px;
  margin-bottom: 4px;
}

.settings-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
}

.settings-value {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
}

.settings-slider {
  width: 100%;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(0, 0, 0, 0.1);
  border-radius: 2px;
  outline: none;
  margin: 4px 0 2px;
}

.settings-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--accent, #0088FF);
  cursor: pointer;
  border: 2px solid white;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
}

.settings-slider-labels {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
</style>
