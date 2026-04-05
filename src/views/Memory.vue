<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onActivated, onDeactivated } from "vue";
import type { ProcessGroup } from "../types";
import {
  memoryResult,
  memoryScanning,
  memoryScanned,
  memoryError,
  scanMemory,
} from "../stores/scanStore";
import { formatSize, MEMORY_CATEGORY_COLORS, MEMORY_BAR_COLORS } from "../utils";

// ---------------------------------------------------------------------------
// Live refresh
// ---------------------------------------------------------------------------

const REFRESH_INTERVAL = 3000; // 3 seconds — fast enough to feel live, light enough to not tax CPU
let refreshTimer: ReturnType<typeof setInterval> | null = null;

const paused = ref(false);
const lastUpdated = ref<Date | null>(null);

/** Kick off a live (non-destructive) refresh. */
async function liveRefresh() {
  if (paused.value) return;
  await scanMemory(true);
  lastUpdated.value = new Date();
}

function startTimer() {
  if (refreshTimer) return;
  refreshTimer = setInterval(liveRefresh, REFRESH_INTERVAL);
}

function stopTimer() {
  if (refreshTimer) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }
}

function togglePause() {
  paused.value = !paused.value;
  if (!paused.value) {
    // Immediately refresh when unpausing
    liveRefresh();
  }
}

// Start on mount, stop on unmount
onMounted(async () => {
  // Initial scan (shows spinner if first time)
  if (!memoryScanned.value) {
    await scanMemory(false);
  }
  lastUpdated.value = new Date();
  startTimer();
});

onActivated(() => startTimer());
onDeactivated(() => stopTimer());
onUnmounted(() => stopTimer());

// ---------------------------------------------------------------------------
// UI state
// ---------------------------------------------------------------------------

// Track which process groups are expanded to show individual processes
const expandedGroups = ref<Set<string>>(new Set());

// Category colors for the memory bar and group badges (from shared tokens)
const categoryColors = MEMORY_CATEGORY_COLORS;
const FALLBACK_MUTED_COLOR = "hsla(215, 15%, 62%, 0.85)"; // matches --muted

function toggleGroup(groupKey: string) {
  const next = new Set(expandedGroups.value);
  if (next.has(groupKey)) {
    next.delete(groupKey);
  } else {
    next.add(groupKey);
  }
  expandedGroups.value = next;
}

function getCategoryColor(category: string): string {
  return categoryColors[category] || FALLBACK_MUTED_COLOR;
}

function formatPercent(pct: number): string {
  if (pct < 0.1) return "<0.1%";
  return pct.toFixed(1) + "%";
}

function timeAgo(d: Date | null): string {
  if (!d) return "";
  const sec = Math.floor((Date.now() - d.getTime()) / 1000);
  if (sec < 2) return "just now";
  if (sec < 60) return `${sec}s ago`;
  return `${Math.floor(sec / 60)}m ago`;
}

// Re-compute "time ago" string every second
const timeAgoStr = ref("");
let clockTimer: ReturnType<typeof setInterval> | null = null;
function startClock() {
  if (clockTimer) return;
  clockTimer = setInterval(() => {
    timeAgoStr.value = timeAgo(lastUpdated.value);
  }, 1000);
}
function stopClock() {
  if (clockTimer) { clearInterval(clockTimer); clockTimer = null; }
}
onMounted(() => startClock());
onActivated(() => startClock());
onDeactivated(() => stopClock());
onUnmounted(() => stopClock());

// Memory bar segments
const memoryBarSegments = computed(() => {
  if (!memoryResult.value) return [];
  const stats = memoryResult.value.stats;
  const total = stats.total_bytes;
  if (total === 0) return [];

  return [
    { label: "App Memory", bytes: stats.app_bytes, color: MEMORY_BAR_COLORS.app, pct: (stats.app_bytes / total) * 100 },
    { label: "Wired", bytes: stats.wired_bytes, color: MEMORY_BAR_COLORS.wired, pct: (stats.wired_bytes / total) * 100 },
    { label: "Compressed", bytes: stats.compressed_bytes, color: MEMORY_BAR_COLORS.compressed, pct: (stats.compressed_bytes / total) * 100 },
    { label: "Inactive", bytes: stats.inactive_bytes, color: MEMORY_BAR_COLORS.inactive, pct: (stats.inactive_bytes / total) * 100 },
    { label: "Free", bytes: stats.free_bytes, color: MEMORY_BAR_COLORS.free, pct: (stats.free_bytes / total) * 100 },
  ];
});

// Memory pressure level
const memoryPressure = computed(() => {
  if (!memoryResult.value) return { label: "Unknown", class: "" };
  const stats = memoryResult.value.stats;
  const usedPct = (stats.used_bytes / stats.total_bytes) * 100;
  if (usedPct > 85) return { label: "High", class: "pressure-high" };
  if (usedPct > 65) return { label: "Moderate", class: "pressure-moderate" };
  return { label: "Low", class: "pressure-low" };
});

// Top groups by memory
const sortedGroups = computed((): ProcessGroup[] => {
  if (!memoryResult.value) return [];
  return [...memoryResult.value.groups];
});
</script>

<template>
  <div class="memory-view">
    <div class="view-header">
      <div>
        <h2 class="view-title">Memory</h2>
        <p class="view-subtitle">
          Live process memory usage and system memory breakdown
        </p>
      </div>
      <div class="header-controls">
        <div class="live-indicator" v-if="memoryResult">
          <span :class="['live-dot', { paused: paused }]"></span>
          <span class="live-label">{{ paused ? "Paused" : "Live" }}</span>
          <span class="live-updated text-muted" v-if="timeAgoStr">{{ timeAgoStr }}</span>
        </div>
        <button
          :class="['btn-pause', { active: paused }]"
          @click="togglePause"
          v-if="memoryResult"
          :title="paused ? 'Resume live updates' : 'Pause live updates'"
        >
          <!-- Pause icon -->
          <svg v-if="!paused" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="4" width="4" height="16" rx="1"/>
            <rect x="14" y="4" width="4" height="16" rx="1"/>
          </svg>
          <!-- Play icon -->
          <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <polygon points="6,4 20,12 6,20"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Error state -->
    <div v-if="memoryError" class="error-message">{{ memoryError }}</div>

    <!-- Initial scanning state (no prior results) -->
    <div v-if="memoryScanning && !memoryResult" class="scanning-state card">
      <div class="spinner"></div>
      <p>Scanning running processes...</p>
    </div>

    <!-- Results -->
    <template v-if="memoryResult">
      <!-- System memory overview -->
      <div class="card stats-card">
        <div class="stats-header">
          <h3>System Memory</h3>
          <div :class="['pressure-badge', memoryPressure.class]">
            Memory Pressure: {{ memoryPressure.label }}
          </div>
        </div>

        <div class="stats-total">
          <span class="stats-total-label">Total RAM</span>
          <span class="stats-total-value">{{ formatSize(memoryResult.stats.total_bytes) }}</span>
        </div>

        <!-- Memory bar -->
        <div class="memory-bar">
          <div
            v-for="seg in memoryBarSegments"
            :key="seg.label"
            class="memory-bar-segment"
            :style="{ width: seg.pct + '%', backgroundColor: seg.color }"
            :title="`${seg.label}: ${formatSize(seg.bytes)} (${seg.pct.toFixed(1)}%)`"
          ></div>
        </div>

        <!-- Legend -->
        <div class="memory-legend">
          <div v-for="seg in memoryBarSegments" :key="seg.label" class="legend-item">
            <span class="legend-dot" :style="{ backgroundColor: seg.color }"></span>
            <span class="legend-label">{{ seg.label }}</span>
            <span class="legend-value">{{ formatSize(seg.bytes) }}</span>
          </div>
        </div>
      </div>

      <!-- Process count summary -->
      <div class="summary-row">
        <span class="summary-stat">
          {{ memoryResult.total_processes }} processes running
        </span>
        <span class="summary-stat">
          {{ memoryResult.groups.length }} groups
        </span>
      </div>

      <!-- Process groups -->
      <div class="groups-list">
        <div
          v-for="group in sortedGroups"
          :key="group.name"
          class="group-card card-flush"
        >
          <div class="group-header" tabindex="0" role="button" @click="toggleGroup(group.name)" @keydown.enter="toggleGroup(group.name)" @keydown.space.prevent="toggleGroup(group.name)">
            <div class="group-left">
              <span
                class="group-color-dot"
                :style="{ backgroundColor: getCategoryColor(group.category) }"
              ></span>
              <div class="group-info">
                <span class="group-name">{{ group.name }}</span>
                <span class="group-desc" v-if="group.description">{{ group.description }}</span>
              </div>
            </div>
            <div class="group-right">
              <span class="group-count">{{ group.process_count }} proc{{ group.process_count === 1 ? '' : 's' }}</span>
              <span class="group-mem">{{ formatSize(group.total_rss_bytes) }}</span>
              <span class="group-pct">{{ formatPercent(group.total_mem_percent) }}</span>
              <span class="expand-chevron" :class="{ expanded: expandedGroups.has(group.name) }">
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
              </span>
            </div>
          </div>

          <!-- Expanded: individual processes -->
          <div v-if="expandedGroups.has(group.name)" class="group-processes">
            <table>
              <thead>
                <tr>
                  <th>PID</th>
                  <th>Name</th>
                  <th>Description</th>
                  <th class="th-right">RSS</th>
                  <th class="th-right">Mem %</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="proc in group.processes" :key="proc.pid">
                  <td class="mono">{{ proc.pid }}</td>
                  <td class="proc-name truncate" :title="proc.command">{{ proc.name }}</td>
                  <td class="proc-desc">{{ proc.description || '-' }}</td>
                  <td class="mono th-right">{{ formatSize(proc.rss_bytes) }}</td>
                  <td class="mono th-right">{{ formatPercent(proc.mem_percent) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </template>

    <!-- Empty state -->
    <div v-if="!memoryScanning && !memoryResult && !memoryError" class="empty-state card">
      <p class="text-muted">Loading memory data...</p>
    </div>
  </div>
</template>

<style scoped>
.memory-view {
  max-width: 1440px;
}

/* Header layout — extends global .view-header with flex for controls */
.view-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}

/* Header controls — live indicator + pause button */
.header-controls {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.live-indicator {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: 12px;
}

.live-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--success);
  animation: pulse 2s ease-in-out infinite;
}

.live-dot.paused {
  background: var(--muted);
  animation: none;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.live-label {
  font-weight: 600;
  color: var(--text-secondary);
}

.live-updated {
  font-size: 11px;
}

.btn-pause {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--sp-2);
  background: var(--surface);
  color: var(--text-secondary);
  border: 1px solid var(--border);
  padding: 0;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}

.btn-pause:hover {
  background: var(--accent-light);
  border-color: var(--accent);
  color: var(--accent);
}

.btn-pause.active {
  background: var(--accent-light);
  border-color: var(--accent);
  color: var(--accent);
}

.scanning-state {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  color: var(--muted);
  font-size: 14px;
}

/* Stats card */
.stats-card {
  margin-bottom: var(--sp-6);
}

.stats-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--sp-4);
}

.stats-header h3 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
}

.pressure-badge {
  font-size: 12px;
  font-weight: 600;
  padding: var(--sp-1) var(--sp-3);
  border-radius: var(--radius-sm);
  transition: background 0.3s, color 0.3s;
}

.pressure-low {
  background: rgba(48, 209, 88, 0.18);
  color: var(--success);
}

.pressure-moderate {
  background: rgba(255, 159, 10, 0.18);
  color: var(--warning);
}

.pressure-high {
  background: rgba(255, 69, 58, 0.18);
  color: var(--danger);
}

.stats-total {
  display: flex;
  align-items: baseline;
  gap: var(--sp-2);
  margin-bottom: var(--sp-3);
}

.stats-total-label {
  font-size: 13px;
  color: var(--muted);
}

.stats-total-value {
  font-size: 20px;
  font-weight: 700;
  color: var(--text);
  font-family: var(--font-mono);
}

/* Memory bar */
.memory-bar {
  display: flex;
  height: 20px;
  border-radius: 10px;
  overflow: hidden;
  background: var(--border);
  margin-bottom: var(--sp-4);
}

.memory-bar-segment {
  height: 100%;
  min-width: 2px;
  transition: width 0.8s ease;
}

.memory-bar-segment:first-child {
  border-radius: 10px 0 0 10px;
}

.memory-bar-segment:last-child {
  border-radius: 0 10px 10px 0;
}

/* Legend */
.memory-legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-4);
}

.legend-item {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: 12px;
}

.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.legend-label {
  color: var(--text-secondary);
}

.legend-value {
  font-weight: 600;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 11px;
  transition: color 0.3s;
}

/* Summary row */
.summary-row {
  display: flex;
  gap: var(--sp-5);
  margin-bottom: var(--sp-4);
}

.summary-stat {
  font-size: 13px;
  color: var(--muted);
  font-weight: 500;
}

/* Group cards */
.groups-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  cursor: pointer;
  transition: background 0.15s;
}

.group-header:hover {
  background: var(--surface-alt);
}

.group-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex: 1;
  min-width: 0;
}

.group-color-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.group-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.group-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.group-desc {
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 1px;
}

.group-right {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  flex-shrink: 0;
}

.group-count {
  font-size: 12px;
  color: var(--muted);
  white-space: nowrap;
}

.group-mem {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  font-family: var(--font-mono);
  white-space: nowrap;
  min-width: 70px;
  text-align: right;
  transition: color 0.3s;
}

.group-pct {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  white-space: nowrap;
  min-width: 46px;
  text-align: right;
}

/* Process table within a group */
.group-processes {
  border-top: 1px solid var(--border-divider);
  overflow-x: auto;
}

.group-processes table {
  font-size: 12px;
}

.group-processes th {
  font-size: 11px;
  padding: var(--sp-2) var(--sp-3);
  background: transparent;
  border-bottom: 1px solid var(--border-divider);
}

.group-processes td {
  padding: 7px var(--sp-3);
  font-size: 12px;
}

.proc-name {
  max-width: 180px;
  font-weight: 500;
}

.proc-desc {
  color: var(--text-secondary);
  font-size: 11px;
  max-width: 280px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.th-right { text-align: right; }
</style>
