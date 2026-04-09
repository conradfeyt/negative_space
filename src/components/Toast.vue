<script setup lang="ts">
/**
 * Toast — Global notification overlay.
 *
 * Renders a stack of auto-dismissing toasts fixed to the bottom-right
 * of the content panel. Mounted once in App.vue.
 */
import { watch } from "vue";
import { toasts, dismissToast } from "../stores/toastStore";
import type { ToastMessage } from "../stores/toastStore";

const AUTO_DISMISS_MS = 60_000;

function scheduleAutoDismiss(toast: ToastMessage) {
  setTimeout(() => dismissToast(toast.id), AUTO_DISMISS_MS);
}

watch(toasts, (current, previous) => {
  const prevIds = new Set((previous ?? []).map(t => t.id));
  for (const t of current) {
    if (!prevIds.has(t.id)) scheduleAutoDismiss(t);
  }
}, { deep: true });
</script>

<template>
  <Teleport to="body">
    <div v-if="toasts.length > 0" class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="`toast--${toast.type}`"
        >
          <span class="toast-message">{{ toast.message }}</span>
          <button class="toast-close" @click="dismissToast(toast.id)">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <path d="M2 2L12 12M12 2L2 12"/>
            </svg>
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 400px;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: 12px;
  font-size: 13px;
  font-weight: 500;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12), 0 1px 4px rgba(0, 0, 0, 0.08);
  pointer-events: auto;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.toast--success {
  background: rgba(3, 137, 51, 0.9);
  color: white;
}

.toast--error {
  background: rgba(255, 57, 60, 0.9);
  color: white;
}

.toast--info {
  background: rgba(0, 136, 255, 0.9);
  color: white;
}

.toast-message {
  flex: 1;
  line-height: 1.4;
}

.toast-close {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  transition: color 0.15s;
  flex-shrink: 0;
}

.toast-close:hover {
  color: white;
}

/* Transition animations */
.toast-enter-active {
  transition: all 0.3s ease;
}

.toast-leave-active {
  transition: all 0.2s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(40px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(40px);
}
</style>
