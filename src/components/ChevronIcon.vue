<template>
  <span
    class="expand-chevron"
    :class="{ expanded }"
  >
    <svg
      v-if="variant === 'filled'"
      :width="size" :height="size"
      viewBox="0 0 16 16"
      fill="currentColor"
    ><path d="M6 3l5 5-5 5V3z"/></svg>
    <svg
      v-else
      :width="size" :height="size"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
    ><path d="M4 2 L8 6 L4 10"/></svg>
  </span>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  expanded?: boolean;
  size?: number;
  variant?: "stroke" | "filled";
}>(), {
  expanded: false,
  size: 12,
  variant: "stroke",
});
</script>
