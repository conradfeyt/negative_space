<template>
  <div class="loading-state">
    <span :class="spinnerClass"></span>
    <span :class="{ 'text-muted': muted }"><slot>{{ message }}</slot></span>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  message?: string;
  small?: boolean;
  muted?: boolean;
}>(), {
  message: "Loading…",
  small: false,
  muted: false,
});

const spinnerClass = "spinner";
</script>
