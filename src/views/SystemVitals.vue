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
import { formatSize } from "../utils";

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
    case "Serious": return "#E5700F";
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

// CPU load bar color
const cpuLoadColor = computed(() => {
  const pct = cpuLoadWidth.value;
  if (pct > 80) return "var(--danger)";
  if (pct > 50) return "var(--warning)";
  return "var(--accent)";
});

// ---------------------------------------------------------------------------
// Thermal card: hottest sensor + 4 category bars
// ---------------------------------------------------------------------------

// The 4 categories we show as vertical bars, in display order
const thermalBarCategories = [
  { id: "cpu", short: "CPU" },
  { id: "gpu", short: "GPU" },
  { id: "storage", short: "SSD" },
  { id: "battery", short: "BAT" },
] as const;

// Get the max temp for a category, normalized to a 0-110 scale for bar height
interface ThermalBar {
  category: string;
  label: string;
  maxTemp: number;
  /** Height as percentage (0-100) — scaled against 110°C ceiling */
  heightPct: number;
  color: string;
}

function tempToColor(t: number): string {
  if (t >= 95) return "hsla(0, 50%, 48%, 0.85)";
  if (t >= 80) return "hsla(25, 55%, 45%, 0.85)";
  if (t >= 65) return "hsla(40, 55%, 45%, 0.85)";
  if (t >= 45) return "hsla(160, 35%, 42%, 0.85)";
  return "hsla(195, 35%, 42%, 0.85)";
}

const thermalBars = computed<ThermalBar[]>(() => {
  if (!thermalResult.value) return [];
  const summaryMap = new Map(
    thermalResult.value.summaries.map(s => [s.category, s])
  );
  return thermalBarCategories
    .filter(c => summaryMap.has(c.id))
    .map(c => {
      const s = summaryMap.get(c.id)!;
      return {
        category: c.id,
        label: c.short,
        maxTemp: s.max_celsius,
        heightPct: Math.min(100, (s.max_celsius / 110) * 100),
        color: tempToColor(s.max_celsius),
      };
    });
});

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
const coreTempStrip = computed(() => {
  if (!thermalResult.value) return [];
  const primaryPattern = /^T[pe]\d[0-9]$/;
  let cores = thermalResult.value.sensors
    .filter(s => s.category === "cpu" && primaryPattern.test(s.key))
    .sort((a, b) => a.key.localeCompare(b.key))
    .map(s => ({
      key: s.key,
      temp: s.temp_celsius,
      color: tempToColor(s.temp_celsius),
    }));
  // Fallback: if pattern matched nothing, take first 24 cpu sensors
  if (cores.length === 0) {
    cores = thermalResult.value.sensors
      .filter(s => s.category === "cpu")
      .sort((a, b) => a.key.localeCompare(b.key))
      .slice(0, 24)
      .map(s => ({
        key: s.key,
        temp: s.temp_celsius,
        color: tempToColor(s.temp_celsius),
      }));
  }
  return cores;
});

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

const batteryHealthColor = computed(() => {
  if (!battery.value) return "var(--muted)";
  const h = battery.value.health_percent;
  if (h >= 80) return "var(--success)";
  if (h >= 50) return "var(--warning)";
  return "var(--danger)";
});

const batteryConditionClass = computed(() => {
  if (!battery.value) return "dot-muted";
  const c = battery.value.condition.toLowerCase();
  if (c === "normal") return "dot-success";
  if (c.includes("service")) return "dot-warning";
  return "dot-danger";
});

// ---------------------------------------------------------------------------
// Memory card
// ---------------------------------------------------------------------------

const memPressure = computed(() => {
  if (!memoryResult.value) return { label: "—", class: "dot-muted", color: "var(--muted)" };
  const s = memoryResult.value.stats;
  const pct = s.total_bytes > 0 ? (s.used_bytes / s.total_bytes) * 100 : 0;
  if (pct >= 90) return { label: "Critical", class: "dot-danger", color: "var(--danger)" };
  if (pct >= 75) return { label: "High", class: "dot-warning", color: "var(--warning)" };
  if (pct >= 50) return { label: "Moderate", class: "dot-success", color: "var(--success)" };
  return { label: "Low", class: "dot-success", color: "var(--success)" };
});

const memUsedPct = computed(() => {
  if (!memoryResult.value) return 0;
  const s = memoryResult.value.stats;
  return s.total_bytes > 0 ? (s.used_bytes / s.total_bytes) * 100 : 0;
});

const memSegments = computed(() => {
  if (!memoryResult.value) return [];
  const s = memoryResult.value.stats;
  const t = s.total_bytes || 1;
  return [
    { label: "App", pct: (s.app_bytes / t) * 100, color: "hsla(195, 45%, 42%, 0.65)" },
    { label: "Wired", pct: (s.wired_bytes / t) * 100, color: "hsla(35, 50%, 45%, 0.6)" },
    { label: "Compressed", pct: (s.compressed_bytes / t) * 100, color: "hsla(280, 30%, 50%, 0.5)" },
    { label: "Free", pct: (s.free_bytes / t) * 100, color: "hsla(140, 20%, 70%, 0.4)" },
  ].filter(s => s.pct > 0.5);
});

// ---------------------------------------------------------------------------
// Storage card
// ---------------------------------------------------------------------------

const storagePct = computed(() => {
  if (!diskUsage.value) return 0;
  return diskUsage.value.percentage;
});

const storageColor = computed(() => {
  const p = storagePct.value;
  if (p > 90) return "var(--danger)";
  if (p > 75) return "var(--warning)";
  return "var(--accent)";
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
                <span class="thermal-dot" :style="{ background: thermalColor }"></span>
                {{ thermalLabel }}
              </div>
            </div>
            <div class="thermal-bars" v-if="thermalBars.length">
              <div
                v-for="bar in thermalBars"
                :key="bar.category"
                class="tbar-col"
              >
                <div class="tbar-track">
                  <div
                    class="tbar-fill"
                    :style="{ height: bar.heightPct + '%', background: bar.color }"
                  ></div>
                </div>
                <span class="tbar-label">{{ bar.label }}</span>
              </div>
            </div>
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
          <div class="core-strip" v-if="coreTempStrip.length">
            <div
              v-for="core in coreTempStrip"
              :key="core.key"
              class="core-pip"
              :style="{ background: core.color }"
              :title="core.key + ': ' + core.temp + '°C'"
            ></div>
          </div>
          <div class="load-averages">
            <span>1m: {{ vitalsResult.load.load_1m.toFixed(1) }}</span>
            <span>5m: {{ vitalsResult.load.load_5m.toFixed(1) }}</span>
            <span>15m: {{ vitalsResult.load.load_15m.toFixed(1) }}</span>
          </div>
        </div>

        <!-- FANS: RPM + percent bars -->
        <div class="stat-card stat-card--fans">
          <div class="stat-label">Fans</div>
          <template v-if="fans.length">
            <div class="stat-hero">
              {{ avgFanRpm }}<span class="stat-unit">RPM</span>
            </div>
            <div class="fan-bars">
              <div v-for="fan in fans" :key="fan.id" class="fan-row">
                <span class="fan-name">{{ fan.name }}</span>
                <div class="fan-track">
                  <div
                    class="fan-fill"
                    :style="{ width: fan.percent + '%' }"
                  ></div>
                </div>
                <span class="fan-rpm mono">{{ Math.round(fan.current_rpm) }}</span>
              </div>
            </div>
          </template>
          <template v-else>
            <div class="stat-hero" style="color: var(--muted)">--</div>
            <div class="stat-detail">No fan data</div>
          </template>
        </div>

        <!-- BATTERY: charge + health -->
        <div class="stat-card stat-card--battery" v-if="battery">
          <div class="stat-label">Battery</div>
          <div class="stat-hero">
            {{ battery.charge_percent }}<span class="stat-unit">%</span>
          </div>
          <div class="battery-status">
            <span v-if="battery.is_charging" class="battery-charging">Charging</span>
            <span v-else-if="battery.ac_connected" class="battery-plugged">Plugged In</span>
            <span v-else class="battery-discharging">On Battery</span>
          </div>
          <div class="battery-details">
            <div class="battery-row">
              <span class="battery-detail-label">Health</span>
              <span class="battery-detail-value" :style="{ color: batteryHealthColor }">{{ battery.health_percent }}%</span>
            </div>
            <div class="battery-row">
              <span class="battery-detail-label">Cycles</span>
              <span class="battery-detail-value mono">{{ battery.cycle_count }}</span>
            </div>
            <div class="battery-row">
              <span class="battery-detail-label">Temp</span>
              <span class="battery-detail-value mono">{{ battery.temperature_celsius.toFixed(1) }}°C</span>
            </div>
          </div>
          <div class="battery-condition">
            <span :class="['thermal-dot', batteryConditionClass]"></span>
            {{ battery.condition }}
          </div>
        </div>

        <!-- MEMORY: pressure + segmented bar -->
        <div class="stat-card stat-card--memory" v-if="memoryResult">
          <div class="stat-label">Memory</div>
          <div class="stat-hero">
            {{ Math.round(memUsedPct) }}<span class="stat-unit">%</span>
          </div>
          <div class="mem-usage-text">
            {{ formatSize(memoryResult.stats.used_bytes) }}
            <span class="stat-dim">/ {{ formatSize(memoryResult.stats.total_bytes) }}</span>
          </div>
          <div class="mem-seg-bar" v-if="memSegments.length">
            <div
              v-for="(seg, i) in memSegments"
              :key="seg.label"
              class="mem-seg"
              :style="{
                flex: seg.pct,
                background: seg.color,
                borderRadius: i === 0 ? '2px 0 0 2px' : i === memSegments.length - 1 ? '0 2px 2px 0' : '0',
              }"
              :title="seg.label + ': ' + seg.pct.toFixed(1) + '%'"
            ></div>
          </div>
          <div class="mem-legend">
            <span v-for="seg in memSegments" :key="seg.label" class="mem-legend-item">
              <span class="mem-legend-dot" :style="{ background: seg.color }"></span>
              {{ seg.label }}
            </span>
          </div>
          <div class="mem-pressure-row">
            <span :class="['thermal-dot', memPressure.class]"></span>
            {{ memPressure.label }} pressure
          </div>
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
  padding: 14px 16px 12px;
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
  letter-spacing: 0.8px;
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

.thermal-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* 4 vertical bars */
.thermal-bars {
  display: flex;
  gap: 8px;
  align-items: flex-end;
  height: 64px;
  flex-shrink: 0;
}

.tbar-col {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  width: 22px;
}

.tbar-track {
  width: 6px;
  height: 48px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 3px;
  overflow: hidden;
  display: flex;
  align-items: flex-end;
}

.tbar-fill {
  width: 100%;
  border-radius: 3px;
  transition: height 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), background 0.3s;
}

.tbar-label {
  font-size: 8px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.45);
  letter-spacing: 0.2px;
  line-height: 1;
  white-space: nowrap;
}

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

/* Core heat strip — row of tiny colored pips, one per CPU core */
.core-strip {
  display: flex;
  gap: 2px;
  margin-top: 8px;
}

.core-pip {
  flex: 1;
  height: 4px;
  border-radius: 1px;
  min-width: 3px;
  max-width: 12px;
  transition: background 0.3s;
}

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
.fan-bars {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 8px;
}

.fan-row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 8px;
}

.fan-name {
  font-size: 10px;
  font-weight: 500;
  color: var(--text-secondary);
  white-space: nowrap;
}

.fan-track {
  height: 3px;
  background: rgba(0, 0, 0, 0.05);
  border-radius: 2px;
  overflow: hidden;
}

.fan-fill {
  height: 100%;
  border-radius: 2px;
  background: var(--accent);
  transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.fan-rpm {
  font-size: 10px;
  color: var(--muted);
  min-width: 32px;
  text-align: right;
}

/* ---------------------------------------------------------------------------
   Battery card
   --------------------------------------------------------------------------- */

.battery-status {
  font-size: 11px;
  font-weight: 500;
  margin-top: 4px;
}

.battery-charging {
  color: var(--success);
}

.battery-plugged {
  color: var(--accent);
}

.battery-discharging {
  color: var(--text-secondary);
}

.battery-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 10px;
}

.battery-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}

.battery-detail-label {
  font-size: 10px;
  font-weight: 500;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.battery-detail-value {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
}

.battery-condition {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 8px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
}

/* ---------------------------------------------------------------------------
   Memory card
   --------------------------------------------------------------------------- */

.mem-usage-text {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-top: 2px;
}

.stat-dim {
  color: var(--muted);
}

.mem-seg-bar {
  display: flex;
  height: 4px;
  border-radius: 2px;
  overflow: hidden;
  margin-top: 10px;
}

.mem-seg {
  min-width: 2px;
  transition: flex 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.mem-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}

.mem-legend-item {
  display: flex;
  align-items: center;
  gap: 3px;
  font-size: 9px;
  font-weight: 500;
  color: var(--muted);
}

.mem-legend-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}

.mem-pressure-row {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 8px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
}

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
  letter-spacing: 0.4px;
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
  letter-spacing: 0.8px;
}

.section-meta {
  font-size: 11px;
  color: var(--muted);
}

/* ---------------------------------------------------------------------------
   CPU Hog cards
   --------------------------------------------------------------------------- */
</style>
