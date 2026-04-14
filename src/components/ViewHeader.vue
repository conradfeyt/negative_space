<template>
  <div class="view-header">
    <div class="view-header-top">
      <div>
        <h2>{{ title }}</h2>
        <p v-if="subtitle" class="text-muted">{{ subtitle }}</p>
      </div>
      <div v-if="$slots.actions" class="header-buttons">
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title: string;
  subtitle?: string;
}>();
</script>

<style scoped>
.header-buttons {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  flex-shrink: 0;
}
</style>
