<script setup lang="ts">
/**
 * SystemVitals — System health dashboard.
 *
 * Live-updating cards showing:
 *   1. Thermal state + category bars
 *   2. CPU load + core heat strip
 *   3. Fan speeds
 *   4. Battery health
 *   5. Memory pressure + breakdown
 *   6. Storage capacity
 *   + Uptime & background agents info bar
 *
 * Refreshes every 3 seconds for a live feel.
 */
import { ref, computed, onMounted, onUnmounted, onActivated, onDeactivated } from "vue";
import {
  vitalsResult,
  vitalsScanning,
  vitalsScanned,
  vitalsError,
  scanVitals,
  thermalResult,
  scanThermal,
  memoryResult,
  scanMemory,
  diskUsage,
  loadDiskUsage,
} from "../stores/scanStore";
import { formatSize, tempToColor, cpuLoadColor as cpuLoadColorFn, storageColor as storageColorFn } from "../utils";
import ThermalCard from "../components/ThermalCard.vue";
import FanCard from "../components/FanCard.vue";
import BatteryCard from "../components/BatteryCard.vue";
import CpuCard from "../components/CpuCard.vue";
import MemoryCard from "../components/MemoryCard.vue";

// ---------------------------------------------------------------------------
// Live refresh — 3s interval
// ---------------------------------------------------------------------------
const REFRESH_INTERVAL = 3000;
let refreshTimer: ReturnType<typeof setInterval> | null = null;
const paused = ref(false);
const lastUpdated = ref<Date | null>(null);

async function liveRefresh() {
  if (paused.value) return;
  await Promise.all([scanVitals(true), scanThermal(true), scanMemory(true), loadDiskUsage()]);
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
  const promises: Promise<void>[] = [];
  if (!vitalsScanned.value) promises.push(scanVitals(false));
  if (!thermalResult.value) promises.push(scanThermal(false));
  if (!memoryResult.value) promises.push(scanMemory(false));
  if (!diskUsage.value) promises.push(loadDiskUsage());
  if (promises.length) await Promise.all(promises);
  lastUpdated.value = new Date();
  startTimer();
});

onActivated(() => startTimer());
onDeactivated(() => stopTimer());
onUnmounted(() => stopTimer());

// ---------------------------------------------------------------------------
// Computed data
// ---------------------------------------------------------------------------

// Thermal state color
const thermalColor = computed(() => {
  if (!vitalsResult.value) return "var(--muted)";
  switch (vitalsResult.value.thermal_state) {
    case "Nominal": return "var(--success)";
    case "Fair": return "var(--warning)";
    case "Serious": return "var(--thermal-serious)";
    case "Critical": return "var(--danger)";
    default: return "var(--muted)";
  }
});

const thermalLabel = computed(() => {
  if (!vitalsResult.value) return "—";
  return vitalsResult.value.thermal_state;
});

// CPU load bar width (0-100%)
const cpuLoadWidth = computed(() => {
  if (!vitalsResult.value) return 0;
  return Math.min(100, vitalsResult.value.load.cpu_usage_percent);
});

// CPU load bar color (canonical thresholds from utils.ts)
const cpuLoadColor = computed(() => cpuLoadColorFn(cpuLoadWidth.value));

// ---------------------------------------------------------------------------
// Thermal card: hottest sensor + 4 category bars
// ---------------------------------------------------------------------------

// Thermal bars now in ThermalCard.vue component

const hottestTemp = computed(() => {
  if (!thermalResult.value?.hottest_sensor) return null;
  return thermalResult.value.hottest_sensor.temp_celsius;
});

const hottestName = computed(() => {
  if (!thermalResult.value?.hottest_sensor) return "";
  return thermalResult.value.hottest_sensor.name;
});

// ---------------------------------------------------------------------------
// CPU card: per-core temperature heat strip
// ---------------------------------------------------------------------------

// Extract individual CPU core temps — one pip per physical core.
// Apple Silicon exposes many sub-sensors per core (e.g. Tp01, Tp0D, Tp0j
// may all be the same P-core). We deduplicate by taking only keys that
// match the "primary die temp" patterns: Tp0{digit} for P-cores, Te0{digit}
// for E-cores. This gives ~10-14 cores on M1 Pro, ~12-24 on M4 Pro.
// Core temp strip now in CpuCard.vue

// ---------------------------------------------------------------------------
// Fan card
// ---------------------------------------------------------------------------
const fans = computed(() => {
  if (!thermalResult.value) return [];
  return thermalResult.value.fans;
});

const avgFanRpm = computed(() => {
  if (!fans.value.length) return 0;
  return Math.round(
    fans.value.reduce((sum, f) => sum + f.current_rpm, 0) / fans.value.length
  );
});

// ---------------------------------------------------------------------------
// Battery card
// ---------------------------------------------------------------------------

const battery = computed(() => vitalsResult.value?.battery ?? null);

// Battery computeds now in BatteryCard.vue

// ---------------------------------------------------------------------------
// Memory card
// ---------------------------------------------------------------------------

// Memory computeds now in MemoryCard.vue

// ---------------------------------------------------------------------------
// Storage card
// ---------------------------------------------------------------------------

const storagePct = computed(() => {
  if (!diskUsage.value) return 0;
  return diskUsage.value.percentage;
});

const storageColor = computed(() => {
  return storageColorFn(storagePct.value);
});

// ---------------------------------------------------------------------------
// Uptime & agents
// ---------------------------------------------------------------------------

const uptime = computed(() => vitalsResult.value?.load.uptime_display ?? "—");
const agentCount = computed(() => vitalsResult.value?.background_agent_count ?? 0);

const timeAgo = computed(() => {
  if (!lastUpdated.value) return "";
  return "Live";
});
</script>

<template>
  <div class="vitals-view">
    <!-- Header -->
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>System Vitals</h2>
          <p class="view-subtitle" v-if="vitalsResult">{{ vitalsResult.headline }}</p>
          <p class="view-subtitle" v-else>Real-time system health and CPU diagnosis</p>
        </div>
        <div class="header-actions">
          <button class="btn-ghost btn-sm" @click="togglePause">
            {{ paused ? "Resume" : "Pause" }}
          </button>
          <span class="live-badge" :class="{ paused }">
            <span class="live-dot"></span>
            {{ paused ? "Paused" : timeAgo }}
          </span>
        </div>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="vitalsScanning && !vitalsScanned" class="loading-state">
      <span class="spinner"></span>
      Scanning system vitals...
    </div>

    <!-- Error state -->
    <div v-if="vitalsError" class="error-message">{{ vitalsError }}</div>

    <!-- Main content -->
    <template v-if="vitalsResult">

      <!-- ================================================================
           SECTION 1: Stats overview — glass cards
           ================================================================ -->
      <div class="stats-row">

        <!-- THERMAL: hottest temp + 4 vertical category bars -->
        <div class="stat-card stat-card--thermal">
          <div class="stat-label">Thermal</div>
          <div class="thermal-content">
            <div class="thermal-left">
              <div class="stat-hero" v-if="hottestTemp !== null" :style="{ color: tempToColor(hottestTemp) }">
                {{ hottestTemp }}<span class="stat-unit">&deg;</span>
              </div>
              <div class="stat-hero" v-else :style="{ color: thermalColor }">
                {{ thermalLabel }}
              </div>
              <div class="thermal-source" v-if="hottestName">{{ hottestName }}</div>
              <div class="thermal-state">
                <span class="status-dot" :style="{ background: thermalColor }"></span>
                {{ thermalLabel }}
              </div>
            </div>
            <ThermalCard v-if="thermalResult" :summaries="thermalResult.summaries" />
          </div>
        </div>

        <!-- CPU: percentage + core heat strip -->
        <div class="stat-card stat-card--cpu">
          <div class="stat-label">CPU</div>
          <div class="stat-hero">
            {{ vitalsResult.load.cpu_usage_percent.toFixed(0) }}<span class="stat-unit">%</span>
          </div>
          <div class="load-bar">
            <div
              class="load-bar-fill"
              :style="{ width: cpuLoadWidth + '%', background: cpuLoadColor }"
            ></div>
          </div>
          <CpuCard v-if="thermalResult" :sensors="thermalResult.sensors" />
          <div class="load-averages">
            <span>1m: {{ vitalsResult.load.load_1m.toFixed(1) }}</span>
            <span>5m: {{ vitalsResult.load.load_5m.toFixed(1) }}</span>
            <span>15m: {{ vitalsResult.load.load_15m.toFixed(1) }}</span>
          </div>
        </div>

        <!-- FANS: RPM + gauge bars -->
        <div class="stat-card stat-card--fans">
          <div class="stat-label">Fans</div>
          <FanCard :fans="fans" :avg-rpm="avgFanRpm" />
        </div>

        <!-- BATTERY: charge + health -->
        <div class="stat-card stat-card--battery" v-if="battery">
          <div class="stat-label">Battery</div>
          <BatteryCard :battery="battery" />
        </div>

        <!-- MEMORY: pressure + ring gauge -->
        <div class="stat-card stat-card--memory" v-if="memoryResult">
          <div class="stat-label">Memory</div>
          <MemoryCard :stats="memoryResult.stats" />
        </div>

        <!-- STORAGE: capacity bar -->
        <div class="stat-card stat-card--storage" v-if="diskUsage">
          <div class="stat-label">Storage</div>
          <div class="stat-hero" :style="{ color: storageColor }">
            {{ storagePct.toFixed(0) }}<span class="stat-unit">%</span>
          </div>
          <div class="storage-usage-text">
            {{ formatSize(diskUsage.used) }} used
            <span class="stat-dim">of {{ formatSize(diskUsage.total) }}</span>
          </div>
          <div class="storage-bar">
            <div
              class="storage-bar-fill"
              :style="{ width: storagePct + '%', background: storageColor }"
            ></div>
          </div>
          <div class="storage-free">
            {{ formatSize(diskUsage.free) }} free
          </div>
        </div>
      </div>

      <!-- ================================================================
           Info strip: uptime + background agents
           ================================================================ -->
      <div class="info-strip">
        <span class="info-strip-item">
          <span class="info-strip-label">Uptime</span>
          <span class="info-strip-value">{{ uptime }}</span>
        </span>
        <span class="info-strip-divider"></span>
        <span class="info-strip-item">
          <span class="info-strip-label">Background Agents</span>
          <span class="info-strip-value">{{ agentCount }}</span>
        </span>
        <span class="info-strip-divider"></span>
        <span class="info-strip-item">
          <span class="info-strip-label">Processes</span>
          <span class="info-strip-value">{{ vitalsResult.total_processes }}</span>
        </span>
      </div>

    </template>
  </div>
</template>

<style scoped>
.vitals-view {
  max-width: 1440px;
}

/* ---------------------------------------------------------------------------
   Header
   --------------------------------------------------------------------------- */
.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.live-badge {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 500;
  color: var(--success-text);
  padding: 3px 10px;
  background: var(--success-tint);
  border-radius: var(--radius-pill);
}

.live-badge.paused {
  color: var(--muted);
  background: rgba(0, 0, 0, 0.04);
}

.live-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--success);
  animation: pulse 2s infinite;
}

.live-badge.paused .live-dot {
  background: var(--muted);
  animation: none;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* ---------------------------------------------------------------------------
   Stats row — glass cards
   --------------------------------------------------------------------------- */
.stats-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
  margin-bottom: var(--sp-6);
}

.stat-card {
  padding: var(--sp-3) var(--sp-4);
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(20px) saturate(1.2);
  -webkit-backdrop-filter: blur(20px) saturate(1.2);
  border: 0.5px solid rgba(255, 255, 255, 0.55);
  box-shadow:
    0 0.5px 0 0 rgba(255, 255, 255, 0.7) inset,
    0 1px 3px rgba(0, 0, 0, 0.04),
    0 4px 12px rgba(0, 0, 0, 0.03);
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.stat-label {
  font-size: 10px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.55);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 6px;
}

.stat-hero {
  font-size: 26px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: -0.5px;
  line-height: 1.1;
  font-variant-numeric: tabular-nums;
}

.stat-unit {
  font-size: 14px;
  font-weight: 500;
  color: var(--muted);
  margin-left: 1px;
}

.stat-detail {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 6px;
  line-height: 1.3;
}

/* ---------------------------------------------------------------------------
   Thermal card — hottest temp + 4 vertical category bars
   --------------------------------------------------------------------------- */
.thermal-content {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 12px;
  flex: 1;
}

.thermal-left {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.thermal-source {
  font-size: 10px;
  font-weight: 500;
  color: var(--muted);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.thermal-state {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-top: 6px;
}

/* Thermal bars now in ThermalCard.vue */

/* ---------------------------------------------------------------------------
   CPU card — load bar + core heat strip
   --------------------------------------------------------------------------- */
.load-bar {
  height: 4px;
  background: rgba(0, 0, 0, 0.05);
  border-radius: 2px;
  margin-top: 10px;
  overflow: hidden;
}

.load-bar-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), background 0.3s;
}

/* Core heat strip styles now in CpuCard.vue */

.load-averages {
  display: flex;
  gap: 10px;
  margin-top: 6px;
  font-size: 10px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

/* ---------------------------------------------------------------------------
   Fans card
   --------------------------------------------------------------------------- */
/* Fan styles now in FanCard.vue */

/* ---------------------------------------------------------------------------
   Battery card
   --------------------------------------------------------------------------- */

/* Battery styles now in BatteryCard.vue */

/* ---------------------------------------------------------------------------
   Memory card
   --------------------------------------------------------------------------- */

/* Memory styles now in MemoryCard.vue */

/* ---------------------------------------------------------------------------
   Storage card
   --------------------------------------------------------------------------- */

.storage-usage-text {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-top: 2px;
}

.storage-bar {
  height: 4px;
  border-radius: 2px;
  background: rgba(0, 0, 0, 0.06);
  overflow: hidden;
  margin-top: 10px;
}

.storage-bar-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.storage-free {
  font-size: 11px;
  font-weight: 500;
  color: var(--muted);
  margin-top: 6px;
}

/* ---------------------------------------------------------------------------
   Info strip
   --------------------------------------------------------------------------- */

.info-strip {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 10px 16px;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.3);
  border: 0.5px solid rgba(255, 255, 255, 0.4);
  margin-bottom: var(--sp-6);
}

.info-strip-item {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.info-strip-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.info-strip-value {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}

.info-strip-divider {
  width: 1px;
  height: 14px;
  background: rgba(0, 0, 0, 0.08);
}

/* Section headers (kept for potential future sections) */
.section-title {
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

/* ---------------------------------------------------------------------------
   CPU Hog cards
   --------------------------------------------------------------------------- */
</style>
