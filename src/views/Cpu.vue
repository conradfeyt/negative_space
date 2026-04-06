<script setup lang="ts">
/**
 * CPU — Process activity sorted by CPU usage.
 *
 * Shows:
 *   1. CPU hog cards — process groups using >1% CPU, with smart suggestions and quit actions
 *   2. Idle apps — high memory but near-zero CPU (quit candidates)
 *
 * Refreshes every 3 seconds for a live feel.
 */
import { ref, computed, onMounted, onUnmounted, onActivated, onDeactivated } from "vue";
import type { VitalsGroup } from "../types";
import {
  vitalsResult,
  vitalsScanning,
  vitalsScanned,
  vitalsError,
  scanVitals,
  quitProcess,
  quitProcessGroup,
} from "../stores/scanStore";
import { formatSize, cpuLoadClass } from "../utils";
import LiveIndicator from "../components/LiveIndicator.vue";

// ---------------------------------------------------------------------------
// Live refresh — 3s interval
// ---------------------------------------------------------------------------
const REFRESH_INTERVAL = 3000;
let refreshTimer: ReturnType<typeof setInterval> | null = null;
const paused = ref(false);
const lastUpdated = ref<Date | null>(null);

async function liveRefresh() {
  if (paused.value) return;
  await scanVitals(true);
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
  if (!paused.value) liveRefresh();
}

onMounted(async () => {
  if (!vitalsScanned.value) await scanVitals(false);
  lastUpdated.value = new Date();
  startTimer();
});

onActivated(() => startTimer());
onDeactivated(() => stopTimer());
onUnmounted(() => stopTimer());

// ---------------------------------------------------------------------------
// Computed data
// ---------------------------------------------------------------------------

// Top CPU hogs: groups with >1% CPU, sorted by CPU desc
const cpuHogs = computed(() => {
  if (!vitalsResult.value) return [];
  return vitalsResult.value.groups.filter(g => g.total_cpu_percent > 1.0);
});

// Idle memory hogs: likely_idle groups with >100MB
const idleApps = computed(() => {
  if (!vitalsResult.value) return [];
  return vitalsResult.value.groups
    .filter(g => g.likely_idle && g.total_rss_bytes > 100 * 1024 * 1024)
    .sort((a, b) => b.total_rss_bytes - a.total_rss_bytes);
});

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

const quitting = ref<Set<string>>(new Set());
const quitSuccess = ref<string | null>(null);

async function handleQuitGroup(group: VitalsGroup) {
  quitting.value.add(group.name);
  try {
    const pids = group.processes.map(p => p.pid);
    const msg = await quitProcessGroup(pids);
    quitSuccess.value = `${group.name}: ${msg}`;
    setTimeout(() => { quitSuccess.value = null; }, 3000);
    setTimeout(() => liveRefresh(), 500);
  } finally {
    quitting.value.delete(group.name);
  }
}

async function handleQuitProcess(pid: number, name: string) {
  try {
    const msg = await quitProcess(pid);
    quitSuccess.value = `${name}: ${msg}`;
    setTimeout(() => { quitSuccess.value = null; }, 3000);
    setTimeout(() => liveRefresh(), 500);
  } catch (e) {
    vitalsError.value = String(e);
  }
}

const expandedGroups = ref<Set<string>>(new Set());

function toggleGroup(name: string) {
  if (expandedGroups.value.has(name)) {
    expandedGroups.value.delete(name);
  } else {
    expandedGroups.value.add(name);
  }
  expandedGroups.value = new Set(expandedGroups.value);
}

function fmtCpu(pct: number): string {
  return pct >= 10 ? `${Math.round(pct)}%` : `${pct.toFixed(1)}%`;
}
</script>

<template>
  <div class="cpu-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>CPU</h2>
          <p class="text-muted">Process activity and resource usage</p>
        </div>
        <div class="header-actions">
          <button class="btn-ghost btn-sm" @click="togglePause">
            {{ paused ? "Resume" : "Pause" }}
          </button>
          <LiveIndicator :paused="paused" />
        </div>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="vitalsScanning && !vitalsScanned" class="loading-state">
      <span class="spinner"></span>
      Scanning processes...
    </div>

    <!-- Error state -->
    <div v-if="vitalsError" class="error-message">{{ vitalsError }}</div>

    <!-- Success toast -->
    <div v-if="quitSuccess" class="success-message">{{ quitSuccess }}</div>

    <template v-if="vitalsResult">

      <!-- Summary -->
      <div class="cpu-summary">
        <span class="summary-item">
          <span class="summary-value">{{ vitalsResult.load.cpu_usage_percent.toFixed(0) }}%</span>
          CPU
        </span>
        <span class="summary-item">
          <span class="summary-value">{{ vitalsResult.total_processes }}</span>
          processes
        </span>
        <span class="summary-item">
          <span class="summary-value">{{ cpuHogs.length }}</span>
          active
        </span>
        <span v-if="idleApps.length > 0" class="summary-item">
          <span class="summary-value">{{ idleApps.length }}</span>
          idle
        </span>
      </div>

      <!-- CPU Activity -->
      <div class="section" v-if="cpuHogs.length > 0">
        <div class="section-header">
          <h3 class="section-label">CPU Activity</h3>
          <span class="section-meta">{{ cpuHogs.length }} active</span>
        </div>

        <div class="hog-list">
          <div
            v-for="group in cpuHogs"
            :key="group.name"
            class="hog-card"
          >
            <div class="hog-header" tabindex="0" role="button" :aria-expanded="expandedGroups.has(group.name)" @click="toggleGroup(group.name)" @keydown.enter="toggleGroup(group.name)" @keydown.space.prevent="toggleGroup(group.name)">
              <div class="hog-left">
                <div class="hog-name-row">
                  <span class="hog-name">{{ group.name }}</span>
                  <span class="hog-category">{{ group.category }}</span>
                  <span class="badge-pill badge-neutral" v-if="group.process_count > 1">
                    {{ group.process_count }}
                  </span>
                </div>
                <div class="hog-metrics">
                  <span class="hog-cpu" :class="cpuLoadClass(group.total_cpu_percent)">{{ fmtCpu(group.total_cpu_percent) }}</span>
                  <span class="hog-mem">{{ formatSize(group.total_rss_bytes) }}</span>
                </div>
              </div>

              <div class="hog-actions">
                <button
                  v-if="group.can_quit"
                  class="btn-quit"
                  :disabled="quitting.has(group.name)"
                  @click.stop="handleQuitGroup(group)"
                >
                  {{ quitting.has(group.name) ? "..." : "Quit" }}
                </button>
                <span class="expand-chevron" :class="{ expanded: expandedGroups.has(group.name) }">
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M4.5 2.5l3.5 3.5-3.5 3.5"/>
                  </svg>
                </span>
              </div>
            </div>

            <div v-if="group.suggestion" class="hog-suggestion">
              {{ group.suggestion }}
            </div>

            <div v-if="expandedGroups.has(group.name)" class="hog-processes">
              <div
                v-for="proc in group.processes"
                :key="proc.pid"
                class="proc-row"
              >
                <span class="proc-name mono">{{ proc.name }}</span>
                <span class="proc-cpu mono">{{ fmtCpu(proc.cpu_percent) }}</span>
                <span class="proc-mem mono">{{ formatSize(proc.rss_bytes) }}</span>
                <span class="proc-pid mono">{{ proc.pid }}</span>
                <button
                  v-if="group.can_quit"
                  class="btn-quit-xs"
                  @click="handleQuitProcess(proc.pid, proc.name)"
                >Quit</button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Idle Apps -->
      <div class="section" v-if="idleApps.length > 0">
        <div class="section-header">
          <h3 class="section-label">Idle Apps</h3>
          <span class="section-meta">{{ idleApps.length }} apps using memory</span>
        </div>

        <div class="idle-list">
          <div
            v-for="group in idleApps"
            :key="group.name"
            class="idle-row"
          >
            <span class="idle-name">{{ group.name }}</span>
            <span class="idle-mem mono">{{ formatSize(group.total_rss_bytes) }}</span>
            <button
              v-if="group.can_quit"
              class="btn-quit-xs"
              :disabled="quitting.has(group.name)"
              @click="handleQuitGroup(group)"
            >
              {{ quitting.has(group.name) ? "..." : "Quit" }}
            </button>
          </div>
        </div>
      </div>

    </template>
  </div>
</template>

<style scoped>
.cpu-view {
  max-width: 1440px;
}

/* Header */
.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* Summary bar */
.cpu-summary {
  display: flex;
  gap: 24px;
  margin-bottom: var(--sp-6);
  padding: 12px 16px;
  border-radius: var(--radius-sm);
  background: var(--glass);
  border: 1px solid var(--glass-border);
}

.summary-item {
  font-size: 12px;
  color: var(--muted);
  font-weight: 500;
}

.summary-value {
  font-weight: 700;
  color: var(--text);
  margin-right: 3px;
}

/* Sections */
.section {
  margin-bottom: var(--sp-8);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: var(--sp-4);
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.55);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.section-meta {
  font-size: 11px;
  color: var(--muted);
}

/* Hog cards */
.hog-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.hog-card {
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--glass);
  border: 1px solid var(--glass-border);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
}

.hog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  cursor: pointer;
  transition: background 0.1s;
}

.hog-header:hover {
  background: rgba(0, 0, 0, 0.015);
}

.hog-left {
  flex: 1;
  min-width: 0;
}

.hog-name-row {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 3px;
}

.hog-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: -0.1px;
}

.hog-category {
  font-size: 10px;
  font-weight: 500;
  color: var(--muted);
}

/* hog-procs: uses global .badge-pill .badge-neutral */

.hog-metrics {
  display: flex;
  gap: 12px;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.hog-cpu {
  font-weight: 600;
  color: var(--text-secondary);
}

.hog-cpu.cpu-hot {
  color: var(--danger);
}

.hog-cpu.cpu-warm {
  color: var(--warning-text);
}

.hog-mem {
  color: var(--muted);
}

.hog-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.btn-quit {
  font-size: 11px;
  font-weight: 600;
  padding: 4px 12px;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  border: 1px solid rgba(0, 0, 0, 0.08);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}

.btn-quit:hover:not(:disabled) {
  background: var(--danger-tint);
  color: var(--danger-text);
  border-color: rgba(217, 75, 75, 0.15);
}

.btn-quit:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.expand-chevron {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  color: var(--muted);
  transition: transform 0.15s;
}

.expand-chevron.expanded {
  transform: rotate(90deg);
}

.hog-suggestion {
  padding: 0 16px 10px;
  font-size: 11px;
  line-height: 1.5;
  color: var(--muted);
}

.hog-processes {
  border-top: 1px solid rgba(0, 0, 0, 0.04);
  padding: 4px 0;
}

.proc-row {
  display: grid;
  grid-template-columns: 1fr 48px 70px 48px auto;
  align-items: center;
  gap: 6px;
  padding: 5px 16px;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.proc-row:hover {
  background: rgba(0, 0, 0, 0.015);
}

.proc-name {
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.proc-cpu {
  font-weight: 600;
  color: var(--text-secondary);
  text-align: right;
}

.proc-mem {
  color: var(--muted);
  text-align: right;
}

.proc-pid {
  font-size: 10px;
  color: rgba(60, 70, 90, 0.35);
  text-align: right;
}

.btn-quit-xs {
  font-size: 10px;
  font-weight: 500;
  padding: 1px 7px;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  border: 1px solid rgba(0, 0, 0, 0.06);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
  justify-self: end;
}

.btn-quit-xs:hover {
  background: var(--danger-tint);
  color: var(--danger-text);
  border-color: rgba(217, 75, 75, 0.15);
}

/* Idle apps */
.idle-list {
  display: flex;
  flex-direction: column;
  gap: 0;
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--glass);
  border: 1px solid var(--glass-border);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
}

.idle-row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.03);
}

.idle-row:last-child {
  border-bottom: none;
}

.idle-row:hover {
  background: rgba(0, 0, 0, 0.015);
}

.idle-name {
  font-size: 12px;
  font-weight: 550;
  color: var(--text);
}

.idle-mem {
  font-size: 11px;
  color: var(--muted);
}
</style>
