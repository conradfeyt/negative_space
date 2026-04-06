<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <div class="card empty-state-rich">
    <div class="empty-icon">
      <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
        <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
        <line x1="9" y1="14" x2="15" y2="14"/>
      </svg>
    </div>
    <p class="empty-title">{{ title }}</p>
    <p v-if="description" class="empty-desc text-muted">{{ description }}</p>
    <slot />
  </div>
</template>

<style scoped>
.empty-state-rich {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 64px 32px;
}

.empty-icon {
  color: var(--muted);
  opacity: 0.4;
  margin-bottom: var(--sp-4);
}

.empty-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: var(--sp-2);
}

.empty-desc {
  font-size: 13px;
  max-width: 380px;
  line-height: 1.6;
}
</style>
