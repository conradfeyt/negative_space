<template>
  <div class="info-strip">
    <template v-for="(item, i) in items" :key="item.label">
      <span v-if="i > 0" class="info-strip-divider" />
      <span class="info-strip-item">
        <span class="info-strip-label">{{ item.label }}</span>
        <span class="info-strip-value">{{ item.value }}</span>
      </span>
    </template>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  items: Array<{ label: string; value: string | number }>;
}>();
</script>

<style scoped>
.info-strip {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 10px 16px;
  border-radius: var(--radius-sm);
  background: var(--glass);
  border: 1px solid var(--glass-border);
  margin-bottom: var(--sp-6);
}

.info-strip-item {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.info-strip-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.info-strip-value {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}

.info-strip-divider {
  width: 1px;
  height: 14px;
  background: rgba(0, 0, 0, 0.08);
}
</style>
