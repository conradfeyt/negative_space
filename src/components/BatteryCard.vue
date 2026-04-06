<script setup lang="ts">
/**
 * BatteryCard — reusable battery dual-ring gauge + condition badge.
 *
 * Outer ring: charge percentage. Inner ring: health percentage.
 * Center overlay shows charge value and charging status.
 * Bottom meta bar: health %, cycle count, condition badge.
 */
import { computed } from "vue";
import type { BatteryInfo } from "../types";

const props = defineProps<{
  battery: BatteryInfo;
}>();

const RING_R = 50;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_R;
const HEALTH_R = 38;
const HEALTH_CIRCUMFERENCE = 2 * Math.PI * HEALTH_R;

const batteryRingColor = computed(() => {
  const c = props.battery.charge_percent;
  if (c > 50) return "var(--success)";
  if (c > 20) return "var(--warning)";
  return "var(--danger)";
});

const batteryDash = computed(() => {
  const frac = props.battery.charge_percent / 100;
  return `${frac * RING_CIRCUMFERENCE} ${(1 - frac) * RING_CIRCUMFERENCE}`;
});

const batteryHealthColor = computed(() => {
  const h = props.battery.health_percent;
  if (h >= 80) return "var(--success)";
  if (h >= 50) return "var(--warning)";
  return "var(--danger)";
});

const healthDash = computed(() => {
  const frac = props.battery.health_percent / 100;
  return `${frac * HEALTH_CIRCUMFERENCE} ${(1 - frac) * HEALTH_CIRCUMFERENCE}`;
});

const batteryConditionClass = computed(() => {
  const c = props.battery.condition.toLowerCase();
  if (c === "normal") return "dot-success";
  if (c.includes("service")) return "dot-warning";
  return "dot-danger";
});
</script>

<template>
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
      <span :class="['status-dot', batteryConditionClass]"></span>
      <span class="bat-meta-value">{{ battery.condition }}</span>
    </span>
  </div>
</template>

<style scoped>
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
  letter-spacing: 0.5px;
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

/* status-dot base + dot-success/warning/danger now in style.css */
</style>
