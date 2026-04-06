<script setup lang="ts">
/**
 * Thermal — Hardware temperature sensors and fan speeds.
 *
 * Reads directly from the Apple SMC (System Management Controller) to show
 * per-sensor temperatures: CPU cores, GPU clusters, SSD, battery, memory,
 * ambient, VRM, and more. Also shows fan RPM.
 *
 * Layout:
 *   1. Assessment headline + sensor count
 *   2. Curated summary cards (CPU, GPU, Storage, Battery averages)
 *   3. Fan status
 *   4. Expandable per-category sensor list
 *
 * Refreshes every 5 seconds for a live feel.
 */
import { ref, computed, onMounted, onUnmounted, onActivated, onDeactivated } from "vue";
import { tempToColor, fanSpeedColor, fanSpeedZone } from "../utils";
import {
  thermalResult,
  thermalScanning,
  thermalScanned,
  thermalError,
  scanThermal,
} from "../stores/scanStore";
import ChipSchematic from "../components/ChipSchematic.vue";
import LiveIndicator from "../components/LiveIndicator.vue";

// ---------------------------------------------------------------------------
// Schematic responsive width — measure container, update on resize
// ---------------------------------------------------------------------------
const schematicRef = ref<HTMLElement | null>(null);
const schematicWidth = ref(680); // sensible default
let resizeObs: ResizeObserver | null = null;

function updateSchematicWidth() {
  if (schematicRef.value) {
    // Subtract padding (16px * 2 = 32px)
    schematicWidth.value = Math.round(schematicRef.value.clientWidth - 32);
  }
}

// ---------------------------------------------------------------------------
// Live refresh — 5s interval
// ---------------------------------------------------------------------------
const REFRESH_INTERVAL = 5000;
let refreshTimer: ReturnType<typeof setInterval> | null = null;
const paused = ref(false);
const lastUpdated = ref<Date | null>(null);

async function liveRefresh() {
  if (paused.value) return;
  await scanThermal(true);
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
  if (!thermalScanned.value) {
    await scanThermal(false);
  }
  lastUpdated.value = new Date();
  startTimer();

  // Observe schematic container for responsive width
  updateSchematicWidth();
  if (schematicRef.value) {
    resizeObs = new ResizeObserver(updateSchematicWidth);
    resizeObs.observe(schematicRef.value);
  }
});

onActivated(() => startTimer());
onDeactivated(() => stopTimer());
onUnmounted(() => {
  stopTimer();
  resizeObs?.disconnect();
});

// ---------------------------------------------------------------------------
// Expandable categories
// ---------------------------------------------------------------------------
const expandedCategories = ref<Set<string>>(new Set());

function toggleCategory(cat: string) {
  if (expandedCategories.value.has(cat)) {
    expandedCategories.value.delete(cat);
  } else {
    expandedCategories.value.add(cat);
  }
  // Force reactivity
  expandedCategories.value = new Set(expandedCategories.value);
}

// ---------------------------------------------------------------------------
// Computed helpers
// ---------------------------------------------------------------------------

// Group sensors by category for the expandable list
const sensorsByCategory = computed(() => {
  type SensorList = import("../types").ThermalSensor[];
  if (!thermalResult.value) return new Map<string, SensorList>();
  const map = new Map<string, SensorList>();
  for (const s of thermalResult.value.sensors) {
    if (!map.has(s.category)) {
      map.set(s.category, []);
    }
    map.get(s.category)!.push(s);
  }
  return map;
});

// Color for a temperature value
const tempColor = tempToColor;

// Fan color and zone — canonical thresholds from utils.ts
const fanColor = fanSpeedColor;
const fanZone = fanSpeedZone;

// Semicircle arc path (same as Dashboard)
function arcPath(cx: number, cy: number, r: number, startDeg: number, endDeg: number): string {
  const toRad = (deg: number) => (deg * Math.PI) / 180;
  const x1 = cx + r * Math.cos(toRad(180 - startDeg));
  const y1 = cy - r * Math.sin(toRad(180 - startDeg));
  const x2 = cx + r * Math.cos(toRad(180 - endDeg));
  const y2 = cy - r * Math.sin(toRad(180 - endDeg));
  const largeArc = endDeg - startDeg > 180 ? 1 : 0;
  return `M${x1},${y1} A${r},${r} 0 ${largeArc} 1 ${x2},${y2}`;
}

function needlePos(cx: number, cy: number, r: number, t: number): { x: number; y: number } {
  const angle = Math.PI * (1 - t);
  return { x: cx + r * Math.cos(angle), y: cy - r * Math.sin(angle) };
}

// Category icon — simple emoji-free labels
function categoryIcon(cat: string): string {
  switch (cat) {
    case "cpu": return "C";
    case "gpu": return "G";
    case "memory": return "M";
    case "storage": return "S";
    case "battery": return "B";
    case "airflow": return "A";
    case "ambient": return "T";
    case "vrm": return "V";
    case "wireless": return "W";
    default: return "?";
  }
}

// Summary card label
const summaryCardOrder = ["cpu", "gpu", "storage", "battery", "memory", "vrm", "airflow", "ambient", "wireless"];

const orderedSummaries = computed(() => {
  if (!thermalResult.value) return [];
  const map = new Map(thermalResult.value.summaries.map(s => [s.category, s]));
  return summaryCardOrder.filter(c => map.has(c)).map(c => map.get(c)!);
});
</script>

<template>
  <div class="thermal-view">
    <div class="view-header">
      <div>
        <h2>Thermal Sensors</h2>
        <p class="text-muted">Hardware temperature readings from SMC</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost btn-sm" @click="togglePause">
          {{ paused ? "Resume" : "Pause" }}
        </button>
        <LiveIndicator :paused="paused" />
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="thermalScanning && !thermalResult" class="loading-state">
      <div class="spinner-sm"></div>
      <span class="text-muted">Reading SMC sensors...</span>
    </div>

    <!-- Error state -->
    <div v-else-if="thermalError" class="error-state card">
      <p class="error-text">{{ thermalError }}</p>
      <button class="btn-primary btn-sm" @click="scanThermal(false)">Retry</button>
    </div>

    <!-- Results -->
    <template v-else-if="thermalResult">

      <!-- Assessment headline -->
      <div class="assessment-bar">
        <span class="assessment-text">{{ thermalResult.assessment }}</span>
        <span class="sensor-count mono">{{ thermalResult.sensor_count }} sensors</span>
      </div>

      <!-- Chip schematic — live per-core thermal visualization -->
      <div ref="schematicRef" class="schematic-section" v-if="thermalResult.sensors.length">
        <ChipSchematic :sensors="thermalResult.sensors" :chip-name="thermalResult.chip_name" :width="schematicWidth" />
      </div>

      <!-- Curated summary cards -->
      <div class="summary-grid">
        <div
          v-for="s in orderedSummaries"
          :key="s.category"
          class="summary-card"
          @click="toggleCategory(s.category)"
        >
          <div class="sc-icon" :style="{ color: tempColor(s.max_celsius) }">
            {{ categoryIcon(s.category) }}
          </div>
          <div class="sc-body">
            <span class="sc-label">{{ s.label }}</span>
            <span class="sc-value mono" :style="{ color: tempColor(s.max_celsius) }">
              {{ s.avg_celsius }}°
            </span>
          </div>
          <div class="sc-meta">
            <span class="sc-max">max {{ s.max_celsius }}°</span>
            <span class="sc-count">{{ s.sensor_count }} sensors</span>
          </div>
        </div>
      </div>

      <!-- Fan status — semicircle gauge (matches Dashboard thermal gauge) -->
      <div v-if="thermalResult.fans.length > 0" class="fans-section">
        <h3 class="section-label">Fans</h3>
        <div class="fans-grid">
          <div v-for="fan in thermalResult.fans" :key="fan.id" class="fan-card">
            <div class="fan-header">
              <span class="fan-name">{{ fan.name }}</span>
              <span class="fan-rpm mono" :style="{ color: fanColor(fan.percent) }">
                {{ Math.round(fan.current_rpm) }} RPM
              </span>
            </div>
            <div class="fan-gauge-wrap">
              <svg class="fan-gauge" viewBox="0 0 120 72">
                <!-- Background zones — thin, understated -->
                <path :d="arcPath(60, 62, 44, 0, 44)"   stroke="hsla(140, 20%, 40%, 0.18)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path :d="arcPath(60, 62, 44, 46, 89)"   stroke="hsla(45, 25%, 42%, 0.18)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path :d="arcPath(60, 62, 44, 91, 134)"  stroke="hsla(25, 30%, 40%, 0.18)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path :d="arcPath(60, 62, 44, 136, 180)" stroke="hsla(0, 30%, 42%, 0.18)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <!-- Active zone highlight -->
                <path v-if="fanZone(fan.percent) === 'nominal'"  :d="arcPath(60, 62, 44, 0, 44)"   stroke="hsla(140, 35%, 35%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path v-else-if="fanZone(fan.percent) === 'fair'"     :d="arcPath(60, 62, 44, 46, 89)"   stroke="hsla(45, 45%, 38%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path v-else-if="fanZone(fan.percent) === 'serious'"  :d="arcPath(60, 62, 44, 91, 134)"  stroke="hsla(25, 50%, 38%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <path v-else-if="fanZone(fan.percent) === 'critical'" :d="arcPath(60, 62, 44, 136, 180)" stroke="hsla(0, 45%, 42%, 0.65)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <!-- Needle -->
                <line x1="60" y1="62"
                  :x2="needlePos(60, 62, 36, fan.percent / 100).x"
                  :y2="needlePos(60, 62, 36, fan.percent / 100).y"
                  :stroke="fanColor(fan.percent)" stroke-width="1" stroke-linecap="round"
                />
                <circle cx="60" cy="62" r="2" :fill="fanColor(fan.percent)"/>
              </svg>
            </div>
            <div class="fan-range">
              <span class="mono">{{ Math.round(fan.min_rpm) }}</span>
              <span class="mono">{{ Math.round(fan.max_rpm) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Expandable sensor list by category -->
      <div class="sensors-section">
        <h3 class="section-label">All Sensors</h3>

        <div
          v-for="s in orderedSummaries"
          :key="'cat-' + s.category"
          class="sensor-category"
        >
          <div class="cat-header" @click="toggleCategory(s.category)">
            <span class="cat-chevron" :class="{ expanded: expandedCategories.has(s.category) }">
              <svg width="12" height="12" viewBox="0 0 10 10">
                <path d="M3 2l4 3-4 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </span>
            <span class="cat-label">{{ s.label }}</span>
            <span class="cat-avg mono">avg {{ s.avg_celsius }}°</span>
            <span class="cat-max mono" :style="{ color: tempColor(s.max_celsius) }">
              max {{ s.max_celsius }}°
            </span>
            <span class="cat-count">{{ s.sensor_count }}</span>
          </div>

          <div v-if="expandedCategories.has(s.category)" class="cat-sensors">
            <div
              v-for="sensor in sensorsByCategory.get(s.category)"
              :key="sensor.key"
              class="sensor-row"
            >
              <span class="sensor-key mono">{{ sensor.key }}</span>
              <span class="sensor-name">{{ sensor.name }}</span>
              <span class="sensor-temp mono" :style="{ color: tempColor(sensor.temp_celsius) }">
                {{ sensor.temp_celsius }}°C
              </span>
              <div class="sensor-bar-track">
                <div
                  class="sensor-bar-fill"
                  :style="{
                    width: Math.min(sensor.temp_celsius / 110 * 100, 100) + '%',
                    background: tempColor(sensor.temp_celsius),
                  }"
                ></div>
              </div>
            </div>
          </div>
        </div>
      </div>

    </template>
  </div>
</template>

<style scoped>
.thermal-view {
  max-width: 1440px;
}

/* Header */
.view-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* Loading / error */
.loading-state {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: var(--sp-6) 0;
}

.error-state {
  padding: var(--sp-5);
}

.error-text {
  color: var(--danger);
  font-size: 13px;
  margin-bottom: var(--sp-3);
}

/* Assessment */
.assessment-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-radius: var(--radius-sm);
  background: var(--glass);
  border: 1px solid var(--glass-border);
  margin-bottom: var(--sp-5);
}

.assessment-text {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  line-height: 1.4;
}

/* Chip schematic */
.schematic-section {
  margin-bottom: var(--sp-6);
  padding: 12px 16px;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.35);
  backdrop-filter: blur(20px) saturate(1.2);
  -webkit-backdrop-filter: blur(20px) saturate(1.2);
  border: 0.5px solid rgba(255, 255, 255, 0.5);
  box-shadow:
    0 0.5px 0 0 rgba(255, 255, 255, 0.6) inset,
    0 1px 3px rgba(0, 0, 0, 0.03);
}

.sensor-count {
  font-size: 11px;
  color: var(--muted);
  flex-shrink: 0;
}

/* Summary cards */
.summary-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 10px;
  margin-bottom: var(--sp-6);
}

.summary-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.12);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.25);
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.04),
    inset 0 0.5px 0 rgba(255, 255, 255, 0.35);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.summary-card:hover {
  background: rgba(255, 255, 255, 0.18);
  border-color: rgba(255, 255, 255, 0.35);
}

.sc-icon {
  font-size: 14px;
  font-weight: 700;
  font-family: var(--font-mono);
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.04);
  flex-shrink: 0;
}

.sc-body {
  display: flex;
  flex-direction: column;
  gap: 1px;
  flex: 1;
  min-width: 0;
}

.sc-label {
  font-size: 10px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.55);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.sc-value {
  font-size: 18px;
  font-weight: 500;
  letter-spacing: -0.5px;
  line-height: 1;
}

.sc-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 1px;
  flex-shrink: 0;
}

.sc-max {
  font-size: 10px;
  color: rgba(60, 70, 90, 0.5);
}

.sc-count {
  font-size: 9px;
  color: rgba(60, 70, 90, 0.35);
}

/* Fans */
.fans-section {
  margin-bottom: var(--sp-6);
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.55);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: var(--sp-3);
}

.fans-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 10px;
}

.fan-card {
  padding: 12px 14px;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.12);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.25);
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.04),
    inset 0 0.5px 0 rgba(255, 255, 255, 0.35);
}

.fan-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}

.fan-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.fan-rpm {
  font-size: 14px;
  font-weight: 500;
}

.fan-gauge-wrap {
  display: flex;
  justify-content: center;
  margin: -2px 0 2px;
}

.fan-gauge {
  width: 100%;
  max-width: 120px;
  height: auto;
}

.fan-range {
  display: flex;
  justify-content: space-between;
  font-size: 9px;
  color: rgba(60, 70, 90, 0.4);
}

/* Sensor categories — expandable */
.sensors-section {
  margin-bottom: var(--sp-6);
}

.sensor-category {
  margin-bottom: 2px;
}

.cat-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
}

.cat-header:hover {
  background: rgba(0, 0, 0, 0.03);
}

.cat-chevron {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  color: var(--muted);
  transition: transform 0.15s;
  flex-shrink: 0;
}

.cat-chevron.expanded {
  transform: rotate(90deg);
}

.cat-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  flex: 1;
}

.cat-avg {
  font-size: 11px;
  color: rgba(60, 70, 90, 0.5);
}

.cat-max {
  font-size: 12px;
  font-weight: 500;
}

.cat-count {
  font-size: 10px;
  color: rgba(60, 70, 90, 0.35);
  background: rgba(0, 0, 0, 0.04);
  padding: 1px 6px;
  border-radius: 8px;
}

/* Individual sensor rows */
.cat-sensors {
  padding: 4px 0 8px 22px;
}

.sensor-row {
  display: grid;
  grid-template-columns: 44px 1fr 52px 80px;
  align-items: center;
  gap: 8px;
  padding: 5px 12px;
  border-radius: 4px;
}

.sensor-row:hover {
  background: rgba(0, 0, 0, 0.02);
}

.sensor-key {
  font-size: 10px;
  color: rgba(60, 70, 90, 0.4);
}

.sensor-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sensor-temp {
  font-size: 13px;
  font-weight: 500;
  text-align: right;
}

.sensor-bar-track {
  height: 2px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 1px;
  overflow: hidden;
}

.sensor-bar-fill {
  height: 100%;
  border-radius: 1px;
  transition: width 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94), background 0.3s;
  opacity: 0.5;
}
</style>
