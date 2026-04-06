<script setup lang="ts">
export interface TabOption {
  value: string
  label: string
  badge?: string | number
}

defineProps<{
  tabs: TabOption[]
  modelValue: string
}>()

defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <div class="tab-bar" role="tablist">
    <button
      v-for="tab in tabs"
      :key="tab.value"
      :class="['tab-btn', { active: modelValue === tab.value }]"
      role="tab"
      :aria-selected="modelValue === tab.value"
      @click="$emit('update:modelValue', tab.value)"
    >
      {{ tab.label }}
      <span v-if="tab.badge != null" class="tab-badge">{{ tab.badge }}</span>
    </button>
  </div>
</template>

<style scoped>
.tab-bar {
  display: inline-flex;
  gap: 2px;
  background: rgba(0, 0, 0, 0.05);
  padding: 3px;
  border-radius: 10px;
}

.tab-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 16px;
  font-size: 13px;
  font-weight: 500;
  color: var(--muted);
  border: none;
  border-radius: 8px;
  background: transparent;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.tab-btn:hover:not(.active) {
  color: var(--text-secondary);
}

.tab-btn.active {
  background: rgba(255, 255, 255, 0.8);
  color: var(--text);
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
}

.tab-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 0 6px;
  border-radius: 10px;
  background: var(--accent-light);
  color: var(--accent-deep);
  line-height: 1.6;
}
</style>
