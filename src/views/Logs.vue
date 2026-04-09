<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize, timeAgo } from "../utils";
import { showToast } from "../stores/toastStore";
import {
  logs,
  logsScanning,
  logsScanned,
  logsError,
  scanLogs,
  deleteFiles,
  totalLogSize,
} from "../stores/scanStore";
import EmptyState from "../components/EmptyState.vue";
import Checkbox from "../components/Checkbox.vue";

const selected = ref<Set<string>>(new Set());
const deleting = ref(false);

async function scan() {
  selected.value = new Set();
  await scanLogs();
}

async function cleanSelected() {
  if (selected.value.size === 0) return;
  deleting.value = true;
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      showToast(`Cleaned ${result.deleted_count} log(s), freed ${formatSize(result.freed_bytes)}`, "success");
      logs.value = logs.value.filter((l) => !selected.value.has(l.path));
      selected.value = new Set();
    }
    if (result.errors.length > 0) showToast(result.errors.join("; "), "error");
  } catch (e) { showToast(String(e), "error"); }
  finally { deleting.value = false; }
}

function toggleSelect(path: string) {
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path); else next.add(path);
  selected.value = next;
}

function toggleAll() {
  if (allSelected.value) selected.value = new Set();
  else selected.value = new Set(logs.value.map((l) => l.path));
}

const allSelected = computed(() => logs.value.length > 0 && selected.value.size === logs.value.length);
const totalSelected = computed(() => logs.value.filter((l) => selected.value.has(l.path)).reduce((sum, l) => sum + l.size, 0));

// ── Log file icon ─────────────────────────────────────────────────────
const logIcon = ref("");
invoke<string>("render_sf_symbol", { name: "log", size: 64, mode: "uttype", style: "plain" })
  .then(b64 => { if (b64) logIcon.value = b64; })
  .catch(e => console.warn('[logs] log icon load failed:', e));

// ── Friendly time ago ─────────────────────────────────────────────────

// ── Grouping by source ────────────────────────────────────────────────
function getLogSource(path: string, _name: string): string {
  const p = path.toLowerCase();
  // /var/log/* → "System"
  if (p.startsWith("/var/log")) return "System";
  // ~/Library/Logs/<AppName>/... → AppName
  const libMatch = path.match(/\/Library\/Logs\/([^/]+)/);
  if (libMatch) return libMatch[1];
  // Fallback to parent directory name
  const parts = path.split("/");
  if (parts.length >= 2) return parts[parts.length - 2];
  return "Other";
}

interface LogGroup {
  source: string;
  entries: typeof logs.value;
  totalSize: number;
}

const groupedLogs = computed<LogGroup[]>(() => {
  const groups: Record<string, typeof logs.value> = {};
  for (const log of logs.value) {
    const source = getLogSource(log.path, log.name);
    if (!groups[source]) groups[source] = [];
    groups[source].push(log);
  }
  return Object.entries(groups)
    .map(([source, entries]) => ({
      source,
      entries: entries.sort((a, b) => b.size - a.size),
      totalSize: entries.reduce((s, e) => s + e.size, 0),
    }))
    .sort((a, b) => b.totalSize - a.totalSize);
});

const collapsedGroups = ref<Set<string>>(new Set());

function toggleGroup(source: string) {
  const next = new Set(collapsedGroups.value);
  if (next.has(source)) next.delete(source); else next.add(source);
  collapsedGroups.value = next;
}

function selectGroup(group: LogGroup) {
  const next = new Set(selected.value);
  for (const e of group.entries) next.add(e.path);
  selected.value = next;
}

function deselectGroup(group: LogGroup) {
  const next = new Set(selected.value);
  for (const e of group.entries) next.delete(e.path);
  selected.value = next;
}

function isGroupAllSelected(group: LogGroup): boolean {
  return group.entries.every(e => selected.value.has(e.path));
}

function shortPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~");
}
</script>

<template>
  <section class="logs-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Logs</h2>
          <p class="text-muted">System and application log files</p>
        </div>
        <button class="btn-primary scan-btn" :disabled="logsScanning" @click="scan">
          <span v-if="logsScanning" class="spinner-sm"></span>
          {{ logsScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <div v-if="logsError" class="error-message">{{ logsError }}</div>

    <div v-if="logsScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning log files...</span>
    </div>

    <EmptyState
      v-else-if="logsScanned && logs.length === 0"
      title="No log files found"
      description="System and application log files will appear here after scanning."
    />

    <template v-else-if="logs.length > 0">
      <div class="results-bar">
        <div class="results-bar-left">
          <Checkbox :model-value="allSelected" @change="toggleAll">Select all</Checkbox>
          <span class="results-count text-muted">{{ logs.length }} log(s) &mdash; {{ formatSize(totalLogSize) }} total</span>
        </div>
        <div class="results-bar-right">
          <span v-if="selected.size > 0" class="selected-info">{{ selected.size }} selected ({{ formatSize(totalSelected) }})</span>
          <button class="btn-danger" :disabled="selected.size === 0 || deleting" @click="cleanSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Cleaning..." : "Clean Selected" }}
          </button>
        </div>
      </div>

      <div class="log-groups">
        <div v-for="group in groupedLogs" :key="group.source" class="log-category">
          <div class="category-header" tabindex="0" role="button" :aria-expanded="!collapsedGroups.has(group.source)" @click="toggleGroup(group.source)" @keydown.enter="toggleGroup(group.source)" @keydown.space.prevent="toggleGroup(group.source)">
            <div class="category-header-left">
              <span class="category-chevron" :class="{ expanded: !collapsedGroups.has(group.source) }">
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
              </span>
              <span class="category-label">{{ group.source }}</span>
              <span class="badge pill badge-neutral">{{ group.entries.length }}</span>
            </div>
            <div class="category-header-right">
              <span class="category-size mono">{{ formatSize(group.totalSize) }}</span>
              <button class="btn-sm btn-secondary" @click.stop="isGroupAllSelected(group) ? deselectGroup(group) : selectGroup(group)">
                {{ isGroupAllSelected(group) ? 'Deselect' : 'Select all' }}
              </button>
            </div>
          </div>

          <div v-if="!collapsedGroups.has(group.source)" class="log-list">
            <div
              v-for="log in group.entries"
              :key="log.path"
              class="log-item"
              :class="{ 'log-item--selected': selected.has(log.path) }"
              @click="toggleSelect(log.path)"
            >
              <div class="log-icon">
                <img v-if="logIcon" :src="logIcon" alt="" width="28" height="28" />
              </div>
              <div class="log-info">
                <span class="log-name">{{ log.name }}</span>
                <span class="log-path text-muted" :title="log.path">{{ shortPath(log.path) }}</span>
              </div>
              <div class="log-size-col">
                <span class="log-size mono">{{ formatSize(log.size) }}</span>
                <span v-if="log.modified" class="log-time text-muted">{{ timeAgo(log.modified) }}</span>
              </div>
              <Checkbox :model-value="selected.has(log.path)" @change="toggleSelect(log.path)" />
            </div>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.logs-view { max-width: 1440px; }

.results-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--sp-3);
}

.results-bar-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.results-bar-right {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}


.results-count { font-size: 12px; }

.log-groups {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.log-category {
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

.category-header:hover { background: rgba(255, 255, 255, 0.3); }

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

.category-chevron.expanded { transform: rotate(90deg); }

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

.log-list {
  display: flex;
  flex-direction: column;
}

.log-item {
  display: grid;
  grid-template-columns: 36px 1fr 90px 28px;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  cursor: pointer;
  transition: background 0.15s ease;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
}

.log-item:last-child { border-bottom: none; }
.log-item:hover { background: rgba(255, 255, 255, 0.3); }
.log-item--selected { background: var(--accent-light); }

.log-icon {
  display: flex;
  align-items: center;
  justify-content: center;
}

.log-icon img { border-radius: 4px; }

.log-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.log-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-path {
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-size-col {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 1px;
}

.log-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
}

.log-time { font-size: 10px; }

</style>
