<script setup lang="ts">
/**
 * FileRow — Presentational row for a single file in the LargeFiles view.
 *
 * Renders: file icon + optional protection shield, name + path,
 * sparse/safety badges, size + time, reveal button, and checkbox.
 * Emits toggle/reveal/unprotect — all data logic lives in the parent.
 */
import type { FileInfo } from "../types";
import { formatSize } from "../utils";
import Checkbox from "./Checkbox.vue";

defineProps<{
  file: FileInfo
  selected: boolean
  isProtected: boolean
  isLocked: boolean
  fileIcon: string
  safetyLabel: string
  safetyClass: string
  safetyTooltip: string
  isSparse: boolean
  diskSize: number
  parentFolder: string
  timeAgo: string
  isTree: boolean
  nativeFolderIcon: string
}>();

const emit = defineEmits<{
  (e: "toggle"): void
  (e: "reveal"): void
  (e: "unprotect"): void
}>();
</script>

<template>
  <div
    class="file-row"
    :class="{
      'file-row--selected': selected,
      'file-row--protected': isProtected,
      'file-row--tree': isTree,
    }"
    @click="emit('toggle')"
  >
    <!-- In tree mode the checkbox comes first in DOM (grid-column handles visual order) -->
    <div v-if="isTree" class="file-row-check">
      <Checkbox v-if="!isLocked" :model-value="selected" @change="emit('toggle')" />
    </div>

    <div class="file-icon-wrap">
      <img v-if="fileIcon" :src="fileIcon" alt="" class="file-row-icon" width="32" height="32" />
      <div v-else class="file-row-icon-placeholder"></div>
      <span v-if="isProtected" class="icon-shield-badge" title="Click to unprotect" @click.stop="emit('unprotect')">
        <svg class="shield-normal" viewBox="0 0 24 24" fill="var(--protect-green)" stroke="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
        <svg class="shield-hover" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="4.5" y1="4.5" x2="19.5" y2="19.5" stroke="var(--danger)" stroke-width="2.5"/></svg>
      </span>
    </div>

    <div class="file-row-info">
      <div class="file-row-name">
        <span class="file-name">{{ file.name }}</span>
        <span v-if="isSparse" class="badge badge-warning pill sparse-badge">Sparse</span>
      </div>
      <div v-if="!isTree" class="file-row-path">{{ parentFolder }}</div>
    </div>

    <span
      v-if="safetyLabel"
      class="badge safety-pill"
      :class="safetyClass"
      :data-tooltip="safetyTooltip"
    >{{ safetyLabel }}</span>
    <span v-else class="safety-pill-placeholder"></span>

    <div class="file-row-size mono">
      <span class="size-value">{{ formatSize(diskSize) }}</span>
      <span v-if="isSparse" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
      <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo }}</span>
    </div>

    <button class="btn-reveal" title="Reveal in Finder" @click.stop="emit('reveal')">
      <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" />
      <svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
    </button>

    <!-- In normal mode the checkbox comes last in DOM -->
    <div v-if="!isTree" class="file-row-check">
      <Checkbox v-if="!isLocked" :model-value="selected" @change="emit('toggle')" />
    </div>
  </div>
</template>

<style scoped>
.file-row {
  display: grid;
  grid-template-columns: 32px 1fr 160px 100px 24px 28px;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  cursor: pointer;
  transition: background 0.12s ease;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.file-row:last-child {
  border-bottom: none;
}

.file-row--tree {
  grid-template-columns: 32px 1fr 100px 24px 28px;
  padding-left: 24px;
}

.file-row--tree .file-icon-wrap {
  grid-column: 1;
}

.file-row--tree .file-row-info {
  grid-column: 2;
}

.file-row--tree .file-row-size {
  grid-column: 3;
}

.file-row--tree .safety-pill,
.file-row--tree .safety-pill-placeholder {
  display: none;
}

.file-row--tree .btn-reveal {
  grid-column: 4;
}

.file-row--tree .file-row-check {
  grid-column: 5;
  grid-row: 1;
}

.file-row:hover {
  background: rgba(255, 255, 255, 0.2);
}

.file-row--selected {
  background: rgba(2, 117, 244, 0.06);
}

.file-row--selected:hover {
  background: rgba(2, 117, 244, 0.10);
}

.file-row-check {
  display: flex;
  align-items: center;
  justify-content: center;
  grid-column: 6;
}

.file-row-info {
  min-width: 0;
  grid-column: 2;
}

.file-icon-wrap {
  position: relative;
  grid-column: 1;
  width: 32px;
  height: 32px;
}

.file-row-icon {
  border-radius: 3px;
}

.file-row-icon-placeholder {
  width: 32px;
  height: 32px;
}

.icon-shield-badge {
  position: absolute;
  top: -3px;
  right: -5px;
  width: 14px;
  height: 14px;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.2));
  cursor: pointer;
  transition: transform 0.1s;
}

.icon-shield-badge svg {
  width: 100%;
  height: 100%;
}

.icon-shield-badge .shield-hover {
  display: none;
}

.icon-shield-badge:hover {
  transform: scale(1.3);
}

.icon-shield-badge:hover .shield-normal {
  display: none;
}

.icon-shield-badge:hover .shield-hover {
  display: block;
}

.file-row--protected .file-name,
.file-row--protected .file-row-path {
  color: var(--green);
}

.file-row-name {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.file-name {
  font-family: var(--font-sans);
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sparse-badge {
  flex-shrink: 0;
}

.file-row-path {
  font-family: var(--font-sans);
  font-size: 10px;
  margin-top: 1px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgba(0, 0, 0, 0.85);
}

.file-row-size {
  grid-column: 4;
  text-align: right;
  white-space: nowrap;
  font-size: 13px;
}

.size-value {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.sparse-logical {
  display: block;
  font-size: 10px;
  margin-top: 1px;
}

.file-time-ago {
  display: block;
  font-size: 10px;
  margin-top: 1px;
}

.safety-pill {
  grid-column: 3;
  justify-self: end;
  position: relative;
  cursor: default;
}

.safety-pill[data-tooltip]:not([data-tooltip=""])::after {
  content: attr(data-tooltip);
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  background: white;
  color: rgba(0, 0, 0, 0.8);
  font-size: 11px;
  font-weight: 400;
  padding: 8px 12px;
  border-radius: 10px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12), 0 0 0 0.5px rgba(0, 0, 0, 0.06);
  white-space: normal;
  min-width: 200px;
  max-width: 320px;
  line-height: 1.4;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.1s ease;
  z-index: 100;
}

.safety-pill[data-tooltip]:not([data-tooltip=""]):hover::after {
  opacity: 1;
}

.safety-pill[data-tooltip]:not([data-tooltip=""])::before {
  content: '';
  position: absolute;
  bottom: calc(100% + 2px);
  left: 50%;
  transform: translateX(-50%);
  border: 6px solid transparent;
  border-top-color: white;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.1s ease;
  z-index: 101;
}

.safety-pill[data-tooltip]:not([data-tooltip=""]):hover::before {
  opacity: 1;
}

.safety-pill-placeholder {
  grid-column: 3;
}

.file-row-explanation {
  font-size: 11px;
  margin-top: 2px;
  line-height: 1.3;
  color: rgba(0, 0, 0, 0.45);
}

/* ---- Reveal in Finder button (grid + hover-reveal) ---- */
.btn-reveal {
  grid-column: 5;
  opacity: 0;
}

.file-row:hover .btn-reveal {
  opacity: 0.8;
}

.file-row .btn-reveal:hover {
  opacity: 1;
}
</style>
