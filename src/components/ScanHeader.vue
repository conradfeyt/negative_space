<!--
  ScanHeader — shared header for scan views.

  Provides: title + subtitle, ScanBar with custom controls slot,
  folder picker with multi-location support, and location chip bar.
-->
<script setup lang="ts">
import ScanBar from "./ScanBar.vue";
import { iconForPath, displayForPath } from "../composables/useScanFolder";

defineProps<{
  title: string;
  subtitle: string;
  scanning: boolean;
  scanLabel?: string;
  disabled?: boolean;
  /** Flat list of scan locations. Empty = default home scan. */
  folders: string[];
}>();

defineEmits<{
  scan: [];
  addFolder: [];
  removeFolder: [index: number];
}>();
</script>

<template>
  <div class="scan-header">
    <div class="scan-header-top">
      <div class="scan-header-title">
        <h2>{{ title }}</h2>
        <p class="text-muted">{{ subtitle }}</p>
      </div>
      <ScanBar :scanning="scanning" :scan-label="scanLabel" :disabled="disabled" @scan="$emit('scan')">
        <slot />
        <span class="scan-bar-divider"></span>
        <button class="scan-bar-folder" @click="$emit('addFolder')" :title="folders.length > 1 ? `${folders.length} scan locations` : 'Choose scan folder'">
          <img v-if="folders.length === 1" :src="iconForPath(folders[0])" alt="" class="scan-bar-folder-icon" />
          <svg v-else width="16" height="16" viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
          <span>{{ folders.length > 1 ? `${folders.length} Locations` : folders.length === 1 ? displayForPath(folders[0]) : 'Home' }}</span>
        </button>
      </ScanBar>
    </div>

    <!-- Location chips -->
    <div class="scan-folders-bar">
      <button v-if="folders.length < 10" class="folder-chip add-chip" @click="$emit('addFolder')">+ Add location</button>
      <template v-if="folders.length >= 2">
        <div class="folder-chip" v-for="(folder, i) in folders" :key="folder" :title="folder">
          <img :src="iconForPath(folder)" alt="" class="folder-chip-icon" />
          <span class="folder-chip-path">{{ displayForPath(folder) }}</span>
          <button class="folder-chip-remove" @click="$emit('removeFolder', i)" title="Remove">&times;</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.scan-header {
  margin-bottom: var(--sp-6);
}

.scan-header-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.scan-header-title h2 {
  font-size: 28px;
  font-weight: 700;
  letter-spacing: -0.5px;
  line-height: 1.15;
}

.scan-header-title p {
  margin-top: 6px;
  font-size: 14px;
}

/* ScanBar folder button */
.scan-bar-folder {
  display: flex;
  align-items: center;
  gap: 5px;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 14px;
  transition: background 0.15s;
  white-space: nowrap;
}

.scan-bar-folder:hover {
  background: rgba(0, 0, 0, 0.06);
}

.scan-bar-folder-icon {
  height: 16px;
  width: auto;
  flex-shrink: 0;
}

.scan-bar-divider {
  width: 1px;
  height: 18px;
  background: var(--border);
  margin: 0 4px;
  flex-shrink: 0;
}

/* Location chip bar */
.scan-folders-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}

.folder-chip {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px 3px 10px;
  background: var(--glass);
  border: 1px solid var(--border);
  border-radius: 14px;
  font-size: 12px;
  color: var(--text);
  max-width: 280px;
}

.folder-chip-icon {
  height: 14px;
  width: auto;
  flex-shrink: 0;
}

.folder-chip-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.folder-chip-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  min-width: 18px;
  min-height: 18px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  border-radius: 50%;
  flex-shrink: 0;
  padding: 0;
  transition: background 0.15s, color 0.15s;
}

.folder-chip-remove:hover {
  background: rgba(217, 75, 75, 0.15);
  color: var(--danger, #d94b4b);
}

.add-chip {
  border-style: dashed;
  cursor: pointer;
  padding: 3px 10px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  transition: border-color 0.15s, color 0.15s;
}

.add-chip:hover {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
