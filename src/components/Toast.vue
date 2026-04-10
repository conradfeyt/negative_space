<script setup lang="ts">
/**
 * Toast — Global notification overlay.
 *
 * Renders a stack of auto-dismissing toasts fixed to the bottom-right.
 * Per-toast close on hover (top-left). "Clear all" button when 2+ toasts.
 * Source icons show which page triggered the notification.
 * Mounted once in App.vue via Teleport.
 */
import { watch, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import { toasts, dismissToast, clearAllToasts } from "../stores/toastStore";
import type { ToastMessage } from "../stores/toastStore";

const router = useRouter();
const activeToastId = ref<number | null>(null);

const sourceRoutes: Record<string, string> = {
  "Caches": "caches",
  "Logs": "logs",
  "Large Files": "large-files",
  "Apps": "apps",
  "Browsers": "browsers",
  "Duplicates": "duplicates",
  "Trash": "trash",
  "Docker": "docker",
  "Security": "security",
  "Vault": "vault",
  "CPU": "cpu",
  "Space Map": "space-map",
  "Packages": "packages",
  "Maintenance": "maintenance",
};

function handleToastClick(toast: ToastMessage) {
  activeToastId.value = toast.id;
  if (toast.source && sourceRoutes[toast.source]) {
    router.push({ name: sourceRoutes[toast.source] });
  }
}

// Swipe-to-dismiss
const swipeState = ref<{ id: number; startX: number; currentX: number } | null>(null);

function onPointerDown(e: PointerEvent, toast: ToastMessage) {
  swipeState.value = { id: toast.id, startX: e.clientX, currentX: e.clientX };
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}

function onPointerMove(e: PointerEvent) {
  if (!swipeState.value) return;
  const dx = e.clientX - swipeState.value.startX;
  // Only allow swiping right
  swipeState.value.currentX = e.clientX;
  const el = e.currentTarget as HTMLElement;
  if (dx > 0) {
    el.style.transform = `translateX(${dx}px)`;
    el.style.opacity = `${Math.max(0, 1 - dx / 200)}`;
  }
}

function onPointerUp(e: PointerEvent) {
  if (!swipeState.value) return;
  const dx = e.clientX - swipeState.value.startX;
  const id = swipeState.value.id;
  const el = e.currentTarget as HTMLElement;
  swipeState.value = null;

  if (dx > 80) {
    // Dismiss — animate out
    el.style.transition = 'transform 0.2s ease, opacity 0.2s ease';
    el.style.transform = 'translateX(300px)';
    el.style.opacity = '0';
    setTimeout(() => dismissToast(id), 200);
  } else {
    // Snap back
    el.style.transition = 'transform 0.2s ease, opacity 0.2s ease';
    el.style.transform = '';
    el.style.opacity = '';
    setTimeout(() => {
      el.style.transition = '';
    }, 200);
  }
}

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

// Source icons — loaded once via SF Symbols
const sourceIcons = ref<Record<string, string>>({});
const iconDefs: Record<string, { name: string; mode: string }> = {
  "Caches":       { name: "archivebox", mode: "sf" },
  "Logs":         { name: "doc.text", mode: "sf" },
  "Large Files":  { name: "doc", mode: "sf" },
  "Apps":         { name: "square.grid.2x2", mode: "sf" },
  "Browsers":     { name: "globe", mode: "sf" },
  "Duplicates":   { name: "doc.on.doc", mode: "sf" },
  "Trash":        { name: "trash", mode: "sf" },
  "Docker":       { name: "shippingbox", mode: "sf" },
  "Security":     { name: "shield", mode: "sf" },
  "Vault":        { name: "lock.shield", mode: "sf" },
  "CPU":          { name: "cpu", mode: "sf" },
  "Space Map":    { name: "chart.pie", mode: "sf" },
  "Packages":     { name: "shippingbox", mode: "sf" },
  "Maintenance":  { name: "wrench", mode: "sf" },
};

onMounted(async () => {
  for (const [key, def] of Object.entries(iconDefs)) {
    try {
      const b64 = await invoke<string>("render_sf_symbol", { name: def.name, size: 24, mode: def.mode, style: "plain" });
      if (b64) sourceIcons.value[key] = b64;
    } catch { /* non-critical */ }
  }
});
</script>

<template>
  <Teleport to="body">
    <div v-if="toasts.length > 0" class="toast-container">
      <!-- Clear all button -->
      <button v-if="toasts.length > 1" class="toast-clear-all" @click="clearAllToasts">
        <svg width="10" height="10" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 3L11 11M11 3L3 11"/></svg>
        <span class="toast-clear-label">Clear all</span>
      </button>

      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="[`toast--${toast.type}`, { 'toast--active': activeToastId === toast.id, 'toast--clickable': !!toast.source }]"
          @click="handleToastClick(toast)"
          @pointerdown="onPointerDown($event, toast)"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
        >
          <!-- Per-toast dismiss (top-left, hover only) -->
          <button class="toast-dismiss" @click.stop="dismissToast(toast.id)">
            <svg width="8" height="8" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M3 3L11 11M11 3L3 11"/></svg>
          </button>

          <!-- Source icon -->
          <span v-if="toast.source && sourceIcons[toast.source]" class="toast-source-wrap">
            <img :src="sourceIcons[toast.source]" alt="" class="toast-source-icon" />
          </span>

          <!-- Message -->
          <span class="toast-message">{{ toast.message }}</span>
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
  align-items: flex-end;
  gap: 8px;
  max-width: 420px;
  pointer-events: none;
}

/* Clear all button */
.toast-clear-all {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0;
  min-width: 28px;
  height: 28px;
  padding: 0;
  border: 0.5px solid rgba(255, 255, 255, 0.4);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.5);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  color: rgba(0, 0, 0, 0.45);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  pointer-events: auto;
  transition: background 0.15s, color 0.15s, padding 0.2s, gap 0.2s;
  overflow: hidden;
}

.toast-clear-label {
  max-width: 0;
  overflow: hidden;
  white-space: nowrap;
  transition: max-width 0.2s ease, opacity 0.15s;
  opacity: 0;
}

.toast-clear-all:hover {
  background: rgba(255, 255, 255, 0.65);
  color: rgba(0, 0, 0, 0.6);
  gap: 5px;
  padding: 0 12px 0 8px;
}

.toast-clear-all:hover .toast-clear-label {
  max-width: 60px;
  opacity: 1;
}

/* Individual toast */
.toast {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 18px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 500;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08), 0 1px 4px rgba(0, 0, 0, 0.04);
  pointer-events: auto;
  backdrop-filter: blur(40px) saturate(1.6);
  -webkit-backdrop-filter: blur(40px) saturate(1.6);
  border: 0.5px solid rgba(255, 255, 255, 0.3);
  width: 100%;
  touch-action: pan-y;
}

.toast--success {
  background: rgba(3, 137, 51, 0.35);
  color: white;
}

.toast--error {
  background: rgba(255, 57, 60, 0.35);
  color: white;
}

.toast--info {
  background: rgba(0, 136, 255, 0.35);
  color: white;
}

.toast--clickable {
  cursor: pointer;
}

.toast--active {
  transform: scale(1.03);
  box-shadow: 0 6px 32px rgba(0, 0, 0, 0.12), 0 2px 8px rgba(0, 0, 0, 0.06);
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.toast--clickable:active {
  transform: scale(0.98);
}

/* Per-toast dismiss (top-left, hover only) */
.toast-dismiss {
  position: absolute;
  top: -5px;
  left: -5px;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.5);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 0.5px solid rgba(255, 255, 255, 0.3);
  color: rgba(0, 0, 0, 0.4);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s;
  padding: 0;
}

.toast:hover .toast-dismiss {
  opacity: 1;
}

.toast-dismiss:hover {
  background: rgba(255, 255, 255, 0.7);
  color: rgba(0, 0, 0, 0.6);
}

/* Source icon */
.toast-source-wrap {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.15);
  flex-shrink: 0;
}

.toast-source-icon {
  width: 16px;
  height: 16px;
  filter: brightness(0) invert(1);
}

.toast-message {
  flex: 1;
  line-height: 1.4;
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
