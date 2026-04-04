<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { hasFullDiskAccess, checkFullDiskAccess } from "../stores/scanStore";

const emit = defineEmits<{
  "fda-granted": [];
}>();

defineProps<{
  title?: string;
  text?: string;
}>();

async function openFdaSettings() {
  try {
    await invoke("open_full_disk_access_settings");
  } catch (_) {
    /* non-critical */
  }
}

async function recheckFda() {
  const hadFda = hasFullDiskAccess.value;
  await checkFullDiskAccess();
  if (!hadFda && hasFullDiskAccess.value === true) {
    emit("fda-granted");
  }
}

defineExpose({ hasFda: hasFullDiskAccess });
</script>

<template>
  <div v-if="hasFullDiskAccess === false" class="fda-warning-banner">
    <span class="fda-warning-dot"></span>
    <div class="fda-warning-body">
      <div class="fda-warning-title">{{ title ?? "Limited scan -- Full Disk Access required" }}</div>
      <div v-if="text" class="fda-warning-text">{{ text }}</div>
      <slot v-else name="text" />
      <div class="fda-warning-actions">
        <button class="btn-fda btn-fda-primary" @click="openFdaSettings">Open System Settings</button>
        <button class="btn-fda btn-fda-secondary" @click="recheckFda">Re-check</button>
      </div>
    </div>
  </div>
</template>
