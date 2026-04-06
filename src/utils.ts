import { invoke } from "@tauri-apps/api/core";

const SPARSE_THRESHOLD = 0.8;

/** Compute actual disk size for a file, respecting sparse files. */
export function fileDiskSize(f: { is_sparse: boolean; actual_size: number; apparent_size: number }): number {
  const sparse = f.is_sparse && f.actual_size < f.apparent_size * SPARSE_THRESHOLD;
  return sparse ? f.actual_size : f.apparent_size;
}

// Temperature thresholds (degrees Celsius)
export const TEMP_CRITICAL = 95;
export const TEMP_HOT = 80;
export const TEMP_WARM = 65;
export const TEMP_COOL = 45;

/** Map a temperature to an HSLA color string. */
export function tempToColor(t: number): string {
  if (t >= TEMP_CRITICAL) return "hsla(0, 50%, 48%, 0.85)";
  if (t >= TEMP_HOT) return "hsla(25, 55%, 45%, 0.85)";
  if (t >= TEMP_WARM) return "hsla(40, 55%, 45%, 0.85)";
  if (t >= TEMP_COOL) return "hsla(160, 35%, 42%, 0.85)";
  return "hsla(195, 35%, 42%, 0.85)";
}

// ---------------------------------------------------------------------------
// Health thresholds — single source of truth for color mapping
// ---------------------------------------------------------------------------

/** CPU load color based on usage percentage. */
export function cpuLoadColor(pct: number): string {
  if (pct > 80) return "var(--danger)";
  if (pct > 50) return "var(--warning)";
  return "var(--accent)";
}

/** CPU load CSS class based on usage percentage. */
export function cpuLoadClass(pct: number): string {
  if (pct > 80) return "cpu-hot";
  if (pct > 50) return "cpu-warm";
  return "";
}

export type MemoryPressureLevel = { label: string; cssClass: string };

/** Memory pressure level based on used percentage (4-tier). */
export function memoryPressureLevel(usedPct: number): MemoryPressureLevel {
  if (usedPct >= 90) return { label: "Critical", cssClass: "pressure-critical" };
  if (usedPct >= 75) return { label: "High", cssClass: "pressure-high" };
  if (usedPct >= 50) return { label: "Moderate", cssClass: "pressure-moderate" };
  return { label: "Low", cssClass: "pressure-low" };
}

/** Memory pressure status-dot variant based on used percentage. */
export function memoryPressureDotClass(usedPct: number): string {
  if (usedPct >= 90) return "dot-danger";
  if (usedPct >= 75) return "dot-warning";
  return "dot-success";
}

/** Fan speed color based on percentage of max RPM. */
export function fanSpeedColor(pct: number): string {
  if (pct >= 80) return "var(--temp-critical)";
  if (pct >= 50) return "var(--temp-warm)";
  return "var(--temp-cool)";
}

/** Fan gauge zone name based on percentage. */
export function fanSpeedZone(pct: number): string {
  if (pct >= 80) return "critical";
  if (pct >= 50) return "serious";
  if (pct >= 25) return "fair";
  return "nominal";
}

/** Storage usage color based on percentage. */
export function storageColor(pct: number): string {
  if (pct > 90) return "var(--danger)";
  if (pct > 75) return "var(--warning)";
  return "var(--accent)";
}

/** Format a date string or Date as a human-readable relative time. */
export function timeAgo(modified: string | Date | null): string {
  if (!modified) return "";
  const date = typeof modified === "string"
    ? new Date(modified.replace(" ", "T"))
    : modified;
  if (isNaN(date.getTime())) return typeof modified === "string" ? modified : "";
  const diff = Date.now() - date.getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days === 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  if (days < 14) return "a week ago";
  if (days < 30) return `${Math.floor(days / 7)} weeks ago`;
  if (days < 60) return "a month ago";
  if (days < 365) return `${Math.floor(days / 30)} months ago`;
  const years = Math.floor(days / 365);
  return years === 1 ? "a year ago" : `${years} years ago`;
}

// ---------------------------------------------------------------------------
// CSS token reader — reads custom properties from :root with fallback
// ---------------------------------------------------------------------------

/** Read a CSS custom property value from :root, with a fallback. */
function cssVar(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return v || fallback;
}

// ---------------------------------------------------------------------------
// Shared color palettes — single source of truth, backed by CSS tokens
// ---------------------------------------------------------------------------

/** File-kind colors used by the Duplicates extension-card display. */
export const KIND_COLORS: Record<string, string> = {
  documents: cssVar("--kind-documents", "#E74C3C"),
  code: cssVar("--kind-code", "#3498DB"),
  archives: cssVar("--kind-archives", "#F39C12"),
  audio: cssVar("--kind-audio", "#9B59B6"),
  video: cssVar("--kind-video", "#27AE60"),
};
export const KIND_COLOR_DEFAULT = cssVar("--kind-default", "#95A5A6");

/** Memory process category colors (Memory.vue group badges + dots). */
export const MEMORY_CATEGORY_COLORS: Record<string, string> = {
  app:         cssVar("--cat-app",         "#00b4d8"),
  system:      cssVar("--cat-system",      "#6c7086"),
  display:     cssVar("--cat-display",     "#a78bfa"),
  networking:  cssVar("--cat-networking",  "#38bdf8"),
  security:    cssVar("--cat-security",    "#f97316"),
  storage:     cssVar("--cat-storage",     "#84cc16"),
  icloud:      cssVar("--cat-icloud",      "#60a5fa"),
  audio:       cssVar("--cat-audio",       "#e879f9"),
  input:       cssVar("--cat-input",       "#a3a3a3"),
  developer:   cssVar("--cat-developer",   "#34d399"),
  background:  cssVar("--cat-background",  "#94a3b8"),
};

/** Memory bar segment colors (system memory breakdown). */
export const MEMORY_BAR_COLORS = {
  app:        cssVar("--mem-app",        "#00b4d8"),
  wired:      cssVar("--mem-wired",      "#f97316"),
  compressed: cssVar("--mem-compressed", "#a78bfa"),
  inactive:   cssVar("--mem-inactive",   "#94a3b8"),
  free:       cssVar("--mem-free",       "#30d158"),
};

/** SpaceMap category fill colors (sunburst legend). */
export const SPACEMAP_CATEGORY_FILLS: Record<string, string> = {
  applications:   cssVar("--space-applications",   "#e53935"),
  documents:      cssVar("--space-documents",      "#f57c00"),
  developer:      cssVar("--space-developer",      "#f9a825"),
  books:          cssVar("--space-books",          "#43a047"),
  icloud:         cssVar("--space-icloud",         "#2196f3"),
  ios_files:      cssVar("--space-ios-files",      "#78909c"),
  mail:           cssVar("--space-mail",           "#1e88e5"),
  messages:       cssVar("--space-messages",       "#43a047"),
  music:          cssVar("--space-music",          "#e53935"),
  music_creation: cssVar("--space-music-creation", "#78909c"),
  photos:         cssVar("--space-photos",         "#ad1457"),
  media:          cssVar("--space-media",          "#8e24aa"),
  bin:            cssVar("--space-bin",            "#78909c"),
  podcasts:       cssVar("--space-podcasts",       "#7b1fa2"),
  other_users:    cssVar("--space-other-users",    "#78909c"),
  docker:         cssVar("--space-docker",         "#00acc1"),
  caches:         cssVar("--space-caches",         "#fb8c00"),
  macos:          cssVar("--space-macos",          "#5c6bc0"),
  system_data:    cssVar("--space-system-data",    "#546e7a"),
  system:         cssVar("--space-system",         "#5c6bc0"),
  other:          cssVar("--space-other",          "#78909c"),
};

/** SpaceMap overview-mode category colors (macOS-style stacked bar). */
export const OVERVIEW_CATEGORY_COLORS: Record<string, string> = {
  applications: "hsla(0, 65%, 58%, 0.9)",
  documents:    "hsla(30, 75%, 55%, 0.9)",
  developer:    "hsla(210, 15%, 55%, 0.85)",
  books:        "hsla(25, 75%, 55%, 0.9)",
  icloud:       "hsla(210, 70%, 55%, 0.9)",
  ios_files:    "hsla(220, 8%, 55%, 0.7)",
  mail:         "hsla(215, 60%, 55%, 0.9)",
  messages:     "hsla(145, 55%, 45%, 0.9)",
  music:        "hsla(0, 65%, 55%, 0.9)",
  music_creation: "hsla(220, 8%, 55%, 0.7)",
  photos:       "hsla(340, 55%, 55%, 0.9)",
  media:        "hsla(280, 40%, 55%, 0.85)",
  bin:          "hsla(220, 8%, 60%, 0.7)",
  podcasts:     "hsla(280, 50%, 50%, 0.9)",
  docker:       "hsla(195, 55%, 48%, 0.85)",
  caches:       "hsla(35, 50%, 50%, 0.8)",
  other_users:  "hsla(220, 8%, 55%, 0.7)",
  macos:        "hsla(220, 10%, 50%, 0.75)",
  system_data:  "hsla(220, 8%, 48%, 0.7)",
  other:        "hsla(220, 8%, 58%, 0.6)",
  free:         "hsla(0, 0%, 85%, 0.3)",
};

/** Dashboard waffle chart category colors (HSLA, opacity-tuned for waffle). */
export const DASHBOARD_CATEGORY_COLORS: Record<string, string> = {
  applications: "hsla(0, 65%, 55%, 0.8)",
  documents:    "hsla(35, 75%, 55%, 0.8)",
  developer:    "hsla(45, 80%, 50%, 0.8)",
  books:        "hsla(145, 50%, 45%, 0.8)",
  icloud:       "hsla(210, 65%, 55%, 0.8)",
  mail:         "hsla(210, 60%, 55%, 0.8)",
  photos:       "hsla(320, 45%, 55%, 0.8)",
  media:        "hsla(280, 40%, 55%, 0.8)",
  bin:          "hsla(220, 10%, 55%, 0.6)",
  docker:       "hsla(195, 55%, 45%, 0.8)",
  caches:       "hsla(35, 45%, 50%, 0.7)",
  macos:        "hsla(220, 15%, 50%, 0.65)",
  system_data:  "hsla(220, 10%, 45%, 0.6)",
  system:       "hsla(220, 15%, 50%, 0.65)",
  other:        "hsla(220, 10%, 60%, 0.45)",
  free:         "hsla(0, 0%, 88%, 0.35)",
};

/** Open a path in Finder. Silently catches errors (best-effort). */
export async function revealInFinder(path: string): Promise<void> {
  try { await invoke("reveal_in_finder", { path }); } catch (_) {}
}

/** Extract lowercase file extension from a filename. */
export function getFileExtension(name: string): string {
  return name.split(".").pop()?.toLowerCase() ?? "";
}

export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";

  const units = ["B", "KB", "MB", "GB", "TB"];
  const bytesPerUnit = 1024;
  const unitIndex = Math.min(
    Math.floor(Math.log(bytes) / Math.log(bytesPerUnit)),
    units.length - 1,
  );
  const value = bytes / Math.pow(bytesPerUnit, unitIndex);

  return `${value.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}
