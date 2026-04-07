<script setup lang="ts">
/**
 * Checkbox — Custom styled checkbox.
 *
 * Visual state is driven entirely by props (not native input :checked).
 * A hidden native input handles click → label → change event flow.
 * watchEffect imperatively syncs the native input's .checked and
 * .indeterminate properties to match props on every render.
 */
import { computed, ref, onMounted, onUpdated } from "vue";

const props = defineProps<{
  modelValue?: boolean
  isOn?: boolean
  disabled?: boolean
  indeterminate?: boolean
}>();

defineEmits<{
  (e: "update:modelValue", value: boolean): void
  (e: "change", event: Event): void
}>();

const uid = `cb-${Math.random().toString(36).slice(2, 9)}`;
const inputRef = ref<HTMLInputElement | null>(null);
const isChecked = computed(() => {
  if (props.isOn !== undefined) return props.isOn;
  if (props.modelValue !== undefined) return props.modelValue;
  return false;
});

function syncInput() {
  if (inputRef.value) {
    inputRef.value.checked = isChecked.value;
    inputRef.value.indeterminate = props.indeterminate ?? false;
  }
}

onMounted(syncInput);
onUpdated(syncInput);
</script>

<template>
  <div class="checkbox" @click.stop>
    <input
      :id="uid"
      ref="inputRef"
      type="checkbox"
      :disabled="disabled"
      @change="$emit('change', $event); $emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <label
      :for="uid"
      :class="{
        'is-checked': isChecked && !indeterminate,
        'is-indeterminate': indeterminate,
        'is-disabled': disabled,
      }"
    >
      <span class="checkbox__box">
        <svg class="checkbox__check" width="12" height="10" viewBox="0 0 12 10">
          <polyline points="1.5 6 4.5 9 10.5 1" />
        </svg>
        <svg class="checkbox__dash" width="10" height="2" viewBox="0 0 10 2">
          <line x1="0" y1="1" x2="10" y2="1" />
        </svg>
      </span>
      <span v-if="$slots.default" class="checkbox__label"><slot /></span>
    </label>
  </div>
</template>

<style scoped>
.checkbox {
  display: inline-block;
}

.checkbox input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
  pointer-events: none;
}

.checkbox label {
  -webkit-user-select: none;
  user-select: none;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 6px;
  overflow: hidden;
  transition: background 0.2s ease;
  display: inline-flex;
  align-items: center;
}

.checkbox label:hover {
  background: rgba(2, 117, 244, 0.06);
}

.checkbox label:hover .checkbox__box {
  border-color: var(--accent);
}

.checkbox__box {
  position: relative;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  flex-shrink: 0;
  background: rgba(0, 0, 0, 0.08);
  border: 1px solid rgba(0, 0, 0, 0.15);
  transition: all 0.2s ease;
  box-shadow: 0 1px 1px rgba(0, 16, 75, 0.05);
}

/* Check mark — hidden by default */
.checkbox__check {
  position: absolute;
  top: 3px;
  left: 2px;
  fill: none;
  stroke: #fff;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-dasharray: 16px;
  stroke-dashoffset: 16px;
  transition: stroke-dashoffset 0.3s ease 0.1s;
}

/* Dash — hidden by default */
.checkbox__dash {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  stroke: #fff;
  stroke-width: 2.5;
  stroke-linecap: round;
  opacity: 0;
  transition: opacity 0.15s ease;
}

/* Checked state — driven by class on label, not :checked pseudo-class */
.is-checked .checkbox__box {
  background: var(--accent);
  border-color: var(--accent);
  animation: checkbox-wave 0.3s ease;
}

.is-checked .checkbox__check {
  stroke-dashoffset: 0;
}

/* Indeterminate state */
.is-indeterminate .checkbox__box {
  background: var(--accent);
  border-color: var(--accent);
}

.is-indeterminate .checkbox__check {
  stroke-dashoffset: 16px;
}

.is-indeterminate .checkbox__dash {
  opacity: 1;
}

/* Disabled */
.is-disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.is-disabled:hover {
  background: transparent;
}

.checkbox__label {
  padding-left: 8px;
  line-height: 18px;
  font-size: 13px;
}

@keyframes checkbox-wave {
  50% { transform: scale(0.9); }
}
</style>
