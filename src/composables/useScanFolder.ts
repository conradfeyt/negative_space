/**
 * useScanFolder — shared folder picker logic for scan views.
 *
 * Provides a reactive scan path, display name, native icon resolution,
 * and a folder picker dialog. Used by LargeFiles, SensitiveContent, and
 * any future view that needs a target directory selector.
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

const KNOWN_DIRS: Record<string, string> = {
  "~": "Home",
  "/": "Macintosh HD",
  "~/Documents": "Documents",
  "~/Desktop": "Desktop",
  "~/Downloads": "Downloads",
  "~/Library/Mobile Documents/com~apple~CloudDocs": "iCloud Drive",
  "/Applications": "Applications",
  "~/Movies": "Movies",
  "~/Pictures": "Pictures",
  "~/Music": "Music",
};

const ICON_DEFS: Record<string, { name: string; mode: string; style: string }> = {
  "~":          { name: "house", mode: "sf", style: "plain" },
  "/":          { name: "externaldrive", mode: "sf", style: "plain" },
  "~/Documents": { name: "doc", mode: "sf", style: "plain" },
  "~/Desktop":  { name: "menubar.dock.rectangle", mode: "sf", style: "plain" },
  "~/Downloads": { name: "arrow.down.circle", mode: "sf", style: "plain" },
  "~/Library/Mobile Documents/com~apple~CloudDocs": { name: "icloud", mode: "sf", style: "plain" },
  "/Applications": { name: "square.grid.2x2", mode: "sf", style: "plain" },
  "~/Movies":   { name: "film", mode: "sf", style: "plain" },
  "~/Pictures": { name: "photo", mode: "sf", style: "plain" },
  "~/Music":    { name: "music.note", mode: "sf", style: "plain" },
};

// Shared icon cache (loaded once, reused across composable instances)
let iconsLoaded = false;
const dirIcons = ref<Record<string, string>>({});
const folderIcon = ref("");

async function loadIcons() {
  if (iconsLoaded) return;
  iconsLoaded = true;

  invoke<string>("render_sf_symbol", { name: "public.folder", size: 32, mode: "uttype", style: "plain" })
    .then(b64 => { if (b64) folderIcon.value = b64; })
    .catch(() => {});

  for (const [key, def] of Object.entries(ICON_DEFS)) {
    try {
      const b64 = await invoke<string>("render_sf_symbol", {
        name: def.name, size: 32, mode: def.mode, style: def.style,
      });
      if (b64) dirIcons.value[key] = b64;
    } catch { /* non-critical */ }
  }
}

function shortenPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~");
}

/** Resolve a path to its icon (base64 SF Symbol) if known, or empty string. */
export function iconForPath(path: string): string {
  loadIcons();
  return dirIcons.value[shortenPath(path)] ?? folderIcon.value;
}

/** Resolve a path to a short display name. */
export function displayForPath(path: string): string {
  const shortened = shortenPath(path);
  if (KNOWN_DIRS[shortened]) return KNOWN_DIRS[shortened];
  const parts = shortened.split("/").filter(Boolean);
  if (parts.length <= 2) return "/" + parts.join("/");
  return "…/" + parts.slice(-2).join("/");
}

export function useScanFolder(storageKey = "scanFolder") {
  const lsKey = `negativ_${storageKey}`;
  const saved = localStorage.getItem(lsKey);
  const scanPath = ref(saved || "~");

  loadIcons();

  const scanPathIcon = computed(() => dirIcons.value[shortenPath(scanPath.value)] ?? "");
  const scanPathDisplay = computed(() => {
    const shortened = shortenPath(scanPath.value);
    return KNOWN_DIRS[shortened] ?? shortened;
  });

  async function pickScanFolder() {
    const folder = await openDialog({
      directory: true,
      multiple: false,
      title: "Choose folder to scan",
      defaultPath: scanPath.value === "~" ? undefined : scanPath.value,
    });
    if (folder) {
      scanPath.value = folder as string;
      localStorage.setItem(lsKey, scanPath.value);
    }
  }

  function resetScanFolder() {
    scanPath.value = "~";
    localStorage.setItem(lsKey, "~");
  }

  return {
    scanPath,
    scanPathIcon,
    scanPathDisplay,
    nativeFolderIcon: folderIcon,
    pickScanFolder,
    resetScanFolder,
  };
}

/**
 * useScanLocations — multi-location picker for scan views.
 *
 * Manages a flat list of scan directories. When empty, the scanner
 * defaults to home ("~"). All locations are equal and removable.
 */
export function useScanLocations(storageKey: string, max = 10) {
  const lsKey = `negativ_${storageKey}_scan_locations`;
  const folders = ref<string[]>([]);

  // Restore from localStorage
  try {
    const saved = localStorage.getItem(lsKey);
    if (saved) folders.value = JSON.parse(saved);
  } catch { /* ignore corrupt data */ }

  function persist() {
    localStorage.setItem(lsKey, JSON.stringify(folders.value));
  }

  async function addFolder() {
    if (folders.value.length >= max) return;
    const folder = await openDialog({
      directory: true,
      multiple: false,
      title: "Add scan location",
    });
    if (!folder) return;
    const path = folder as string;
    if (folders.value.includes(path)) return;
    folders.value.push(path);
    persist();
  }

  function removeFolder(index: number) {
    folders.value.splice(index, 1);
    persist();
  }

  return {
    folders,
    addFolder,
    removeFolder,
  };
}
