<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize } from "../utils";
import {
  browserResult,
  browserScanning,
  browserScanned,
  browserError,
  scanBrowsers,
  cleanBrowserData,
} from "../stores/scanStore";
import type { BrowserInfo, BrowserDataCategory } from "../types";
import FdaWarningBanner from "../components/FdaWarningBanner.vue";
import EmptyState from "../components/EmptyState.vue";
import Checkbox from "../components/Checkbox.vue";

// ── Browser app icons via NSWorkspace ─────────────────────────────────
const browserIcons = ref<Record<string, string>>({});

const BROWSER_APP_PATHS: Record<string, string> = {
  "chrome": "/Applications/Google Chrome.app",
  "google_chrome": "/Applications/Google Chrome.app",
  "edge": "/Applications/Microsoft Edge.app",
  "microsoft_edge": "/Applications/Microsoft Edge.app",
  "safari": "/Applications/Safari.app",
  "firefox": "/Applications/Firefox.app",
  "brave": "/Applications/Brave Browser.app",
  "opera": "/Applications/Opera.app",
  "vivaldi": "/Applications/Vivaldi.app",
  "arc": "/Applications/Arc.app",
};

async function loadBrowserIcon(browserId: string) {
  if (browserIcons.value[browserId] !== undefined) return;
  browserIcons.value[browserId] = "";
  const appPath = BROWSER_APP_PATHS[browserId.toLowerCase()];
  if (!appPath) return;
  try {
    const b64 = await invoke<string>("render_sf_symbol", { name: appPath, size: 64, mode: "app", style: "plain" });
    if (b64) browserIcons.value[browserId] = b64;
  } catch { /* non-critical */ }
}

function getBrowserIcon(browserId: string): string {
  if (browserIcons.value[browserId] === undefined) loadBrowserIcon(browserId);
  return browserIcons.value[browserId] || "";
}

// Track which browser panels are expanded
const expandedBrowsers = ref<Set<string>>(new Set());

// Track selected paths for cleaning: Map<"browserId:categoryId", paths[]>
const selected = ref<Set<string>>(new Set());

// Cleaning state
const cleaning = ref(false);
const successMsg = ref("");
const cleanError = ref("");

// Confirmation modal state
const showConfirmModal = ref(false);
const confirmMessage = ref("");
const pendingCleanPaths = ref<string[]>([]);
async function scan() {
  successMsg.value = "";
  cleanError.value = "";
  selected.value = new Set();
  expandedBrowsers.value = new Set();
  await scanBrowsers();
  // Auto-expand browsers that have data
  if (browserResult.value) {
    for (const b of browserResult.value.browsers) {
      if (b.total_size > 0 || b.data_categories.some((c) => c.tcc_protected)) {
        expandedBrowsers.value.add(b.id);
      }
    }
  }
}

function toggleBrowser(id: string) {
  const next = new Set(expandedBrowsers.value);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  expandedBrowsers.value = next;
}

/** Build a unique key for a browser+category combination */
function catKey(browserId: string, catId: string): string {
  return `${browserId}:${catId}`;
}

function toggleCategory(browserId: string, cat: BrowserDataCategory) {
  // Don't allow selecting TCC-protected categories when no FDA
  if (cat.tcc_protected && !browserResult.value?.has_fda) return;
  // Don't allow selecting 0-size categories
  if (cat.size === 0 && !cat.tcc_protected) return;

  const key = catKey(browserId, cat.id);
  const next = new Set(selected.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  selected.value = next;
}

function isCatSelected(browserId: string, catId: string): boolean {
  return selected.value.has(catKey(browserId, catId));
}

/** Get all paths for the currently selected categories */
function getSelectedPaths(): string[] {
  if (!browserResult.value) return [];
  const paths: string[] = [];
  for (const browser of browserResult.value.browsers) {
    for (const cat of browser.data_categories) {
      if (selected.value.has(catKey(browser.id, cat.id))) {
        paths.push(...cat.paths);
      }
    }
  }
  return paths;
}

/** Get warnings for selected unsafe categories */
function getSelectedWarnings(): string[] {
  if (!browserResult.value) return [];
  const warnings: string[] = [];
  for (const browser of browserResult.value.browsers) {
    for (const cat of browser.data_categories) {
      if (
        selected.value.has(catKey(browser.id, cat.id)) &&
        !cat.safe_to_clean &&
        cat.warning
      ) {
        warnings.push(cat.warning);
      }
    }
  }
  return warnings;
}

const totalSelected = computed(() => {
  if (!browserResult.value) return 0;
  let total = 0;
  for (const browser of browserResult.value.browsers) {
    for (const cat of browser.data_categories) {
      if (selected.value.has(catKey(browser.id, cat.id))) {
        total += cat.size;
      }
    }
  }
  return total;
});

async function cleanSelected() {
  if (selected.value.size === 0) return;

  const paths = getSelectedPaths();
  const warnings = getSelectedWarnings();

  if (warnings.length > 0) {
    // Show confirmation modal for unsafe operations
    confirmMessage.value = warnings.join("\n");
    pendingCleanPaths.value = paths;
    showConfirmModal.value = true;
    return;
  }

  // Safe categories — clean directly
  await doClean(paths);
}

async function confirmClean() {
  showConfirmModal.value = false;
  await doClean(pendingCleanPaths.value);
  pendingCleanPaths.value = [];
}

function cancelClean() {
  showConfirmModal.value = false;
  pendingCleanPaths.value = [];
}

async function doClean(paths: string[]) {
  cleaning.value = true;
  cleanError.value = "";
  successMsg.value = "";
  try {
    const result = await cleanBrowserData(paths);
    if (result.success) {
      successMsg.value = `Cleaned ${result.deleted_count} item(s), freed ${formatSize(result.freed_bytes)}`;
    }
    if (result.errors.length > 0) {
      cleanError.value = result.errors.join("; ");
    }
    // Re-scan to update sizes
    selected.value = new Set();
    await scanBrowsers();
  } catch (e) {
    cleanError.value = String(e);
  } finally {
    cleaning.value = false;
  }
}

/** Select all safe-to-clean categories for a browser */
function selectAllSafe(browser: BrowserInfo) {
  const next = new Set(selected.value);
  for (const cat of browser.data_categories) {
    if (cat.safe_to_clean && cat.size > 0) {
      next.add(catKey(browser.id, cat.id));
    }
  }
  selected.value = next;
}

</script>

<template>
  <section class="browsers-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Browser Cleanup</h2>
          <p class="text-muted">
            Cache, cookies, history, and session data
          </p>
        </div>
        <button
          class="btn-primary scan-btn"
          :disabled="browserScanning"
          @click="scan"
        >
          <span v-if="browserScanning" class="spinner-sm"></span>
          {{ browserScanning ? "Scanning..." : "Scan Browsers" }}
        </button>
      </div>
    </div>

    <!-- FDA warning for Safari -->
    <FdaWarningBanner
      title="Safari data requires Full Disk Access"
      text="Safari stores data in TCC-protected directories. Without Full Disk Access, Safari cache, cookies, and history cannot be scanned or cleaned. Other browsers are not affected."
    />

    <!-- Error/success messages -->
    <div v-if="browserError" class="error-message">{{ browserError }}</div>
    <div v-if="cleanError" class="error-message">{{ cleanError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <!-- Loading state -->
    <div v-if="browserScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning browsers...</span>
    </div>

    <!-- Empty state -->
    <EmptyState
      v-else-if="browserScanned && (!browserResult || browserResult.browsers.length === 0)"
      title="No browsers with cleanable data found"
      description="Browser cache, cookies, and history data will appear here after scanning."
    />

    <!-- Results -->
    <template v-else-if="browserResult && browserResult.browsers.length > 0">
      <!-- Summary bar -->
      <div class="summary-bar">
        <span class="results-count">
          {{ browserResult.browsers.length }} browser(s) --
          {{ formatSize(browserResult.total_size) }} total cleanable data
        </span>
        <div class="results-actions">
          <span v-if="selected.size > 0" class="selected-info">
            {{ selected.size }} selected ({{ formatSize(totalSelected) }})
          </span>
          <button
            class="btn-danger"
            :disabled="selected.size === 0 || cleaning"
            @click="cleanSelected"
          >
            <span v-if="cleaning" class="spinner-sm"></span>
            {{ cleaning ? "Cleaning..." : "Clean Selected" }}
          </button>
        </div>
      </div>

      <!-- Browser cards -->
      <div class="browser-list">
        <div
          v-for="browser in browserResult.browsers"
          :key="browser.id"
          class="card-flush browser-card"
        >
          <!-- Browser header (click to expand/collapse) -->
          <div
            class="browser-header"
            @click="toggleBrowser(browser.id)"
          >
            <div class="browser-header-left">
              <span
                class="expand-chevron"
                :class="{ expanded: expandedBrowsers.has(browser.id) }"
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
              </span>
              <img v-if="getBrowserIcon(browser.id)" :src="getBrowserIcon(browser.id)" :alt="browser.name" class="browser-icon" width="44" height="44" />
              <div class="browser-info">
                <span class="browser-name">{{ browser.name }}</span>
                <span class="browser-meta text-muted">
                  {{ browser.data_categories.length }} categories
                  <span v-if="!browser.installed" class="badge badge-warning">
                    Not installed (leftover data)
                  </span>
                </span>
              </div>
            </div>
            <div class="browser-header-right">
              <span class="browser-size">
                {{ formatSize(browser.total_size) }}
              </span>
              <div class="browser-quick-actions" @click.stop>
                <button
                  class="btn-sm btn-secondary"
                  @click="selectAllSafe(browser)"
                  title="Select safe-to-clean categories (cache)"
                >
                  Select Safe
                </button>
              </div>
            </div>
          </div>

          <!-- Browser data categories (expanded) -->
          <div
            v-if="expandedBrowsers.has(browser.id)"
            class="browser-categories"
          >
            <div
              v-for="cat in browser.data_categories"
              :key="cat.id"
              :class="[
                'category-row',
                {
                  selected: isCatSelected(browser.id, cat.id),
                  disabled:
                    (cat.tcc_protected && !browserResult.has_fda) ||
                    (cat.size === 0 && !cat.tcc_protected),
                },
              ]"
              @click="toggleCategory(browser.id, cat)"
            >
              <div class="category-left">
                <Checkbox
                  :model-value="isCatSelected(browser.id, cat.id)"
                  :disabled="(cat.tcc_protected && !browserResult.has_fda) || (cat.size === 0 && !cat.tcc_protected)"
                  @change="toggleCategory(browser.id, cat)"
                />
                <div class="category-info">
                  <div class="category-label-row">
                    <span class="category-label">{{ cat.label }}</span>
                    <span v-if="cat.safe_to_clean" class="badge badge-success">
                      Safe
                    </span>
                    <span v-else class="badge badge-warning">Caution</span>
                    <span
                      v-if="cat.tcc_protected && !browserResult.has_fda"
                      class="badge badge-danger"
                    >
                      Needs FDA
                    </span>
                  </div>
                  <span
                    v-if="!cat.safe_to_clean && cat.warning"
                    class="category-warning text-muted"
                  >
                    {{ cat.warning }}
                  </span>
                  <span
                    v-if="cat.tcc_protected && !browserResult.has_fda"
                    class="category-warning text-muted"
                  >
                    Requires Full Disk Access to scan and clean
                  </span>
                </div>
              </div>
              <div class="category-right">
                <span class="category-size">
                  {{
                    cat.tcc_protected && !browserResult.has_fda
                      ? "--"
                      : formatSize(cat.size)
                  }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Confirmation modal for unsafe operations -->
    <Teleport to="body">
      <div v-if="showConfirmModal" class="modal-overlay" role="dialog" aria-modal="true" aria-labelledby="modal-title" @click.self="cancelClean">
        <div class="modal-content">
          <h3 id="modal-title">Confirm Cleanup</h3>
          <p class="modal-warning-text">
            The selected items include data that may affect your browsing
            experience:
          </p>
          <ul class="modal-warning-list">
            <li
              v-for="(warning, idx) in confirmMessage.split('\n')"
              :key="idx"
            >
              {{ warning }}
            </li>
          </ul>
          <p class="modal-warning-text">
            This action cannot be undone. Continue?
          </p>
          <div class="modal-actions">
            <button class="btn-secondary" @click="cancelClean">Cancel</button>
            <button class="btn-danger" @click="confirmClean">
              Clean Anyway
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.browsers-view {
  max-width: 1440px;
}

/* Browser list */
.browser-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}

.browser-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-4) var(--sp-5);
  cursor: pointer;
  transition: background 0.15s;
}

.browser-header:hover {
  background: var(--surface-alt);
}

.browser-header-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.browser-icon {
  border-radius: 6px;
  flex-shrink: 0;
}

.browser-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.browser-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.browser-meta {
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.browser-header-right {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
}

.browser-size {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.browser-quick-actions {
  display: flex;
  gap: var(--sp-2);
}

/* Category rows */
.browser-categories {
  border-top: 1px solid var(--border-divider);
}

.category-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-3) var(--sp-5) var(--sp-3) 48px;
  cursor: pointer;
  transition: background 0.15s ease;
  border-bottom: 1px solid var(--border-divider);
}

.category-row:last-child {
  border-bottom: none;
}

.category-row:hover:not(.disabled) {
  background: var(--surface-alt);
}

.category-row.selected {
  background: var(--accent-light);
}

.category-row.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.category-left {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-3);
  flex: 1;
}


.category-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.category-label-row {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.category-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.category-warning {
  font-size: 11px;
  line-height: 1.4;
}

.category-right {
  flex-shrink: 0;
  margin-left: var(--sp-4);
}

.category-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

/* Confirmation modal (view-specific content styles) */
.modal-warning-text {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: var(--sp-3);
}

.modal-warning-list {
  list-style: none;
  padding: 0;
  margin: 0 0 var(--sp-4) 0;
}

.modal-warning-list li {
  font-size: 13px;
  color: var(--danger);
  padding: var(--sp-1) 0 var(--sp-1) var(--sp-4);
  position: relative;
}

.modal-warning-list li::before {
  content: "";
  position: absolute;
  left: 0;
  top: 10px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--danger);
}
</style>
