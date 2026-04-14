<template>
  <div class="kind-filter">
    <FilterPill :icon="icon" :active="modelValue.length > 0" @click="open = !open">
      <button
        v-for="key in modelValue"
        :key="key"
        class="filter-chip"
        :title="`Remove ${labelFor(key)} filter`"
        @click="remove(key)"
      >
        {{ labelFor(key) }}
        <svg width="10" height="10" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 3l6 6M9 3l-6 6"/></svg>
      </button>
    </FilterPill>
    <template v-if="open">
      <div class="filter-backdrop" @click="open = false" />
      <div class="filter-dropdown">
        <label
          v-for="opt in options"
          :key="opt.key"
          class="filter-option"
          :class="{ 'filter-option--disabled': counts && !counts[opt.key] }"
        >
          <Checkbox
            :model-value="modelValue.includes(opt.key)"
            :disabled="!!counts && !counts[opt.key]"
            @change="toggle(opt.key)"
          />
          <span class="filter-option-label">{{ opt.label }}</span>
          <span v-if="counts && counts[opt.key]" class="filter-option-count text-muted">{{ counts[opt.key] }}</span>
        </label>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import Checkbox from "./Checkbox.vue";
import FilterPill from "./FilterPill.vue";

const props = defineProps<{
  options: Array<{ key: string; label: string }>;
  modelValue: string[];
  counts?: Record<string, number>;
  icon?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string[]];
}>();

const open = ref(false);

function labelFor(key: string): string {
  return props.options.find((o) => o.key === key)?.label ?? key;
}

function toggle(key: string) {
  const next = props.modelValue.includes(key)
    ? props.modelValue.filter((k) => k !== key)
    : [...props.modelValue, key];
  emit("update:modelValue", next);
}

function remove(key: string) {
  emit("update:modelValue", props.modelValue.filter((k) => k !== key));
}
</script>

<style scoped>
.kind-filter {
  position: relative;
}

.filter-chip {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px 4px 10px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
  background: rgba(0, 0, 0, 0.05);
  border: none;
  border-radius: 12px;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s;
}

.filter-chip:hover {
  background: rgba(0, 0, 0, 0.1);
}

.filter-chip svg {
  opacity: 0.4;
  flex-shrink: 0;
}

.filter-chip:hover svg {
  opacity: 0.7;
}

.filter-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9;
}

.filter-dropdown {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  z-index: 10;
  min-width: 180px;
  padding: 6px;
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12), 0 2px 6px rgba(0, 0, 0, 0.06);
}

.filter-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.1s;
}

.filter-option:hover {
  background: rgba(0, 0, 0, 0.04);
}

.filter-option--disabled {
  opacity: 0.35;
  pointer-events: none;
}

.filter-option-label {
  flex: 1;
  font-size: 13px;
  font-weight: 450;
  color: var(--text);
}

.filter-option-count {
  font-size: 11px;
  font-weight: 500;
}
</style>
