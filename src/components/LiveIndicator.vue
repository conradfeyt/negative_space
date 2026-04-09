<script setup lang="ts">
/**
 * LiveIndicator — Shared pill badge showing live/paused state.
 *
 * Used by: Cpu, SystemVitals, Memory, Thermal views.
 * Shows a pulsing green dot + "Live" text when active,
 * muted dot + "Paused" when paused.
 * Optionally displays last-updated timestamp.
 */
defineProps<{
  paused?: boolean
}>()
</script>

<template>
  <span class="live-badge" :class="{ paused }">
    <span class="live-dot"></span>
    {{ paused ? "Paused" : "Live" }}
  </span>
</template>

<style scoped>
.live-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 500;
  color: var(--success-text);
  padding: 3px 8px;
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
  animation: live-pulse 2s infinite;
}

.live-badge.paused .live-dot {
  background: var(--muted);
  animation: none;
}

@keyframes live-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}
</style>
