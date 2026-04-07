<script setup lang="ts">
/**
 * ThermalCard — reusable thermal bar display.
 *
 * Shows category temperature bars (CPU, GPU, SSD, BAT) as horizontal
 * gradient strips with marker needles.
 */
import { computed } from "vue";
import { tempToColor } from "../utils";
import type { CategorySummary } from "../types";

const props = defineProps<{
  summaries: CategorySummary[];
}>();

const thermalBarCategories = [
  { id: "cpu", short: "CPU" },
  { id: "gpu", short: "GPU" },
  { id: "storage", short: "SSD" },
  { id: "battery", short: "BAT" },
] as const;

interface ThermalBar {
  category: string;
  label: string;
  maxTemp: number;
  heightPct: number;
  color: string;
}

const thermalBars = computed<ThermalBar[]>(() => {
  const summaryMap = new Map(props.summaries.map(s => [s.category, s]));
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
</script>

<template>
  <div class="thermal-strips" v-if="thermalBars.length" aria-label="Thermal temperature bars by component category">
    <div v-for="bar in thermalBars" :key="bar.category" class="tstrip-row">
      <span class="tstrip-label">{{ bar.label }}</span>
      <div class="tstrip-track">
        <div class="tstrip-marker" :style="{ left: bar.heightPct + '%' }"></div>
      </div>
      <span class="tstrip-temp mono" :style="{ color: bar.color }">{{ bar.maxTemp }}°</span>
    </div>
  </div>
</template>

<style scoped>
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
  letter-spacing: 0.5px;
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
</style>
