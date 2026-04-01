/**
 * Shared composable to read scan area preferences from localStorage.
 * Used by Dashboard (Scan All) and LargeFiles to know which paths to skip.
 */

// Default scan areas — must match the ids in Settings.vue
const DEFAULT_AREAS: Record<string, { path: string; enabled: boolean }> = {
  desktop: { path: "~/Desktop", enabled: true },
  documents: { path: "~/Documents", enabled: true },
  downloads: { path: "~/Downloads", enabled: true },
  pictures: { path: "~/Pictures", enabled: true },
  movies: { path: "~/Movies", enabled: true },
  music: { path: "~/Music", enabled: true },
  library: { path: "~/Library", enabled: true },
  applications: { path: "/Applications", enabled: true },
};

/**
 * Returns an array of paths that the user has disabled in Settings.
 * These should be passed as `skip_paths` to the `scan_large_files` command.
 */
export function getDisabledPaths(): string[] {
  const disabled: string[] = [];

  try {
    const saved = localStorage.getItem("negative_space_scan_areas");
    if (saved) {
      const parsed = JSON.parse(saved) as Record<string, boolean>;
      for (const [id, area] of Object.entries(DEFAULT_AREAS)) {
        if (parsed[id] === false) {
          disabled.push(area.path);
        }
      }
    }
  } catch (_) {
    // ignore errors, return empty (scan everything)
  }

  return disabled;
}
