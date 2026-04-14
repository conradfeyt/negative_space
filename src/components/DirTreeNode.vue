<script setup lang="ts">
/**
 * DirTreeNode — Recursive directory node for the LargeFiles directory view.
 *
 * Renders a collapsible directory header with a group checkbox, then recurses
 * into child directories. At MAX_DEPTH (3), descendants are flattened into
 * a single file list instead of continuing to nest.
 */
import type { FileInfo } from "../types";
import type { DirNode } from "../composables/useFileGrouping";
import { collectFiles } from "../composables/useFileGrouping";
import { formatSize } from "../utils";
import Checkbox from "./Checkbox.vue";
import FileRow from "./FileRow.vue";
import ChevronIcon from "../components/ChevronIcon.vue";

const MAX_DEPTH = 3;

const props = defineProps<{
  node: DirNode
  depth: number
  collapsedGroups: Set<string>
  selected: Set<string>
  getFileIcon: (name: string) => string
  safetyLabel: (path: string) => string
  safetyClass: (path: string) => string
  safetyTooltip: (path: string) => string
  isProtected: (path: string) => boolean
  isLocked: (path: string) => boolean
  isSparse: (file: FileInfo) => boolean
  diskSize: (file: FileInfo) => number
  parentFolder: (path: string) => string
  timeAgo: (file: FileInfo) => string
  nativeFolderIcon: string
  isGroupAllSelected: (files: FileInfo[]) => boolean
  isGroupPartialSelected: (files: FileInfo[]) => boolean
}>();

const emit = defineEmits<{
  (e: "toggle-group", key: string): void
  (e: "toggle-group-select", files: FileInfo[]): void
  (e: "toggle-select", path: string): void
  (e: "reveal", path: string): void
  (e: "unprotect", path: string): void
}>();

const isAtMaxDepth = props.depth >= MAX_DEPTH;
</script>

<template>
  <div class="dir-tree" :style="{ '--depth': depth }">
    <div class="dir-node">
      <div
        class="dir-header"
        tabindex="0"
        role="button"
        :aria-expanded="!collapsedGroups.has(node.key)"
        @click="emit('toggle-group', node.key)"
        @keydown.enter="emit('toggle-group', node.key)"
        @keydown.space.prevent="emit('toggle-group', node.key)"
      >
        <div class="dir-header-left">
          <ChevronIcon :expanded="!collapsedGroups.has(node.key)" variant="filled" :size="10" />
          <span class="dir-header-icon">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
            </svg>
          </span>
          <span class="dir-path mono">{{ depth === 0 ? (node.path || node.name) : node.name }}</span>
        </div>
        <div class="dir-header-right">
          <span class="dir-meta text-muted">{{ node.totalFiles }} file(s)</span>
          <span class="dir-size mono">{{ formatSize(node.totalSize) }}</span>
          <Checkbox
            :model-value="isGroupAllSelected(collectFiles(node))"
            :indeterminate="isGroupPartialSelected(collectFiles(node))"
            @change="emit('toggle-group-select', collectFiles(node))"
          />
        </div>
      </div>

      <template v-if="!collapsedGroups.has(node.key)">
        <!-- At max depth: flatten all descendants -->
        <template v-if="isAtMaxDepth">
          <div class="file-list file-list-indented">
            <FileRow
              v-for="file in collectFiles(node)"
              :key="file.path"
              :file="file"
              :selected="selected.has(file.path)"
              :is-protected="isProtected(file.path)"
              :is-locked="isLocked(file.path)"
              :file-icon="getFileIcon(file.name)"
              :safety-label="safetyLabel(file.path)"
              :safety-class="safetyClass(file.path)"
              :safety-tooltip="safetyTooltip(file.path)"
              :is-sparse="isSparse(file)"
              :disk-size="diskSize(file)"
              :parent-folder="parentFolder(file.path)"
              :time-ago="timeAgo(file)"
              :is-tree="true"
              :native-folder-icon="nativeFolderIcon"
              @toggle="emit('toggle-select', file.path)"
              @reveal="emit('reveal', file.path)"
              @unprotect="emit('unprotect', file.path)"
            />
          </div>
        </template>

        <!-- Below max depth: recurse into children, then render own files -->
        <template v-else>
          <DirTreeNode
            v-for="child in node.children"
            :key="child.key"
            :node="child"
            :depth="depth + 1"
            :collapsed-groups="collapsedGroups"
            :selected="selected"
            :get-file-icon="getFileIcon"
            :safety-label="safetyLabel"
            :safety-class="safetyClass"
            :safety-tooltip="safetyTooltip"
            :is-protected="isProtected"
            :is-locked="isLocked"
            :is-sparse="isSparse"
            :disk-size="diskSize"
            :parent-folder="parentFolder"
            :time-ago="timeAgo"
            :native-folder-icon="nativeFolderIcon"
            :is-group-all-selected="isGroupAllSelected"
            :is-group-partial-selected="isGroupPartialSelected"
            @toggle-group="emit('toggle-group', $event)"
            @toggle-group-select="emit('toggle-group-select', $event)"
            @toggle-select="emit('toggle-select', $event)"
            @reveal="emit('reveal', $event)"
            @unprotect="emit('unprotect', $event)"
          />

          <div v-if="node.files.length > 0" class="file-list file-list-indented">
            <FileRow
              v-for="file in node.files"
              :key="file.path"
              :file="file"
              :selected="selected.has(file.path)"
              :is-protected="isProtected(file.path)"
              :is-locked="isLocked(file.path)"
              :file-icon="getFileIcon(file.name)"
              :safety-label="safetyLabel(file.path)"
              :safety-class="safetyClass(file.path)"
              :safety-tooltip="safetyTooltip(file.path)"
              :is-sparse="isSparse(file)"
              :disk-size="diskSize(file)"
              :parent-folder="parentFolder(file.path)"
              :time-ago="timeAgo(file)"
              :is-tree="true"
              :native-folder-icon="nativeFolderIcon"
              @toggle="emit('toggle-select', file.path)"
              @reveal="emit('reveal', file.path)"
              @unprotect="emit('unprotect', file.path)"
            />
          </div>
        </template>
      </template>
    </div>
  </div>
</template>

<script lang="ts">
export default { name: "DirTreeNode" };
</script>

<style scoped>
.dir-tree {
  padding-left: calc(var(--depth, 0) * 20px);
}

.dir-node {
  margin-bottom: 1px;
}

.dir-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 14px 6px 12px;
  cursor: pointer;
  transition: background 0.15s ease;
  user-select: none;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
}

.dir-header:hover {
  background: rgba(255, 255, 255, 0.15);
}

.dir-header-left {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  min-width: 0;
}

.dir-header-icon {
  color: var(--muted);
  flex-shrink: 0;
  display: flex;
}

.dir-path {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.dir-meta {
  font-size: 11px;
  white-space: nowrap;
}

.dir-size {
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
}

.file-list {
  display: flex;
  flex-direction: column;
}

.file-list-indented {
  padding-left: var(--sp-5);
}
</style>
