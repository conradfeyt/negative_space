<script setup lang="ts">
/**
 * TabBar — Unified segmented control / tab bar component.
 *
 * Accepts either `tabs` or `options` prop (both work identically).
 * Supports disabled items, optional badge, and scoped slot for custom content.
 */
export interface TabOption {
  value: string
  label: string
  badge?: string | number
  disabled?: boolean
}

const props = defineProps<{
  tabs?: TabOption[]
  options?: TabOption[]
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function items(): TabOption[] {
  return props.tabs ?? props.options ?? [];
}

function select(item: TabOption) {
  if (!item.disabled) {
    emit('update:modelValue', item.value)
  }
}
</script>

<template>
  <div class="tab-bar" role="tablist">
    <button
      v-for="item in items()"
      :key="item.value"
      :class="{ active: modelValue === item.value }"
      :disabled="item.disabled"
      role="tab"
      :aria-selected="modelValue === item.value"
      @click="select(item)"
    >
      <slot :option="item" :active="modelValue === item.value">
        {{ item.label }}
        <span v-if="item.badge != null" class="tab-badge">{{ item.badge }}</span>
      </slot>
    </button>
  </div>
</template>

<style scoped>
.tab-bar {
  display: inline-flex;
  background: rgb(216 216 216 / 50%);
  padding: 5px;
  border-radius: 20px;
  gap: 5px;
}

.tab-bar button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 16px;
  font-size: 13px;
  font-weight: 500;
  border-radius: 18px;
  border: none;
  background: transparent;
  color: #212430;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, box-shadow 0.15s;
}

.tab-bar button:hover:not(.active):not(:disabled) {
  background: rgba(0, 0, 0, 0.1);
}

.tab-bar button.active {
  background: rgba(255, 255, 255, 0.6);
  color: var(--text);
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
}

.tab-bar button:disabled {
  opacity: 0.35;
  pointer-events: none;
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
