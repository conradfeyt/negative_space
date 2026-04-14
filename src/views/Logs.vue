<script setup lang="ts">
import { ref, computed, watch } from "vue";
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
import LoadingState from "../components/LoadingState.vue";
import Checkbox from "../components/Checkbox.vue";
import StickyBar from "../components/StickyBar.vue";
import CollapsibleSection from "../components/CollapsibleSection.vue";
import ViewHeader from "../components/ViewHeader.vue";

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
const partialSelected = computed(() => selected.value.size > 0 && selected.value.size < logs.value.length);
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

watch(logsError, (err) => {
  if (err) showToast(err, "error");
});
</script>

<template>
  <section class="logs-view">
    <ViewHeader
      title="Logs"
      subtitle="System and application log files"
    >
      <template #actions>
        <button class="btn-primary scan-btn" :disabled="logsScanning" @click="scan">
          <span v-if="logsScanning" class="spinner-sm"></span>
          {{ logsScanning ? "Scanning..." : "Scan" }}
        </button>
      </template>
    </ViewHeader>

    <LoadingState v-if="logsScanning" message="Scanning log files..." />

    <EmptyState
      v-else-if="logsScanned && logs.length === 0"
      title="No log files found"
      description="System and application log files will appear here after scanning."
    />

    <template v-else-if="logs.length > 0">
      <StickyBar>
        <Checkbox :model-value="allSelected" :indeterminate="partialSelected" @change="toggleAll" />
        <span v-if="selected.size === 0" class="results-count">{{ logs.length }} log(s) &mdash; {{ formatSize(totalLogSize) }}</span>
        <span v-else-if="allSelected" class="results-count">{{ selected.size }} selected &mdash; {{ formatSize(totalSelected) }}</span>
        <span v-else class="results-count">{{ selected.size }} of {{ logs.length }} selected &mdash; {{ formatSize(totalSelected) }}</span>
        <template #actions>
          <button class="btn-danger" :disabled="selected.size === 0 || deleting" @click="cleanSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Cleaning..." : "Clean Selected" }}
          </button>
        </template>
      </StickyBar>

      <div class="log-groups">
        <div v-for="group in groupedLogs" :key="group.source" class="log-category">
          <CollapsibleSection
            :expanded="!collapsedGroups.has(group.source)"
            @toggle="toggleGroup(group.source)"
          >
            <template #header>
              <div class="category-header-left">
                <span class="category-label">{{ group.source }}</span>
                <span class="badge pill badge-neutral">{{ group.entries.length }}</span>
              </div>
              <div class="category-header-right">
                <span class="category-size mono">{{ formatSize(group.totalSize) }}</span>
                <button class="btn-sm btn-secondary" @click.stop="isGroupAllSelected(group) ? deselectGroup(group) : selectGroup(group)">
                  {{ isGroupAllSelected(group) ? 'Deselect' : 'Select all' }}
                </button>
              </div>
            </template>
            <div class="log-list">
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
          </CollapsibleSection>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.logs-view { max-width: 1440px; }

.results-count {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
}

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

.log-category :deep(.collapsible-header) {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.15s ease;
}

.log-category :deep(.collapsible-header:hover) {
  background: rgba(255, 255, 255, 0.3);
}

.category-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
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
