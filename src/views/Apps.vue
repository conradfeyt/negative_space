<script setup lang="ts">
import { ref, computed } from "vue";
import type { AppInfo } from "../types";
import { formatSize } from "../utils";
import {
  apps,
  appsScanning,
  appsScanned,
  appsError,
  scanApps,
  uninstallApp as storeUninstallApp,
  hasFullDiskAccess,
} from "../stores/scanStore";
import FdaWarningBanner from "../components/FdaWarningBanner.vue";
import StatCard from "../components/StatCard.vue";
import EmptyState from "../components/EmptyState.vue";

const expanded = ref<Set<string>>(new Set());
const uninstalling = ref<string | null>(null);
const confirmUninstall = ref<string | null>(null);
const successMsg = ref("");
const searchQuery = ref("");

// Summary stats
const totalFootprint = computed(() =>
  apps.value.reduce((sum, a) => sum + a.footprint, 0)
);
const totalLeftovers = computed(() =>
  apps.value.reduce((sum, a) => sum + a.leftover_size, 0)
);
const homebrewCount = computed(() =>
  apps.value.filter((a) => a.install_source === "homebrew").length
);
const appStoreCount = computed(() =>
  apps.value.filter((a) => a.install_source === "app-store").length
);

// Filtered apps based on search
const filteredApps = computed(() => {
  if (!searchQuery.value.trim()) return apps.value;
  const q = searchQuery.value.toLowerCase();
  return apps.value.filter(
    (a) =>
      a.name.toLowerCase().includes(q) ||
      a.bundle_id.toLowerCase().includes(q) ||
      a.install_source.includes(q)
  );
});

async function scan() {
  successMsg.value = "";
  expanded.value = new Set();
  searchQuery.value = "";
  await scanApps();
}

function requestUninstall(app: AppInfo) {
  confirmUninstall.value = app.path;
}

function cancelUninstall() {
  confirmUninstall.value = null;
}

async function handleUninstall(app: AppInfo) {
  confirmUninstall.value = null;
  uninstalling.value = app.path;
  successMsg.value = "";
  try {
    const result = await storeUninstallApp(app.path, true);
    if (result.success) {
      successMsg.value = `Uninstalled "${app.name}", freed ${formatSize(result.freed_bytes)}`;
      apps.value = apps.value.filter((a) => a.path !== app.path);
    }
    if (result.errors.length > 0) {
      appsError.value = result.errors.join("; ");
    }
  } catch (e) {
    appsError.value = String(e);
  } finally {
    uninstalling.value = null;
  }
}

function toggleExpand(path: string) {
  const next = new Set(expanded.value);
  if (next.has(path)) {
    next.delete(path);
  } else {
    next.add(path);
  }
  expanded.value = next;
}

function sourceLabel(source: string): string {
  if (source === "homebrew") return "Homebrew";
  if (source === "app-store") return "App Store";
  return "";
}
</script>

<template>
  <section class="apps-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Apps</h2>
          <p class="text-muted">
            Manage applications and their full disk footprint
          </p>
        </div>
        <button
          class="btn-primary scan-btn"
          :disabled="appsScanning"
          @click="scan"
        >
          <span v-if="appsScanning" class="spinner-sm"></span>
          {{ appsScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <FdaWarningBanner
      title="Leftover detection limited -- Full Disk Access required"
      text="Without Full Disk Access, leftover files in ~/Library cannot be detected. Apps are still listed with their bundle sizes."
    />

    <div v-if="appsError" class="error-message">{{ appsError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <div v-if="appsScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning applications and detecting leftovers...</span>
    </div>

    <EmptyState
      v-else-if="appsScanned && apps.length === 0"
      title="No applications found"
      description="Installed applications and their disk footprint will appear here after scanning."
    />

    <template v-else-if="apps.length > 0">
      <!-- Summary stats -->
      <div class="stats-row">
        <StatCard :value="String(apps.length)" label="Apps found" />
        <StatCard :value="formatSize(totalFootprint)" label="Total footprint" />
        <StatCard v-if="totalLeftovers > 0" :value="formatSize(totalLeftovers)" label="Leftover data" value-color="var(--warning)" />
        <StatCard v-if="homebrewCount > 0" :value="String(homebrewCount)" label="via Homebrew" />
        <StatCard v-if="appStoreCount > 0" :value="String(appStoreCount)" label="via App Store" />
      </div>

      <!-- Search -->
      <div class="search-bar">
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Filter apps..."
          class="search-input"
        />
        <span v-if="searchQuery" class="search-count text-muted">
          {{ filteredApps.length }} of {{ apps.length }}
        </span>
      </div>

      <!-- App list -->
      <div class="app-list">
        <div v-for="app in filteredApps" :key="app.path" class="card-flush app-item">
          <div class="app-row" @click="toggleExpand(app.path)">
            <!-- Icon -->
            <div class="app-icon-wrapper">
              <img
                v-if="app.icon_base64"
                :src="app.icon_base64"
                :alt="app.name"
                class="app-icon"
              />
              <div v-else class="app-icon-placeholder">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="3" y="3" width="18" height="18" rx="4"/>
                  <path d="M8 12h8M12 8v8"/>
                </svg>
              </div>
            </div>

            <!-- Info -->
            <div class="app-info">
              <div class="app-name-row">
                <span
                  class="expand-chevron"
                  :class="{ expanded: expanded.has(app.path) }"
                >
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
                </span>
                <span class="app-name">{{ app.name }}</span>
                <span v-if="app.install_source !== 'manual'" class="source-badge" :class="'source-' + app.install_source">
                  {{ sourceLabel(app.install_source) }}
                </span>
              </div>
              <div class="app-meta">
                <span class="mono text-muted">{{ app.bundle_id || "---" }}</span>
              </div>
            </div>

            <!-- Size columns -->
            <div class="app-sizes">
              <div class="size-col">
                <span class="size-label">App</span>
                <span class="size-value mono">{{ formatSize(app.size) }}</span>
              </div>
              <div v-if="app.leftover_size > 0" class="size-col size-col-leftover">
                <span class="size-label">Leftovers</span>
                <span class="size-value mono size-warning">{{ formatSize(app.leftover_size) }}</span>
              </div>
              <div class="size-col size-col-footprint">
                <span class="size-label">Footprint</span>
                <span class="size-value mono size-accent" :class="{ 'size-large': app.footprint > 1_000_000_000 }">
                  {{ formatSize(app.footprint) }}
                </span>
              </div>
            </div>

            <!-- Actions -->
            <div class="app-actions">
              <template v-if="confirmUninstall === app.path">
                <span class="confirm-text">Uninstall this app?</span>
                <button class="btn-secondary btn-sm" @click.stop="cancelUninstall">
                  Cancel
                </button>
                <button
                  class="btn-danger btn-sm"
                  :disabled="uninstalling === app.path"
                  @click.stop="handleUninstall(app)"
                >
                  <span v-if="uninstalling === app.path" class="spinner-sm"></span>
                  {{ uninstalling === app.path ? "Removing..." : "Yes, Uninstall" }}
                </button>
              </template>
              <button
                v-else
                class="btn-danger btn-sm"
                :disabled="!!uninstalling"
                @click.stop="requestUninstall(app)"
              >
                Uninstall
              </button>
            </div>
          </div>

          <!-- Expanded details -->
          <div v-if="expanded.has(app.path)" class="app-expanded">
            <div class="expanded-section">
              <span class="expanded-label">Application path</span>
              <span class="mono expanded-path">{{ app.path }}</span>
            </div>

            <!-- Footprint breakdown bar -->
            <div v-if="app.leftover_size > 0" class="expanded-section">
              <span class="expanded-label">Footprint breakdown</span>
              <div class="footprint-bar">
                <div
                  class="footprint-bar-app"
                  :style="{ width: Math.round((app.size / app.footprint) * 100) + '%' }"
                >
                  <span v-if="(app.size / app.footprint) > 0.15" class="footprint-bar-text">
                    App {{ formatSize(app.size) }}
                  </span>
                </div>
                <div
                  class="footprint-bar-leftover"
                  :style="{ width: Math.round((app.leftover_size / app.footprint) * 100) + '%' }"
                >
                  <span v-if="(app.leftover_size / app.footprint) > 0.15" class="footprint-bar-text">
                    Leftovers {{ formatSize(app.leftover_size) }}
                  </span>
                </div>
              </div>
            </div>

            <div v-if="app.leftover_paths.length > 0" class="expanded-section">
              <span class="expanded-label">
                Leftover locations ({{ app.leftover_paths.length }})
              </span>
              <ul class="leftover-list">
                <li v-for="lpath in app.leftover_paths" :key="lpath" class="mono">
                  {{ lpath }}
                </li>
              </ul>
            </div>
            <div v-else class="expanded-section">
              <span class="text-muted">
                {{ hasFullDiskAccess ? 'No leftover files detected' : 'Grant Full Disk Access to detect leftovers' }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.apps-view {
  max-width: 1440px;
}

/* Summary stats row */
.stats-row {
  display: flex;
  gap: 8px;
  margin-bottom: var(--sp-5);
  flex-wrap: wrap;
}


/* Search bar */
.search-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: var(--sp-4);
}

.search-input {
  flex: 1;
  padding: 8px 14px;
  font-size: 13px;
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.5);
  color: var(--text);
  outline: none;
  transition: border-color 0.2s;
}

.search-input:focus {
  border-color: var(--accent);
}

.search-input::placeholder {
  color: var(--muted);
}

.search-count {
  font-size: 12px;
  white-space: nowrap;
}

/* App list */
.app-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.app-row {
  display: flex;
  align-items: center;
  padding: var(--sp-3) var(--sp-4);
  cursor: pointer;
  transition: background 0.15s ease;
  gap: 12px;
}

.app-row:hover {
  background: var(--surface-alt);
}

/* App icon */
.app-icon-wrapper {
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.app-icon {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  /* Subtle shadow to lift icon off the glass background */
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.app-icon-placeholder {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.04);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
}

/* App info */
.app-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.app-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.app-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app-meta {
  padding-left: 20px;
  font-size: 11px;
}

/* Size columns */
.app-sizes {
  display: flex;
  gap: 16px;
  flex-shrink: 0;
  margin-left: auto;
}

.size-col {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 1px;
  min-width: 64px;
}

.size-label {
  font-size: 10px;
  color: var(--muted);
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.size-value {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap;
}

.size-warning {
  color: var(--warning);
}

.size-accent {
  color: var(--text);
}

.size-large {
  color: var(--danger);
}

/* Actions */
.app-actions {
  flex-shrink: 0;
  margin-left: 8px;
}

/* Expanded details */
.app-expanded {
  border-top: 1px solid var(--border-divider);
  padding: var(--sp-4) var(--sp-5) var(--sp-4) 64px;
  background: transparent;
  animation: slideDown 0.15s ease;
}

.expanded-section {
  margin-bottom: var(--sp-3);
}

.expanded-section:last-child {
  margin-bottom: 0;
}

.expanded-label {
  display: block;
  font-size: 11px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: var(--sp-1);
}

.expanded-path {
  font-size: 12px;
  color: var(--text-secondary);
  word-break: break-all;
}

/* Footprint breakdown bar */
.footprint-bar {
  display: flex;
  height: 22px;
  border-radius: 6px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.04);
}

.footprint-bar-app {
  background: rgba(59, 199, 232, 0.30);
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 4px;
  transition: width 0.3s ease;
}

.footprint-bar-leftover {
  background: rgba(251, 146, 60, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 4px;
  transition: width 0.3s ease;
}

.footprint-bar-text {
  font-size: 10px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  padding: 0 6px;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Leftover list */
.leftover-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
}

.leftover-list li {
  font-size: 11px;
  color: var(--text-secondary);
  padding: var(--sp-1) var(--sp-2);
  background: transparent;
  border-radius: var(--radius-sm);
  word-break: break-all;
}
</style>
