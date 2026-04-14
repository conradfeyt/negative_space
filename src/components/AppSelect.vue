<!--
  AppSelect — Custom dropdown select with icon support.

  Replaces native <select> with a fully styled dropdown that supports:
  - Icons alongside option labels (in trigger and dropdown)
  - Glassmorphism dropdown matching the design system
  - Compact mode for inline ScanBar usage
  - SF Pro font throughout (no WebKit serif fallback)
-->
<script setup lang="ts">
import { ref, computed, onUnmounted, nextTick } from "vue";

export interface SelectOption {
  value: string | number;
  label: string;
  icon?: string;
}

const props = withDefaults(defineProps<{
  modelValue: string | number;
  options: SelectOption[];
  placeholder?: string;
  compact?: boolean;
  title?: string;
}>(), {
  placeholder: "Select…",
  compact: false,
});

const emit = defineEmits<{
  "update:modelValue": [value: string | number];
}>();

const isOpen = ref(false);
const triggerEl = ref<HTMLElement | null>(null);
const dropdownStyle = ref<Record<string, string>>({});

const selected = computed(() =>
  props.options.find((o) => o.value === props.modelValue)
);

function toggle() {
  if (isOpen.value) {
    close();
  } else {
    open();
  }
}

function open() {
  isOpen.value = true;
  nextTick(positionDropdown);
  setTimeout(() => {
    document.addEventListener("click", onClickOutside);
    document.addEventListener("keydown", onKeydown);
  }, 0);
}

function close() {
  isOpen.value = false;
  document.removeEventListener("click", onClickOutside);
  document.removeEventListener("keydown", onKeydown);
}

function pick(value: string | number) {
  emit("update:modelValue", value);
  close();
}

function onClickOutside(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (triggerEl.value?.contains(target)) return;
  const dropdown = document.querySelector(".app-select-dropdown");
  if (dropdown?.contains(target)) return;
  close();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") close();
}

function positionDropdown() {
  if (!triggerEl.value) return;
  const rect = triggerEl.value.getBoundingClientRect();
  const spaceBelow = window.innerHeight - rect.bottom;
  const dropUp = spaceBelow < 200 && rect.top > 200;

  dropdownStyle.value = {
    left: `${rect.left}px`,
    minWidth: `${rect.width}px`,
    ...(dropUp
      ? { bottom: `${window.innerHeight - rect.top + 4}px` }
      : { top: `${rect.bottom + 4}px` }),
  };
}

onUnmounted(() => {
  document.removeEventListener("click", onClickOutside);
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div
    ref="triggerEl"
    class="app-select"
    :class="{ 'app-select--compact': compact, 'app-select--open': isOpen }"
    :title="title"
    tabindex="0"
    role="combobox"
    :aria-expanded="isOpen"
    @click.stop="toggle"
    @keydown.enter.prevent="toggle"
    @keydown.space.prevent="toggle"
  >
    <img v-if="selected?.icon" :src="selected.icon" alt="" class="app-select-icon" />
    <span class="app-select-label">{{ selected?.label ?? placeholder }}</span>
    <svg class="app-select-chevron" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M3 4.5L6 7.5L9 4.5"/>
    </svg>
  </div>

  <Teleport to="body">
    <div
      v-if="isOpen"
      class="app-select-dropdown"
      :style="dropdownStyle"
      @click.stop
    >
      <button
        v-for="opt in options"
        :key="String(opt.value)"
        class="app-select-option"
        :class="{ 'app-select-option--selected': opt.value === modelValue }"
        @click="pick(opt.value)"
      >
        <img v-if="opt.icon" :src="opt.icon" alt="" class="app-select-option-icon" />
        <span class="app-select-option-label">{{ opt.label }}</span>
        <svg v-if="opt.value === modelValue" class="app-select-check" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="2 6 5 9 10 3"/>
        </svg>
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
/* ── Trigger ─────────────────────────────────────── */
.app-select {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  padding-right: 28px;
  position: relative;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 8px);
  background: rgba(255, 255, 255, 0.5);
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  cursor: pointer;
  outline: none;
  user-select: none;
  transition: border-color 0.2s, background-color 0.2s, box-shadow 0.2s;
}

.app-select:hover {
  background-color: rgba(255, 255, 255, 0.65);
}

.app-select:focus-visible {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-light);
}

.app-select--open {
  border-color: var(--accent);
  background-color: rgba(255, 255, 255, 0.7);
}

/* Compact mode — for ScanBar inline usage */
.app-select--compact {
  border: none;
  background: transparent;
  padding: 4px 22px 4px 2px;
  border-radius: 0;
}

.app-select--compact:hover {
  background: transparent;
}

.app-select--compact:focus-visible {
  box-shadow: none;
  border: none;
}

.app-select--compact.app-select--open {
  background: transparent;
  border: none;
}

.app-select-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.app-select-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app-select-chevron {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  opacity: 0.5;
  flex-shrink: 0;
  transition: transform 0.15s;
  color: #5C6070;
}

.app-select--compact .app-select-chevron {
  right: 2px;
}

.app-select--open .app-select-chevron {
  transform: translateY(-50%) rotate(180deg);
}
</style>

<style>
/* ── Dropdown (teleported to body, needs global styles) ── */
.app-select-dropdown {
  position: fixed;
  z-index: 9999;
  padding: 4px;
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15), 0 2px 6px rgba(0, 0, 0, 0.06);
  -webkit-app-region: no-drag;
}

.app-select-option {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  background: transparent;
  font-family: inherit;
  font-size: 13px;
  font-weight: 450;
  color: var(--text);
  cursor: pointer;
  border-radius: 8px;
  text-align: left;
  transition: background 0.1s;
}

.app-select-option:hover {
  background: rgba(0, 0, 0, 0.04);
}

.app-select-option--selected {
  font-weight: 550;
}

.app-select-option-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.app-select-option-label {
  flex: 1;
}

.app-select-check {
  flex-shrink: 0;
  color: var(--accent, #0088FF);
}
</style>
