<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue';

const props = defineProps<{
  visible: boolean;
  title: string;
  wide?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close');
}

watch(
  () => props.visible,
  (v) => {
    if (v) {
      document.addEventListener('keydown', onKeydown);
    } else {
      document.removeEventListener('keydown', onKeydown);
    }
  },
  { immediate: true },
);

onMounted(() => {
  if (props.visible) document.addEventListener('keydown', onKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click.self="emit('close')">
      <div class="modal-dialog" :class="{ wide }">
        <div v-if="$slots.icon" class="modal-icon">
          <slot name="icon" />
        </div>
        <h3>{{ title }}</h3>
        <div class="modal-body">
          <slot />
        </div>
        <div class="modal-actions">
          <slot name="actions" />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.modal-dialog {
  background: rgba(255, 255, 255, 0.82);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border-radius: var(--radius-lg);
  border: 1px solid rgba(255, 255, 255, 0.5);
  padding: 32px;
  max-width: 360px;
  text-align: center;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.15), 0 4px 12px rgba(0, 0, 0, 0.08);
}

.modal-dialog.wide {
  width: 740px;
  max-width: 740px;
  text-align: left;
  height: 70vh;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-dialog.wide .modal-body {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 4px 0;
}

.modal-dialog.wide h3 {
  text-align: left;
}

.modal-dialog.wide :deep(p) {
  text-align: left;
}

.modal-dialog.wide .modal-actions {
  justify-content: flex-end;
  padding-top: 16px;
  border-top: 1px solid var(--border-divider);
  margin-top: 8px;
}

.modal-icon {
  margin-bottom: 16px;
}

.modal-dialog h3 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 8px;
}

.modal-dialog :deep(p) {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: 20px;
}

.modal-actions {
  display: flex;
  justify-content: center;
  gap: 8px;
}
</style>
