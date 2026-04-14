<!--
  FilterPill — Pill-shaped filter icon button with optional badge.

  Provides: white pill container, icon button with blue hover/active states,
  optional badge count, and a default slot for extra content (e.g. filter chips).

  Used by KindFilterBar (Duplicates) and SensitiveContent label filters.
-->
<script setup lang="ts">
defineProps<{
  icon?: string;
  active?: boolean;
  badge?: number | null;
}>();

defineEmits<{
  click: [];
}>();
</script>

<template>
  <div class="filter-pill">
    <button
      class="filter-pill-btn"
      :class="{ 'filter-pill-btn--active': active }"
      @click="$emit('click')"
    >
      <img v-if="icon" :src="icon" alt="Filter" width="16" height="16" />
    </button>
    <slot />
    <span v-if="badge != null && badge > 0" class="filter-pill-badge">{{ badge }}</span>
  </div>
</template>

<style scoped>
.filter-pill {
  display: flex;
  align-items: center;
  position: relative;
  background: rgba(255, 255, 255, 0.5);
  border: 1px solid var(--border);
  border-radius: 22px;
  padding: 3px;
  gap: 4px;
  flex-shrink: 0;
}

.filter-pill-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 14px;
  background: transparent;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s;
}

.filter-pill-btn:hover {
  background: rgba(0, 136, 255, 0.08);
}

.filter-pill-btn--active {
  background: rgba(0, 136, 255, 0.1);
}

.filter-pill-btn--active img {
  filter: brightness(0) saturate(100%) invert(40%) sepia(90%) saturate(1500%) hue-rotate(190deg) brightness(100%);
}

.filter-pill-badge {
  position: absolute;
  top: -6px;
  right: -6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 14px;
  height: 14px;
  border-radius: 7px;
  background: var(--accent);
  color: #fff;
  font-size: 9px;
  font-weight: 600;
  padding: 0 3px;
}
</style>
