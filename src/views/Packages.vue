<script setup lang="ts">
import { ref, computed } from "vue";
import { formatSize } from "../utils";
import {
  packagesResult,
  packagesScanning,
  packagesScanned,
  packagesError,
  scanPackages,
} from "../stores/scanStore";
import StatCard from "../components/StatCard.vue";
import EmptyState from "../components/EmptyState.vue";
import SegmentedControl from "../components/SegmentedControl.vue";

const expandedManagers = ref<Set<string>>(new Set());
const expandedRuntimes = ref<Set<string>>(new Set());
const showDeps = ref<Set<string>>(new Set());
const filter = ref<"all" | "top-level">("top-level");

async function scan() {
  expandedManagers.value = new Set();
  expandedRuntimes.value = new Set();
  showDeps.value = new Set();
  await scanPackages();
}

function toggleManager(id: string) {
  const next = new Set(expandedManagers.value);
  next.has(id) ? next.delete(id) : next.add(id);
  expandedManagers.value = next;
}

function toggleRuntime(id: string) {
  const next = new Set(expandedRuntimes.value);
  next.has(id) ? next.delete(id) : next.add(id);
  expandedRuntimes.value = next;
}

function toggleDeps(pkgKey: string) {
  const next = new Set(showDeps.value);
  next.has(pkgKey) ? next.delete(pkgKey) : next.add(pkgKey);
  showDeps.value = next;
}

// Filtered packages per manager
function visiblePackages(managerId: string) {
  const mgr = packagesResult.value?.managers.find((m) => m.id === managerId);
  if (!mgr) return [];
  if (filter.value === "top-level") return mgr.packages.filter((p) => p.is_top_level);
  return mgr.packages;
}

const totalManagerSize = computed(() =>
  packagesResult.value?.managers.reduce((s, m) => s + m.total_size, 0) ?? 0
);

const totalRuntimeSize = computed(() =>
  packagesResult.value?.runtimes.reduce((s, r) => s + r.total_size, 0) ?? 0
);
</script>

<template>
  <section class="packages-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Packages</h2>
          <p class="text-muted">
            Package managers, installed packages, and language runtimes
          </p>
        </div>
        <button
          class="btn-primary scan-btn"
          :disabled="packagesScanning"
          @click="scan"
        >
          <span v-if="packagesScanning" class="spinner-sm"></span>
          {{ packagesScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <div v-if="packagesError" class="error-message">{{ packagesError }}</div>

    <div v-if="packagesScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Detecting package managers and runtimes...</span>
    </div>

    <div v-else-if="packagesScanned && packagesResult">
      <!-- Summary -->
      <div class="stats-row">
        <StatCard :value="String(packagesResult.managers.length)" label="Package managers" />
        <StatCard :value="String(packagesResult.runtimes.length)" label="Runtimes" />
        <StatCard :value="formatSize(packagesResult.total_size)" label="Total disk usage" />
      </div>

      <!-- Package Managers -->
      <div v-if="packagesResult.managers.length > 0" class="section">
        <div class="section-header">
          <h3 class="section-title">Package Managers</h3>
          <span class="section-size mono text-muted">{{ formatSize(totalManagerSize) }}</span>
        </div>

        <div class="section-toolbar">
          <SegmentedControl
            :options="[
              { value: 'top-level', label: 'Top-level' },
              { value: 'all', label: 'All packages' },
            ]"
            v-model="filter"
          />
        </div>

        <div v-for="mgr in packagesResult.managers" :key="mgr.id" class="card-flush manager-card">
          <!-- Manager header -->
          <div class="manager-header" tabindex="0" role="button" :aria-expanded="expandedManagers.has(mgr.id)" @click="toggleManager(mgr.id)" @keydown.enter="toggleManager(mgr.id)" @keydown.space.prevent="toggleManager(mgr.id)">
            <span
              class="expand-chevron"
              :class="{ expanded: expandedManagers.has(mgr.id) }"
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
            </span>
            <div class="manager-title">
              <span class="manager-name">{{ mgr.name }}</span>
              <span class="text-muted manager-version">{{ mgr.version }}</span>
            </div>
            <div class="manager-stats">
              <span class="manager-count text-muted">
                {{ mgr.packages.filter(p => p.is_top_level).length }} packages
                <span v-if="mgr.total_package_count > mgr.packages.filter(p => p.is_top_level).length">
                  ({{ mgr.total_package_count }} with deps)
                </span>
              </span>
              <span class="manager-size mono">{{ formatSize(mgr.total_size) }}</span>
            </div>
          </div>

          <!-- Expanded: package list -->
          <div v-if="expandedManagers.has(mgr.id)" class="manager-body">
            <div class="manager-path mono text-muted">{{ mgr.install_path }}</div>

            <div class="pkg-list">
              <div
                v-for="pkg in visiblePackages(mgr.id)"
                :key="mgr.id + '/' + pkg.name"
                class="pkg-item"
              >
                <div class="pkg-row">
                  <div class="pkg-info">
                    <span class="pkg-name">{{ pkg.name }}</span>
                    <span class="pkg-version mono text-muted">{{ pkg.version }}</span>
                    <span v-if="!pkg.is_top_level" class="badge-pill badge-neutral">dep</span>
                  </div>
                  <div class="pkg-right">
                    <span v-if="pkg.size > 0" class="pkg-size mono">{{ formatSize(pkg.size) }}</span>
                    <button
                      v-if="pkg.dependencies.length > 0"
                      class="badge-pill badge-accent deps-toggle"
                      @click.stop="toggleDeps(mgr.id + '/' + pkg.name)"
                    >
                      {{ showDeps.has(mgr.id + '/' + pkg.name) ? 'hide deps' : pkg.dependencies.length + ' deps' }}
                    </button>
                  </div>
                </div>

                <!-- Dependency list -->
                <div
                  v-if="showDeps.has(mgr.id + '/' + pkg.name) && pkg.dependencies.length > 0"
                  class="pkg-deps"
                >
                  <span
                    v-for="dep in pkg.dependencies"
                    :key="dep"
                    class="dep-tag mono"
                  >{{ dep }}</span>
                </div>

                <!-- Removal info -->
                <div v-if="showDeps.has(mgr.id + '/' + pkg.name)" class="pkg-removal">
                  <div class="removal-cmd">
                    <span class="removal-label">Remove:</span>
                    <code class="mono">{{ pkg.uninstall_command }}</code>
                  </div>
                  <div v-if="pkg.removal_warning" class="removal-warning">
                    {{ pkg.removal_warning }}
                  </div>
                </div>
              </div>
            </div>

            <!-- Manager-level uninstall hint -->
            <div class="manager-hint">
              <span class="removal-label">Manager:</span>
              <span class="text-muted">{{ mgr.uninstall_hint }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Runtimes -->
      <div v-if="packagesResult.runtimes.length > 0" class="section">
        <div class="section-header">
          <h3 class="section-title">Runtimes</h3>
          <span class="section-size mono text-muted">{{ formatSize(totalRuntimeSize) }}</span>
        </div>

        <div v-for="rt in packagesResult.runtimes" :key="rt.id" class="card-flush runtime-card">
          <div class="runtime-header" tabindex="0" role="button" :aria-expanded="expandedRuntimes.has(rt.id)" @click="toggleRuntime(rt.id)" @keydown.enter="toggleRuntime(rt.id)" @keydown.space.prevent="toggleRuntime(rt.id)">
            <span
              class="expand-chevron"
              :class="{ expanded: expandedRuntimes.has(rt.id) }"
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
            </span>
            <div class="runtime-title">
              <span class="runtime-name">{{ rt.name }}</span>
              <span class="source-badge" :class="'source-' + rt.install_method">
                {{ rt.install_method }}
              </span>
            </div>
            <div class="runtime-stats">
              <span class="text-muted">{{ rt.versions.length }} version{{ rt.versions.length !== 1 ? 's' : '' }}</span>
              <span class="runtime-size mono">{{ formatSize(rt.total_size) }}</span>
            </div>
          </div>

          <div v-if="expandedRuntimes.has(rt.id)" class="runtime-body">
            <div class="runtime-path mono text-muted">{{ rt.install_path }}</div>

            <div class="version-list">
              <div
                v-for="ver in rt.versions"
                :key="ver.path"
                class="version-item"
              >
                <div class="version-info">
                  <span class="version-name">{{ ver.version }}</span>
                  <span v-if="ver.active" class="badge-pill badge-accent">active</span>
                </div>
                <div class="version-right">
                  <span v-if="ver.size > 0" class="version-size mono">{{ formatSize(ver.size) }}</span>
                </div>
              </div>
            </div>

            <!-- Removal info -->
            <div class="runtime-removal">
              <div class="removal-cmd">
                <span class="removal-label">Remove:</span>
                <span class="text-muted">{{ rt.uninstall_hint }}</span>
              </div>
              <div v-if="rt.removal_warning" class="removal-warning">
                {{ rt.removal_warning }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Empty state -->
      <EmptyState
        v-if="packagesResult.managers.length === 0 && packagesResult.runtimes.length === 0"
        title="No package managers or runtimes detected"
        description="Homebrew, npm, pip, Cargo, and other package managers will appear here if installed."
      />
    </div>
  </section>
</template>

<style scoped>
.packages-view {
  max-width: 1440px;
}

/* Stats row */
.stats-row {
  display: flex;
  gap: 8px;
  margin-bottom: var(--sp-6);
  flex-wrap: wrap;
}


/* Sections */
.section {
  margin-bottom: var(--sp-8);
}

.section-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: var(--sp-3);
}

.section-header .section-title {
  margin-bottom: 0;
}

.section-size {
  font-size: 13px;
}

.section-toolbar {
  display: flex;
  gap: 4px;
  margin-bottom: var(--sp-3);
}

/* Manager cards */
.manager-card,
.runtime-card {
  margin-bottom: var(--sp-2);
}

.manager-header,
.runtime-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: var(--sp-3) var(--sp-4);
  cursor: pointer;
  transition: background 0.15s;
}

.manager-header:hover,
.runtime-header:hover {
  background: var(--surface-alt);
}

.manager-title,
.runtime-title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.manager-name,
.runtime-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.manager-version {
  font-size: 12px;
}

.manager-stats,
.runtime-stats {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  font-size: 13px;
}

.manager-size,
.runtime-size {
  font-weight: 600;
  color: var(--text);
  min-width: 64px;
  text-align: right;
}

.manager-count {
  font-size: 12px;
}

/* Manager body */
.manager-body,
.runtime-body {
  border-top: 1px solid var(--border-divider);
  padding: var(--sp-3) var(--sp-4) var(--sp-4);
  animation: slideDown 0.15s ease;
}

.manager-path,
.runtime-path {
  font-size: 11px;
  margin-bottom: var(--sp-3);
  padding: 4px 8px;
  background: rgba(0, 0, 0, 0.03);
  border-radius: 4px;
  word-break: break-all;
}

/* Package list */
.pkg-list {
  display: flex;
  flex-direction: column;
}

.pkg-item {
  border-bottom: 1px solid var(--border-divider);
  padding: 6px 0;
}

.pkg-item:last-child {
  border-bottom: none;
}

.pkg-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.pkg-info {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  flex: 1;
}

.pkg-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.pkg-version {
  font-size: 11px;
}

/* dep-badge: uses global .badge-pill .badge-neutral */

.pkg-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.pkg-size {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  min-width: 56px;
  text-align: right;
}

.deps-toggle {
  cursor: pointer;
  border: none;
  transition: opacity 0.15s;
}

.deps-toggle:hover {
  opacity: 0.75;
}

.pkg-deps {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 6px 0 4px;
}

.dep-tag {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.04);
  color: var(--text-secondary);
}

/* Removal info */
.pkg-removal,
.runtime-removal {
  padding: 6px 0 2px;
}

.removal-cmd {
  display: flex;
  align-items: baseline;
  gap: 6px;
  font-size: 12px;
  margin-bottom: 4px;
}

.removal-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  flex-shrink: 0;
}

.removal-cmd code {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.04);
  color: var(--text);
  word-break: break-all;
}

.removal-warning {
  font-size: 12px;
  color: var(--warning);
  padding: 4px 0;
  line-height: 1.5;
}

.manager-hint {
  margin-top: var(--sp-3);
  padding-top: var(--sp-3);
  border-top: 1px solid var(--border-divider);
  font-size: 12px;
  display: flex;
  align-items: baseline;
  gap: 6px;
}

/* Version list (runtimes) */
.version-list {
  display: flex;
  flex-direction: column;
}

.version-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-divider);
}

.version-item:last-child {
  border-bottom: none;
}

.version-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.version-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

/* active-badge: uses global .badge-pill .badge-accent */

.version-right {
  flex-shrink: 0;
}

.version-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
}

</style>
