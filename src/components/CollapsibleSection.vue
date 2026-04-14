<template>
  <div class="collapsible-section">
    <div
      class="collapsible-header"
      tabindex="0"
      role="button"
      :aria-expanded="expanded"
      @click="$emit('toggle')"
      @keydown.enter="$emit('toggle')"
      @keydown.space.prevent="$emit('toggle')"
    >
      <ChevronIcon :expanded="expanded" :variant="variant" :size="size" />
      <slot name="header">
        <span class="collapsible-title">{{ title }}</span>
      </slot>
    </div>
    <div v-if="expanded" class="collapsible-body">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import ChevronIcon from "./ChevronIcon.vue";

withDefaults(defineProps<{
  expanded?: boolean;
  title?: string;
  variant?: "stroke" | "filled";
  size?: number;
}>(), {
  expanded: false,
  title: "",
  variant: "stroke",
  size: 12,
});

defineEmits<{ toggle: [] }>();
</script>
