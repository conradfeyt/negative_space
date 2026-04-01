<script setup lang="ts">
/**
 * Dashboard — unified system overview.
 *
 * Top: 6 glassmorphic health cards (Thermal, CPU, Fans, Battery, Memory, Storage)
 * Middle: Info strip (uptime, agents, processes)
 * Bottom: Quick Scan — scan-all cleanup summary
 *
 * Live-updates every 5s.
 */
import { computed, watch, onMounted, onUnmounted, onActivated, onDeactivated } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import { formatSize } from "../utils";
import {
  diskUsage,
  scanAllRunning,
  scanAllStep,
  scanAllDone,
  hasFullDiskAccess,
  domainStatus,
  totalReclaimable as storeTotalReclaimable,
  domainsScanned,
  totalDomains,
  loadDiskUsage,
  scanAll,
  checkFullDiskAccess,
  memoryResult,
  scanMemory,
  vitalsResult,
  scanVitals,
  thermalResult,
  scanThermal,
  intelligenceAvailable,
  scanSummary,
  generateScanSummary,
} from "../stores/scanStore";

async function openFdaSettings() {
  try { await invoke("open_full_disk_access_settings"); } catch (_) {}
}

const router = useRouter();
const navigateTo = (id: string) => router.push({ name: id });

// ---------------------------------------------------------------------------
// Live refresh
// ---------------------------------------------------------------------------
const POLL_INTERVAL = 5000;
let pollTimer: ReturnType<typeof setInterval> | null = null;

async function poll() {
  await Promise.all([scanMemory(true), scanVitals(true), scanThermal(true), loadDiskUsage()]);
}

function startPolling() {
  if (pollTimer) return;
  pollTimer = setInterval(poll, POLL_INTERVAL);
}

function stopPolling() {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
}

// ---------------------------------------------------------------------------
// Scan-all computeds
// ---------------------------------------------------------------------------
const domainMeta: Record<string, { label: string }> = {
  caches: { label: "Caches" }, logs: { label: "Log Files" },
  largeFiles: { label: "Large Files" }, apps: { label: "Apps" },
  browsers: { label: "Browsers" }, trash: { label: "Trash" },
  docker: { label: "Docker" }, security: { label: "Security" },
};

const totalFound = computed(() =>
  Object.values(domainStatus.value).reduce((sum, d) => sum + d.itemCount, 0)
);

const hasAnyResults = computed(() =>
  Object.values(domainStatus.value).some((d) => d.status === "done" || d.status === "error")
);

const domainEntries = computed(() =>
  Object.entries(domainStatus.value).map(([key, info]) => ({
    key, ...info, meta: domainMeta[key] ?? { label: key },
  }))
);

const scanProgress = computed(() => {
  if (!scanAllRunning.value && !scanAllDone.value) return 0;
  return Math.round((domainsScanned.value / totalDomains.value) * 100);
});

// ---------------------------------------------------------------------------
// Thermal card
// ---------------------------------------------------------------------------
const thermalBarCategories = [
  { id: "cpu", short: "CPU" }, { id: "gpu", short: "GPU" },
  { id: "storage", short: "SSD" }, { id: "battery", short: "BAT" },
] as const;

function tempToColor(t: number): string {
  if (t >= 95) return "hsla(0, 50%, 48%, 0.85)";
  if (t >= 80) return "hsla(25, 55%, 45%, 0.85)";
  if (t >= 65) return "hsla(40, 55%, 45%, 0.85)";
  if (t >= 45) return "hsla(160, 35%, 42%, 0.85)";
  return "hsla(195, 35%, 42%, 0.85)";
}

const thermalBars = computed(() => {
  if (!thermalResult.value) return [];
  const summaryMap = new Map(thermalResult.value.summaries.map(s => [s.category, s]));
  return thermalBarCategories
    .filter(c => summaryMap.has(c.id))
    .map(c => {
      const s = summaryMap.get(c.id)!;
      return {
        category: c.id, label: c.short, maxTemp: s.max_celsius,
        heightPct: Math.min(100, (s.max_celsius / 110) * 100),
        color: tempToColor(s.max_celsius),
      };
    });
});

const hottestTemp = computed(() => thermalResult.value?.hottest_sensor?.temp_celsius ?? null);
const hottestName = computed(() => thermalResult.value?.hottest_sensor?.name ?? "");

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

const thermalLabel = computed(() => vitalsResult.value?.thermal_state ?? "—");

// ---------------------------------------------------------------------------
// CPU card
// ---------------------------------------------------------------------------
const cpuLoadWidth = computed(() => {
  if (!vitalsResult.value) return 0;
  return Math.min(100, vitalsResult.value.load.cpu_usage_percent);
});

const cpuLoadColor = computed(() => {
  const pct = cpuLoadWidth.value;
  if (pct > 80) return "var(--danger)";
  if (pct > 50) return "var(--warning)";
  return "var(--accent)";
});

const coreTempStrip = computed(() => {
  if (!thermalResult.value) return [];
  const primaryPattern = /^T[pe]\d[0-9]$/;
  let cores = thermalResult.value.sensors
    .filter(s => s.category === "cpu" && primaryPattern.test(s.key))
    .sort((a, b) => a.key.localeCompare(b.key))
    .map(s => ({ key: s.key, temp: s.temp_celsius, color: tempToColor(s.temp_celsius) }));
  if (cores.length === 0) {
    cores = thermalResult.value.sensors
      .filter(s => s.category === "cpu")
      .sort((a, b) => a.key.localeCompare(b.key))
      .slice(0, 24)
      .map(s => ({ key: s.key, temp: s.temp_celsius, color: tempToColor(s.temp_celsius) }));
  }
  return cores;
});

// ---------------------------------------------------------------------------
// Fans card
// ---------------------------------------------------------------------------
const fans = computed(() => thermalResult.value?.fans ?? []);

function fanArc(cx: number, cy: number, r: number, startDeg: number, endDeg: number): string {
  const toRad = (d: number) => (d * Math.PI) / 180;
  const x1 = cx + r * Math.cos(toRad(180 - startDeg));
  const y1 = cy - r * Math.sin(toRad(180 - startDeg));
  const x2 = cx + r * Math.cos(toRad(180 - endDeg));
  const y2 = cy - r * Math.sin(toRad(180 - endDeg));
  const large = endDeg - startDeg > 180 ? 1 : 0;
  return `M${x1},${y1} A${r},${r} 0 ${large} 1 ${x2},${y2}`;
}

function fanNeedle(cx: number, cy: number, r: number, t: number): { x: number; y: number } {
  const angle = Math.PI * (1 - Math.max(0, Math.min(1, t)));
  return { x: cx + r * Math.cos(angle), y: cy - r * Math.sin(angle) };
}

function fanGaugeColor(pct: number): string {
  if (pct > 70) return "hsla(0, 45%, 45%, 0.7)";
  if (pct > 40) return "hsla(40, 50%, 42%, 0.7)";
  return "hsla(195, 40%, 40%, 0.6)";
}
const avgFanRpm = computed(() => {
  if (!fans.value.length) return 0;
  return Math.round(fans.value.reduce((sum, f) => sum + f.current_rpm, 0) / fans.value.length);
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
  if (!memoryResult.value) return { label: "—", class: "dot-muted" };
  const s = memoryResult.value.stats;
  const pct = s.total_bytes > 0 ? (s.used_bytes / s.total_bytes) * 100 : 0;
  if (pct >= 90) return { label: "Critical", class: "dot-danger" };
  if (pct >= 75) return { label: "High", class: "dot-warning" };
  return { label: "Low", class: "dot-success" };
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
  ].filter(seg => seg.pct > 0.5);
});

// ---------------------------------------------------------------------------
// Storage card
// ---------------------------------------------------------------------------
const storagePct = computed(() => diskUsage.value?.percentage ?? 0);
const storageColor = computed(() => {
  if (storagePct.value > 90) return "var(--danger)";
  if (storagePct.value > 75) return "var(--warning)";
  return "var(--accent)";
});

// ---------------------------------------------------------------------------
// Info strip
// ---------------------------------------------------------------------------
const uptime = computed(() => vitalsResult.value?.load.uptime_display ?? "—");
const agentCount = computed(() => vitalsResult.value?.background_agent_count ?? 0);
const headline = computed(() => vitalsResult.value?.headline ?? "");

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
onMounted(() => {
  loadDiskUsage();
  checkFullDiskAccess();
  Promise.all([
    memoryResult.value ? scanMemory(true) : scanMemory(false),
    vitalsResult.value ? scanVitals(true) : scanVitals(false),
    thermalResult.value ? scanThermal(true) : scanThermal(false),
  ]);
  startPolling();
});

// Generate AI summary after scan-all completes
watch(scanAllDone, (done) => {
  if (done && intelligenceAvailable.value) {
    void generateScanSummary();
  }
});

onActivated(() => startPolling());
onDeactivated(() => stopPolling());
onUnmounted(() => stopPolling());
</script>

<template>
  <div class="dashboard">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Dashboard</h2>
          <p class="text-muted" v-if="headline">{{ headline }}</p>
          <p class="text-muted" v-else>System overview</p>
        </div>
      </div>
    </div>

    <!-- ================================================================
         Health cards — 3x2 glassmorphic grid
         ================================================================ -->
    <div class="stats-row">

      <!-- THERMAL -->
      <div class="stat-card stat-card--thermal" @click="navigateTo('thermal')">
        <div class="stat-label">Thermal</div>
        <div class="stat-hero" v-if="hottestTemp !== null" :style="{ color: tempToColor(hottestTemp) }">
          {{ hottestTemp }}<span class="stat-unit">&deg;C</span>
        </div>
        <div class="stat-hero" v-else :style="{ color: thermalColor }">{{ thermalLabel }}</div>
        <div class="thermal-meta">
          <span class="thermal-dot" :style="{ background: thermalColor }"></span>
          <span>{{ thermalLabel }}</span>
          <span v-if="hottestName" class="thermal-source">{{ hottestName }}</span>
        </div>
        <div class="thermal-bars" v-if="thermalBars.length">
          <div v-for="bar in thermalBars" :key="bar.category" class="tbar-col">
            <div class="tbar-track">
              <div class="tbar-fill" :style="{ height: bar.heightPct + '%', background: bar.color }"></div>
            </div>
            <span class="tbar-label">{{ bar.label }}</span>
          </div>
        </div>
      </div>

      <!-- CPU -->
      <div class="stat-card stat-card--cpu" @click="navigateTo('cpu')">
        <div class="stat-label">CPU</div>
        <div class="cpu-top">
          <div class="stat-hero" v-if="vitalsResult">
            {{ vitalsResult.load.cpu_usage_percent.toFixed(0) }}<span class="stat-unit">%</span>
          </div>
          <div class="cpu-cores-label" v-if="vitalsResult">{{ vitalsResult.load.cpu_cores }} cores</div>
        </div>
        <div class="cpu-viz">
          <div class="load-bar" v-if="vitalsResult">
            <div class="load-bar-fill" :style="{ width: cpuLoadWidth + '%', background: cpuLoadColor }"></div>
          </div>
          <div class="core-strip" v-if="coreTempStrip.length">
            <div v-for="core in coreTempStrip" :key="core.key" class="core-pip"
              :style="{ background: core.color }" :title="core.key + ': ' + core.temp + '°C'"></div>
          </div>
        </div>
        <div class="load-averages" v-if="vitalsResult">
          <span>1m: {{ vitalsResult.load.load_1m.toFixed(1) }}</span>
          <span>5m: {{ vitalsResult.load.load_5m.toFixed(1) }}</span>
          <span>15m: {{ vitalsResult.load.load_15m.toFixed(1) }}</span>
        </div>
      </div>

      <!-- FANS -->
      <div class="stat-card">
        <div class="stat-label">Fans</div>
        <template v-if="fans.length">
          <div class="stat-hero">{{ avgFanRpm }}<span class="stat-unit">RPM</span></div>
          <div class="fan-items">
            <div v-for="fan in fans" :key="fan.id" class="fan-item">
              <svg class="fan-mini-gauge" viewBox="0 0 48 30">
                <!-- 4 background zones -->
                <path :d="fanArc(24, 27, 19, 0, 44)" stroke="hsla(140, 20%, 40%, 0.15)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path :d="fanArc(24, 27, 19, 46, 89)" stroke="hsla(45, 25%, 42%, 0.15)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path :d="fanArc(24, 27, 19, 91, 134)" stroke="hsla(25, 30%, 40%, 0.15)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path :d="fanArc(24, 27, 19, 136, 180)" stroke="hsla(0, 30%, 42%, 0.15)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <!-- Active zone -->
                <path v-if="fan.percent <= 25" :d="fanArc(24, 27, 19, 0, 44)" stroke="hsla(140, 35%, 35%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path v-else-if="fan.percent <= 50" :d="fanArc(24, 27, 19, 46, 89)" stroke="hsla(45, 45%, 38%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path v-else-if="fan.percent <= 75" :d="fanArc(24, 27, 19, 91, 134)" stroke="hsla(25, 50%, 38%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path v-else :d="fanArc(24, 27, 19, 136, 180)" stroke="hsla(0, 45%, 42%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <!-- Needle -->
                <line x1="24" y1="27"
                  :x2="fanNeedle(24, 27, 13, fan.percent / 100).x"
                  :y2="fanNeedle(24, 27, 13, fan.percent / 100).y"
                  :stroke="fanGaugeColor(fan.percent)" stroke-width="1" stroke-linecap="round"/>
                <circle cx="24" cy="27" r="1.5" :fill="fanGaugeColor(fan.percent)"/>
              </svg>
            </div>
          </div>
          <div class="fan-bars">
            <div v-for="fan in fans" :key="'b'+fan.id" class="fan-bar-row">
              <span class="fan-bar-name">{{ fan.name }}</span>
              <div class="fan-bar-track">
                <div class="fan-bar-fill" :style="{ width: fan.percent + '%' }"></div>
              </div>
              <span class="fan-bar-rpm mono">{{ Math.round(fan.current_rpm) }}</span>
            </div>
          </div>
        </template>
        <template v-else>
          <div class="stat-hero" style="color: var(--muted)">--</div>
          <div class="stat-detail">No fan data</div>
        </template>
      </div>

      <!-- BATTERY -->
      <div class="stat-card" v-if="battery">
        <div class="stat-label">Battery</div>
        <div class="stat-hero">{{ battery.charge_percent }}<span class="stat-unit">%</span></div>
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

      <!-- MEMORY -->
      <div class="stat-card" @click="navigateTo('memory')" v-if="memoryResult">
        <div>
          <div class="stat-label">Memory</div>
          <div class="stat-hero">{{ Math.round(memUsedPct) }}<span class="stat-unit">%</span></div>
          <div class="mem-usage-text">
            {{ formatSize(memoryResult.stats.used_bytes) }}
            <span class="stat-dim">/ {{ formatSize(memoryResult.stats.total_bytes) }}</span>
          </div>
        </div>
        <div class="card-bottom">
          <div class="mem-seg-bar" v-if="memSegments.length">
            <div v-for="(seg, i) in memSegments" :key="seg.label" class="mem-seg"
              :style="{ flex: seg.pct, background: seg.color,
                borderRadius: i === 0 ? '2px 0 0 2px' : i === memSegments.length - 1 ? '0 2px 2px 0' : '0',
              }" :title="seg.label + ': ' + seg.pct.toFixed(1) + '%'"></div>
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
      </div>

      <!-- STORAGE -->
      <div class="stat-card" @click="navigateTo('space-map')" v-if="diskUsage">
        <div>
          <div class="stat-label">Storage</div>
          <div class="stat-hero" :style="{ color: storageColor }">
            {{ storagePct.toFixed(0) }}<span class="stat-unit">%</span>
          </div>
          <div class="storage-usage-text">
            {{ formatSize(diskUsage.used) }} used
            <span class="stat-dim">of {{ formatSize(diskUsage.total) }}</span>
          </div>
        </div>
        <div class="card-bottom">
          <div class="storage-bar">
            <div class="storage-bar-fill" :style="{ width: storagePct + '%', background: storageColor }"></div>
          </div>
          <div class="storage-free">{{ formatSize(diskUsage.free) }} free</div>
        </div>
      </div>
    </div>

    <!-- ================================================================
         Info strip
         ================================================================ -->
    <div class="info-strip" v-if="vitalsResult">
      <span class="info-strip-item">
        <span class="info-strip-label">Uptime</span>
        <span class="info-strip-value">{{ uptime }}</span>
      </span>
      <span class="info-strip-divider"></span>
      <span class="info-strip-item">
        <span class="info-strip-label">Agents</span>
        <span class="info-strip-value">{{ agentCount }}</span>
      </span>
      <span class="info-strip-divider"></span>
      <span class="info-strip-item">
        <span class="info-strip-label">Processes</span>
        <span class="info-strip-value">{{ vitalsResult.total_processes }}</span>
      </span>
    </div>

    <!-- ================================================================
         AI Scan Summary (shown after scan-all completes)
         ================================================================ -->
    <div v-if="scanSummary && scanSummary.summary" class="ai-summary-card">
      <p class="ai-summary-text">{{ scanSummary.summary }}</p>
      <span v-if="scanSummary.ai_generated" class="ai-badge">AI</span>
    </div>

    <!-- ================================================================
         Quick Scan
         ================================================================ -->
    <div class="card scan-section">
      <div class="scan-header">
        <div>
          <h3>Quick Scan</h3>
          <p class="text-muted">Scan all cleanup domains at once</p>
        </div>
        <button class="btn-primary scan-btn" :disabled="scanAllRunning" @click="scanAll">
          <span v-if="scanAllRunning" class="spinner-sm"></span>
          {{ scanAllRunning ? "Scanning..." : scanAllDone ? "Rescan" : "Scan All" }}
        </button>
      </div>

      <div v-if="hasFullDiskAccess === false" class="fda-notice">
        <svg class="fda-notice-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <span class="fda-notice-text">Results may be incomplete -- Full Disk Access not granted.</span>
        <button class="fda-notice-btn" @click="openFdaSettings">Grant access</button>
      </div>

      <div v-if="scanAllRunning" class="scan-progress">
        <div class="scan-progress-header">
          <span class="scan-progress-label">{{ scanAllStep.replace('_', ' ') }}</span>
          <span class="scan-progress-count mono">{{ domainsScanned }} / {{ totalDomains }}</span>
        </div>
        <div class="scan-progress-track">
          <div class="scan-progress-fill" :style="{ width: scanProgress + '%' }"></div>
        </div>
      </div>

      <div v-if="hasAnyResults || scanAllRunning" class="domain-list">
        <div v-for="entry in domainEntries" :key="entry.key" :class="['domain-row', `domain-${entry.status}`]">
          <span class="domain-indicator">
            <span v-if="entry.status === 'scanning'" class="spinner-xs"></span>
            <svg v-else-if="entry.status === 'done'" class="domain-check" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <svg v-else-if="entry.status === 'error'" class="domain-error-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>
            </svg>
            <span v-else class="domain-idle-dot"></span>
          </span>
          <span class="domain-name">{{ entry.meta.label }}</span>
          <span class="domain-result" v-if="entry.status === 'done'">
            <span class="domain-size mono">{{ entry.key === 'security' ? entry.itemCount + ' findings' : formatSize(entry.totalSize) }}</span>
            <span class="domain-count">{{ entry.itemCount }} items</span>
          </span>
          <span class="domain-result" v-else-if="entry.status === 'error'">
            <span class="domain-error-text">Failed</span>
          </span>
        </div>
      </div>

      <div v-if="scanAllDone && !scanAllRunning" class="scan-summary">
        <span class="summary-value mono">{{ formatSize(storeTotalReclaimable) }}</span>
        <span class="summary-label">ready to review</span>
        <span class="summary-detail">{{ totalFound }} items across {{ domainsScanned }} domains</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard {
  max-width: 740px;
}

/* ======================================================================
   Stats row — glassmorphic cards (from Vitals)
   ====================================================================== */
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
  justify-content: space-between;
  min-width: 0;
  overflow: hidden;
  cursor: pointer;
  transition: box-shadow 0.15s;
}

.stat-card:hover {
  box-shadow:
    0 0.5px 0 0 rgba(255, 255, 255, 0.7) inset,
    0 2px 6px rgba(0, 0, 0, 0.06),
    0 8px 20px rgba(0, 0, 0, 0.05);
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
}

.card-bottom {
  margin-top: auto;
}

.stat-dim { color: var(--muted); }

/* Thermal */
.stat-card--thermal { justify-content: space-between; }
.thermal-meta { display: flex; align-items: center; gap: 5px; font-size: 10px; font-weight: 500; color: var(--text-secondary); margin-top: 2px; }
.thermal-source { color: var(--muted); }
.thermal-source::before { content: "\00b7"; margin-right: 5px; }
.thermal-dot { width: 5px; height: 5px; border-radius: 50%; flex-shrink: 0; }
.thermal-bars { display: flex; gap: 6px; align-items: flex-end; height: 56px; margin-top: auto; }
.tbar-col { display: flex; flex-direction: column; align-items: center; gap: 4px; flex: 1; }
.tbar-track { width: 8px; height: 40px; background: rgba(0, 0, 0, 0.04); border-radius: 4px; overflow: hidden; display: flex; align-items: flex-end; }
.tbar-fill { width: 100%; border-radius: 3px; transition: height 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), background 0.3s; }
.tbar-label { font-size: 8px; font-weight: 600; color: rgba(60, 65, 80, 0.45); letter-spacing: 0.2px; line-height: 1; white-space: nowrap; }

/* CPU */
.stat-card--cpu { justify-content: space-between; }
.cpu-top { }
.cpu-cores-label { font-size: 10px; font-weight: 500; color: var(--muted); margin-top: 2px; }
.cpu-viz { margin-top: auto; }
.load-bar { height: 4px; background: rgba(0, 0, 0, 0.05); border-radius: 2px; overflow: hidden; }
.load-bar-fill { height: 100%; border-radius: 2px; transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), background 0.3s; }
.core-strip { display: flex; gap: 2px; margin-top: 8px; }
.core-pip { flex: 1; height: 4px; border-radius: 1px; min-width: 3px; max-width: 12px; transition: background 0.3s; }
.load-averages { display: flex; gap: 10px; margin-top: 6px; font-size: 10px; color: var(--muted); font-variant-numeric: tabular-nums; }

/* Fans */
.fan-items { display: flex; gap: 12px; margin-top: 10px; justify-content: center; padding-bottom: 8px; border-bottom: 1px solid rgba(0, 0, 0, 0.04); }
.fan-item { display: flex; flex-direction: column; align-items: center; gap: 2px; }
.fan-mini-gauge { width: 52px; height: 32px; }
.fan-bars { display: flex; flex-direction: column; gap: 5px; margin-top: 6px; }
.fan-bar-row { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 8px; }
.fan-bar-name { font-size: 10px; font-weight: 500; color: var(--text-secondary); white-space: nowrap; }
.fan-bar-track { height: 3px; background: rgba(0, 0, 0, 0.05); border-radius: 2px; overflow: hidden; }
.fan-bar-fill { height: 100%; border-radius: 2px; background: var(--accent); transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94); }
.fan-bar-rpm { font-size: 10px; color: var(--muted); min-width: 32px; text-align: right; font-variant-numeric: tabular-nums; }

/* Battery */
.battery-status { font-size: 11px; font-weight: 500; margin-top: 4px; }
.battery-charging { color: var(--success); }
.battery-plugged { color: var(--accent); }
.battery-discharging { color: var(--text-secondary); }
.battery-details { display: flex; flex-direction: column; gap: 4px; margin-top: 10px; }
.battery-row { display: flex; justify-content: space-between; align-items: baseline; }
.battery-detail-label { font-size: 10px; font-weight: 500; color: var(--muted); text-transform: uppercase; letter-spacing: 0.3px; }
.battery-detail-value { font-size: 12px; font-weight: 600; color: var(--text); }
.battery-condition { display: flex; align-items: center; gap: 5px; margin-top: 8px; font-size: 11px; font-weight: 500; color: var(--text-secondary); }

/* Memory */
.mem-usage-text { font-size: 11px; font-weight: 500; color: var(--text-secondary); margin-top: 2px; }
.mem-seg-bar { display: flex; height: 4px; border-radius: 2px; overflow: hidden; margin-top: 10px; }
.mem-seg { min-width: 2px; transition: flex 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94); }
.mem-legend { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
.mem-legend-item { display: flex; align-items: center; gap: 3px; font-size: 9px; font-weight: 500; color: var(--muted); }
.mem-legend-dot { width: 5px; height: 5px; border-radius: 50%; }
.mem-pressure-row { display: flex; align-items: center; gap: 5px; margin-top: 8px; font-size: 11px; font-weight: 500; color: var(--text-secondary); }

/* Storage */
.storage-usage-text { font-size: 11px; font-weight: 500; color: var(--text-secondary); margin-top: 2px; }
.storage-bar { height: 4px; border-radius: 2px; background: rgba(0, 0, 0, 0.06); overflow: hidden; margin-top: 10px; }
.storage-bar-fill { height: 100%; border-radius: 2px; transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94); }
.storage-free { font-size: 11px; font-weight: 500; color: var(--muted); margin-top: 6px; }

/* ======================================================================
   Info strip
   ====================================================================== */
.info-strip { display: flex; align-items: center; gap: 16px; padding: 10px 16px; border-radius: var(--radius-sm); background: rgba(255, 255, 255, 0.3); border: 0.5px solid rgba(255, 255, 255, 0.4); margin-bottom: var(--sp-6); }
.info-strip-item { display: flex; align-items: baseline; gap: 6px; }
.info-strip-label { font-size: 10px; font-weight: 600; color: var(--muted); text-transform: uppercase; letter-spacing: 0.4px; }
.info-strip-value { font-size: 12px; font-weight: 600; color: var(--text); font-variant-numeric: tabular-nums; }
.info-strip-divider { width: 1px; height: 14px; background: rgba(0, 0, 0, 0.08); }

/* ======================================================================
   AI Summary card
   ====================================================================== */
.ai-summary-card {
  padding: 14px 18px;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.35);
  border: 0.5px solid rgba(255, 255, 255, 0.45);
  margin-bottom: var(--sp-6);
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.ai-summary-text {
  font-size: 13px;
  line-height: 1.6;
  color: var(--text);
  flex: 1;
}

.ai-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(0, 180, 216, 0.12);
  color: var(--accent-deep);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  flex-shrink: 0;
}

/* ======================================================================
   Quick Scan section
   ====================================================================== */
.scan-section { margin-bottom: var(--sp-6); }
.scan-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--sp-4); }
.scan-header h3 { font-size: 16px; font-weight: 700; }
.scan-header p { font-size: 13px; margin-top: 2px; }

/* FDA notice */
.fda-notice { display: flex; align-items: center; gap: 8px; padding: 8px 12px; border-radius: var(--radius-sm); background: var(--warning-tint); font-size: 12px; color: var(--warning-text); margin-bottom: var(--sp-4); }
.fda-notice-icon { flex-shrink: 0; }
.fda-notice-text { flex: 1; }
.fda-notice-btn { font-size: 12px; font-weight: 600; padding: 3px 10px; border-radius: 6px; background: transparent; color: var(--warning-text); border: 1px solid currentColor; cursor: pointer; white-space: nowrap; }

/* Progress */
.scan-progress { margin-bottom: var(--sp-4); }
.scan-progress-header { display: flex; justify-content: space-between; margin-bottom: 6px; }
.scan-progress-label { font-size: 12px; font-weight: 500; color: var(--text-secondary); text-transform: capitalize; }
.scan-progress-count { font-size: 12px; color: var(--muted); }
.scan-progress-track { height: 3px; background: rgba(0, 0, 0, 0.06); border-radius: 2px; overflow: hidden; }
.scan-progress-fill { height: 100%; background: var(--accent); border-radius: 2px; transition: width 0.3s ease; }

/* Domain list */
.domain-list { display: flex; flex-direction: column; }
.domain-row { display: grid; grid-template-columns: 20px 1fr auto; align-items: center; gap: 8px; padding: 7px 0; border-bottom: 1px solid rgba(0, 0, 0, 0.04); font-size: 13px; }
.domain-row:last-child { border-bottom: none; }
.domain-indicator { display: flex; align-items: center; justify-content: center; }
.domain-check { color: var(--success); }
.domain-error-icon { color: var(--danger); }
.domain-idle-dot { width: 6px; height: 6px; border-radius: 50%; background: rgba(0, 0, 0, 0.08); }
.domain-name { font-weight: 500; color: var(--text); }
.domain-result { display: flex; gap: 8px; align-items: baseline; }
.domain-size { font-weight: 600; color: var(--text); font-size: 13px; }
.domain-count { font-size: 12px; color: var(--muted); }
.domain-error-text { font-size: 12px; color: var(--danger); font-weight: 500; }
.domain-scanning .domain-name { color: var(--text-secondary); }

/* Summary */
.scan-summary { display: flex; flex-direction: column; align-items: center; padding: var(--sp-5) 0 var(--sp-3); }
.summary-value { font-size: 28px; font-weight: 700; color: var(--text); letter-spacing: -0.5px; }
.summary-label { font-size: 13px; color: var(--text-secondary); font-weight: 500; margin-top: 2px; }
.summary-detail { font-size: 12px; color: var(--muted); margin-top: 4px; }
</style>
