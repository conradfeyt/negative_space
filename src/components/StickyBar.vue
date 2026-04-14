<script setup lang="ts">
/**
 * StickyBar — Sticky action bar with glass effect when scrolled.
 *
 * Uses an IntersectionObserver on a sentinel element to detect when the bar
 * has stuck to the top of the scroll container. Applies a frosted-glass
 * background with full-bleed padding when stuck.
 *
 * Slots:
 *   default — left side content (checkbox, summary text)
 *   actions — right side content (buttons)
 */
import { ref, watch, onMounted, onUnmounted } from "vue";

const sentinel = ref<HTMLElement | null>(null);
const isStuck = ref(false);
let observer: IntersectionObserver | null = null;

function connect() {
  observer?.disconnect();
  isStuck.value = false;
  if (sentinel.value) {
    observer = new IntersectionObserver(
      ([entry]) => { isStuck.value = !entry.isIntersecting; },
      { threshold: 0 }
    );
    observer.observe(sentinel.value);
  }
}

watch(sentinel, connect);
onMounted(connect);
onUnmounted(() => { observer?.disconnect(); });
</script>

<template>
  <div ref="sentinel" class="sticky-sentinel"></div>
  <div class="sticky-bar" :class="{ 'is-stuck': isStuck }">
    <div class="sticky-bar-left">
      <slot />
    </div>
    <div class="sticky-bar-actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.sticky-sentinel {
  height: 0;
  margin: 0;
  padding: 0;
}

.sticky-bar {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  margin-bottom: var(--sp-4);
  transition: background 0.2s, border-color 0.2s;
  border-bottom: 1px solid transparent;
}

.sticky-bar.is-stuck {
  margin-left: -40px;
  margin-right: -40px;
  width: calc(100% + 80px);
  padding: 10px 40px;
  background: rgba(235, 232, 238, 0.92);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom-color: rgba(0, 0, 0, 0.06);
}

.sticky-bar-left {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.sticky-bar-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}
</style>
