<template>
  <div
    class="progress-track"
    :class="sizeClass"
    role="progressbar"
    :aria-valuenow="percent"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div
      class="progress-fill"
      :class="`progress-fill--${variant}`"
      :style="{ width: percent + '%' }"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(defineProps<{
  percent: number;
  variant?: "accent" | "success" | "warning" | "danger";
  size?: "default" | "thin" | "thick";
}>(), {
  variant: "accent",
  size: "default",
});

const sizeClass = computed(() =>
  props.size === "default" ? "" : `progress-track--${props.size}`
);
</script>
