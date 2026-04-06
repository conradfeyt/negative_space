<script setup lang="ts">
/**
 * Toast — auto-dismissing notification with close button.
 * Usage: <Toast v-if="msg" :message="msg" :type="'success'" @dismiss="msg = ''" />
 */
import { onMounted, onUnmounted } from "vue";

const props = withDefaults(defineProps<{
  message: string;
  type?: "success" | "error" | "info";
  duration?: number;
}>(), {
  type: "success",
  duration: 4000,
});

const emit = defineEmits<{ dismiss: [] }>();

let timer: ReturnType<typeof setTimeout> | null = null;

onMounted(() => {
  if (props.duration > 0) {
    timer = setTimeout(() => emit("dismiss"), props.duration);
  }
});

onUnmounted(() => {
  if (timer) clearTimeout(timer);
});
</script>

<template>
  <div :class="['toast', `toast--${type}`]" @click="emit('dismiss')">
    <span class="toast-message">{{ message }}</span>
    <button class="btn-close" @click.stop="emit('dismiss')">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <path d="M18 6L6 18M6 6l12 12"/>
      </svg>
    </button>
  </div>
</template>

<style scoped>
.toast {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  animation: toastIn 0.25s ease;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08), 0 1px 3px rgba(0, 0, 0, 0.06);
}

.toast--success {
  background: var(--success-tint);
  border: 1px solid rgba(30, 142, 90, 0.15);
  color: var(--success-text);
}

.toast--error {
  background: var(--danger-tint);
  border: 1px solid rgba(217, 75, 75, 0.15);
  color: var(--danger-text);
}

.toast--info {
  background: var(--info-tint);
  border: 1px solid rgba(20, 138, 160, 0.15);
  color: var(--info-text);
}

.toast-message {
  flex: 1;
  line-height: 1.4;
}


@keyframes toastIn {
  from { opacity: 0; transform: translateY(-8px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
