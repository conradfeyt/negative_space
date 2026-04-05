<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { FileInfo } from "../types";
import { formatSize, fileDiskSize, timeAgo, revealInFinder } from "../utils";
import {
  largeFiles,
  largeFilesScanning,
  largeFilesScanned,
  largeFilesError,
  largeFilesCurrentDir,
  scanLargeFiles,
  deleteFiles,
  fileClassifications,
  classifyFiles,
  vaultEntries,
  toggleProtected,
  isProtected,
} from "../stores/scanStore";
import FdaWarningBanner from "../components/FdaWarningBanner.vue";

const selected = ref<Set<string>>(new Set());
const deleting = ref(false);
const successMsg = ref("");
const minSizeMb = ref(100);
const scanPath = ref("~");

/** Track which groups are collapsed */
const collapsedGroups = ref<Set<string>>(new Set());

type SortMode = "size" | "directory" | "safety" | "type";
const sortMode = ref<SortMode>("size");

async function scan() {
  successMsg.value = "";
  selected.value = new Set();
  collapsedGroups.value = new Set();
  await scanLargeFiles(scanPath.value, minSizeMb.value);
}

// Classify files when scan completes
watch(largeFilesScanned, (scanned) => {
  if (scanned && largeFiles.value.length > 0) {
    const files = largeFiles.value.map(f => ({
      path: f.path,
      name: f.name,
      size: f.apparent_size,
      file_type: f.name.split(".").pop()?.toLowerCase() ?? "",
    }));
    classifyFiles(files);
  }
});

// Also classify on mount if scan results already exist (from cache)
try {
  if (largeFilesScanned.value && largeFiles.value.length > 0 && fileClassifications.value.size === 0) {
    const files = largeFiles.value.map(f => ({
      path: f.path,
      name: f.name,
      size: f.apparent_size,
      file_type: f.name.split(".").pop()?.toLowerCase() ?? "",
      modified: f.modified || null,
    }));
    classifyFiles(files);
  }
} catch { /* non-critical */ }

function getClassification(path: string) {
  return fileClassifications.value.get(path);
}

function safetyColor(safety: string): string {
  switch (safety) {
    case "safe":
    case "safe_stale": return "var(--success)";
    case "safe_rebuild": return "hsla(195, 55%, 42%, 0.85)";
    case "probably_safe": return "var(--accent)";
    case "risky": return "var(--danger)";
    case "vaulted": return "hsla(280, 40%, 50%, 0.85)";
    default: return "var(--muted)";
  }
}

function safetyLabel(safety: string): string {
  switch (safety) {
    case "safe": return "Safe to delete";
    case "safe_stale": return "Safe — older than 90 days";
    case "safe_rebuild": return "Safe — recently modified";
    case "probably_safe": return "Likely safe";
    case "risky": return "Caution";
    case "vaulted": return "Vaulted — do not delete";
    default: return "";
  }
}

async function exportSelected() {
  if (selected.value.size === 0) return;
  const selectedPaths = selected.value;

  function mapFile(f: FileInfo) {
    const c = getClassification(f.path);
    return {
      path: f.path,
      name: f.name,
      size: diskSize(f),
      size_formatted: formatSize(diskSize(f)),
      apparent_size: f.apparent_size,
      is_sparse: f.is_sparse,
      modified: f.modified,
      safety: c?.safety ?? "unknown",
      explanation: c?.explanation ?? null,
    };
  }

  // Export using the same grouping as the current view
  const groups: Record<string, { label: string; files: ReturnType<typeof mapFile>[] }> = {};
  let totalFiles = 0;
  let totalSize = 0;

  for (const group of activeGroups.value) {
    const selectedInGroup = group.flatFiles.filter(f => selectedPaths.has(f.path));
    if (selectedInGroup.length === 0) continue;
    const mapped = selectedInGroup.map(mapFile);
    groups[group.id] = { label: group.label, files: mapped };
    totalFiles += mapped.length;
    totalSize += mapped.reduce((s, f) => s + f.size, 0);
  }

  const exportData = {
    exported: new Date().toISOString(),
    view_mode: sortMode.value,
    total_files: totalFiles,
    total_size: totalSize,
    total_size_formatted: formatSize(totalSize),
    groups,
  };

  const json = JSON.stringify(exportData, null, 2);
  const path = await save({
    defaultPath: `negativ-large-files-${sortMode.value}-${new Date().toISOString().slice(0, 10)}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  try {
    await invoke("export_disk_map", { data: json, path });
    successMsg.value = `Exported ${totalFiles} file(s) to ${path}`;
  } catch (e) {
    largeFilesError.value = `Export failed: ${e}`;
  }
}

function protectSelected() {
  for (const path of selected.value) {
    toggleProtected(path);
  }
  selected.value = new Set();
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
      // Update the on-disk cache so deleted files don't reappear on next launch
      invoke("save_scan_cache", { domain: "large-files", data: JSON.stringify(largeFiles.value) }).catch(e => console.warn('[large-files] cache save failed:', e));
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


function isVaulted(path: string): boolean {
  return getClassification(path)?.safety === "vaulted";
}

function isLocked(path: string): boolean {
  return isVaulted(path) || isProtected(path);
}

// Map vault archive hashes back to original names
const vaultNameMap = computed(() => {
  const map = new Map<string, string>();
  for (const entry of vaultEntries.value) {
    const originalName = entry.original_path.split("/").pop() ?? entry.vault_filename;
    // Key by multiple formats to handle .zst / .tar.zst variants
    const hash = entry.vault_filename.split(".")[0];
    map.set(hash, originalName);
    map.set(entry.vault_filename, originalName);
    map.set(entry.id, originalName);
    // Also key by blake3_hash if available
    if (entry.blake3_hash) map.set(entry.blake3_hash, originalName);
  }
  return map;
});

function vaultDisplayName(filename: string): string {
  // Try full filename, then hash prefix, then strip .tar.zst/.zst
  return vaultNameMap.value.get(filename)
    ?? vaultNameMap.value.get(filename.split(".")[0])
    ?? vaultNameMap.value.get(filename.replace(/\.tar\.zst$/, ".zst"))
    ?? filename;
}

// File type icons — cached by extension
const fileIconCache = ref<Record<string, string>>({});

async function loadFileIcon(ext: string) {
  if (fileIconCache.value[ext] || ext === "") return;
  fileIconCache.value[ext] = ""; // mark as loading
  try {
    // Use a dummy path with the extension — NSWorkspace returns the icon for that file type
    const base64 = await invoke<string>("render_sf_symbol", {
      name: ext,
      size: 64,
      mode: "uttype",
      style: "plain",
    });
    if (base64) fileIconCache.value[ext] = base64;
  } catch { /* non-critical */ }
}

function getFileIcon(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (!fileIconCache.value[ext] && ext) loadFileIcon(ext);
  return fileIconCache.value[ext] || "";
}

// Native macOS folder icon for reveal-in-finder buttons
const nativeFolderIcon = ref("");
invoke<string>("render_sf_symbol", { name: "public.folder", size: 32, mode: "uttype", style: "plain" })
  .then(b64 => { if (b64) nativeFolderIcon.value = b64; })
  .catch(e => console.warn('[large-files] folder icon load failed:', e));

// Preload common extensions on mount
const commonExts = ["png", "jpg", "log", "dmg", "qcow2", "jar", "a", "rlib", "so", "dylib", "img", "raw", "db", "f3d", "zip", "zst", "bin", "dill", "fst", "dat", "pack", "idx"];
for (const ext of commonExts) loadFileIcon(ext);

// Load vault manifest for name resolution
invoke<any[]>("get_vault_entries").then((entries) => {
  vaultEntries.value = entries;
}).catch(e => console.warn('[large-files] vault entries load failed:', e));

function toggleSelect(path: string) {
  if (isLocked(path)) return;
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  selected.value = next;
}

function toggleAll() {
  if (allSelected.value) selected.value = new Set();
  else selected.value = new Set(largeFiles.value.filter(f => !isLocked(f.path)).map((f) => f.path));
}

function toggleGroup(groupId: string) {
  const next = new Set(collapsedGroups.value);
  if (next.has(groupId)) next.delete(groupId);
  else next.add(groupId);
  collapsedGroups.value = next;
}

function toggleGroupSelect(files: FileInfo[]) {
  const selectableFiles = files.filter(f => !isLocked(f.path));
  const paths = selectableFiles.map((f) => f.path);
  const allGroupSelected = paths.every((p) => selected.value.has(p));
  const next = new Set(selected.value);
  if (allGroupSelected) {
    paths.forEach((p) => next.delete(p));
  } else {
    paths.forEach((p) => next.add(p));
  }
  selected.value = next;
}

const selectableCount = computed(() => largeFiles.value.filter(f => !isLocked(f.path)).length);
const allSelected = computed(
  () => selectableCount.value > 0 && selected.value.size >= selectableCount.value
);

const totalSelected = computed(() =>
  largeFiles.value
    .filter((f) => selected.value.has(f.path))
    .reduce((sum, f) => sum + diskSize(f), 0)
);

function isSparse(file: FileInfo): boolean {
  return file.is_sparse && file.actual_size < file.apparent_size * 0.8;
}

const diskSize = fileDiskSize;

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

/** Format a timestamp string like "2025-01-14 22:27:26" as a friendly relative time. */

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

const safetyGroupOrder = ["safe", "safe_stale", "safe_rebuild", "probably_safe", "unknown", "risky"] as const;

const safetyGroupMeta: Record<string, { label: string; description: string }> = {
  safe: { label: "Safe to Delete", description: "Logs, temporary files, and cached installers that regenerate automatically" },
  safe_stale: { label: "Safe — Older than 90 Days", description: "Caches and build artifacts not modified in 90+ days. Deleting is safe — your next build may need to re-download them." },
  safe_rebuild: { label: "Safe — Recently Modified", description: "Caches and build artifacts modified in the last 90 days. Still safe to delete but will need to rebuild or re-download." },
  probably_safe: { label: "Likely Safe", description: "Review recommended — these are usually safe but may be needed by some apps" },
  unknown: { label: "Unknown", description: "No classification available — check before deleting" },
  risky: { label: "Caution", description: "May contain user-created content or important application data" },
  vaulted: { label: "Vaulted Archives", description: "Files archived by Negativ_ Vault. Manage these from the Vault view — do not delete directly." },
};

const safetyGroupedFiles = computed<CategoryGroup[]>(() => {
  const buckets: Record<string, FileInfo[]> = { safe: [], safe_stale: [], safe_rebuild: [], probably_safe: [], unknown: [], risky: [], vaulted: [] };

  for (const file of largeFiles.value) {
    const c = getClassification(file.path);
    const safety = c?.safety ?? "unknown";
    (buckets[safety] ?? buckets.unknown).push(file);
  }

  const bySize = (a: FileInfo, b: FileInfo) => diskSize(b) - diskSize(a);
  const groups: CategoryGroup[] = [];

  for (const key of safetyGroupOrder) {
    const files = buckets[key] ?? [];
    if (files.length === 0) continue;
    const sorted = [...files].sort(bySize);
    const meta = safetyGroupMeta[key];
    groups.push({
      id: `safety-${key}`,
      label: meta.label,
      description: meta.description,
      totalSize: files.reduce((s, f) => s + diskSize(f), 0),
      totalFiles: files.length,
      flatFiles: sorted,
      tree: buildDirTree(files, `safety-${key}`),
    });
  }

  return groups;
});

// Separate vaulted files from everything else
const vaultedFiles = computed(() =>
  largeFiles.value.filter(f => isVaulted(f.path)).sort((a, b) => diskSize(b) - diskSize(a))
);

const vaultedTotalSize = computed(() =>
  vaultedFiles.value.reduce((s, f) => s + diskSize(f), 0)
);

// Filter vaulted files out of groups
const typeLabels: Record<string, string> = {
  // Libraries & compiled
  a: "Libraries", rlib: "Libraries", dylib: "Libraries", so: "Libraries", framework: "Frameworks",
  jar: "Java / Gradle", dex: "Java / Gradle",
  // Logs
  log: "Log Files",
  // Disk & VM images
  dmg: "Disk Images", iso: "Disk Images", img: "Disk Images", qcow2: "Virtual Disks", raw: "Virtual Disks", vmdk: "Virtual Disks",
  // Archives
  zst: "Compressed Archives", gz: "Compressed Archives", zip: "Compressed Archives", tar: "Archives", xz: "Compressed Archives",
  // Git
  pack: "Git Data", idx: "Git Data",
  // Binary / data
  bin: "Binary Data", dat: "Data Files", values: "Data Files",
  // Databases
  db: "Databases", sqlite: "Databases", sql: "Databases", vscdb: "Databases",
  // App data
  autosave: "Autosaves", backup: "Backups",
  // Simulator / mobile
  fst: "Simulator Data", adat: "Simulator Data", apk: "Android Packages", dill: "Flutter Data",
  // Media
  dash: "Audio / Media", mp4: "Audio / Media", mov: "Audio / Media", mp3: "Audio / Media",
  png: "Images", jpg: "Images", jpeg: "Images", heic: "Images", tiff: "Images",
  // Design / CAD
  f3d: "CAD / Design", propcol: "CAD / Design",
  // Dev tools
  ort: "ML Models",
};

// Known binary names that don't have extensions
const knownBinaries = new Set([
  "node", "claude", "opencode", "clang-18", "clang-check", "clang-scan-deps",
  "rust-lld", "lld", "trufflehog", "Flutter", "FlutterMacOS",
  "Electron Framework", "Microsoft Edge Framework", "QtWebEngineCore",
]);

function fileTypeGroup(name: string): string {
  // Check compound extensions first
  if (name.endsWith(".tar.zst")) return "Compressed Archives";
  // Known binaries without extensions
  if (knownBinaries.has(name)) return "Developer Tools";
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  // If "extension" looks like a hash, version number, or is very long — it's not a real extension
  if (!ext || ext.length > 10 || /^[0-9a-f]{8,}$/.test(ext) || /^\d+$/.test(ext)) return "Other";
  return typeLabels[ext] ?? "Other";
}

const typeGroupedFiles = computed<CategoryGroup[]>(() => {
  const buckets: Record<string, FileInfo[]> = {};
  for (const file of largeFiles.value) {
    if (isVaulted(file.path)) continue;
    const group = fileTypeGroup(file.name);
    if (!buckets[group]) buckets[group] = [];
    buckets[group].push(file);
  }

  const bySize = (a: FileInfo, b: FileInfo) => diskSize(b) - diskSize(a);
  return Object.entries(buckets)
    .map(([label, files]) => {
      const sorted = [...files].sort(bySize);
      return {
        id: `type-${label}`,
        label,
        description: `${files.length} file(s)`,
        totalSize: files.reduce((s, f) => s + diskSize(f), 0),
        totalFiles: files.length,
        flatFiles: sorted,
        tree: buildDirTree(sorted, `type-${label}`),
      };
    })
    .sort((a, b) => b.totalSize - a.totalSize);
});

const activeGroups = computed(() => {
  const base = sortMode.value === "safety" ? safetyGroupedFiles.value
    : sortMode.value === "type" ? typeGroupedFiles.value
    : groupedFiles.value;
  return base
    .map(g => {
      const filtered = g.flatFiles.filter(f => !isVaulted(f.path));
      return {
        ...g,
        flatFiles: filtered,
        totalFiles: filtered.length,
        totalSize: filtered.reduce((s, f) => s + diskSize(f), 0),
        tree: buildDirTree(filtered, g.id),
      };
    })
    .filter(g => g.flatFiles.length > 0);
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
    <div class="view-header lf-header">
      <div>
        <h2>Large Files</h2>
        <p class="text-muted">Find and remove large files taking up space</p>
      </div>
      <div class="lf-controls">
        <label for="lf-min-size" class="sr-only">Minimum size (MB)</label>
        <input id="lf-min-size" v-model.number="minSizeMb" type="number" min="1" max="10000" class="size-input" title="Minimum size (MB)" @blur="minSizeMb = Math.max(1, Math.min(10000, minSizeMb || 1))" />
        <span class="lf-controls-label text-muted">MB in</span>
        <label for="lf-scan-path" class="sr-only">Search path</label>
        <input id="lf-scan-path" v-model="scanPath" type="text" class="path-input" title="Search path" />
        <button class="btn-primary scan-btn" :disabled="largeFilesScanning" @click="scan">
          <span v-if="largeFilesScanning" class="spinner-sm"></span>
          {{ largeFilesScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <!-- FDA warning -->
    <FdaWarningBanner
      text="Without Full Disk Access, Desktop, Documents, Downloads, and other protected folders are skipped to avoid macOS permission prompts."
      @fda-granted="scan"
    />

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
          <label class="select-all-label">
            <input type="checkbox" :checked="allSelected" @change="toggleAll" />
          </label>
          <span class="results-count">{{ largeFiles.length }} file(s)</span>
          <span class="results-total-size">{{ formatSize(totalLargeFileSize) }} total</span>
          <span v-if="selected.size > 0" class="selected-info">
            {{ selected.size }} selected ({{ formatSize(totalSelected) }})
          </span>
        </div>
        <div class="results-actions">
          <div class="sort-toggle">
            <button class="sort-btn" :class="{ 'sort-btn--active': sortMode === 'size' }" @click="sortMode = 'size'">Size</button>
            <button class="sort-btn" :class="{ 'sort-btn--active': sortMode === 'directory' }" @click="sortMode = 'directory'">Directory</button>
            <button class="sort-btn" :class="{ 'sort-btn--active': sortMode === 'safety' }" @click="sortMode = 'safety'" v-if="fileClassifications.size > 0">Safety</button>
            <button class="sort-btn" :class="{ 'sort-btn--active': sortMode === 'type' }" @click="sortMode = 'type'">Type</button>
          </div>
          <button class="btn-secondary protect-action-btn" :disabled="selected.size === 0" @click="protectSelected" title="Toggle protection on selected files">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
            Protect
          </button>
          <button class="btn-secondary export-btn" :disabled="selected.size === 0" @click="exportSelected">Export</button>
          <button class="btn-danger" :disabled="selected.size === 0 || deleting" @click="deleteSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Deleting..." : "Delete Selected" }}
          </button>
        </div>
      </div>

      <!-- Vaulted archives — locked section at top -->
      <div v-if="vaultedFiles.length > 0" class="file-group vault-group">
        <div class="group-header vault-header" @click="toggleGroup('vaulted')">
          <div class="group-header-left">
            <span class="expand-chevron" :class="{ expanded: !collapsedGroups.has('vaulted') }">
              <svg viewBox="0 0 16 16" fill="currentColor"><path d="M6 3l5 5-5 5V3z"/></svg>
            </span>
            <div class="group-title-block">
              <span class="group-title vault-title">Vaulted Archives</span>
              <span class="group-meta text-muted">
                {{ vaultedFiles.length }} file(s) &middot; {{ formatSize(vaultedTotalSize) }}
              </span>
            </div>
          </div>
        </div>
        <div v-if="!collapsedGroups.has('vaulted')" class="group-description vault-description">
          Archived by Negativ_ Vault. Manage from the Vault view — do not delete directly.
        </div>
        <div v-if="!collapsedGroups.has('vaulted')" class="vault-file-list">
          <div v-for="file in vaultedFiles" :key="file.path" class="vault-file-row">
            <span class="vault-badge">Vaulted</span>
            <span class="vault-file-name">{{ vaultDisplayName(file.name) }}</span>
            <span class="vault-file-size mono">{{ formatSize(diskSize(file)) }}</span>
            <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
              <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
            </button>
          </div>
        </div>
      </div>

      <!-- Category groups -->
      <div v-for="group in activeGroups" :key="group.id" class="file-group">

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
        <template v-if="!collapsedGroups.has(group.id) && sortMode !== 'directory'">
          <div class="file-list">
            <div
              v-for="file in group.flatFiles"
              :key="file.path"
              class="file-row"
              :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
              @click="toggleSelect(file.path)"
            >
              <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <span v-if="isProtected(file.path)" class="icon-shield-badge" title="Click to unprotect" @click.stop="toggleProtected(file.path)">
                  <svg class="shield-normal" viewBox="0 0 24 24" fill="hsla(145, 55%, 45%, 1)" stroke="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                  <svg class="shield-hover" viewBox="0 0 24 24" fill="none" stroke="#d94b4b" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="4.5" y1="4.5" x2="19.5" y2="19.5" stroke="#d94b4b" stroke-width="2.5"/></svg>
                </span>
              </div>
              <div class="file-row-info">
                <div class="file-row-name">
                  <span class="file-name">{{ file.name }}</span>
                  <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                </div>
                <div class="file-row-path">{{ parentFolder(file.path) }}</div>
              </div>
              <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="safety-pill" :data-tooltip="getClassification(file.path)?.explanation || ''" :style="{ background: safetyColor(getClassification(file.path)?.safety ?? 'unknown') }">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
              <div class="file-row-size mono">
                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
              </div>
              <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
              </button>
              <div class="file-row-check">
                <input v-if="!isLocked(file.path)" type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
              </div>
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
                                      <div v-if="!collapsedGroups.has(d3.key)" class="file-list file-list-indented">
                                        <div
                                          v-for="file in collectFiles(d3)"
                                          :key="file.path"
                                          class="file-row file-row--tree"
                                          :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                                          @click="toggleSelect(file.path)"
                                        >
                                          <div class="file-row-check">
                                            <input v-if="!isLocked(file.path)" type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                                          </div>
                                          <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="hsla(145, 55%, 45%, 1)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
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
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                                          </div>
                                          <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="safety-pill" :style="{ background: safetyColor(getClassification(file.path)?.safety ?? 'unknown') }" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                                          <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                            <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
                                          </button>
                                        </div>
                                      </div>
                                    </div>
                                  </template>

                                  <!-- Files at depth 2 -->
                                  <div v-if="d2.files.length > 0" class="file-list file-list-indented">
                                    <div
                                      v-for="file in d2.files"
                                      :key="file.path"
                                      class="file-row file-row--tree"
                                      :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                                      @click="toggleSelect(file.path)"
                                    >
                                      <div class="file-row-check">
                                        <input v-if="!isLocked(file.path)" type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                                      </div>
                                      <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="hsla(145, 55%, 45%, 1)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
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
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                                      </div>
                                      <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="safety-pill" :style="{ background: safetyColor(getClassification(file.path)?.safety ?? 'unknown') }" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                                      <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                        <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
                                      </button>
                                    </div>
                                  </div>
                                </template>
                              </div>
                            </div>
                          </template>

                          <!-- Files at depth 1 -->
                          <div v-if="d1.files.length > 0" class="file-list file-list-indented">
                            <div
                              v-for="file in d1.files"
                              :key="file.path"
                              class="file-row file-row--tree"
                              :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                              @click="toggleSelect(file.path)"
                            >
                              <div class="file-row-check">
                                <input v-if="!isLocked(file.path)" type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                              </div>
                              <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="hsla(145, 55%, 45%, 1)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
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
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                              </div>
                              <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="safety-pill" :style="{ background: safetyColor(getClassification(file.path)?.safety ?? 'unknown') }" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                              <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                                <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
                              </button>
                            </div>
                          </div>
                        </template>
                      </div>
                    </div>
                  </template>

                  <!-- Files at depth 0 (directly under top-level dir) -->
                  <div v-if="child.files.length > 0" class="file-list file-list-indented">
                    <div
                      v-for="file in child.files"
                      :key="file.path"
                      class="file-row file-row--tree"
                      :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
                      @click="toggleSelect(file.path)"
                    >
                      <div class="file-row-check">
                        <input v-if="!isLocked(file.path)" type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
                      </div>
                      <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <svg v-if="isProtected(file.path)" class="icon-shield-badge" viewBox="0 0 24 24" fill="hsla(145, 55%, 45%, 1)" stroke="none" title="Click to unprotect" @click.stop="toggleProtected(file.path)"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
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
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
                      </div>
                      <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="safety-pill" :style="{ background: safetyColor(getClassification(file.path)?.safety ?? 'unknown') }" :title="getClassification(file.path)?.explanation || ''">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
                      <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                        <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
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
              :class="{ 'file-row--selected': selected.has(file.path), 'file-row--protected': isProtected(file.path) }"
              @click="toggleSelect(file.path)"
            >
              <div class="file-icon-wrap">
                <img v-if="getFileIcon(file.name)" :src="getFileIcon(file.name)" alt="" class="file-row-icon" width="32" height="32" />
                <div v-else class="file-row-icon-placeholder"></div>
                <span v-if="isProtected(file.path)" class="icon-shield-badge" title="Click to unprotect" @click.stop="toggleProtected(file.path)">
                  <svg class="shield-normal" viewBox="0 0 24 24" fill="hsla(145, 55%, 45%, 1)" stroke="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                  <svg class="shield-hover" viewBox="0 0 24 24" fill="none" stroke="#d94b4b" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="4.5" y1="4.5" x2="19.5" y2="19.5" stroke="#d94b4b" stroke-width="2.5"/></svg>
                </span>
              </div>
              <div class="file-row-info">
                <div class="file-row-name">
                  <span class="file-name">{{ file.name }}</span>
                  <span v-if="isSparse(file)" class="badge badge-warning badge-pill sparse-badge">Sparse</span>
                </div>
                <div class="file-row-path">{{ parentFolder(file.path) }}</div>
              </div>
              <span v-if="getClassification(file.path) && safetyLabel(getClassification(file.path)?.safety ?? 'unknown')" class="safety-pill" :data-tooltip="getClassification(file.path)?.explanation || ''" :style="{ background: safetyColor(getClassification(file.path)?.safety ?? 'unknown') }">{{ safetyLabel(getClassification(file.path)?.safety ?? 'unknown') }}</span>
              <span v-else class="safety-pill-placeholder"></span>
              <div class="file-row-size mono">
                <span class="size-value">{{ formatSize(diskSize(file)) }}</span>
                <span v-if="isSparse(file)" class="sparse-logical text-muted">{{ formatSize(file.apparent_size) }} logical</span>
                <span v-if="file.modified" class="file-time-ago text-muted">{{ timeAgo(file.modified) }}</span>
              </div>
              <button class="reveal-btn" title="Reveal in Finder" @click.stop="revealInFinder(file.path)">
                <img v-if="nativeFolderIcon" :src="nativeFolderIcon" alt="" width="16" height="16" /><svg v-else viewBox="0 0 20 20" fill="currentColor"><path d="M2 4.5A1.5 1.5 0 013.5 3h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0010.621 5H16.5A1.5 1.5 0 0118 6.5v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5v-10z"/></svg>
              </button>
              <div class="file-row-check">
                <input v-if="!isLocked(file.path)" type="checkbox" :checked="selected.has(file.path)" @click.stop @change="toggleSelect(file.path)" />
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.large-files { max-width: 1440px; }
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
  margin-bottom: var(--sp-4);
  padding: 0;
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
  color: white;
}

.sort-btn--active:hover {
  background: var(--accent-hover);
}

/* ---- Category group ---- */
.file-group {
  margin-bottom: var(--sp-4);
  background: rgba(255, 255, 255, 0.3);
  border-radius: 12px;
  border: 0.5px solid rgba(255, 255, 255, 0.5);
  overflow: hidden;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.15s ease;
  user-select: none;
}

.group-header:hover {
  background: rgba(255, 255, 255, 0.2);
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
  font-size: 11px;
  padding: 0 16px 8px 16px;
  color: rgba(0, 0, 0, 0.85);
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
  padding: 6px 16px 6px 12px;
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
  display: grid;
  grid-template-columns: auto 100px 24px 28px;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.dir-meta {
  justify-self: end;
}

.dir-size {
  justify-self: end;
}

.dir-header-right .group-check {
  grid-column: 4;
  justify-self: center;
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

.expand-chevron--sm svg {
  width: 10px;
  height: 10px;
}

/* ---- File list ---- */
.file-list {
  display: flex;
  flex-direction: column;
}

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

.file-row--tree .reveal-btn {
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
  background: rgba(59, 199, 232, 0.06);
}

.file-row--selected:hover {
  background: rgba(59, 199, 232, 0.10);
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

.file-row--protected > *:not(.file-icon-wrap) {
  opacity: 0.4;
}

.file-row--protected .file-row-icon,
.file-row--protected .file-row-icon-placeholder {
  opacity: 0.4;
}

.file-row--protected .icon-shield-badge {
  opacity: 1;
}

.protect-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.protect-action-btn svg {
  color: hsla(145, 55%, 45%, 0.9);
}

.file-row-name {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}

.file-name {
  font-family: "SF Pro Display", "SF Pro Text", -apple-system, sans-serif; font-size: 13px;
  font-weight: 500;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sparse-badge {
  flex-shrink: 0;
}

/* Header with inline controls */
.lf-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.lf-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.lf-controls .size-input {
  width: 60px;
}

.lf-controls .path-input {
  width: 120px;
}

.lf-controls-label {
  font-size: 11px;
  white-space: nowrap;
}

/* Vault section */
.vault-group {
  border: 1px solid hsla(280, 40%, 60%, 0.15);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.55);
  padding: 4px 0;
  margin-bottom: var(--sp-4);
}

.vault-title {
  color: hsla(280, 40%, 45%, 0.9);
}

.vault-description {
  color: hsla(280, 35%, 35%, 0.7);
  font-style: italic;
  font-size: 11px;
}

.vault-file-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 0;
}

.vault-file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  border-bottom: 1px solid hsla(280, 20%, 50%, 0.08);
}

.vault-file-row:hover {
  background: hsla(280, 30%, 70%, 0.08);
  opacity: 0.8;
}

.vault-badge {
  color: hsla(280, 40%, 50%, 0.85);
  border-color: hsla(280, 40%, 50%, 0.35);
  font-size: 9px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 6px;
  border: 1px solid;
  flex-shrink: 0;
  white-space: nowrap;
}

.vault-file-name {
  font-size: 12px;
  color: hsla(280, 30%, 30%, 0.8);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vault-file-size {
  font-size: 12px;
  color: hsla(280, 30%, 30%, 0.6);
  flex-shrink: 0;
}

.safety-badge {
  display: none;
}

.safety-pill {
  grid-column: 3;
  justify-self: end;
  position: relative;
  font-size: 9px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 10px;
  color: white;
  line-height: 1.4;
  letter-spacing: 0.02em;
  white-space: nowrap;
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

.file-row-path {
  font-family: "SF Pro Display", "SF Pro Text", -apple-system, sans-serif; font-size: 10px;
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

/* ---- Protect toggle button ---- */
.protect-btn {
  grid-column: 6;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: hsla(280, 40%, 55%, 0.7);
  opacity: 0;
  cursor: pointer;
  flex-shrink: 0;
  transition: opacity 0.15s, color 0.15s;
}

.file-row:hover .protect-btn {
  opacity: 0.7;
}

.protect-btn svg {
  width: 16px;
  height: 16px;
}

.protect-btn:hover {
  opacity: 0.8;
}

.protect-btn--active {
  color: hsla(280, 40%, 50%, 0.85);
  opacity: 0.85;
}

.protect-btn--active:hover {
  opacity: 1;
}

/* ---- Reveal in Finder button ---- */
.reveal-btn {
  grid-column: 5;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  opacity: 0;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease, opacity 0.15s;
  flex-shrink: 0;
}

.reveal-btn svg,
.reveal-btn img {
  width: 16px;
  height: 16px;
}

.file-row:hover .reveal-btn {
  opacity: 0.8;
}

.reveal-btn:hover {
  background: rgba(255, 255, 255, 0.5);
  opacity: 1;
}

.reveal-btn:active {
  transform: scale(0.92);
}

.file-list-indented { padding-left: var(--sp-5); }

/* Screen-reader-only: visually hidden but accessible to assistive tech */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>

