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

/** File-kind colors used by the Duplicates extension-card display. */
export const KIND_COLORS: Record<string, string> = {
  documents: "#E74C3C",
  code: "#3498DB",
  archives: "#F39C12",
  audio: "#9B59B6",
  video: "#27AE60",
};
export const KIND_COLOR_DEFAULT = "#95A5A6";

/** Open a path in Finder. Silently catches errors (best-effort). */
export async function revealInFinder(path: string): Promise<void> {
  try { await invoke("reveal_in_finder", { path }); } catch (_) {}
}

export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";

  const units = ["B", "KB", "MB", "GB", "TB"];
  const k = 1024;
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const index = Math.min(i, units.length - 1);
  const value = bytes / Math.pow(k, index);

  return `${value.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}
