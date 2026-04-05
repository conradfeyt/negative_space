<script setup lang="ts">
/**
 * FanCard — reusable fan gauge + bar display.
 *
 * Shows mini semicircular gauges per fan with colored zone arcs,
 * plus a simple percent bar row beneath.
 */
import type { FanReading } from "../types";

defineProps<{
  fans: FanReading[];
  avgRpm: number;
}>();

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
</script>

<template>
  <template v-if="fans.length">
    <div class="stat-hero">{{ avgRpm }}<span class="stat-unit">RPM</span></div>
    <div class="fan-items">
      <div v-for="fan in fans" :key="fan.id" class="fan-item">
        <svg class="fan-mini-gauge" viewBox="0 0 48 30" role="img" :aria-label="'Fan ' + fan.id + ' speed gauge at ' + fan.percent + '%'">
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
</template>

<style scoped>
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

.fan-items {
  display: flex;
  gap: 12px;
  margin-top: 10px;
  justify-content: center;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
}

.fan-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.fan-mini-gauge {
  width: 52px;
  height: 32px;
}

.fan-bars {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin-top: 6px;
}

.fan-bar-row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 8px;
}

.fan-bar-name {
  font-size: 10px;
  font-weight: 500;
  color: var(--text-secondary);
  white-space: nowrap;
}

.fan-bar-track {
  height: 3px;
  background: rgba(0, 0, 0, 0.05);
  border-radius: 2px;
  overflow: hidden;
}

.fan-bar-fill {
  height: 100%;
  border-radius: 2px;
  background: var(--accent);
  transition: width 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.fan-bar-rpm {
  font-size: 10px;
  color: var(--muted);
  min-width: 32px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}
</style>
