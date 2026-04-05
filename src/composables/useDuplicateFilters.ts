/**
 * Composable for filtering duplicate file groups by file kind.
 *
 * Extracted from Duplicates.vue to keep the view focused on UI concerns.
 */
import { ref, computed, type Ref } from "vue";
import { getFileExtension, KIND_COLORS, KIND_COLOR_DEFAULT } from "../utils";
import type { DuplicateGroup, DuplicateScanResult } from "../types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type FileKind = "all" | "images" | "documents" | "audio" | "video" | "archives" | "code" | "other";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const KIND_EXTENSIONS: Record<Exclude<FileKind, "all" | "other">, string[]> = {
  images: ["jpg", "jpeg", "png", "gif", "webp", "heic", "heif", "tiff", "tif", "bmp", "svg", "ico", "raw", "cr2", "nef", "arw", "dng", "psd", "ai"],
  documents: ["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "rtf", "csv", "pages", "numbers", "keynote", "odt", "ods", "odp", "epub", "md"],
  audio: ["mp3", "wav", "aac", "flac", "ogg", "m4a", "wma", "aiff", "alac", "opus"],
  video: ["mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "ts", "vob"],
  archives: ["zip", "tar", "gz", "tgz", "rar", "7z", "dmg", "iso", "pkg", "deb", "bz2", "xz", "zst"],
  code: ["js", "ts", "jsx", "tsx", "py", "rs", "go", "java", "c", "cpp", "h", "hpp", "swift", "rb", "php", "css", "scss", "html", "json", "xml", "yaml", "yml", "toml", "sh", "sql", "vue", "svelte"],
};

export const KIND_LABELS: Record<FileKind, string> = {
  all: "All",
  images: "Images",
  documents: "Documents",
  audio: "Audio",
  video: "Video",
  archives: "Archives",
  code: "Code",
  other: "Other",
};

// ---------------------------------------------------------------------------
// Pure helpers (exported for reuse)
// ---------------------------------------------------------------------------

export function getFileKind(name: string): FileKind {
  const ext = getFileExtension(name);
  for (const [kind, exts] of Object.entries(KIND_EXTENSIONS)) {
    if (exts.includes(ext)) return kind as FileKind;
  }
  return "other";
}

export function extCardColor(name: string): string {
  return KIND_COLORS[getFileKind(name)] ?? KIND_COLOR_DEFAULT;
}

export function isImageFile(name: string): boolean {
  return getFileKind(name) === "images";
}

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export function useDuplicateFilters(duplicateResult: Ref<DuplicateScanResult | null>) {
  const activeKindFilter = ref<FileKind>("all");

  const kindCounts = computed(() => {
    if (!duplicateResult.value) return {};
    const counts: Record<string, { groups: number; wasted: number }> = {};
    for (const group of duplicateResult.value.groups) {
      const kind = getFileKind(group.files[0].name);
      if (!counts[kind]) counts[kind] = { groups: 0, wasted: 0 };
      counts[kind].groups++;
      counts[kind].wasted += group.wasted_bytes;
    }
    return counts;
  });

  const filteredGroups = computed<DuplicateGroup[]>(() => {
    if (!duplicateResult.value) return [];
    if (activeKindFilter.value === "all") return duplicateResult.value.groups;
    return duplicateResult.value.groups.filter(
      (g) => getFileKind(g.files[0].name) === activeKindFilter.value
    );
  });

  return {
    activeKindFilter,
    kindCounts,
    filteredGroups,
  };
}
