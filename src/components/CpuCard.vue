<script setup lang="ts">
/**
 * CpuCard — reusable CPU core temperature heatmap grid.
 *
 * Shows a grid of colored cells, one per physical CPU core,
 * colored by temperature.
 */
import { computed } from "vue";
import { tempToColor } from "../utils";
import type { ThermalSensor } from "../types";

const props = defineProps<{
  sensors: ThermalSensor[];
}>();

const coreTempStrip = computed(() => {
  const primaryPattern = /^T[pe]\d[0-9]$/;
  let cores = props.sensors
    .filter(s => s.category === "cpu" && primaryPattern.test(s.key))
    .sort((a, b) => a.key.localeCompare(b.key))
    .map(s => ({ key: s.key, temp: s.temp_celsius, color: tempToColor(s.temp_celsius) }));
  if (cores.length === 0) {
    cores = props.sensors
      .filter(s => s.category === "cpu")
      .sort((a, b) => a.key.localeCompare(b.key))
      .slice(0, 24)
      .map(s => ({ key: s.key, temp: s.temp_celsius, color: tempToColor(s.temp_celsius) }));
  }
  return cores;
});
</script>

<template>
  <div class="cpu-heatmap" v-if="coreTempStrip.length">
    <div v-for="core in coreTempStrip" :key="core.key" class="cpu-heat-cell"
      :style="{ background: core.color }" :title="core.key + ': ' + core.temp + '°C'">
    </div>
  </div>
</template>

<style scoped>
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
</style>
