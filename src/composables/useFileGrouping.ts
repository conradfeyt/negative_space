/**
 * Composable that groups, sorts, and categorizes a flat list of FileInfo[]
 * into CategoryGroup[] based on a switchable sort mode.
 *
 * Extracted from LargeFiles.vue to keep the view focused on UI concerns.
 */
import { ref, computed, type Ref } from "vue";
import type { FileInfo } from "../types";
import { fileDiskSize, getFileExtension } from "../utils";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type SortMode = "size" | "directory" | "safety" | "type";
export type FileCategory = "user" | "system";

/** A node in the directory tree. Internal nodes have children and/or files. */
export interface DirNode {
  name: string;
  path: string;
  key: string;
  children: DirNode[];
  files: FileInfo[];
  totalSize: number;
  totalFiles: number;
}

export interface CategoryGroup {
  id: string;
  label: string;
  description: string;
  totalSize: number;
  totalFiles: number;
  flatFiles: FileInfo[];
  tree: DirNode;
}

// ---------------------------------------------------------------------------
// Constants
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

const typeLabels: Record<string, string> = {
  a: "Libraries", rlib: "Libraries", dylib: "Libraries", so: "Libraries", framework: "Frameworks",
  jar: "Java / Gradle", dex: "Java / Gradle",
  log: "Log Files",
  dmg: "Disk Images", iso: "Disk Images", img: "Disk Images", qcow2: "Virtual Disks", raw: "Virtual Disks", vmdk: "Virtual Disks",
  zst: "Compressed Archives", gz: "Compressed Archives", zip: "Compressed Archives", tar: "Archives", xz: "Compressed Archives",
  pack: "Git Data", idx: "Git Data",
  bin: "Binary Data", dat: "Data Files", values: "Data Files",
  db: "Databases", sqlite: "Databases", sql: "Databases", vscdb: "Databases",
  autosave: "Autosaves", backup: "Backups",
  fst: "Simulator Data", adat: "Simulator Data", apk: "Android Packages", dill: "Flutter Data",
  dash: "Audio / Media", mp4: "Audio / Media", mov: "Audio / Media", mp3: "Audio / Media",
  png: "Images", jpg: "Images", jpeg: "Images", heic: "Images", tiff: "Images",
  f3d: "CAD / Design", propcol: "CAD / Design",
  ort: "ML Models",
};

const knownBinaries = new Set([
  "node", "claude", "opencode", "clang-18", "clang-check", "clang-scan-deps",
  "rust-lld", "lld", "trufflehog", "Flutter", "FlutterMacOS",
  "Electron Framework", "Microsoft Edge Framework", "QtWebEngineCore",
]);

const safetyGroupOrder = ["safe", "safe_stale", "safe_rebuild", "probably_safe", "unknown", "risky"] as const;

const safetyGroupMeta: Record<string, { label: string; description: string }> = {
  safe: { label: "Safe to Delete", description: "Logs, temporary files, and cached installers that regenerate automatically" },
  safe_stale: { label: "Safe — Older than 90 Days", description: "Caches and build artifacts not modified in 90+ days. Deleting is safe — your next build may need to re-download them." },
  safe_rebuild: { label: "Safe — Recently Modified", description: "Caches and build artifacts modified in the last 90 days. Still safe to delete but will need to rebuild or re-download." },
  probably_safe: { label: "Likely Safe", description: "Review recommended — these are usually safe but may be needed by some apps" },
  unknown: { label: "Unknown", description: "No classification available — check before deleting" },
  risky: { label: "Caution", description: "May contain user-created content or important application data" },
  archived: { label: "Archived", description: "Files compressed by Negativ_ Archive. Manage from the Archive view — do not delete directly." },
};

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

const diskSize = fileDiskSize;

function hasHiddenPathComponent(path: string): boolean {
  return /\/\.[a-zA-Z0-9]/.test(path);
}

export function categorize(file: FileInfo): FileCategory {
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

export function displayPath(path: string): string {
  const home = path.match(/^\/Users\/[^/]+/);
  if (home) return path.replace(home[0], "~");
  return path;
}

export function parentFolder(path: string): string {
  const display = displayPath(path);
  const lastSlash = display.lastIndexOf("/");
  if (lastSlash <= 0) return display;
  return display.substring(0, lastSlash);
}

export function fileTypeGroup(name: string): string {
  if (name.endsWith(".tar.zst")) return "Compressed Archives";
  if (knownBinaries.has(name)) return "Developer Tools";
  const ext = getFileExtension(name);
  if (!ext || ext.length > 10 || /^[0-9a-f]{8,}$/.test(ext) || /^\d+$/.test(ext)) return "Other";
  return typeLabels[ext] ?? "Other";
}

// ---------------------------------------------------------------------------
// Directory tree builder
// ---------------------------------------------------------------------------

export function buildDirTree(files: FileInfo[], keyPrefix: string): DirNode {
  const root: DirNode = {
    name: "",
    path: "",
    key: keyPrefix,
    children: [],
    files: [],
    totalSize: 0,
    totalFiles: 0,
  };

  const nodeMap = new Map<string, DirNode>();
  nodeMap.set("", root);

  function getOrCreateNode(dirPath: string): DirNode {
    if (nodeMap.has(dirPath)) return nodeMap.get(dirPath)!;
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

  for (const file of files) {
    const fileDirPath = parentFolder(file.path);
    const dirNode = getOrCreateNode(fileDirPath);
    dirNode.files.push(file);
  }

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
    node.files.sort((a, b) => diskSize(b) - diskSize(a));
    node.totalSize = size;
    node.totalFiles = count;
  }
  computeSizes(root);

  function collapse(node: DirNode): void {
    for (const child of node.children) {
      collapse(child);
    }
    while (node.children.length === 1 && node.files.length === 0) {
      const onlyChild = node.children[0];
      if (node.name) {
        node.name = node.name + "/" + onlyChild.name;
      } else {
        node.name = onlyChild.name;
      }
      node.path = onlyChild.path;
      node.key = onlyChild.key;
      node.children = onlyChild.children;
      node.files = onlyChild.files;
    }
    node.children.sort((a, b) => b.totalSize - a.totalSize);
  }
  collapse(root);

  return root;
}

/** Collect all FileInfo objects from a DirNode subtree */
export function collectFiles(node: DirNode): FileInfo[] {
  const result: FileInfo[] = [...node.files];
  for (const child of node.children) {
    result.push(...collectFiles(child));
  }
  return result;
}

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export interface UseFileGroupingOptions {
  files: Ref<FileInfo[]>;
  /** Return the safety classification string for a file path, or undefined */
  getClassification: (path: string) => { safety: string } | undefined;
  /** Return true if a file has been archived */
  isArchived: (path: string) => boolean;
}

export function useFileGrouping(opts: UseFileGroupingOptions) {
  const { files, getClassification, isArchived } = opts;

  const sortMode = ref<SortMode>("size");

  const bySize = (a: FileInfo, b: FileInfo) => diskSize(b) - diskSize(a);

  // User / System grouping
  const groupedFiles = computed<CategoryGroup[]>(() => {
    const userFiles: FileInfo[] = [];
    const systemFiles: FileInfo[] = [];

    for (const file of files.value) {
      if (categorize(file) === "user") {
        userFiles.push(file);
      } else {
        systemFiles.push(file);
      }
    }

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

  // Safety grouping
  const safetyGroupedFiles = computed<CategoryGroup[]>(() => {
    const buckets: Record<string, FileInfo[]> = { safe: [], safe_stale: [], safe_rebuild: [], probably_safe: [], unknown: [], risky: [], archived: [] };

    for (const file of files.value) {
      const c = getClassification(file.path);
      const safety = c?.safety ?? "unknown";
      (buckets[safety] ?? buckets.unknown).push(file);
    }

    const groups: CategoryGroup[] = [];
    for (const key of safetyGroupOrder) {
      const bucket = buckets[key] ?? [];
      if (bucket.length === 0) continue;
      const sorted = [...bucket].sort(bySize);
      const meta = safetyGroupMeta[key];
      groups.push({
        id: `safety-${key}`,
        label: meta.label,
        description: meta.description,
        totalSize: bucket.reduce((s, f) => s + diskSize(f), 0),
        totalFiles: bucket.length,
        flatFiles: sorted,
        tree: buildDirTree(bucket, `safety-${key}`),
      });
    }

    return groups;
  });

  // Type grouping
  const typeGroupedFiles = computed<CategoryGroup[]>(() => {
    const buckets: Record<string, FileInfo[]> = {};
    for (const file of files.value) {
      if (isArchived(file.path)) continue;
      const group = fileTypeGroup(file.name);
      if (!buckets[group]) buckets[group] = [];
      buckets[group].push(file);
    }

    return Object.entries(buckets)
      .map(([label, bucket]) => {
        const sorted = [...bucket].sort(bySize);
        return {
          id: `type-${label}`,
          label,
          description: `${bucket.length} file(s)`,
          totalSize: bucket.reduce((s, f) => s + diskSize(f), 0),
          totalFiles: bucket.length,
          flatFiles: sorted,
          tree: buildDirTree(sorted, `type-${label}`),
        };
      })
      .sort((a, b) => b.totalSize - a.totalSize);
  });

  const archivedFiles = computed(() =>
    files.value.filter(f => isArchived(f.path)).sort((a, b) => diskSize(b) - diskSize(a))
  );

  const archivedTotalSize = computed(() =>
    archivedFiles.value.reduce((s, f) => s + diskSize(f), 0)
  );

  const activeGroups = computed(() => {
    const base = sortMode.value === "safety" ? safetyGroupedFiles.value
      : sortMode.value === "type" ? typeGroupedFiles.value
      : groupedFiles.value;
    return base
      .map(g => {
        const filtered = g.flatFiles.filter(f => !isArchived(f.path));
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
    files.value.reduce((sum, f) => sum + diskSize(f), 0)
  );

  return {
    sortMode,
    activeGroups,
    groupedFiles,
    safetyGroupedFiles,
    typeGroupedFiles,
    archivedFiles,
    archivedTotalSize,
    totalLargeFileSize,
  };
}
