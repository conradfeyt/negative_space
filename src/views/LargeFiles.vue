<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { FileInfo } from "../types";
import { formatSize } from "../utils";
import {
  largeFiles,
  largeFilesScanning,
  largeFilesScanned,
  largeFilesError,
  largeFilesCurrentDir,
  scanLargeFiles,
  deleteFiles,
  hasFullDiskAccess,
  checkFullDiskAccess,
} from "../stores/scanStore";

const selected = ref<Set<string>>(new Set());
const deleting = ref(false);
const successMsg = ref("");
const minSizeMb = ref(100);
const scanPath = ref("~");

/** Track which groups are collapsed */
const collapsedGroups = ref<Set<string>>(new Set());

type SortMode = "size" | "directory";
const sortMode = ref<SortMode>("size");

async function openFdaSettings() {
  try { await invoke("open_full_disk_access_settings"); } catch (_) {}
}

async function recheckFda() {
  await checkFullDiskAccess();
}

async function scan() {
  successMsg.value = "";
  selected.value = new Set();
  collapsedGroups.value = new Set();
  await scanLargeFiles(scanPath.value, minSizeMb.value);
}

async function deleteSelected() {
  if (selected.value.size === 0) return;
  deleting.value = true;
  successMsg.value = "";
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      successMsg.value = `Deleted ${result.deleted_count} file(s), freed ${formatSize(result.freed_bytes)}`;
      largeFiles.value = largeFiles.value.filter((f) => !selected.value.has(f.path));
      selected.value = new Set();
    }
    if (result.errors.length > 0) {
      largeFilesError.value = result.errors.join("; ");
    }
  } catch (e) {
    largeFilesError.value = String(e);
  } finally {
    deleting.value = false;
  }
}

async function revealInFinder(path: string) {
  try { await invoke("reveal_in_finder", { path }); } catch (_) {}
}

function toggleSelect(path: string) {
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  selected.value = next;
}

function toggleAll() {
  if (allSelected.value) selected.value = new Set();
  else selected.value = new Set(largeFiles.value.map((f) => f.path));
}

function toggleGroup(groupId: string) {
  const next = new Set(collapsedGroups.value);
  if (next.has(groupId)) next.delete(groupId);
  else next.add(groupId);
  collapsedGroups.value = next;
}

function toggleGroupSelect(files: FileInfo[]) {
  const paths = files.map((f) => f.path);
  const allGroupSelected = paths.every((p) => selected.value.has(p));
  const next = new Set(selected.value);
  if (allGroupSelected) {
    paths.forEach((p) => next.delete(p));
  } else {
    paths.forEach((p) => next.add(p));
  }
  selected.value = next;
}

const allSelected = computed(
  () => largeFiles.value.length > 0 && selected.value.size === largeFiles.value.length
);

const totalSelected = computed(() =>
  largeFiles.value
    .filter((f) => selected.value.has(f.path))
    .reduce((sum, f) => sum + diskSize(f), 0)
);

function isSparse(file: FileInfo): boolean {
  return file.is_sparse && file.actual_size < file.apparent_size * 0.8;
}

function diskSize(file: FileInfo): number {
  return isSparse(file) ? file.actual_size : file.apparent_size;
}

// ---------------------------------------------------------------------------
// File categorization
// ---------------------------------------------------------------------------

const USER_PATH_PATTERNS = [
  "/Documents/", "/Downloads/", "/Desktop/", "/Movies/",
  "/Music/", "/Pictures/", "/Photos/", "/Public/", "/iCloud/",
];

const DEV_EXTENSIONS = new Set([
  ".hprof", ".pack", ".idx", ".jar", ".war", ".class", ".dSYM",
  ".o", ".a", ".dylib", ".so", ".wasm", ".ipa", ".xcarchive",
  ".vmdk", ".qcow2", ".vdi", ".gguf", ".bin", ".safetensors",
  ".onnx", ".pt", ".pth",
]);

type FileCategory = "user" | "system";

function hasHiddenPathComponent(path: string): boolean {
  return /\/\.[a-zA-Z0-9]/.test(path);
}

function categorize(file: FileInfo): FileCategory {
  const path = file.path;
  if (hasHiddenPathComponent(path)) return "system";
  const lastDot = file.name.lastIndexOf(".");
  if (lastDot > 0) {
    const ext = file.name.substring(lastDot).toLowerCase();
    if (DEV_EXTENSIONS.has(ext)) return "system";
  }
  for (const pattern of USER_PATH_PATTERNS) {
    if (path.includes(pattern)) return "user";
  }
  return "system";
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

function displayPath(path: string): string {
  const home = path.match(/^\/Users\/[^/]+/);
  if (home) return path.replace(home[0], "~");
  return path;
}

function parentFolder(path: string): string {
  const display = displayPath(path);
  const lastSlash = display.lastIndexOf("/");
  if (lastSlash <= 0) return display;
  return display.substring(0, lastSlash);
}

// ---------------------------------------------------------------------------
// Directory tree data structure
// ---------------------------------------------------------------------------

/** A node in the directory tree. Internal nodes have children and/or files. */
interface DirNode {
  /** Display name for this segment (e.g. "Application Support") */
  name: string;
  /** Full display path from root (e.g. "~/Library/Application Support") */
  path: string;
  /** Unique key for collapse state */
  key: string;
  /** Child directories (sorted by totalSize desc) */
  children: DirNode[];
  /** Files directly in this directory (sorted by size desc) */
  files: FileInfo[];
  /** Aggregate size of ALL files in this subtree */
  totalSize: number;
  /** Total count of ALL files in this subtree */
  totalFiles: number;
}

/**
 * Build a directory tree from a flat list of files.
 *
 * 1. Insert each file into a trie based on its display path segments
 * 2. Compute aggregate sizes bottom-up
 * 3. Collapse single-child chains (~/a/b/c where b has only child c → "b/c")
 * 4. Sort children by totalSize desc at each level
 */
function buildDirTree(files: FileInfo[], keyPrefix: string): DirNode {
  // Root node represents the common ancestor
  const root: DirNode = {
    name: "",
    path: "",
    key: keyPrefix,
    children: [],
    files: [],
    totalSize: 0,
    totalFiles: 0,
  };

  // Build a map of path → DirNode for fast lookup during insertion
  const nodeMap = new Map<string, DirNode>();
  nodeMap.set("", root);

  function getOrCreateNode(dirPath: string): DirNode {
    if (nodeMap.has(dirPath)) return nodeMap.get(dirPath)!;

    // Find parent path
    const lastSlash = dirPath.lastIndexOf("/");
    const parentPath = lastSlash > 0 ? dirPath.substring(0, lastSlash) : "";
    const segmentName = lastSlash >= 0 ? dirPath.substring(lastSlash + 1) : dirPath;

    const parent = getOrCreateNode(parentPath);
    const node: DirNode = {
      name: segmentName,
      path: dirPath,
      key: keyPrefix + ":" + dirPath,
      children: [],
      files: [],
      totalSize: 0,
      totalFiles: 0,
    };
    parent.children.push(node);
    nodeMap.set(dirPath, node);
    return node;
  }

  // Insert files
  for (const file of files) {
    const fileDirPath = parentFolder(file.path);
    const dirNode = getOrCreateNode(fileDirPath);
    dirNode.files.push(file);
  }

  // Compute aggregate sizes bottom-up (post-order traversal)
  function computeSizes(node: DirNode): void {
    let size = 0;
    let count = 0;

    for (const child of node.children) {
      computeSizes(child);
      size += child.totalSize;
      count += child.totalFiles;
    }

    for (const file of node.files) {
      size += diskSize(file);
      count += 1;
    }

    // Sort files within node by size desc
    node.files.sort((a, b) => diskSize(b) - diskSize(a));

    node.totalSize = size;
    node.totalFiles = count;
  }
  computeSizes(root);

  // Collapse single-child directory chains to reduce excessive nesting.
  // E.g. if ~/Library only has one child "Application Support", merge them
  // into a single node "~/Library/Application Support".
  function collapse(node: DirNode): void {
    // First collapse children recursively
    for (const child of node.children) {
      collapse(child);
    }

    // If this node has exactly one child and no files of its own,
    // merge the child into this node
    while (node.children.length === 1 && node.files.length === 0) {
      const onlyChild = node.children[0];
      // Merge: absorb child's name into ours
      if (node.name) {
        node.name = node.name + "/" + onlyChild.name;
      } else {
        node.name = onlyChild.name;
      }
      node.path = onlyChild.path;
      node.key = onlyChild.key;
      node.children = onlyChild.children;
      node.files = onlyChild.files;
      // totalSize and totalFiles stay the same (already computed)
    }

    // Sort children by totalSize desc
    node.children.sort((a, b) => b.totalSize - a.totalSize);
  }
  collapse(root);

  return root;
}

/** Collect all FileInfo objects from a DirNode subtree */
function collectFiles(node: DirNode): FileInfo[] {
  const result: FileInfo[] = [...node.files];
  for (const child of node.children) {
    result.push(...collectFiles(child));
  }
  return result;
}

// ---------------------------------------------------------------------------
// Top-level grouping (User / System)
// ---------------------------------------------------------------------------

interface CategoryGroup {
  id: string;
  label: string;
  description: string;
  totalSize: number;
  totalFiles: number;
  /** Size mode: flat sorted list */
  flatFiles: FileInfo[];
  /** Directory mode: tree root */
  tree: DirNode;
}

const groupedFiles = computed<CategoryGroup[]>(() => {
  const userFiles: FileInfo[] = [];
  const systemFiles: FileInfo[] = [];

  for (const file of largeFiles.value) {
    if (categorize(file) === "user") {
      userFiles.push(file);
    } else {
      systemFiles.push(file);
    }
  }

  const bySize = (a: FileInfo, b: FileInfo) => diskSize(b) - diskSize(a);
  const groups: CategoryGroup[] = [];

  if (userFiles.length > 0) {
    const sorted = [...userFiles].sort(bySize);
    groups.push({
      id: "user",
      label: "User Files",
      description: "Documents, Downloads, Desktop, and personal files",
      totalSize: userFiles.reduce((s, f) => s + diskSize(f), 0),
      totalFiles: userFiles.length,
      flatFiles: sorted,
      tree: buildDirTree(userFiles, "user"),
    });
  }

  if (systemFiles.length > 0) {
    const sorted = [...systemFiles].sort(bySize);
    groups.push({
      id: "system",
      label: "System & Development",
      description: "Libraries, caches, build artifacts, SDK data, and dev tools",
      totalSize: systemFiles.reduce((s, f) => s + diskSize(f), 0),
      totalFiles: systemFiles.length,
      flatFiles: sorted,
      tree: buildDirTree(systemFiles, "system"),
    });
  }

  return groups;
});

const totalLargeFileSize = computed(() =>
  largeFiles.value.reduce((sum, f) => sum + diskSize(f), 0)
);

function isGroupAllSelected(files: FileInfo[]): boolean {
  return files.length > 0 && files.every((f) => selected.value.has(f.path));
}

function isGroupPartialSelected(files: FileInfo[]): boolean {
  const selCount = files.filter((f) => selected.value.has(f.path)).length;
  return selCount > 0 && selCount < files.length;
}
</script>

<template>
  <div class="large-files">
    <div class="view-header">
      <h2>Large Files</h2>
      <p class="text-muted">Find and remove large files taking up space</p>
    </div>

    <div class="card controls-card">
      <div class="controls-row">
        <div class="control-group">
          <label class="control-label">Minimum size (MB)</label>
          <input v-model.number="minSizeMb" type="number" min="1" class="size-input" />
        </div>
        <div class="control-group">
          <label class="control-label">Search path</label>
          <input v-model="scanPath" type="text" class="path-input" />
        </div>
        <button class="btn-primary scan-btn" :disabled="largeFilesScanning" @click="scan">
          <span v-if="largeFilesScanning" class="spinner-sm"></span>
          {{ largeFilesScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <!-- FDA warning -->
    <div v-if="hasFullDiskAccess === false" class="fda-warning-banner">
      <span class="fda-warning-dot"></span>
      <div class="fda-warning-body">
        <div class="fda-warning-title">Limited scan -- Full Disk Access required</div>
        <div class="fda-warning-text">
          Without Full Disk Access, Desktop, Documents, Downloads, and other
          protected folders are skipped to avoid macOS permission prompts.
        </div>
        <div class="fda-warning-actions">
          <button class="btn-fda btn-fda-primary" @click="openFdaSettings">Open System Settings</button>
          <button class="btn-fda btn-fda-secondary" @click="recheckFda">Re-check</button>
        </div>
      </div>
    </div>

    <div v-if="largeFilesError" class="error-message">{{ largeFilesError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <!-- Scanning progress -->
    <div v-if="largeFilesScanning" class="scan-progress-bar">
      <div class="scan-progress-left">
        <span class="spinner-xs"></span>
        <span class="scan-progress-label">Scanning...</span>
      </div>
      <div class="scan-progress-dir mono truncate">{{ largeFilesCurrentDir }}</div>
    </div>

    <!-- Empty state -->
    <div v-if="!largeFilesScanning && largeFilesScanned && largeFiles.length === 0" class="card empty-state">
      <p class="text-muted">No files found larger than {{ minSizeMb }} MB</p>
    </div>

    <!-- Results -->
    <div v-if="largeFiles.length > 0" class="results-container">

      <!-- Summary bar -->
      <div class="results-summary">
        <div class="summary-left">
          <span class="results-count">{{ largeFiles.length }} file(s)</span>
          <span class="results-total-size">{{ formatSize(totalLargeFileSize) }} total</span>
        </div>
        <div class="results-actions">
          <div class="sort-toggle">
            <button class="sort-btn" :class="{ 'sort-btn--active': sortMode === 'size' }" @click="sortMode = 'size'">Size</button>
            <button class="sort-btn" :class="{ 'sort-btn--active': sortMode === 'directory' }" @click="sortMode = 'directory'">Directory</button>
          </div>
          <label class="select-all-label">
            <input type="checkbox" :checked="allSelected" @change="toggleAll" />
            <span>Select all</span>
          </label>
          <span v-if="selected.size > 0" class="selected-info">
            {{ selected.size }} selected ({{ formatSize(totalSelected) }})
          </span>
          <button class="btn-danger" :disabled="selected.size === 0 || deleting" @click="deleteSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Deleting..." : "Delete Selected" }}
          </button>
        </div>
      </div>

      <!-- Category groups -->
      <div v-for="group in groupedFiles" :key="group.id" class="file-group">

        <!-- Category header -->
        <div class="group-header" @click="toggleGroup(group.id)">
          <div class="group-header-left">
            <span class="expand-chevron" :class="{ expanded: !collapsedGroups.has(group.id) }">
              <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
            </span>
            <div class="group-title-block">
              <span class="group-title">{{ group.label }}</span>
              <span class="group-meta text-muted">
                {{ group.totalFiles }} file(s) &middot; {{ formatSize(group.totalSize) }}
              </span>
            </div>
          </div>
          <label class="group-check" @click.stop>
            <input
              type="checkbox"
              :checked="isGroupAllSelected(group.flatFiles)"
              :indeterminate="isGroupPartialSelected(group.flatFiles)"
              @change="toggleGroupSelect(group.flatFiles)"
            />
          </label>
        </div>

        <div v-if="!collapsedGroups.has(group.id)" class="group-description text-muted">
          {{ group.description }}
        </div>

        <!-- ===== SIZE MODE: flat file list ===== -->
        <template v-if="!collapsedGroups.has(group.id) && sortMode === 'size'">
          <div class="file-list">
            <div
              v-for="file in group.flatFiles"
              :key="file.path"
              class="file-row"
              :class="{ 'file-row--selected': selected.has(file.path) }"
              @click="toggleSelect(file.path)"
            >
              <div class="file-row-check">
                <input type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
              </div>
              <div class="file-row-info">
                <div class="file-row-name">
                  <span class="file-name">{{ file.name }}</span>
                  <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                </div>
                <div class="file-row-path text-muted mono">{{ parentFolder(file.path) }}</div>
              </div>
              <div class="file-row-size mono">
                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
              </div>
              <div class="file-row-date text-muted">{{ file.modified }}</div>
              <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                  <circle cx="8" cy="9" r="2"/>
                </svg>
              </button>
            </div>
          </div>
        </template>

        <!-- ===== DIRECTORY MODE: recursive tree ===== -->
        <template v-if="!collapsedGroups.has(group.id) && sortMode === 'directory'">
          <!-- Render top-level children of the tree root (root itself is a virtual node) -->
          <template v-for="child in group.tree.children" :key="child.key">
            <div class="dir-tree" :style="{ '--depth': 0 }">
              <!-- Recursive directory node rendering -->
              <div class="dir-node">
                <div class="dir-header" @click="toggleGroup(child.key)">
                  <div class="dir-header-left">
                    <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(child.key) }">
                      <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                    </span>
                    <span class="dir-header-icon">
                      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                      </svg>
                    </span>
                    <span class="dir-path mono">{{ child.path || child.name }}</span>
                  </div>
                  <div class="dir-header-right">
                    <span class="dir-meta text-muted">{{ child.totalFiles }} file(s)</span>
                    <span class="dir-size mono">{{ formatSize(child.totalSize) }}</span>
                    <label class="group-check" @click.stop>
                      <input
                        type="checkbox"
                        :checked="isGroupAllSelected(collectFiles(child))"
                        :indeterminate="isGroupPartialSelected(collectFiles(child))"
                        @change="toggleGroupSelect(collectFiles(child))"
                      />
                    </label>
                  </div>
                </div>

                <template v-if="!collapsedGroups.has(child.key)">
                  <!-- Sub-directories (depth 1) -->
                  <template v-for="d1 in child.children" :key="d1.key">
                    <div class="dir-tree" :style="{ '--depth': 1 }">
                      <div class="dir-node">
                        <div class="dir-header" @click="toggleGroup(d1.key)">
                          <div class="dir-header-left">
                            <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(d1.key) }">
                              <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                            </span>
                            <span class="dir-header-icon">
                              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                              </svg>
                            </span>
                            <span class="dir-path mono">{{ d1.name }}</span>
                          </div>
                          <div class="dir-header-right">
                            <span class="dir-meta text-muted">{{ d1.totalFiles }} file(s)</span>
                            <span class="dir-size mono">{{ formatSize(d1.totalSize) }}</span>
                            <label class="group-check" @click.stop>
                              <input type="checkbox"
                                :checked="isGroupAllSelected(collectFiles(d1))"
                                :indeterminate="isGroupPartialSelected(collectFiles(d1))"
                                @change="toggleGroupSelect(collectFiles(d1))"
                              />
                            </label>
                          </div>
                        </div>

                        <template v-if="!collapsedGroups.has(d1.key)">
                          <!-- Sub-directories (depth 2) -->
                          <template v-for="d2 in d1.children" :key="d2.key">
                            <div class="dir-tree" :style="{ '--depth': 2 }">
                              <div class="dir-node">
                                <div class="dir-header" @click="toggleGroup(d2.key)">
                                  <div class="dir-header-left">
                                    <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(d2.key) }">
                                      <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                                    </span>
                                    <span class="dir-header-icon">
                                      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                                      </svg>
                                    </span>
                                    <span class="dir-path mono">{{ d2.name }}</span>
                                  </div>
                                  <div class="dir-header-right">
                                    <span class="dir-meta text-muted">{{ d2.totalFiles }} file(s)</span>
                                    <span class="dir-size mono">{{ formatSize(d2.totalSize) }}</span>
                                    <label class="group-check" @click.stop>
                                      <input type="checkbox"
                                        :checked="isGroupAllSelected(collectFiles(d2))"
                                        :indeterminate="isGroupPartialSelected(collectFiles(d2))"
                                        @change="toggleGroupSelect(collectFiles(d2))"
                                      />
                                    </label>
                                  </div>
                                </div>

                                <template v-if="!collapsedGroups.has(d2.key)">
                                  <!-- Deeper levels: just show files flat -->
                                  <template v-for="d3 in d2.children" :key="d3.key">
                                    <div class="dir-tree" :style="{ '--depth': 3 }">
                                      <div class="dir-header" @click="toggleGroup(d3.key)">
                                        <div class="dir-header-left">
                                          <span class="expand-chevron expand-chevron--sm" :class="{ expanded: !collapsedGroups.has(d3.key) }">
                                            <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
                                          </span>
                                          <span class="dir-header-icon">
                                            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                              <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                                            </svg>
                                          </span>
                                          <span class="dir-path mono">{{ d3.name }}</span>
                                        </div>
                                        <div class="dir-header-right">
                                          <span class="dir-meta text-muted">{{ d3.totalFiles }} file(s)</span>
                                          <span class="dir-size mono">{{ formatSize(d3.totalSize) }}</span>
                                          <label class="group-check" @click.stop>
                                            <input type="checkbox"
                                              :checked="isGroupAllSelected(collectFiles(d3))"
                                              :indeterminate="isGroupPartialSelected(collectFiles(d3))"
                                              @change="toggleGroupSelect(collectFiles(d3))"
                                            />
                                          </label>
                                        </div>
                                      </div>
                                      <!-- Files inside depth-3+ dirs shown flat -->
                                      <div v-if="!collapsedGroups.has(d3.key)" class="file-list" :style="{ paddingLeft: '20px' }">
                                        <div
                                          v-for="file in collectFiles(d3)"
                                          :key="file.path"
                                          class="file-row file-row--tree"
                                          :class="{ 'file-row--selected': selected.has(file.path) }"
                                          @click="toggleSelect(file.path)"
                                        >
                                          <div class="file-row-check">
                                            <input type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                                          </div>
                                          <div class="file-row-info">
                                            <div class="file-row-name">
                                              <span class="file-name">{{ file.name }}</span>
                                              <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                                            </div>
                                          </div>
                                          <div class="file-row-size mono">
                                            <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                                            <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                                          </div>
                                          <div class="file-row-date text-muted">{{ file.modified }}</div>
                                          <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                              <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                                              <circle cx="8" cy="9" r="2"/>
                                            </svg>
                                          </button>
                                        </div>
                                      </div>
                                    </div>
                                  </template>

                                  <!-- Files at depth 2 -->
                                  <div v-if="d2.files.length > 0" class="file-list" :style="{ paddingLeft: '20px' }">
                                    <div
                                      v-for="file in d2.files"
                                      :key="file.path"
                                      class="file-row file-row--tree"
                                      :class="{ 'file-row--selected': selected.has(file.path) }"
                                      @click="toggleSelect(file.path)"
                                    >
                                      <div class="file-row-check">
                                        <input type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                                      </div>
                                      <div class="file-row-info">
                                        <div class="file-row-name">
                                          <span class="file-name">{{ file.name }}</span>
                                          <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                                        </div>
                                      </div>
                                      <div class="file-row-size mono">
                                        <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                                        <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                                      </div>
                                      <div class="file-row-date text-muted">{{ file.modified }}</div>
                                      <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                          <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                                          <circle cx="8" cy="9" r="2"/>
                                        </svg>
                                      </button>
                                    </div>
                                  </div>
                                </template>
                              </div>
                            </div>
                          </template>

                          <!-- Files at depth 1 -->
                          <div v-if="d1.files.length > 0" class="file-list" :style="{ paddingLeft: '20px' }">
                            <div
                              v-for="file in d1.files"
                              :key="file.path"
                              class="file-row file-row--tree"
                              :class="{ 'file-row--selected': selected.has(file.path) }"
                              @click="toggleSelect(file.path)"
                            >
                              <div class="file-row-check">
                                <input type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                              </div>
                              <div class="file-row-info">
                                <div class="file-row-name">
                                  <span class="file-name">{{ file.name }}</span>
                                  <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                                </div>
                              </div>
                              <div class="file-row-size mono">
                                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                              </div>
                              <div class="file-row-date text-muted">{{ file.modified }}</div>
                              <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                  <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                                  <circle cx="8" cy="9" r="2"/>
                                </svg>
                              </button>
                            </div>
                          </div>
                        </template>
                      </div>
                    </div>
                  </template>

                  <!-- Files at depth 0 (directly under top-level dir) -->
                  <div v-if="child.files.length > 0" class="file-list" :style="{ paddingLeft: '20px' }">
                    <div
                      v-for="file in child.files"
                      :key="file.path"
                      class="file-row file-row--tree"
                      :class="{ 'file-row--selected': selected.has(file.path) }"
                      @click="toggleSelect(file.path)"
                    >
                      <div class="file-row-check">
                        <input type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                      </div>
                      <div class="file-row-info">
                        <div class="file-row-name">
                          <span class="file-name">{{ file.name }}</span>
                          <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                        </div>
                      </div>
                      <div class="file-row-size mono">
                        <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                        <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                      </div>
                      <div class="file-row-date text-muted">{{ file.modified }}</div>
                      <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                          <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                          <circle cx="8" cy="9" r="2"/>
                        </svg>
                      </button>
                    </div>
                  </div>
                </template>
              </div>
            </div>
          </template>

          <!-- Files at tree root level (shouldn't normally happen, but handle gracefully) -->
          <div v-if="group.tree.files.length > 0" class="file-list">
            <div
              v-for="file in group.tree.files"
              :key="file.path"
              class="file-row"
              :class="{ 'file-row--selected': selected.has(file.path) }"
              @click="toggleSelect(file.path)"
            >
              <div class="file-row-check">
                <input type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
              </div>
              <div class="file-row-info">
                <div class="file-row-name">
                  <span class="file-name">{{ file.name }}</span>
                  <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                </div>
                <div class="file-row-path text-muted mono">{{ parentFolder(file.path) }}</div>
              </div>
              <div class="file-row-size mono">
                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
              </div>
              <div class="file-row-date text-muted">{{ file.modified }}</div>
              <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M2.5 2.5h4l1.5 1.5h5.5v9h-11z"/>
                  <circle cx="8" cy="9" r="2"/>
                </svg>
              </button>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.large-files { max-width: 780px; }
.controls-card { margin-bottom: var(--sp-6); }
.controls-row { display: flex; align-items: flex-end; gap: var(--sp-4); }
.control-group { display: flex; flex-direction: column; gap: var(--sp-1); }
.control-label { font-size: 12px; font-weight: 400; color: var(--muted); }
.size-input { width: 100px; }
.path-input { width: 240px; }

/* ---- Scanning progress bar ---- */
.scan-progress-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
  padding: 10px var(--sp-5);
  margin-bottom: var(--sp-5);
  border-radius: var(--radius-sm);
  background: var(--info-tint);
  border: 1px solid rgba(20, 138, 160, 0.10);
  animation: fadeIn 0.2s ease;
}

.scan-progress-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.scan-progress-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--info-text);
}

.scan-progress-dir {
  font-size: 11px;
  color: var(--text-secondary);
  text-align: right;
  min-width: 0;
}

/* ---- Results summary bar ---- */
.results-summary {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--sp-6);
  padding: 0 var(--sp-1);
  flex-wrap: wrap;
  gap: var(--sp-3);
}

.summary-left {
  display: flex;
  align-items: baseline;
  gap: var(--sp-3);
}

.results-count {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.results-total-size {
  font-size: 13px;
  color: var(--muted);
}

.results-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.select-all-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}

/* ---- Sort toggle ---- */
.sort-toggle {
  display: flex;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid var(--border);
}

.sort-btn {
  font-size: 12px;
  font-weight: 500;
  padding: 5px 14px;
  border-radius: 0;
  background: rgba(255, 255, 255, 0.3);
  color: var(--text-secondary);
  border: none;
  transition: background 0.15s ease, color 0.15s ease;
}

.sort-btn:hover {
  background: rgba(255, 255, 255, 0.5);
}

.sort-btn--active {
  background: var(--accent);
  color: #fff;
}

.sort-btn--active:hover {
  background: var(--accent-hover);
}

/* ---- Category group ---- */
.file-group {
  margin-bottom: var(--sp-8);
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-3) var(--sp-2);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background 0.15s ease;
  user-select: none;
}

.group-header:hover {
  background: rgba(255, 255, 255, 0.25);
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.group-title-block {
  display: flex;
  align-items: baseline;
  gap: var(--sp-3);
}

.group-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: -0.2px;
}

.group-meta {
  font-size: 12px;
}

.group-check {
  cursor: pointer;
}

.group-description {
  font-size: 12px;
  padding: 0 var(--sp-2) var(--sp-3) 34px;
}

/* ---- Directory tree ---- */
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
  padding: var(--sp-2) var(--sp-2) var(--sp-2) var(--sp-3);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background 0.15s ease;
  user-select: none;
}

.dir-header:hover {
  background: rgba(255, 255, 255, 0.2);
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
  gap: var(--sp-3);
  flex-shrink: 0;
}

.dir-meta {
  font-size: 11px;
}

.dir-size {
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
}

.expand-chevron--sm svg {
  width: 10px;
  height: 10px;
}

/* ---- File list ---- */
.file-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.file-row {
  display: grid;
  grid-template-columns: 36px 1fr auto auto 32px;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3) var(--sp-4);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background 0.12s ease;
}

.file-row--tree {
  padding-left: var(--sp-6);
}

.file-row:hover {
  background: rgba(255, 255, 255, 0.35);
}

.file-row--selected {
  background: rgba(59, 199, 232, 0.06);
}

.file-row--selected:hover {
  background: rgba(59, 199, 232, 0.10);
}

.file-row-check {
  display: flex;
  align-items: center;
  justify-content: center;
}

.file-row-info {
  min-width: 0;
}

.file-row-name {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.file-name {
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
  font-size: 11px;
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-row-size {
  text-align: right;
  white-space: nowrap;
  padding-right: var(--sp-2);
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

.file-row-date {
  font-size: 12px;
  white-space: nowrap;
  min-width: 90px;
  text-align: right;
}

/* ---- Reveal in Finder button ---- */
.reveal-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  transition: background 0.15s ease, color 0.15s ease, transform 0.15s ease;
  flex-shrink: 0;
}

.reveal-btn:hover {
  background: rgba(255, 255, 255, 0.5);
  color: var(--accent);
}

.reveal-btn:active {
  transform: scale(0.92);
}
</style>
