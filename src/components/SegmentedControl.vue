<script setup lang="ts">
export interface SegmentOption {
  value: string
  label: string
  disabled?: boolean
}

defineProps<{
  options: SegmentOption[]
  modelValue: string
  pill?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function select(option: SegmentOption) {
  if (!option.disabled) {
    emit('update:modelValue', option.value)
  }
}
</script>

<template>
  <div
    class="segmented-control"
    :class="{ 'segmented-control--pill': pill }"
    role="group"
  >
    <button
      v-for="opt in options"
      :key="opt.value"
      class="segmented-control__btn"
      :class="{ 'segmented-control__btn--active': modelValue === opt.value }"
      :disabled="opt.disabled"
      @click="select(opt)"
    >
      <slot :option="opt" :active="modelValue === opt.value">
        {{ opt.label }}
      </slot>
    </button>
  </div>
</template>

<style scoped>
.segmented-control {
  display: inline-flex;
  gap: 2px;
  background: rgba(0, 0, 0, 0.04);
  padding: 2px;
  border-radius: 8px;
}

.segmented-control--pill {
  border-radius: 22px;
}

.segmented-control__btn {
  font-size: 12px;
  font-weight: 500;
  padding: 4px 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.segmented-control--pill .segmented-control__btn {
  border-radius: 20px;
}

.segmented-control__btn:hover:not(:disabled):not(.segmented-control__btn--active) {
  background: rgba(0, 0, 0, 0.04);
}

.segmented-control__btn--active {
  background: var(--accent);
  color: white;
  font-weight: 600;
}

.segmented-control__btn:disabled {
  opacity: 0.35;
  pointer-events: none;
}
</style>
