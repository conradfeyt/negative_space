<template>
  <div class="inline-alert" :class="`inline-alert--${variant}`">
    <span class="inline-alert-icon">
      <slot name="icon">
        <svg v-if="variant === 'warning'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="16" x2="12" y2="12"/>
          <line x1="12" y1="8" x2="12.01" y2="8"/>
        </svg>
      </slot>
    </span>
    <div class="inline-alert-body">
      <slot />
    </div>
    <button
      v-if="dismissible"
      class="inline-alert-dismiss"
      aria-label="Dismiss"
      @click="$emit('dismiss')"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  variant?: "info" | "warning";
  dismissible?: boolean;
}>(), {
  variant: "info",
  dismissible: false,
});

defineEmits<{ dismiss: [] }>();
</script>

<style scoped>
.inline-alert {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: var(--glass);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: var(--radius-lg);
  font-size: 12.5px;
  line-height: 1.45;
  margin-bottom: var(--sp-3);
}

.inline-alert--warning .inline-alert-icon {
  color: var(--orange, #f59e0b);
  opacity: 0.85;
}

.inline-alert--info .inline-alert-icon {
  color: var(--accent);
  opacity: 0.85;
}

.inline-alert-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
}

.inline-alert-body {
  flex: 1;
  min-width: 0;
  color: var(--text-primary);
}

.inline-alert-dismiss {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: none;
  color: var(--muted);
  cursor: pointer;
  border-radius: 50%;
  transition: background 0.15s, color 0.15s;
}

.inline-alert-dismiss:hover {
  background: rgba(0, 0, 0, 0.06);
  color: var(--text);
}
</style>
