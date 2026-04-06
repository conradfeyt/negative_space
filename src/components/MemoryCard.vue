<script setup lang="ts">
/**
 * MemoryCard — reusable memory ring gauge + segment breakdown.
 *
 * Segmented ring showing App / Wired / Compressed memory usage,
 * center label with used/total, legend with per-segment sizes,
 * and a pressure indicator.
 */
import { computed } from "vue";
import { formatSize, memoryPressureDotClass, memoryPressureLevel } from "../utils";
import type { MemoryStats } from "../types";

const props = defineProps<{
  stats: MemoryStats;
}>();

const RING_R = 50;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_R;

const memUsedPct = computed(() => {
  return props.stats.total_bytes > 0 ? (props.stats.used_bytes / props.stats.total_bytes) * 100 : 0;
});

const memRingSegments = computed(() => {
  const t = props.stats.total_bytes || 1;
  const segs = [
    { label: "App", bytes: props.stats.app_bytes, color: "var(--mem-app)" },
    { label: "Wired", bytes: props.stats.wired_bytes, color: "var(--mem-wired)" },
    { label: "Compressed", bytes: props.stats.compressed_bytes, color: "var(--mem-compressed)" },
  ].filter(seg => seg.bytes > 0);

  let cumulative = 0;
  return segs.map(seg => {
    const frac = seg.bytes / t;
    const dashLen = frac * RING_CIRCUMFERENCE;
    const gapLen = RING_CIRCUMFERENCE - dashLen;
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

const memSegments = computed(() => {
  const t = props.stats.total_bytes || 1;
  return [
    { label: "App", bytes: props.stats.app_bytes, pct: (props.stats.app_bytes / t) * 100, color: "var(--mem-app)" },
    { label: "Wired", bytes: props.stats.wired_bytes, pct: (props.stats.wired_bytes / t) * 100, color: "var(--mem-wired)" },
    { label: "Compressed", bytes: props.stats.compressed_bytes, pct: (props.stats.compressed_bytes / t) * 100, color: "var(--mem-compressed)" },
    { label: "Free", bytes: props.stats.free_bytes, pct: (props.stats.free_bytes / t) * 100, color: "var(--mem-free)" },
  ].filter(seg => seg.pct > 0.5);
});

const memPressure = computed(() => {
  const pct = memUsedPct.value;
  const level = memoryPressureLevel(pct);
  return { label: level.label, class: memoryPressureDotClass(pct) };
});
</script>

<template>
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
      <span class="mem-ring-value">{{ formatSize(stats.used_bytes) }}</span>
      <span class="mem-ring-total">of {{ formatSize(stats.total_bytes) }}</span>
    </div>
  </div>
  <div class="mem-ring-legend">
    <span v-for="seg in memSegments" :key="seg.label" class="mem-ring-legend-item">
      <span class="mem-ring-legend-dot" :style="{ background: seg.color }"></span>
      {{ seg.label }} {{ formatSize(seg.bytes) }}
    </span>
  </div>
  <div class="mem-pressure-row">
    <span :class="['status-dot', memPressure.class]"></span>
    {{ memPressure.label }} pressure
  </div>
</template>

<style scoped>
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

.mem-pressure-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  margin-top: 8px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
}

/* status-dot base + dot-success/warning/danger now in style.css */
</style>
