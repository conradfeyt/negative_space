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
import { ref, computed, watch, onMounted, onUnmounted, onActivated, onDeactivated } from "vue";
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
  largeFiles,
  largeFilesScanned,
  lastScanTime,
  diskMapResult,
} from "../stores/scanStore";

async function openFdaSettings() {
  try { await invoke("open_full_disk_access_settings"); } catch (_) {}
}

async function revealInFinder(path: string) {
  try { await invoke("reveal_in_finder", { path }); } catch (_) {}
}

const router = useRouter();
const navigateTo = (id: string) => router.push({ name: id });

// ---------------------------------------------------------------------------
// Live refresh
// ---------------------------------------------------------------------------
const POLL_INTERVAL = 5000;
let pollTimer: ReturnType<typeof setInterval> | null = null;

async function poll() {
  updateLastScanAgo();
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
  docker: { label: "Docker" },
};

// Domains included in quick scan (excludes security — too slow, run from Security view)
const quickScanDomains = new Set(Object.keys(domainMeta));

const totalFound = computed(() =>
  Object.entries(domainStatus.value)
    .filter(([k]) => quickScanDomains.has(k))
    .reduce((sum, [, d]) => sum + d.itemCount, 0)
);

const hasAnyResults = computed(() =>
  Object.entries(domainStatus.value)
    .filter(([k]) => quickScanDomains.has(k))
    .some(([, d]) => d.status === "done" || d.status === "error")
);

const domainEntries = computed(() =>
  Object.entries(domainStatus.value)
    .filter(([k]) => quickScanDomains.has(k))
    .map(([key, info]) => ({
      key, ...info, meta: domainMeta[key] ?? { label: key },
    }))
);

const quickScanTotal = computed(() => quickScanDomains.size);
const quickScanDone = computed(() =>
  Object.entries(domainStatus.value)
    .filter(([k]) => quickScanDomains.has(k))
    .filter(([, d]) => d.status === "done").length
);

const scanProgress = computed(() => {
  if (!scanAllRunning.value && !scanAllDone.value) return 0;
  return Math.round((quickScanDone.value / quickScanTotal.value) * 100);
});

// ---------------------------------------------------------------------------
// Right panel — largest files + reclaimable summary
// ---------------------------------------------------------------------------
const topFiles = computed(() =>
  [...largeFiles.value]
    .sort((a, b) => {
      const sizeA = a.is_sparse && a.actual_size < a.apparent_size * 0.8 ? a.actual_size : a.apparent_size;
      const sizeB = b.is_sparse && b.actual_size < b.apparent_size * 0.8 ? b.actual_size : b.apparent_size;
      return sizeB - sizeA;
    })
    .slice(0, 5)
);

function fileIcon(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    mov: "\uD83C\uDFAC", mp4: "\uD83C\uDFAC", mkv: "\uD83C\uDFAC", avi: "\uD83C\uDFAC",
    dmg: "\uD83D\uDCBF", iso: "\uD83D\uDCBF", pkg: "\uD83D\uDCBF",
    zip: "\uD83D\uDCE6", tar: "\uD83D\uDCE6", gz: "\uD83D\uDCE6", xz: "\uD83D\uDCE6",
    jpg: "\uD83D\uDDBC", png: "\uD83D\uDDBC", heic: "\uD83D\uDDBC", raw: "\uD83D\uDDBC",
    qcow2: "\uD83D\uDC33", vmdk: "\uD83D\uDC33",
  };
  return map[ext] ?? "\uD83D\uDCC4";
}

function fileDir(path: string): string {
  const home = path.replace(/^\/Users\/[^/]+/, "~");
  const parts = home.split("/");
  parts.pop();
  return parts.join("/");
}

function diskSize(file: { is_sparse: boolean; actual_size: number; apparent_size: number }): number {
  return file.is_sparse && file.actual_size < file.apparent_size * 0.8 ? file.actual_size : file.apparent_size;
}

const reclaimBreakdown = computed(() => {
  const entries: string[] = [];
  for (const [key, info] of Object.entries(domainStatus.value)) {
    if (info.status === "done" && info.totalSize > 0 && key !== "security") {
      const label = domainMeta[key]?.label ?? key;
      entries.push(`${label} ${formatSize(info.totalSize)}`);
    }
  }
  return entries.join(" \u00B7 ");
});

// Last scan relative time
const lastScanAgo = ref("");
function updateLastScanAgo() {
  if (!lastScanTime.value) { lastScanAgo.value = ""; return; }
  const secs = Math.floor((Date.now() - lastScanTime.value) / 1000);
  if (secs < 60) lastScanAgo.value = "just now";
  else if (secs < 3600) lastScanAgo.value = `${Math.floor(secs / 60)} min ago`;
  else if (secs < 86400) lastScanAgo.value = `${Math.floor(secs / 3600)} hr ago`;
  else lastScanAgo.value = `${Math.floor(secs / 86400)}d ago`;
}

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
// cpuLoadWidth/cpuLoadColor removed — load bar replaced with heatmap grid

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

// Battery ring
const batteryRingColor = computed(() => {
  if (!battery.value) return "var(--muted)";
  const c = battery.value.charge_percent;
  if (c > 50) return "var(--success)";
  if (c > 20) return "var(--warning)";
  return "var(--danger)";
});

const batteryDash = computed(() => {
  if (!battery.value) return `0 ${RING_CIRCUMFERENCE}`;
  const frac = battery.value.charge_percent / 100;
  return `${frac * RING_CIRCUMFERENCE} ${(1 - frac) * RING_CIRCUMFERENCE}`;
});

const HEALTH_R = 38;
const HEALTH_CIRCUMFERENCE = 2 * Math.PI * HEALTH_R;

const healthDash = computed(() => {
  if (!battery.value) return `0 ${HEALTH_CIRCUMFERENCE}`;
  const frac = battery.value.health_percent / 100;
  return `${frac * HEALTH_CIRCUMFERENCE} ${(1 - frac) * HEALTH_CIRCUMFERENCE}`;
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
    { label: "App", bytes: s.app_bytes, pct: (s.app_bytes / t) * 100, color: "hsla(195, 45%, 42%, 0.65)" },
    { label: "Wired", bytes: s.wired_bytes, pct: (s.wired_bytes / t) * 100, color: "hsla(35, 50%, 45%, 0.6)" },
    { label: "Compressed", bytes: s.compressed_bytes, pct: (s.compressed_bytes / t) * 100, color: "hsla(280, 30%, 50%, 0.5)" },
    { label: "Free", bytes: s.free_bytes, pct: (s.free_bytes / t) * 100, color: "hsla(140, 20%, 70%, 0.4)" },
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
// Storage card — waffle chart
// ---------------------------------------------------------------------------
const categoryColors: Record<string, string> = {
  applications: "hsla(0, 65%, 55%, 0.8)",
  documents:    "hsla(35, 75%, 55%, 0.8)",
  developer:    "hsla(45, 80%, 50%, 0.8)",
  books:        "hsla(145, 50%, 45%, 0.8)",
  icloud:       "hsla(210, 65%, 55%, 0.8)",
  mail:         "hsla(210, 60%, 55%, 0.8)",
  photos:       "hsla(320, 45%, 55%, 0.8)",
  media:        "hsla(280, 40%, 55%, 0.8)",
  bin:          "hsla(220, 10%, 55%, 0.6)",
  docker:       "hsla(195, 55%, 45%, 0.8)",
  caches:       "hsla(35, 45%, 50%, 0.7)",
  macos:        "hsla(220, 15%, 50%, 0.65)",
  system_data:  "hsla(220, 10%, 45%, 0.6)",
  system:       "hsla(220, 15%, 50%, 0.65)",
  other:        "hsla(220, 10%, 60%, 0.45)",
  free:         "hsla(0, 0%, 88%, 0.35)",
};

const categoryLabels: Record<string, string> = {
  applications: "Apps", documents: "Docs", media: "Media",
  developer: "Dev", books: "Books", mail: "Mail",
  photos: "Photos", icloud: "iCloud", bin: "Bin", docker: "Docker",
  caches: "Caches", macos: "macOS", system_data: "System",
  system: "System", other: "Other", free: "Free",
};

const waffleCells = computed(() => {
  if (!diskMapResult.value || !diskUsage.value) return null;
  const total = diskUsage.value.total;
  if (total <= 0) return null;

  // Aggregate children by category
  const byCategory: Record<string, number> = {};
  for (const child of diskMapResult.value.root.children) {
    const cat = child.category || "other";
    byCategory[cat] = (byCategory[cat] || 0) + child.size;
  }

  // Convert to percentages of total disk, sorted by size desc
  const entries = Object.entries(byCategory)
    .map(([cat, size]) => ({ cat, size, pct: (size / total) * 100 }))
    .filter(e => e.pct >= 0.5)
    .sort((a, b) => b.size - a.size);

  // Assign cell counts (out of 100)
  const TOTAL_CELLS = 100;
  let assigned = 0;
  const cellEntries: { cat: string; cells: number }[] = [];

  for (const e of entries) {
    const cells = Math.max(1, Math.round((e.pct / 100) * TOTAL_CELLS));
    cellEntries.push({ cat: e.cat, cells });
    assigned += cells;
  }

  const freeCells = Math.max(0, TOTAL_CELLS - assigned);
  cellEntries.push({ cat: "free", cells: freeCells });

  // Adjust to exactly 100 — trim from largest if over, add to largest if under
  let sum = cellEntries.reduce((s, e) => s + e.cells, 0);
  while (sum > TOTAL_CELLS) {
    const largest = cellEntries.reduce((a, b) => a.cells > b.cells ? a : b);
    largest.cells--;
    sum--;
  }
  while (sum < TOTAL_CELLS) {
    const largest = cellEntries.reduce((a, b) => a.cells > b.cells ? a : b);
    largest.cells++;
    sum++;
  }

  // Flatten to array of 100 color strings
  const cells: { color: string; cat: string }[] = [];
  for (const e of cellEntries) {
    for (let i = 0; i < e.cells; i++) {
      cells.push({ color: categoryColors[e.cat] || categoryColors.other, cat: e.cat });
    }
  }

  // Legend entries (only categories with cells)
  const legend = cellEntries
    .filter(e => e.cells > 0 && e.cat !== "free")
    .map(e => ({ cat: e.cat, label: categoryLabels[e.cat] || e.cat, color: categoryColors[e.cat] || categoryColors.other }));

  return { cells, legend };
});

// ---------------------------------------------------------------------------
// Memory card — ring gauge segments
// ---------------------------------------------------------------------------
const RING_R = 50;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_R;

const memRingSegments = computed(() => {
  if (!memoryResult.value) return [];
  const s = memoryResult.value.stats;
  const t = s.total_bytes || 1;
  // Build segments in order: App, Wired, Compressed (skip Free — it's the gap)
  const segs = [
    { label: "App", bytes: s.app_bytes, color: "hsla(195, 55%, 42%, 0.75)" },
    { label: "Wired", bytes: s.wired_bytes, color: "hsla(35, 55%, 45%, 0.7)" },
    { label: "Compressed", bytes: s.compressed_bytes, color: "hsla(280, 35%, 50%, 0.65)" },
  ].filter(seg => seg.bytes > 0);

  let cumulative = 0;
  return segs.map(seg => {
    const frac = seg.bytes / t;
    const dashLen = frac * RING_CIRCUMFERENCE;
    const gapLen = RING_CIRCUMFERENCE - dashLen;
    // SVG circle starts at 3 o'clock, offset rotates to 12 o'clock (+ 90deg = circumference/4)
    const offset = RING_CIRCUMFERENCE * 0.25 - cumulative * RING_CIRCUMFERENCE;
    cumulative += frac;
    return {
      label: seg.label,
      color: seg.color,
      dash: `${dashLen} ${gapLen}`,
      offset: `${offset}`,
    };
  });
});

const memCardBg = computed(() => {
  const pct = memUsedPct.value;
  if (pct >= 90) return "rgba(217, 75, 75, 0.08)";
  if (pct >= 75) return "rgba(229, 163, 15, 0.06)";
  return "rgba(255, 255, 255, 0.45)";
});

// ---------------------------------------------------------------------------
// Memory card — top consumers
// ---------------------------------------------------------------------------
const topMemConsumers = computed(() => {
  if (!memoryResult.value?.groups) return [];
  return [...memoryResult.value.groups]
    .sort((a, b) => b.total_rss_bytes - a.total_rss_bytes)
    .slice(0, 3);
});

// ---------------------------------------------------------------------------
// Storage card — reclaimable hint from scan data
// ---------------------------------------------------------------------------
const storageReclaimable = computed(() => {
  const total = Object.entries(domainStatus.value)
    .filter(([k]) => k !== "security")
    .reduce((sum, [, d]) => sum + d.totalSize, 0);
  return total;
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
  updateLastScanAgo();
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
        <span v-if="lastScanAgo" class="last-scan-label text-muted">Last scan: {{ lastScanAgo }}</span>
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
        <div class="thermal-strips" v-if="thermalBars.length">
          <div v-for="bar in thermalBars" :key="bar.category" class="tstrip-row">
            <span class="tstrip-label">{{ bar.label }}</span>
            <div class="tstrip-track">
              <div class="tstrip-marker" :style="{ left: bar.heightPct + '%' }"></div>
            </div>
            <span class="tstrip-temp mono" :style="{ color: bar.color }">{{ bar.maxTemp }}°</span>
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
        <div class="cpu-heatmap" v-if="coreTempStrip.length">
          <div v-for="core in coreTempStrip" :key="core.key" class="cpu-heat-cell"
            :style="{ background: core.color }" :title="core.key + ': ' + core.temp + '°C'">
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
      <div class="stat-card stat-card--battery" v-if="battery">
        <div class="stat-label">Battery</div>
        <div class="bat-ring-row">
          <svg class="bat-ring-svg" viewBox="0 0 120 120">
            <circle cx="60" cy="60" r="50" fill="none" stroke="rgba(0,0,0,0.06)" stroke-width="10"/>
            <circle cx="60" cy="60" r="50" fill="none"
              :stroke="batteryRingColor" stroke-width="10" stroke-linecap="round"
              :stroke-dasharray="batteryDash" :stroke-dashoffset="RING_CIRCUMFERENCE * 0.25"
              :style="{ transition: 'stroke-dasharray 0.6s ease' }"
            />
            <!-- Health arc (inner, thinner) -->
            <circle cx="60" cy="60" r="38" fill="none" stroke="rgba(0,0,0,0.04)" stroke-width="4"/>
            <circle cx="60" cy="60" r="38" fill="none"
              :stroke="batteryHealthColor" stroke-width="4" stroke-linecap="round"
              :stroke-dasharray="healthDash" :stroke-dashoffset="HEALTH_CIRCUMFERENCE * 0.25"
              :style="{ transition: 'stroke-dasharray 0.6s ease' }"
            />
          </svg>
          <div class="bat-ring-center">
            <span class="bat-ring-value">{{ battery.charge_percent }}%</span>
            <span class="bat-ring-status">
              <span v-if="battery.is_charging">Charging</span>
              <span v-else-if="battery.ac_connected">Plugged In</span>
              <span v-else>On Battery</span>
            </span>
          </div>
        </div>
        <div class="bat-meta">
          <span class="bat-meta-item">
            <span class="bat-meta-label">Health</span>
            <span class="bat-meta-value" :style="{ color: batteryHealthColor }">{{ battery.health_percent }}%</span>
          </span>
          <span class="bat-meta-divider"></span>
          <span class="bat-meta-item">
            <span class="bat-meta-label">Cycles</span>
            <span class="bat-meta-value mono">{{ battery.cycle_count }}</span>
          </span>
          <span class="bat-meta-divider"></span>
          <span class="bat-meta-item">
            <span :class="['thermal-dot', batteryConditionClass]"></span>
            <span class="bat-meta-value">{{ battery.condition }}</span>
          </span>
        </div>
      </div>

      <!-- MEMORY -->
      <div class="stat-card stat-card--memory" @click="navigateTo('memory')" v-if="memoryResult"
        :style="{ background: memCardBg }">
        <div class="stat-label">Memory</div>
        <div class="mem-ring-row">
          <svg class="mem-ring-svg" viewBox="0 0 120 120">
            <!-- Background track -->
            <circle cx="60" cy="60" r="50" fill="none" stroke="rgba(0,0,0,0.06)" stroke-width="10"/>
            <!-- Segmented arc: each memory segment drawn as a stroke-dasharray arc -->
            <circle v-for="seg in memRingSegments" :key="seg.label"
              cx="60" cy="60" r="50" fill="none"
              :stroke="seg.color" stroke-width="10" stroke-linecap="butt"
              :stroke-dasharray="seg.dash" :stroke-dashoffset="seg.offset"
              :style="{ transition: 'stroke-dasharray 0.6s ease, stroke-dashoffset 0.6s ease' }"
            />
          </svg>
          <div class="mem-ring-center">
            <span class="mem-ring-value">{{ formatSize(memoryResult.stats.used_bytes) }}</span>
            <span class="mem-ring-total">of {{ formatSize(memoryResult.stats.total_bytes) }}</span>
          </div>
        </div>
        <div class="mem-ring-legend">
          <span v-for="seg in memSegments" :key="seg.label" class="mem-ring-legend-item">
            <span class="mem-ring-legend-dot" :style="{ background: seg.color }"></span>
            {{ seg.label }} {{ formatSize(seg.bytes) }}
          </span>
        </div>
        <div class="mem-pressure-row">
          <span :class="['thermal-dot', memPressure.class]"></span>
          {{ memPressure.label }} pressure
        </div>
        <div v-if="topMemConsumers.length" class="mem-top-consumers">
          <div v-for="g in topMemConsumers" :key="g.name" class="mem-consumer-row">
            <span class="mem-consumer-name">{{ g.name }}</span>
            <span class="mem-consumer-size mono">{{ formatSize(g.total_rss_bytes) }}</span>
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
          <!-- Waffle chart when disk map data available -->
          <template v-if="waffleCells">
            <div class="waffle-grid">
              <span v-for="(cell, i) in waffleCells.cells" :key="i" class="waffle-cell" :style="{ background: cell.color }" :title="categoryLabels[cell.cat] || cell.cat"></span>
            </div>
            <div class="waffle-legend">
              <span v-for="entry in waffleCells.legend" :key="entry.cat" class="waffle-legend-item">
                <span class="waffle-legend-dot" :style="{ background: entry.color }"></span>
                {{ entry.label }}
              </span>
            </div>
          </template>
          <!-- Fallback bar when no disk map -->
          <div v-else class="storage-bar">
            <div class="storage-bar-fill" :style="{ width: storagePct + '%', background: storageColor }"></div>
          </div>
          <div class="storage-detail-row">
            <span class="storage-free">{{ formatSize(diskUsage.free) }} free</span>
            <span v-if="storageReclaimable > 0" class="storage-reclaimable">{{ formatSize(storageReclaimable) }} reclaimable</span>
          </div>
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
         Bottom row — Left Panel + Quick Scan
         ================================================================ -->
    <div class="bottom-row">

    <!-- Left panel — Largest Files + Reclaimable -->
    <div class="left-panel">

      <!-- Largest Files -->
      <div class="card side-card side-card-files">
        <div class="side-card-title">Largest Files</div>
        <template v-if="largeFilesScanned && topFiles.length > 0">
          <div
            v-for="file in topFiles"
            :key="file.path"
            class="file-row"
            @click="revealInFinder(file.path)"
          >
            <span class="file-icon">{{ fileIcon(file.name) }}</span>
            <div class="file-info">
              <div class="file-name">{{ file.name }}</div>
              <div class="file-path">{{ fileDir(file.path) }}</div>
            </div>
            <span class="file-size mono">{{ formatSize(diskSize(file)) }}</span>
          </div>
        </template>
        <p v-else class="text-muted side-empty">Run a scan to see largest files</p>
      </div>

      <!-- Reclaimable Space -->
      <div class="card side-card side-card-reclaim">
        <div class="side-card-title">Reclaimable Space</div>
        <div class="reclaim-row">
          <span class="reclaim-value mono">{{ formatSize(storeTotalReclaimable) }}</span>
          <span v-if="hasAnyResults" class="reclaim-sub text-muted">safe to remove</span>
        </div>
        <p v-if="reclaimBreakdown" class="reclaim-breakdown text-muted">{{ reclaimBreakdown }}</p>
        <button
          class="btn-primary scan-btn reclaim-scan-btn"
          :disabled="scanAllRunning"
          @click="scanAll"
        >
          {{ scanAllRunning ? "Scanning..." : scanAllDone ? "Rescan All" : "Run Full Scan" }}
        </button>
      </div>

    </div>

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
          <span class="scan-progress-count mono">{{ quickScanDone }} / {{ quickScanTotal }}</span>
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
        <span class="summary-detail">{{ totalFound }} items across {{ quickScanDone }} domains</span>
      </div>
    </div>

    </div><!-- /bottom-row -->
  </div>
</template>

<style scoped>
.dashboard {
  max-width: 1440px;
}

/* Last scan timestamp */
.view-header-top {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.last-scan-label {
  font-size: 11px;
  font-weight: 400;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

/* Bottom row — left panel + scan */
.bottom-row {
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: 14px;
  align-items: stretch;
}

@media (max-width: 720px) {
  .bottom-row {
    grid-template-columns: 1fr;
  }
}

/* Left panel */
.left-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.side-card {
  padding: 16px 18px;
}

.side-card-files {
  flex: 1;
}

.side-card-title {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text);
  opacity: 0.45;
  margin-bottom: 12px;
}

/* File rows */
.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 0;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.15s;
}

.file-row:last-child { border-bottom: none; }

.file-row:hover {
  background: rgba(255, 255, 255, 0.35);
  padding-left: 4px;
}

.file-icon {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  flex-shrink: 0;
}

.file-info {
  flex: 1;
  min-width: 0;
}

.file-name {
  font-size: 11px;
  font-weight: 400;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-path {
  font-size: 9px;
  color: var(--text);
  opacity: 0.4;
  font-weight: 300;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-size {
  font-size: 11px;
  font-weight: 300;
  color: var(--text);
  opacity: 0.6;
  flex-shrink: 0;
}

.side-empty {
  font-size: 11px;
  text-align: center;
  padding: 16px 0;
}

/* Reclaimable space card */
.reclaim-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 4px;
}

.reclaim-value {
  font-size: 26px;
  font-weight: 200;
  color: var(--text);
  letter-spacing: -0.03em;
  line-height: 1;
}

.reclaim-sub {
  font-size: 10px;
}

.reclaim-breakdown {
  font-size: 10px;
  font-weight: 300;
  margin-bottom: 14px;
  line-height: 1.4;
}

.reclaim-scan-btn {
  width: 100%;
  border-radius: 12px;
  font-size: 12px;
  padding: 10px;
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
  overflow: visible;
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
/* Thermal gradient strips */
.thermal-strips {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 12px;
}

.tstrip-row {
  display: grid;
  grid-template-columns: 28px 1fr 32px;
  align-items: center;
  gap: 6px;
}

.tstrip-label {
  font-size: 9px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.5);
  letter-spacing: 0.2px;
}

.tstrip-track {
  height: 6px;
  border-radius: 3px;
  background: linear-gradient(90deg,
    hsla(195, 50%, 55%, 0.35) 0%,
    hsla(50, 60%, 50%, 0.4) 45%,
    hsla(25, 60%, 50%, 0.5) 70%,
    hsla(0, 55%, 50%, 0.6) 100%
  );
  position: relative;
}

.tstrip-marker {
  position: absolute;
  top: -2px;
  width: 3px;
  height: 10px;
  border-radius: 1.5px;
  background: var(--text);
  transform: translateX(-1.5px);
  transition: left 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.tstrip-temp {
  font-size: 10px;
  font-weight: 500;
  text-align: right;
}

/* CPU */
.stat-card--cpu { justify-content: space-between; }
.cpu-top { }
.cpu-cores-label { font-size: 10px; font-weight: 500; color: var(--muted); margin-top: 2px; }

.cpu-heatmap {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(16px, 1fr));
  gap: 3px;
  margin-top: 12px;
}

.cpu-heat-cell {
  aspect-ratio: 1;
  border-radius: 3px;
  transition: background 0.4s ease;
  min-height: 16px;
}

.load-averages { display: flex; gap: 10px; margin-top: 8px; font-size: 10px; color: var(--muted); font-variant-numeric: tabular-nums; }

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
/* Battery ring gauge */
.bat-ring-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 8px 0 4px;
}

.bat-ring-svg {
  width: 130px;
  height: 130px;
}

.bat-ring-center {
  position: absolute;
  display: flex;
  flex-direction: column;
  align-items: center;
  pointer-events: none;
}

.bat-ring-value {
  font-size: 22px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
}

.bat-ring-status {
  font-size: 10px;
  font-weight: 500;
  color: var(--muted);
}

.bat-meta {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 6px;
}

.bat-meta-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.bat-meta-label {
  font-size: 9px;
  font-weight: 500;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.bat-meta-value {
  font-size: 11px;
  font-weight: 600;
  color: var(--text);
}

.bat-meta-divider {
  width: 1px;
  height: 12px;
  background: rgba(0, 0, 0, 0.08);
}

/* Memory */
.mem-usage-text { font-size: 11px; font-weight: 500; color: var(--text-secondary); margin-top: 2px; }
.mem-seg-bar { display: flex; height: 4px; border-radius: 2px; overflow: hidden; margin-top: 10px; }
.mem-seg { min-width: 2px; transition: flex 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94); }
/* Memory ring gauge */
.stat-card--memory { transition: background 0.5s ease; }

.mem-ring-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 8px 0 4px;
}

.mem-ring-svg {
  width: 130px;
  height: 130px;
  transform: rotate(0deg);
}

.mem-ring-center {
  position: absolute;
  display: flex;
  flex-direction: column;
  align-items: center;
  pointer-events: none;
}

.mem-ring-value {
  font-size: 18px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
}

.mem-ring-total {
  font-size: 10px;
  font-weight: 400;
  color: var(--muted);
}

.mem-ring-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
  margin-top: 4px;
}

.mem-ring-legend-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 9px;
  font-weight: 500;
  color: var(--text-secondary);
}

.mem-ring-legend-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.mem-pressure-row { display: flex; align-items: center; justify-content: center; gap: 5px; margin-top: 8px; font-size: 11px; font-weight: 500; color: var(--text-secondary); }

.mem-top-consumers { margin-top: 10px; padding-top: 8px; border-top: 0.5px solid rgba(0, 0, 0, 0.06); }
.mem-consumer-row { display: flex; align-items: center; justify-content: space-between; padding: 2px 0; }
.mem-consumer-name { font-size: 10px; font-weight: 400; color: var(--text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 65%; }
.mem-consumer-size { font-size: 10px; font-weight: 400; color: var(--muted); flex-shrink: 0; }

/* Storage */
.storage-usage-text { font-size: 11px; font-weight: 500; color: var(--text-secondary); margin-top: 2px; }
.storage-bar { height: 4px; border-radius: 2px; background: rgba(0, 0, 0, 0.06); overflow: hidden; margin-top: 10px; }
.storage-bar-fill { height: 100%; border-radius: 2px; transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94); }
.storage-detail-row { display: flex; align-items: center; justify-content: space-between; margin-top: 6px; }
.storage-free { font-size: 11px; font-weight: 500; color: var(--muted); }
.storage-reclaimable { font-size: 10px; font-weight: 500; color: var(--accent); }

/* Waffle chart */
.waffle-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(14px, 1fr));
  gap: 2px;
  margin-top: 10px;
}
.waffle-cell {
  height: 14px;
  border-radius: 2px;
  transition: background 0.3s;
}
.waffle-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}
.waffle-legend-item {
  display: flex;
  align-items: center;
  gap: 3px;
  font-size: 9px;
  font-weight: 500;
  color: var(--muted);
}
.waffle-legend-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  flex-shrink: 0;
}

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
.scan-section { margin-bottom: 0; display: flex; flex-direction: column; }
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
