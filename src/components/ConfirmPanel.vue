<template>
  <div class="confirm-block">
    <p class="confirm-text">
      <slot />
    </p>
    <div class="confirm-actions">
      <button class="btn-secondary" @click="$emit('cancel')">
        Cancel
      </button>
      <button
        :class="danger ? 'btn-danger' : 'btn-primary'"
        :disabled="loading"
        @click="$emit('confirm')"
      >
        <span v-if="loading" class="spinner-sm"></span>
        {{ loading && loadingLabel ? loadingLabel : confirmLabel }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  confirmLabel?: string;
  loadingLabel?: string;
  danger?: boolean;
  loading?: boolean;
}>(), {
  confirmLabel: "Confirm",
  loadingLabel: "",
  danger: true,
  loading: false,
});

defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<style scoped>
.confirm-block {
  text-align: center;
  padding: var(--sp-2) 0;
}

.confirm-text {
  font-size: 14px;
  color: var(--text);
  margin-bottom: var(--sp-4);
  line-height: 1.6;
}

.confirm-actions {
  display: flex;
  justify-content: center;
  gap: 10px;
}

.confirm-actions .btn-danger,
.confirm-actions .btn-primary {
  display: flex;
  align-items: center;
  gap: 6px;
}
</style>
