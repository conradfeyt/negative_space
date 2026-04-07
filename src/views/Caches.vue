<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize } from "../utils";
import {
  caches,
  cachesScanning,
  cachesScanned,
  cachesError,
  scanCaches,
  deleteFiles,
  totalCacheSize,
} from "../stores/scanStore";
import FdaWarningBanner from "../components/FdaWarningBanner.vue";
import EmptyState from "../components/EmptyState.vue";
import Checkbox from "../components/Checkbox.vue";

// Native macOS blue folder icon via NSWorkspace.shared.icon(forFile:)
const folderIcon = ref("");
invoke<string>("render_sf_symbol", { name: "public.folder", size: 64, mode: "uttype", style: "plain" })
  .then(b64 => { if (b64) folderIcon.value = b64; })
  .catch(e => console.warn('[caches] folder icon load failed:', e));


const selected = ref<Set<string>>(new Set());
const deleting = ref(false);
const successMsg = ref("");
const deleteError = ref("");

async function scan() {
  successMsg.value = "";
  deleteError.value = "";
  selected.value = new Set();
  await scanCaches();
}

async function cleanSelected() {
  if (selected.value.size === 0) return;
  deleting.value = true;
  deleteError.value = "";
  successMsg.value = "";
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      successMsg.value = `Cleaned ${result.deleted_count} cache(s), freed ${formatSize(result.freed_bytes)}`;
      caches.value = caches.value.filter((e) => !selected.value.has(e.path));
      selected.value = new Set();
    }
    if (result.errors.length > 0) deleteError.value = result.errors.join("; ");
  } catch (e) { deleteError.value = String(e); }
  finally { deleting.value = false; }
}

function toggleSelect(path: string) {
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path); else next.add(path);
  selected.value = next;
}

function toggleAll() {
  if (allSelected.value) selected.value = new Set();
  else selected.value = new Set(caches.value.map((e) => e.path));
}

const allSelected = computed(() => caches.value.length > 0 && selected.value.size === caches.value.length);
const totalSelected = computed(() => caches.value.filter((e) => selected.value.has(e.path)).reduce((sum, e) => sum + e.size, 0));

// ── Cache categorisation ──────────────────────────────────────────────
type CacheCategory = "browsers" | "devtools" | "apple" | "apps" | "other";

const BROWSER_NAMES = ["google", "chrome", "firefox", "safari", "microsoft edge", "brave", "opera", "vivaldi", "arc"];
const DEVTOOL_NAMES = ["homebrew", "pip", "npm", "node-gyp", "cargo", "rustup", "gradle", "maven", "cocoapods", "flutter", "dart", "go-build", "yarn", "pnpm", "bun", "negative-space", "claude"];

function categoriseCache(name: string, path: string): CacheCategory {
  const lower = name.toLowerCase();
  const pathLower = path.toLowerCase();

  // Browsers
  if (BROWSER_NAMES.some(b => lower.includes(b) || pathLower.includes(b))) return "browsers";
  if (lower.includes("com.microsoft.edgemac")) return "browsers";

  // Apple system
  if (lower.startsWith("com.apple.") || lower.startsWith("apple")) return "apple";

  // Dev tools
  if (DEVTOOL_NAMES.some(d => lower.includes(d))) return "devtools";
  if (pathLower.includes("xcode") || lower.includes("dt.xcode")) return "devtools";

  // Known app patterns
  if (lower.startsWith("com.") || lower.startsWith("org.") || lower.startsWith("io.") || lower.startsWith("net.")) return "apps";

  return "other";
}

interface CacheGroup {
  id: CacheCategory;
  label: string;
  entries: typeof caches.value;
  totalSize: number;
  totalItems: number;
}

const CATEGORY_LABELS: Record<CacheCategory, string> = {
  browsers: "Browsers",
  devtools: "Developer Tools",
  apple: "Apple / System",
  apps: "Applications",
  other: "Other",
};

const CATEGORY_ORDER: CacheCategory[] = ["browsers", "devtools", "apps", "apple", "other"];

const groupedCaches = computed<CacheGroup[]>(() => {
  const groups: Record<CacheCategory, typeof caches.value> = {
    browsers: [], devtools: [], apple: [], apps: [], other: [],
  };
  for (const entry of caches.value) {
    const cat = categoriseCache(entry.name, entry.path);
    groups[cat].push(entry);
  }
  return CATEGORY_ORDER
    .filter(cat => groups[cat].length > 0)
    .map(cat => ({
      id: cat,
      label: CATEGORY_LABELS[cat],
      entries: groups[cat].sort((a, b) => b.size - a.size),
      totalSize: groups[cat].reduce((s, e) => s + e.size, 0),
      totalItems: groups[cat].reduce((s, e) => s + e.item_count, 0),
    }));
});

const collapsedCategories = ref<Set<string>>(new Set());

function toggleCategory(id: string) {
  const next = new Set(collapsedCategories.value);
  if (next.has(id)) next.delete(id); else next.add(id);
  collapsedCategories.value = next;
}

function selectCategory(group: CacheGroup) {
  const next = new Set(selected.value);
  for (const e of group.entries) next.add(e.path);
  selected.value = next;
}

function deselectCategory(group: CacheGroup) {
  const next = new Set(selected.value);
  for (const e of group.entries) next.delete(e.path);
  selected.value = next;
}

function isCategoryAllSelected(group: CacheGroup): boolean {
  return group.entries.every(e => selected.value.has(e.path));
}
</script>

<template>
  <section class="caches-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Caches</h2>
          <p class="text-muted">Application and system caches</p>
        </div>
        <button class="btn-primary scan-btn" :disabled="cachesScanning" @click="scan">
          <span v-if="cachesScanning" class="spinner-sm"></span>
          {{ cachesScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <FdaWarningBanner
      text="Without Full Disk Access, only Xcode and CoreSimulator caches are shown. Other app caches in ~/Library/Caches are skipped."
    />

    <div v-if="cachesError" class="error-message">{{ cachesError }}</div>
    <div v-if="deleteError" class="error-message">{{ deleteError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <div v-if="cachesScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning caches...</span>
    </div>

    <EmptyState
      v-else-if="cachesScanned && caches.length === 0"
      title="No caches found"
      description="Application and system caches will appear here after scanning."
    />

    <template v-else-if="caches.length > 0">
      <div class="summary-bar">
        <span class="results-count">
          {{ caches.length }} cache(s) -- {{ formatSize(totalCacheSize) }} total
        </span>
        <div class="results-actions">
          <span v-if="selected.size > 0" class="selected-info">
            {{ selected.size }} selected ({{ formatSize(totalSelected) }})
          </span>
          <button class="btn-danger" :disabled="selected.size === 0 || deleting" @click="cleanSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Cleaning..." : "Clean Selected" }}
          </button>
        </div>
      </div>

      <div class="select-all-row">
        <Checkbox :is-on="allSelected" @change="toggleAll">Select all</Checkbox>
      </div>

      <div class="cache-groups">
        <div v-for="group in groupedCaches" :key="group.id" class="cache-category">
          <!-- Category header -->
          <div class="category-header" tabindex="0" role="button" :aria-expanded="!collapsedCategories.has(group.id)" @click="toggleCategory(group.id)" @keydown.enter="toggleCategory(group.id)" @keydown.space.prevent="toggleCategory(group.id)">
            <div class="category-header-left">
              <span class="category-chevron" :class="{ expanded: !collapsedCategories.has(group.id) }">
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
              </span>
              <span class="category-label">{{ group.label }}</span>
              <span class="badge pill badge-neutral">{{ group.entries.length }}</span>
            </div>
            <div class="category-header-right">
              <span class="category-size mono">{{ formatSize(group.totalSize) }}</span>
              <button
                class="btn-sm btn-secondary"
                @click.stop="isCategoryAllSelected(group) ? deselectCategory(group) : selectCategory(group)"
              >{{ isCategoryAllSelected(group) ? 'Deselect' : 'Select all' }}</button>
            </div>
          </div>

          <!-- Category items -->
          <div v-if="!collapsedCategories.has(group.id)" class="cache-list">
            <div
              v-for="entry in group.entries"
              :key="entry.path"
              class="cache-item"
              :class="{ 'cache-item--selected': selected.has(entry.path) }"
              @click="toggleSelect(entry.path)"
            >
              <div class="cache-icon">
                <img v-if="folderIcon" :src="folderIcon" alt="" width="28" height="28" />
              </div>
              <div class="cache-info">
                <span class="cache-name">{{ entry.name }}</span>
                <span class="cache-path text-muted" :title="entry.path">{{ entry.path.replace(/^\/Users\/[^/]+/, '~') }}</span>
              </div>
              <div class="cache-size-col">
                <span class="cache-size mono">{{ formatSize(entry.size) }}</span>
                <span class="cache-count text-muted">{{ entry.item_count.toLocaleString() }} items</span>
              </div>
              <Checkbox :is-on="selected.has(entry.path)" @change="toggleSelect(entry.path)" />
            </div>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.caches-view { max-width: 1440px; }

.cache-groups {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.cache-category {
  background: var(--glass);
  border-radius: var(--radius-md);
  border: 1px solid rgba(0, 0, 0, 0.05);
  overflow: hidden;
}

.category-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.15s ease;
}

.category-header:hover {
  background: rgba(255, 255, 255, 0.3);
}

.category-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.category-chevron {
  display: flex;
  transition: transform 0.15s;
  color: var(--muted);
}

.category-chevron.expanded {
  transform: rotate(90deg);
}

.category-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

/* category-count: uses global .badge .pill .badge-neutral */

.category-header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.category-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.cache-list {
  display: flex;
  flex-direction: column;
}

.cache-item {
  display: grid;
  grid-template-columns: 44px 1fr 90px 28px;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  cursor: pointer;
  border-radius: 10px;
  transition: background 0.15s ease;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
}

.cache-item:hover {
  background: rgba(255, 255, 255, 0.3);
}

.cache-item--selected {
  background: rgba(2, 117, 244, 0.06);
}

.cache-icon {
  display: flex;
  align-items: center;
  justify-content: center;
}

.cache-icon img {
  border-radius: 6px;
}

.cache-icon-fallback {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  opacity: 0.4;
}

.cache-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.cache-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cache-path {
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cache-size-col {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 1px;
}

.cache-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
}

.cache-count {
  font-size: 10px;
}

.select-all-row { margin-bottom: var(--sp-2); padding: 0 var(--sp-1); }
</style>
