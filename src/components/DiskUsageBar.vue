<template>
  <div class="disk-bar-container">
    <div class="disk-bar">
      <div
        class="disk-bar-fill"
        :style="{ width: pct + '%' }"
        :class="barClass"
      />
    </div>
    <div v-if="showLabels" class="disk-bar-labels">
      <span>{{ formatSize(used) }} used</span>
      <span>{{ formatSize(total - used) }} free</span>
      <span class="text-muted">{{ formatSize(total) }} total</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatSize } from "../utils";

const props = withDefaults(defineProps<{
  used: number;
  total: number;
  showLabels?: boolean;
}>(), {
  showLabels: true,
});

const pct = computed(() =>
  props.total > 0 ? Math.min(100, (props.used / props.total) * 100) : 0
);

const barClass = computed(() => {
  if (pct.value > 90) return "disk-bar-danger";
  if (pct.value > 80) return "disk-bar-warning";
  return "";
});
</script>

<style scoped>
.disk-bar-container {
  margin-bottom: var(--sp-4);
}

.disk-bar {
  height: 10px;
  background: var(--border);
  border-radius: 5px;
  overflow: hidden;
  margin-bottom: var(--sp-2);
}

.disk-bar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 5px;
  transition: width 0.5s ease;
}

.disk-bar-fill.disk-bar-warning {
  background: var(--warning);
}

.disk-bar-fill.disk-bar-danger {
  background: var(--danger);
}

.disk-bar-labels {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-secondary);
}
</style>
