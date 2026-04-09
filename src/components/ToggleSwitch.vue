<script setup lang="ts">
/**
 * ToggleSwitch — macOS-style toggle switch.
 *
 * Supports v-model, disabled state, and visible focus indicator.
 */
defineProps<{
  modelValue: boolean
  disabled?: boolean
}>();

defineEmits<{
  (e: "update:modelValue", value: boolean): void
}>();
</script>

<template>
  <label class="toggle" :class="{ 'toggle--disabled': disabled }">
    <input
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      @change="$emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span class="toggle-slider"></span>
  </label>
</template>

<style scoped>
.toggle {
  position: relative;
  display: inline-block;
  width: 37px;
  height: 16px;
  cursor: pointer;
  flex-shrink: 0;
}

.toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.18);
  border-radius: 10px;
  transition: background 0.2s ease, box-shadow 0.15s ease;
}

.toggle-slider::before {
  content: "";
  position: absolute;
  height: 12px;
  width: 21px;
  left: 2px;
  top: 2px;
  background: white;
  border-radius: 7px;
  transition: transform 0.2s ease;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15), 0 0 0 0.5px rgba(0, 0, 0, 0.04);
}

.toggle input:checked + .toggle-slider {
  background: var(--accent);
}

.toggle input:checked + .toggle-slider::before {
  transform: translateX(12px);
}

/* Focus indicator */
.toggle input:focus-visible + .toggle-slider {
  box-shadow: 0 0 0 3px var(--accent-light);
}

/* Disabled state */
.toggle--disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}
</style>
