<script setup lang="ts">
defineProps<{
  value: string
  label: string
  highlight?: boolean
  valueColor?: string
  labelColor?: string
}>()
</script>

<template>
  <div
    class="stat-card"
    :class="{ 'stat-card--highlight': highlight }"
  >
    <span
      class="stat-card__value"
      :style="valueColor ? { color: valueColor } : undefined"
    >{{ value }}</span>
    <span
      class="stat-card__label"
      :style="labelColor ? { color: labelColor } : undefined"
    >{{ label }}</span>
  </div>
</template>

<style scoped>
.stat-card {
  background: var(--surface);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  padding: 14px 16px;
  flex: 1;
  min-width: 0;
}

.stat-card__value {
  display: block;
  font-size: 18px;
  font-weight: 700;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}

.stat-card__label {
  display: block;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
  margin-top: 2px;
}

.stat-card--highlight .stat-card__value {
  color: var(--accent);
}
</style>
