<script setup lang="ts">
withDefaults(
  defineProps<{
    scanning?: boolean
    scanLabel?: string
    disabled?: boolean
  }>(),
  { scanning: false, scanLabel: "Scan", disabled: false }
)

defineEmits<{
  (e: "scan"): void
}>()
</script>

<template>
  <div class="scan-bar">
    <slot />
    <button
      class="scan-bar-btn"
      :disabled="disabled || scanning"
      @click="$emit('scan')"
    >
      <span v-if="scanning" class="spinner-sm"></span>
      {{ scanning ? "Scanning..." : scanLabel }}
    </button>
  </div>
</template>

<style scoped>
.scan-bar {
  display: flex;
  align-items: center;
  background: rgba(255, 255, 255, 0.5);
  border: 1px solid var(--border);
  border-radius: 22px;
  padding: 3px 3px 3px 12px;
  gap: 0;
  flex-shrink: 0;
}

.scan-bar-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 20px;
  font-size: 13px;
  font-weight: 600;
  border: none;
  border-radius: 18px;
  background: var(--accent);
  color: white;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s;
}

.scan-bar-btn:hover {
  background: var(--accent-hover);
}

.scan-bar-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
