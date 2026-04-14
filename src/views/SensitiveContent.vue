<script setup lang="ts">
import { ref, shallowRef, triggerRef, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { formatSize } from "../utils";
import { showToast } from "../stores/toastStore";
import {
  nsfwResult,
  nsfwScanning,
  nsfwScanned,
  nsfwError,
  nsfwProgress,
  scanNsfw,
  stopNsfwScan,
  dismissNsfwPaths,
  deleteFiles,
  excludedLabels,
  toggleExcludedLabel,
  labelWeights,
  getLabelWeight,
  setLabelWeight,
  EXPOSED_LABELS,
} from "../stores/scanStore";
import { compressToVault, moveFilesToDirectory } from "../stores/vaultStore";
import { open } from "@tauri-apps/plugin-dialog";
import { useScanLocations } from "../composables/useScanFolder";
import ScanHeader from "../components/ScanHeader.vue";
import StickyBar from "../components/StickyBar.vue";
import Checkbox from "../components/Checkbox.vue";
import EmptyState from "../components/EmptyState.vue";
import ToggleSwitch from "../components/ToggleSwitch.vue";
import ProgressBar from "../components/ProgressBar.vue";
import InlineAlert from "../components/InlineAlert.vue";
import NsfwImageCard from "../components/NsfwImageCard.vue";
import AppSelect from "../components/AppSelect.vue";
import type { SelectOption } from "../components/AppSelect.vue";
import TimelineRail from "../components/TimelineRail.vue";
import type { TimelineYear } from "../components/TimelineRail.vue";
import type { NsfwFile, DetectedLabel, OperationProgress } from "../types";

// ── Scan settings ─────────────────────────────────────────────────────────

type Sensitivity = "conservative" | "normal" | "aggressive";

const SENSITIVITY_KEY = "negativ_nsfw_sensitivity";
const MIN_SIZE_KEY = "negativ_nsfw_min_size";

const sensitivity = ref<Sensitivity>(
  (localStorage.getItem(SENSITIVITY_KEY) as Sensitivity) || "normal"
);
const minSizeMb = ref(
  Number(localStorage.getItem(MIN_SIZE_KEY)) || 0
);
const showContent = ref(false);
const selectedAction = ref<string>("move");

// Action icons
const actionOptions = ref<SelectOption[]>([
  { value: "move", label: "Move" },
  { value: "vault", label: "Vault" },
  { value: "dismiss", label: "Dismiss" },
  { value: "delete", label: "Delete" },
]);

// Load SF Symbol icons for actions
const ACTION_ICONS: Record<string, string> = {
  move: "folder",
  vault: "lock.shield",
  dismiss: "eye.slash",
  delete: "trash",
};

for (const [action, sfName] of Object.entries(ACTION_ICONS)) {
  invoke<string>("render_sf_symbol", { name: sfName, size: 32, mode: "sf", style: "plain" })
    .then((b64) => {
      if (!b64) return;
      const opt = actionOptions.value.find((o) => o.value === action);
      if (opt) opt.icon = b64;
    })
    .catch(() => {});
}

function executeAction() {
  switch (selectedAction.value) {
    case "move": moveSelected(); break;
    case "vault": vaultSelected(); break;
    case "dismiss": dismissSelected(); break;
    case "delete": deleteSelected(); break;
  }
}
const selected = shallowRef(new Set<string>());
const cleaning = ref(false);
const opProgress = ref<OperationProgress | null>(null);
const opLabel = computed(() => {
  const p = opProgress.value;
  if (!p) return "";
  const labels: Record<string, string> = { move: "Moving", vault: "Vaulting", compress: "Compressing", delete: "Deleting" };
  const verb = labels[p.operation] ?? p.operation;
  return `${verb} ${p.processed} / ${p.total}${p.current_file ? " — " + p.current_file : ""}`;
});
const opPercent = computed(() => {
  const p = opProgress.value;
  if (!p || p.total === 0) return 0;
  return Math.round((p.processed / p.total) * 100);
});
let lastClickedPath: string | null = null;

const { folders: scanFolders, addFolder, removeFolder } = useScanLocations("nsfw");

// Settings popover
const showSettings = ref(false);

function onSettingsClickOutside(e: MouseEvent) {
  const el = (e.target as HTMLElement).closest('.settings-popover');
  if (!el) showSettings.value = false;
}

function onSettingsEsc(e: KeyboardEvent) {
  if (e.key === "Escape") showSettings.value = false;
}

watch(showSettings, (isOpen) => {
  if (isOpen) {
    setTimeout(() => {
      document.addEventListener("click", onSettingsClickOutside);
      document.addEventListener("keydown", onSettingsEsc);
    }, 0);
  } else {
    document.removeEventListener("click", onSettingsClickOutside);
    document.removeEventListener("keydown", onSettingsEsc);
  }
});

// Native SF Symbol icons for sticky bar
const filterIcon = ref("");
const eyeOpenIcon = ref("");
const eyeClosedIcon = ref("");
invoke<string>("render_sf_symbol", { name: "line.3.horizontal.decrease", size: 32, mode: "sf", style: "plain" })
  .then((b64) => { if (b64) filterIcon.value = b64; })
  .catch(() => {});
invoke<string>("render_sf_symbol", { name: "eye", size: 32, mode: "sf", style: "plain" })
  .then((b64) => { if (b64) eyeOpenIcon.value = b64; })
  .catch(() => {});
invoke<string>("render_sf_symbol", { name: "eye.slash", size: 32, mode: "sf", style: "plain" })
  .then((b64) => { if (b64) eyeClosedIcon.value = b64; })
  .catch(() => {});

const sensitivityStops: Sensitivity[] = ["conservative", "normal", "aggressive"];
const nsfwSizeStops = [0, 1, 5, 10];

const thresholdMap: Record<Sensitivity, number> = {
  conservative: 0.7,
  normal: 0.5,
  aggressive: 0.25,
};

function scan() {
  selected.value.clear();
  triggerRef(selected);
  selectedSize.value = 0;
  lastClickedPath = null;
  scanNsfw(thresholdMap[sensitivity.value], minSizeMb.value, scanFolders.value[0] || "~");
}

// ── Computed ──────────────────────────────────────────────────────────────

const rawFlagged = computed(() => nsfwResult.value?.flagged ?? []);
const showFilterPopover = ref(false);
const filterDisabled = ref(false);

const filtersActive = computed(() => {
  return excludedLabels.value.size > 0 || Object.keys(labelWeights.value).length > 0;
});

function resetFilters() {
  for (const label of [...excludedLabels.value]) {
    toggleExcludedLabel(label);
  }
  for (const label of Object.keys(labelWeights.value)) {
    setLabelWeight(label, 1.0);
  }
  filterDisabled.value = false;
}
const filterPos = ref<{ x: number; y: number } | null>(null);
const dragging = ref(false);
const dragOffset = ref({ x: 0, y: 0 });

function onFilterDragStart(e: MouseEvent) {
  const el = (e.target as HTMLElement).closest('.filter-popover') as HTMLElement | null;
  if (!el) return;
  dragging.value = true;
  const rect = el.getBoundingClientRect();
  dragOffset.value = { x: e.clientX - rect.left, y: e.clientY - rect.top };
  const w = rect.width;
  const h = rect.height;
  const onMove = (ev: MouseEvent) => {
    const TITLE_BAR = 48;
    const x = Math.max(0, Math.min(window.innerWidth - w, ev.clientX - dragOffset.value.x));
    const y = Math.max(TITLE_BAR, Math.min(window.innerHeight - h, ev.clientY - dragOffset.value.y));
    filterPos.value = { x, y };
  };
  const onUp = () => {
    dragging.value = false;
    window.removeEventListener('mousemove', onMove);
    window.removeEventListener('mouseup', onUp);
  };
  window.addEventListener('mousemove', onMove);
  window.addEventListener('mouseup', onUp);
}

function closeFilterPopover() {
  showFilterPopover.value = false;
  filterPos.value = null;
}

function normalizeLabel(raw: unknown): DetectedLabel {
  if (typeof raw === "string") return { label: raw, confidence: 0 };
  const obj = raw as DetectedLabel;
  return { label: obj.label ?? String(raw), confidence: obj.confidence ?? 0 };
}

function filePassesFilter(f: NsfwFile): boolean {
  if (filterDisabled.value) return true;
  const excl = excludedLabels.value;
  const hasCustomWeights = Object.keys(labelWeights.value).length > 0;
  if (excl.size === 0 && !hasCustomWeights) return true;
  if (!f.detected_labels?.length) return true;
  const exposed = f.detected_labels.filter((raw) => EXPOSED_LABELS.has(normalizeLabel(raw).label));
  if (exposed.length === 0) return true;
  const floor = thresholdMap[sensitivity.value];
  return exposed.some((raw) => {
    const d = normalizeLabel(raw);
    const effective = d.confidence * getLabelWeight(d.label);
    return !excl.has(d.label) && effective >= floor;
  });
}

const filteredFlagged = computed(() => rawFlagged.value.filter(filePassesFilter));
const filteredPathSet = computed(() => new Set(filteredFlagged.value.map(f => f.path)));

const flaggedFiles = computed(() => {
  if (!filtersActive.value || filterDisabled.value) return rawFlagged.value;
  return showFilterPopover.value ? rawFlagged.value : filteredFlagged.value;
});

const excludedCount = computed(() => {
  return rawFlagged.value.length - filteredFlagged.value.length;
});


const allFilesOrdered = computed(() => {
  const files: NsfwFile[] = [];
  for (const g of groupedByDate.value) {
    if (g.subGroups) {
      for (const sg of g.subGroups) for (const f of sg.files) files.push(f);
    } else {
      for (const f of g.files) files.push(f);
    }
  }
  return files;
});

const selectedCount = computed(() => selected.value.size);
const selectedSize = shallowRef(0);

function recalcSelectedSize() {
  const sel = selected.value;
  if (sel.size === 0) { selectedSize.value = 0; return; }
  let total = 0;
  for (const f of flaggedFiles.value) {
    if (sel.has(f.path)) total += f.size;
  }
  selectedSize.value = total;
}

type PhaseState = "pending" | "active" | "done";

const discoveryState = computed<PhaseState>(() => {
  const p = nsfwProgress.value;
  if (!p) return "pending";
  if (p.phase === "discovery") return "active";
  return "done";
});
const discoveryLabel = computed(() => {
  const p = nsfwProgress.value;
  if (!p || p.phase === "discovery") {
    const count = p?.images_discovered ?? 0;
    return count > 0 ? `Found ${count.toLocaleString()} images…` : "Discovering images…";
  }
  return `${p.images_discovered.toLocaleString()} images found`;
});

const classifyState = computed<PhaseState>(() => {
  const p = nsfwProgress.value;
  if (!p || p.phase === "discovery") return "pending";
  if (p.phase === "classifying") return "active";
  return "done";
});
const classifyPercent = computed(() => {
  const p = nsfwProgress.value;
  if (!p || p.total_images === 0) return 0;
  return Math.round((p.images_processed / p.total_images) * 100);
});
const classifyLabel = computed(() => {
  const p = nsfwProgress.value;
  if (!p || p.phase === "discovery") return "Classify";
  if (p.phase === "classifying") {
    return `${p.images_processed.toLocaleString()} / ${p.total_images.toLocaleString()}`;
  }
  return `${p.total_images.toLocaleString()} classified`;
});

const thumbState = computed<PhaseState>(() => {
  const p = nsfwProgress.value;
  if (!p || p.phase === "discovery" || p.phase === "classifying") return "pending";
  if (p.phase === "thumbnails") return "active";
  return "done";
});
const thumbPercent = computed(() => {
  const p = nsfwProgress.value;
  if (!p || p.total_thumbnails === 0) return 0;
  return Math.round((p.thumbnails_processed / p.total_thumbnails) * 100);
});
const thumbLabel = computed(() => {
  const p = nsfwProgress.value;
  if (!p || p.total_thumbnails === 0) return "Generate thumbnails";
  if (p.phase === "thumbnails") {
    return `${p.thumbnails_processed.toLocaleString()} / ${p.total_thumbnails.toLocaleString()}`;
  }
  return `${p.total_thumbnails.toLocaleString()} thumbnails`;
});

function bestDate(f: NsfwFile): string {
  const raw = f.date_taken ?? f.modified;
  if (!raw) return "";
  return raw.includes("T") ? raw : raw.replace(" ", "T");
}

function parseDate(f: NsfwFile): Date | null {
  const s = bestDate(f);
  if (!s) return null;
  const d = new Date(s);
  return isNaN(d.getTime()) ? null : d;
}

interface DateGroup {
  label: string;
  files: NsfwFile[];
  totalSize: number;
  subGroups?: { label: string; files: NsfwFile[] }[];
}

const YEAR_COLLAPSE_MAX = 10;
const DAY_EXPAND_MIN = 20;

const groupedByDate = computed<DateGroup[]>(() => {
  // Step 1: bucket every file by month key
  const monthMap = new Map<string, { year: string; files: NsfwFile[]; ts: number }>();
  for (const f of flaggedFiles.value) {
    const d = parseDate(f);
    if (!d) {
      const entry = monthMap.get("Unknown date") ?? { year: "", files: [], ts: 0 };
      entry.files.push(f);
      monthMap.set("Unknown date", entry);
      continue;
    }
    const year = String(d.getFullYear());
    const key = d.toLocaleDateString("en-US", { year: "numeric", month: "long" });
    const entry = monthMap.get(key) ?? { year, files: [], ts: d.getTime() };
    entry.files.push(f);
    if (!monthMap.has(key)) monthMap.set(key, entry);
  }

  // Step 2: count images per year to find sparse years
  const yearCounts = new Map<string, number>();
  for (const [key, { year, files }] of monthMap) {
    if (key === "Unknown date") continue;
    yearCounts.set(year, (yearCounts.get(year) ?? 0) + files.length);
  }

  // Step 3: build groups with adaptive granularity
  const groups: DateGroup[] = [];
  const collapsedYears = new Set<string>();

  // Collapse sparse years
  for (const [year, count] of yearCounts) {
    if (count <= YEAR_COLLAPSE_MAX) collapsedYears.add(year);
  }

  // Merge collapsed-year months
  const yearBuckets = new Map<string, NsfwFile[]>();
  const remainingMonths: [string, { year: string; files: NsfwFile[]; ts: number }][] = [];
  for (const [key, entry] of monthMap) {
    if (key === "Unknown date") {
      remainingMonths.push([key, entry]);
    } else if (collapsedYears.has(entry.year)) {
      const bucket = yearBuckets.get(entry.year) ?? [];
      bucket.push(...entry.files);
      yearBuckets.set(entry.year, bucket);
    } else {
      remainingMonths.push([key, entry]);
    }
  }

  // Add year-level groups
  for (const [year, files] of yearBuckets) {
    groups.push({ label: year, files, totalSize: files.reduce((s, f) => s + f.size, 0) });
  }

  // Add month-level groups, expanding dense months into day sub-groups
  for (const [label, { files }] of remainingMonths) {
    const totalSize = files.reduce((s, f) => s + f.size, 0);
    if (files.length >= DAY_EXPAND_MIN && label !== "Unknown date") {
      const dayMap = new Map<string, NsfwFile[]>();
      for (const f of files) {
        const d = parseDate(f);
        const dayKey = d
          ? d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" })
          : "Unknown";
        if (!dayMap.has(dayKey)) dayMap.set(dayKey, []);
        dayMap.get(dayKey)!.push(f);
      }
      const subGroups = Array.from(dayMap.entries())
        .map(([dl, df]) => ({ label: dl, files: df }))
        .sort((a, b) => {
          const da = parseDate(a.files[0])?.getTime() ?? 0;
          const db = parseDate(b.files[0])?.getTime() ?? 0;
          return db - da;
        });
      groups.push({ label, files, totalSize, subGroups });
    } else {
      groups.push({ label, files, totalSize });
    }
  }

  groups.sort((a, b) => {
    const da = parseDate(a.files[0])?.getTime() ?? 0;
    const db = parseDate(b.files[0])?.getTime() ?? 0;
    return db - da;
  });

  return groups;
});

// ── Actions ───────────────────────────────────────────────────────────────

function toggleFile(path: string, event?: MouseEvent) {
  const s = selected.value;

  if (event?.shiftKey && lastClickedPath && lastClickedPath !== path) {
    const ordered = allFilesOrdered.value;
    const idxA = ordered.findIndex((f) => f.path === lastClickedPath);
    const idxB = ordered.findIndex((f) => f.path === path);
    if (idxA !== -1 && idxB !== -1) {
      const lo = Math.min(idxA, idxB);
      const hi = Math.max(idxA, idxB);
      for (let i = lo; i <= hi; i++) s.add(ordered[i].path);
      lastClickedPath = path;
      triggerRef(selected);
      recalcSelectedSize();
      return;
    }
  }

  if (s.has(path)) s.delete(path);
  else s.add(path);
  lastClickedPath = path;
  triggerRef(selected);
  recalcSelectedSize();
}

function selectAll() {
  const s = selected.value;
  for (const f of flaggedFiles.value) s.add(f.path);
  triggerRef(selected);
  recalcSelectedSize();
}

function deselectAll() {
  selected.value.clear();
  triggerRef(selected);
  selectedSize.value = 0;
}

function toggleSelectAll() {
  if (selected.value.size === flaggedFiles.value.length) deselectAll();
  else selectAll();
}

async function deleteSelected() {
  if (selected.value.size === 0) return;
  cleaning.value = true;
  try {
    const selectedFiles = flaggedFiles.value.filter((f) => selected.value.has(f.path));
    const photoAssetIds = selectedFiles
      .filter((f) => f.photo_asset_id)
      .map((f) => f.photo_asset_id!);
    const regularPaths = selectedFiles
      .filter((f) => !f.photo_asset_id)
      .map((f) => f.path);
    const allPaths = selectedFiles.map((f) => f.path);

    let deletedCount = 0;
    let freedBytes = 0;

    if (photoAssetIds.length > 0) {
      const count = await invoke<number>("delete_photo_assets", { assetIds: photoAssetIds });
      deletedCount += count;
    }

    if (regularPaths.length > 0) {
      const result = await deleteFiles(regularPaths);
      deletedCount += result.deleted_count;
      freedBytes += result.freed_bytes;
    }

    await dismissNsfwPaths(allPaths);

    const msg = freedBytes > 0
      ? `Deleted ${deletedCount} file(s), freed ${formatSize(freedBytes)}`
      : `Deleted ${deletedCount} file(s)`;
    showToast(msg, "success");

    selected.value.clear();
    triggerRef(selected);
    selectedSize.value = 0;
    if (nsfwResult.value) {
      nsfwResult.value = {
        ...nsfwResult.value,
        flagged: nsfwResult.value.flagged.filter((f) => !allPaths.includes(f.path)),
      };
    }
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    cleaning.value = false;
  }
}

async function vaultSelected() {
  if (selected.value.size === 0) return;
  cleaning.value = true;
  try {
    const paths = Array.from(selected.value);
    const pathSet = selected.value;
    const result = await compressToVault(paths);
    if (result.success) {
      await dismissNsfwPaths(paths);
      showToast(
        `Secured ${result.files_compressed} file(s) in vault, saved ${formatSize(result.total_savings)}`,
        "success"
      );
      pathSet.clear();
      triggerRef(selected);
      selectedSize.value = 0;
      if (nsfwResult.value) {
        nsfwResult.value = {
          ...nsfwResult.value,
          flagged: nsfwResult.value.flagged.filter((f) => !paths.includes(f.path)),
        };
      }
    } else {
      showToast(result.errors.join(", "), "error");
    }
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    cleaning.value = false;
  }
}

async function dismissSelected() {
  if (selected.value.size === 0) return;
  const paths = Array.from(selected.value);
  await dismissNsfwPaths(paths);
  selected.value.clear();
  triggerRef(selected);
  selectedSize.value = 0;
  showToast(`Dismissed ${paths.length} file(s) from future scans`, "info");
}

const MOVE_TARGET_KEY = "negativ_sensitive_move_target";

async function moveSelected() {
  if (selected.value.size === 0) return;

  const lastDir = localStorage.getItem(MOVE_TARGET_KEY) ?? undefined;
  const picked = await open({
    directory: true,
    multiple: false,
    defaultPath: lastDir,
    title: "Choose destination folder",
  });
  if (!picked) return;

  const targetDir = typeof picked === "string" ? picked : picked[0];
  if (!targetDir) return;
  localStorage.setItem(MOVE_TARGET_KEY, targetDir);

  cleaning.value = true;
  try {
    const paths = Array.from(selected.value);
    const result = await moveFilesToDirectory(paths, targetDir);
    if (result.files_moved > 0) {
      await dismissNsfwPaths(paths);
      showToast(`Moved ${result.files_moved} file(s)`, "success");
      selected.value.clear();
      triggerRef(selected);
      selectedSize.value = 0;
      if (nsfwResult.value) {
        nsfwResult.value = {
          ...nsfwResult.value,
          flagged: nsfwResult.value.flagged.filter((f) => !paths.includes(f.path)),
        };
      }
    }
    if (result.errors.length > 0) {
      showToast(result.errors.join(", "), "error");
    }
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    cleaning.value = false;
  }
}

watch(sensitivity, (v) => localStorage.setItem(SENSITIVITY_KEY, v));
watch(minSizeMb, (v) => localStorage.setItem(MIN_SIZE_KEY, String(v)));
watch(nsfwError, (err) => { if (err) showToast(err, "error"); });

const exposedLabelList = [...EXPOSED_LABELS];

// ── Info popover (owned here so v-memo on cards isn't affected) ──────────
const infoFile = ref<NsfwFile | null>(null);

function showInfoPopover(file: NsfwFile) {
  infoFile.value = file;
}

const infoSortedLabels = computed(() => {
  if (!infoFile.value?.detected_labels?.length) return [];
  return infoFile.value.detected_labels
    .map(normalizeLabel)
    .sort((a, b) => b.confidence - a.confidence);
});

function isExposedLabel(l: string): boolean {
  return EXPOSED_LABELS.has(l);
}

function infoBarClass(d: DetectedLabel): string {
  if (!d.label || !EXPOSED_LABELS.has(d.label)) return "bar--neutral";
  if (d.confidence >= 0.7) return "bar--high";
  if (d.confidence >= 0.4) return "bar--med";
  return "bar--low";
}

function formatLabelName(l: string): string {
  if (l === "NSFW_SCORE") return "General";
  return l.replace(/_EXPOSED$/, "").replace(/_/g, " ").toLowerCase().replace(/^\w/, (c) => c.toUpperCase());
}

// ── Timeline ─────────────────────────────────────────────────────────────

const TIMELINE_THRESHOLD = 12;
const showTimeline = computed(() => flaggedFiles.value.length >= TIMELINE_THRESHOLD);

const timelineYears = computed<TimelineYear[]>(() => {
  if (!showTimeline.value) return [];

  // Build year → months map from groups
  const yearMap = new Map<string, { key: string; months: { key: string; label: string; count: number }[]; total: number; collapsed: boolean }>();

  for (const g of groupedByDate.value) {
    // Year-only group (already collapsed by groupedByDate)
    if (/^\d{4}$/.test(g.label)) {
      yearMap.set(g.label, { key: g.label, months: [], total: g.files.length, collapsed: true });
      continue;
    }
    const parts = g.label.split(" ");
    const year = parts[parts.length - 1] || "????";
    const month = parts.slice(0, -1).join(" ");
    const shortMonth = month.substring(0, 3);

    if (!yearMap.has(year)) {
      yearMap.set(year, { key: "", months: [], total: 0, collapsed: false });
    }
    const entry = yearMap.get(year)!;
    entry.months.push({ key: g.label, label: shortMonth, count: g.files.length });
    entry.total += g.files.length;
    if (!entry.key) entry.key = g.label; // first month = nav target for year
  }

  return Array.from(yearMap.entries()).map(([year, data]) => ({
    year,
    key: data.key || year,
    totalCount: data.total,
    collapsed: data.collapsed,
    months: data.months,
  }));
});

const activeGroupKey = ref<string>("");
const scrollContainer = ref<HTMLElement | null>(null);

// Track active group by finding the topmost visible group header on scroll
let scrollTicking = false;

function updateActiveGroup() {
  const container = scrollContainer.value;
  if (!container) return;
  const headers = container.querySelectorAll<HTMLElement>("[data-group-key]");
  if (headers.length === 0) return;

  // Find the content scroll container (.content element)
  const scrollEl = document.querySelector(".content");
  if (!scrollEl) return;
  const scrollRect = scrollEl.getBoundingClientRect();
  // Trigger line at 30% from top of viewport
  const triggerY = scrollRect.top + scrollRect.height * 0.3;

  let best: HTMLElement | null = null;
  for (const h of headers) {
    const rect = h.getBoundingClientRect();
    if (rect.top <= triggerY) {
      best = h;
    } else {
      break; // headers are in DOM order (top to bottom)
    }
  }
  if (best) {
    const key = best.dataset.groupKey || "";
    if (key !== activeGroupKey.value) {
      activeGroupKey.value = key;
    }
  }
}

function onGroupScroll() {
  if (scrollTicking) return;
  scrollTicking = true;
  requestAnimationFrame(() => {
    updateActiveGroup();
    scrollTicking = false;
  });
}

let groupScrollTarget: Element | null = null;

function setupObserver() {
  cleanupGroupScroll();
  groupScrollTarget = document.querySelector(".content");
  if (groupScrollTarget) {
    groupScrollTarget.addEventListener("scroll", onGroupScroll, { passive: true });
  }
}

function cleanupObserver() {
  cleanupGroupScroll();
}

function cleanupGroupScroll() {
  if (groupScrollTarget) {
    groupScrollTarget.removeEventListener("scroll", onGroupScroll);
    groupScrollTarget = null;
  }
}

function navigateToGroup(key: string) {
  const el = scrollContainer.value?.querySelector<HTMLElement>(`[data-group-key="${CSS.escape(key)}"]`);
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "start" });
    activeGroupKey.value = key;
  }
}

watch(showTimeline, (v) => {
  if (v) nextTick(setupObserver);
  else cleanupObserver();
});

watch(flaggedFiles, () => {
  if (showTimeline.value) nextTick(setupObserver);
});

let unlistenOp: (() => void) | null = null;

onMounted(async () => {
  if (showTimeline.value) nextTick(setupObserver);
  unlistenOp = await listen<OperationProgress>("operation-progress", (event) => {
    const p = event.payload;
    if (p.processed >= p.total) {
      opProgress.value = null;
    } else {
      opProgress.value = p;
    }
  });
});

onUnmounted(() => {
  cleanupObserver();
  if (unlistenOp) { unlistenOp(); unlistenOp = null; }
});
</script>

<template>
  <section class="sensitive-content-view">
    <ScanHeader
      title="Sensitive Content"
      subtitle="Find and manage sensitive images on your disk"
      :scanning="nsfwScanning"
      scan-label="Scan"
      :disabled="nsfwScanning"
      :folders="scanFolders"
      @scan="scan"
      @add-folder="addFolder"
      @remove-folder="removeFolder"
    >
      <button class="scan-bar-settings" @click.stop="showSettings = !showSettings" title="Scan settings">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="4" y1="21" x2="4" y2="14" /><line x1="4" y1="10" x2="4" y2="3" />
          <line x1="12" y1="21" x2="12" y2="12" /><line x1="12" y1="8" x2="12" y2="3" />
          <line x1="20" y1="21" x2="20" y2="16" /><line x1="20" y1="12" x2="20" y2="3" />
          <line x1="1" y1="14" x2="7" y2="14" />
          <line x1="9" y1="8" x2="15" y2="8" />
          <line x1="17" y1="16" x2="23" y2="16" />
        </svg>
      </button>
    </ScanHeader>

    <!-- Settings popover -->
    <Teleport to="body">
      <div v-if="showSettings" class="settings-popover" @click.stop>
        <div class="settings-popover-header">
          <div class="settings-popover-title">Scan settings</div>
          <button class="settings-popover-close" @click="showSettings = false">&times;</button>
        </div>

        <div class="settings-row">
          <label class="settings-label">Sensitivity</label>
          <span class="settings-value">{{ sensitivity[0].toUpperCase() + sensitivity.slice(1) }}</span>
        </div>
        <input
          type="range"
          class="settings-slider"
          :value="sensitivityStops.indexOf(sensitivity)"
          min="0"
          :max="sensitivityStops.length - 1"
          step="1"
          @input="sensitivity = sensitivityStops[Number(($event.target as HTMLInputElement).value)]"
        />
        <div class="settings-slider-labels">
          <span>Conservative</span>
          <span>Aggressive</span>
        </div>

        <div class="settings-row">
          <label class="settings-label">Minimum file size</label>
          <span class="settings-value">{{ minSizeMb === 0 ? 'Any size' : minSizeMb + ' MB' }}</span>
        </div>
        <input
          type="range"
          class="settings-slider"
          :value="nsfwSizeStops.indexOf(minSizeMb)"
          min="0"
          :max="nsfwSizeStops.length - 1"
          step="1"
          @input="minSizeMb = nsfwSizeStops[Number(($event.target as HTMLInputElement).value)]"
        />
        <div class="settings-slider-labels">
          <span>Any</span>
          <span>10 MB</span>
        </div>
      </div>
    </Teleport>

    <!-- Progress (3-phase stepped) -->
    <div v-if="nsfwScanning" class="progress-section">
      <div class="scan-phases">
        <!-- Discovery -->
        <div class="phase-row" :class="'phase--' + discoveryState">
          <span class="phase-icon">
            <svg v-if="discoveryState === 'done'" width="14" height="14" viewBox="0 0 20 20" fill="var(--accent)"><path d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"/></svg>
            <span v-else-if="discoveryState === 'active'" class="phase-dot phase-dot--active"></span>
            <span v-else class="phase-dot"></span>
          </span>
          <span class="phase-name">Discover</span>
          <span class="phase-stat text-muted">{{ discoveryLabel }}</span>
        </div>
        <div class="phase-bar-track" :class="'phase--' + discoveryState">
          <div v-if="discoveryState === 'active'" class="phase-bar-indeterminate"></div>
          <div v-else-if="discoveryState === 'done'" class="phase-bar-fill" style="width:100%"></div>
        </div>

        <!-- Classification -->
        <div class="phase-row" :class="'phase--' + classifyState">
          <span class="phase-icon">
            <svg v-if="classifyState === 'done'" width="14" height="14" viewBox="0 0 20 20" fill="var(--accent)"><path d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"/></svg>
            <span v-else-if="classifyState === 'active'" class="phase-dot phase-dot--active"></span>
            <span v-else class="phase-dot"></span>
          </span>
          <span class="phase-name">Classify</span>
          <span class="phase-stat text-muted">{{ classifyLabel }}</span>
        </div>
        <div class="phase-bar-track" :class="'phase--' + classifyState">
          <div v-if="classifyState === 'active'" class="phase-bar-fill" :style="{ width: classifyPercent + '%' }"></div>
          <div v-else-if="classifyState === 'done'" class="phase-bar-fill" style="width:100%"></div>
        </div>

        <!-- Thumbnails -->
        <div class="phase-row" :class="'phase--' + thumbState">
          <span class="phase-icon">
            <svg v-if="thumbState === 'done'" width="14" height="14" viewBox="0 0 20 20" fill="var(--accent)"><path d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"/></svg>
            <span v-else-if="thumbState === 'active'" class="phase-dot phase-dot--active"></span>
            <span v-else class="phase-dot"></span>
          </span>
          <span class="phase-name">Thumbnails</span>
          <span class="phase-stat text-muted">{{ thumbLabel }}</span>
        </div>
        <div class="phase-bar-track" :class="'phase--' + thumbState">
          <div v-if="thumbState === 'active'" class="phase-bar-fill" :style="{ width: thumbPercent + '%' }"></div>
          <div v-else-if="thumbState === 'done'" class="phase-bar-fill" style="width:100%"></div>
        </div>
      </div>
      <div class="progress-row">
        <span></span>
        <button class="btn-ghost btn-sm" @click="stopNsfwScan">Stop</button>
      </div>
    </div>

    <!-- Warnings (e.g. inaccessible Photos Library) -->
    <InlineAlert
      v-for="(warning, wi) in (nsfwResult?.warnings ?? [])"
      :key="wi"
      variant="warning"
    >{{ warning }}</InlineAlert>

    <!-- Results -->
    <template v-if="nsfwScanned && !nsfwScanning">
      <!-- Stats -->
      <p class="scan-summary" v-if="nsfwResult">
        {{ nsfwResult.images_scanned.toLocaleString() }} images scanned · {{ flaggedFiles.length }} flagged · {{ (nsfwResult.scan_duration_ms / 1000).toFixed(1) }}s
      </p>

      <!-- Sticky bar with controls -->
      <!-- PREVIOUS IMPL (preserved for revert):
        Left: FilterPill (white pill + blue active + badge) | Eye icon (bare) | Checkbox | "404 flagged"
        Right: AppSelect (Move ▾) | Apply button
      -->
      <StickyBar v-if="flaggedFiles.length > 0">
        <!-- Checkbox + count (primary info) -->
        <Checkbox
          :model-value="selectedCount > 0 && selectedCount === flaggedFiles.length"
          :indeterminate="selectedCount > 0 && selectedCount < flaggedFiles.length"
          @update:model-value="toggleSelectAll"
        />
        <span v-if="selectedCount > 0" class="bar-count">
          {{ selectedCount }} selected
          <span class="bar-count-size">({{ formatSize(selectedSize) }})</span>
        </span>
        <span v-else class="bar-count">
          {{ flaggedFiles.length }} flagged
        </span>

        <template #actions>
          <!-- Tool buttons -->
          <div class="filter-btn-wrap" v-if="nsfwResult">
            <button
              class="bar-tool-btn"
              :class="{ 'bar-tool-btn--active': filtersActive && !filterDisabled }"
              title="Label filters"
              @click.stop="showFilterPopover ? closeFilterPopover() : (showFilterPopover = true)"
            >
              <img v-if="filterIcon" :src="filterIcon" alt="Filter" width="15" height="15" />
              <span v-if="filtersActive && !filterDisabled" class="bar-tool-badge">{{ excludedLabels.size }}</span>
            </button>
            <Teleport to="body">
              <div
                v-if="showFilterPopover"
                class="filter-popover"
                :style="filterPos ? { left: filterPos.x + 'px', top: filterPos.y + 'px' } : {}"
                :class="{ 'filter-popover--placed': !!filterPos }"
              >
                <div class="filter-popover-header" @mousedown.prevent="onFilterDragStart">
                  <div class="filter-popover-title">Label filters</div>
                  <button class="filter-popover-close" @click="closeFilterPopover" title="Close">&times;</button>
                </div>
                  <p class="filter-popover-hint">Toggle off labels you don't want to trigger flags</p>
                  <div class="filter-popover-list">
                    <div
                      v-for="label in exposedLabelList"
                      :key="label"
                      class="filter-label-item"
                    >
                      <label class="filter-label-row">
                        <ToggleSwitch
                          :model-value="!excludedLabels.has(label)"
                          @update:model-value="toggleExcludedLabel(label)"
                        />
                        <span class="filter-label-name">{{ formatLabelName(label) }}</span>
                        <span
                          v-if="!excludedLabels.has(label) && getLabelWeight(label) !== 1.0"
                          class="filter-weight-value"
                        >{{ Math.round(getLabelWeight(label) * 100) }}%</span>
                      </label>
                      <div v-if="!excludedLabels.has(label)" class="filter-weight-row">
                        <input
                          type="range"
                          class="filter-weight-slider"
                          min="0"
                          max="200"
                          step="10"
                          :value="Math.round(getLabelWeight(label) * 100)"
                          @input="setLabelWeight(label, Number(($event.target as HTMLInputElement).value) / 100)"
                        />
                        <span class="filter-weight-labels">
                          <span>Less sensitive</span>
                          <span>More</span>
                        </span>
                      </div>
                    </div>
                  </div>
                  <div class="filter-popover-footer">
                    <label class="filter-disable-row">
                      <input type="checkbox" v-model="filterDisabled" />
                      <span>Disable filters</span>
                    </label>
                    <button
                      v-if="filtersActive"
                      class="btn-ghost filter-reset-btn"
                      @click="resetFilters"
                    >Reset all</button>
                  </div>
                  <p v-if="excludedCount > 0 && !filterDisabled" class="filter-popover-note">
                    {{ excludedCount }} of {{ rawFlagged.length }} image{{ excludedCount === 1 ? '' : 's' }} hidden
                  </p>
              </div>
            </Teleport>
          </div>

          <button
            class="bar-tool-btn"
            :class="{ 'bar-tool-btn--active': showContent }"
            :title="showContent ? 'Hide content' : 'Show content'"
            @click="showContent = !showContent"
          >
            <img v-if="showContent && eyeOpenIcon" :src="eyeOpenIcon" alt="Visible" width="15" height="15" />
            <img v-else-if="eyeClosedIcon" :src="eyeClosedIcon" alt="Hidden" width="15" height="15" />
          </button>

          <span class="bar-divider"></span>

          <!-- Action group (joined select + button) -->
          <div class="action-group">
            <AppSelect v-model="selectedAction" :options="actionOptions" class="action-group-select" />
            <button
              class="btn-primary action-group-btn"
              :disabled="selectedCount === 0 || cleaning"
              @click="executeAction"
            >
              <span v-if="cleaning" class="spinner-sm"></span>
              Apply
            </button>
          </div>
        </template>
      </StickyBar>

      <!-- Operation progress (move/vault/delete) -->
      <div v-if="opProgress" class="op-progress-section">
        <ProgressBar :percent="opPercent" size="thin" />
        <p class="op-progress-text text-muted">{{ opLabel }}</p>
      </div>

      <!-- Empty state -->
      <EmptyState
        v-if="flaggedFiles.length === 0"
        title="No sensitive content found"
        :description="`Scanned ${nsfwResult?.images_scanned.toLocaleString() ?? 0} images — nothing flagged at this sensitivity level.`"
      >
        <template #icon>
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
            <polyline points="9 12 11 14 15 10"/>
          </svg>
        </template>
      </EmptyState>

      <!-- Grouped by date -->
      <div v-if="flaggedFiles.length > 0" class="grouped-layout" ref="scrollContainer">
        <div class="grouped-main">
          <div
            v-for="group in groupedByDate"
            :key="group.label"
            class="date-group"
            :data-group-key="group.label"
          >
            <div class="date-group-header">
              <span class="date-label">{{ group.label }}</span>
              <span class="badge badge-neutral pill">{{ group.files.length }}</span>
            </div>

            <!-- Day sub-groups for dense months -->
            <template v-if="group.subGroups">
              <div v-for="sub in group.subGroups" :key="sub.label" class="day-sub-group">
                <div class="day-sub-header">
                  <span class="day-label">{{ sub.label }}</span>
                  <span class="day-count text-muted">{{ sub.files.length }}</span>
                </div>
                <div class="nsfw-grid">
                  <NsfwImageCard
                    v-for="file in sub.files"
                    :key="file.path"
                    :file="file"
                    :selected="selected.has(file.path)"
                    :blurred="!showContent"
                    :dimmed="showFilterPopover && !filteredPathSet.has(file.path)"
                    :confidence-floor="thresholdMap[sensitivity]"
                    @toggle="toggleFile(file.path, $event)"
                    @show-info="showInfoPopover"
                  />
                </div>
              </div>
            </template>

            <!-- Flat grid for normal groups -->
            <div v-else class="nsfw-grid">
              <NsfwImageCard
                v-for="file in group.files"
                :key="file.path"
                :file="file"
                :selected="selected.has(file.path)"
                :blurred="!showContent"
                :dimmed="showFilterPopover && !filteredPathSet.has(file.path)"
                :confidence-floor="thresholdMap[sensitivity]"
                @toggle="toggleFile(file.path, $event)"
                @show-info="showInfoPopover"
              />
            </div>
          </div>
        </div>
        <TimelineRail
          v-if="showTimeline"
          :years="timelineYears"
          :active-key="activeGroupKey"
          @navigate="navigateToGroup"
        />
      </div>
    </template>

    <!-- Initial state (no scan yet) -->
    <EmptyState
      v-if="!nsfwScanned && !nsfwScanning && !nsfwError"
      title="Scan for sensitive content"
      description="Detect NSFW images on your disk using on-device AI classification. Nothing leaves your Mac."
    >
      <template #icon>
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/>
          <line x1="1" y1="1" x2="23" y2="23"/>
        </svg>
      </template>
    </EmptyState>

    <!-- Info popover (owned at parent level to avoid v-memo conflicts) -->
    <div v-if="infoFile" class="nsfw-popover-overlay" @click="infoFile = null">
      <div class="nsfw-popover" @click.stop>
        <div class="nsfw-popover-header">
          <div class="nsfw-popover-title">Detection breakdown</div>
          <span class="nsfw-popover-filename">{{ infoFile.name }}</span>
        </div>
        <div
          v-for="d in infoSortedLabels"
          :key="d.label"
          class="nsfw-popover-row"
        >
          <span class="nsfw-popover-label" :class="{ 'nsfw-popover-label--exposed': isExposedLabel(d.label) }">{{ formatLabelName(d.label) }}</span>
          <span class="nsfw-popover-bar-track">
            <span class="nsfw-popover-bar-fill" :class="infoBarClass(d)" :style="{ width: Math.round(d.confidence * 100) + '%' }"></span>
          </span>
          <span class="nsfw-popover-conf">
            {{ Math.round(d.confidence * 100) }}%
            <span v-if="getLabelWeight(d.label) !== 1.0" class="nsfw-popover-weighted">
              → {{ Math.round(d.confidence * getLabelWeight(d.label) * 100) }}%
            </span>
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.sensitive-content-view {
  max-width: 1440px;
}

/* ScanBar settings button */
.scan-bar-settings {
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 8px;
  transition: background 0.15s;
  opacity: 0.7;
}

.scan-bar-settings:hover {
  background: rgba(0, 0, 0, 0.06);
  opacity: 1;
}

/* Progress */
.progress-section {
  margin-bottom: var(--sp-6);
}

.progress-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: var(--sp-2);
}

/* Scan phases */
.scan-phases {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.phase-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  transition: opacity 0.2s;
}

.phase-row.phase--pending {
  opacity: 0.35;
}

.phase-row.phase--done .phase-name {
  color: var(--muted);
}

.phase-icon {
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.phase-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border);
}

.phase-dot--active {
  background: var(--accent);
  animation: phase-pulse 1.2s ease-in-out infinite;
}

@keyframes phase-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(0.85); }
}

.phase-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  min-width: 80px;
}

.phase-stat {
  font-size: 12px;
  margin-left: auto;
}

.phase-bar-track {
  height: 3px;
  background: rgba(0, 0, 0, 0.06);
  border-radius: 2px;
  overflow: hidden;
  margin-left: 22px;
  margin-bottom: 4px;
  transition: opacity 0.2s;
}

.phase-bar-track.phase--pending {
  opacity: 0.3;
}

.phase-bar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.phase-bar-track.phase--done .phase-bar-fill {
  background: var(--accent);
  opacity: 0.4;
}

.phase-bar-indeterminate {
  height: 100%;
  width: 30%;
  background: var(--accent);
  border-radius: 2px;
  animation: indeterminate-slide 1.5s ease-in-out infinite;
}

@keyframes indeterminate-slide {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(400%); }
}

/* Stats row */
.scan-summary {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: var(--sp-3);
}

/* Sticky bar extras */
/* ── Sticky bar: count ─────────────────────────────── */
.bar-count {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
}

.bar-count-size {
  font-weight: 400;
  color: var(--text-secondary);
}

/* ── Sticky bar: tool buttons ─────────────────────── */
/* Action group — joined select + apply button, compact height */
.action-group {
  display: flex;
  align-items: stretch;
  height: 30px;
}

.action-group :deep(.app-select) {
  padding: 0 24px 0 8px;
  font-size: 12px;
  border-top-right-radius: 0;
  border-bottom-right-radius: 0;
  border-right: none;
}

.action-group :deep(.app-select-icon) {
  width: 14px;
  height: 14px;
}

.action-group :deep(.app-select-chevron) {
  right: 6px;
}

.action-group-btn {
  padding: 0 14px;
  font-size: 12px;
  border: 1px solid var(--accent);
  border-top-left-radius: 0;
  border-bottom-left-radius: 0;
}

.action-group-btn:disabled {
  border-color: var(--accent);
}

.bar-divider {
  width: 1px;
  height: 18px;
  background: rgba(0, 0, 0, 0.1);
  flex-shrink: 0;
}

.bar-tool-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s, color 0.15s;
}

.bar-tool-btn:hover {
  background: rgba(0, 136, 255, 0.08);
}

.bar-tool-btn--active {
  color: var(--accent);
  background: rgba(0, 136, 255, 0.1);
}

.bar-tool-btn--active img {
  filter: brightness(0) saturate(100%) invert(40%) sepia(90%) saturate(1500%) hue-rotate(190deg) brightness(100%);
}

.bar-tool-badge {
  position: absolute;
  top: 1px;
  right: 1px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 14px;
  height: 14px;
  border-radius: 7px;
  background: var(--accent);
  color: #fff;
  font-size: 9px;
  font-weight: 600;
  padding: 0 3px;
}


.filter-btn-wrap {
  position: relative;
}

/* Grid */
.nsfw-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: var(--sp-4);
  user-select: none;
  -webkit-user-select: none;
}

/* Grouped layout with timeline */
.grouped-layout {
  display: flex;
  gap: var(--sp-4);
}

.grouped-main {
  flex: 1;
  min-width: 0;
}

/* Date groups */
.date-group {
  margin-bottom: var(--sp-6);
}

.date-group-header {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  margin-bottom: var(--sp-3);
}

.date-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

/* Day sub-groups within dense months */
.day-sub-group {
  margin-bottom: var(--sp-3);
}

.day-sub-header {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  margin-bottom: var(--sp-2);
  padding-left: 2px;
}

.day-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
}

.day-count {
  font-size: 11px;
}

/* Operation progress */
.op-progress-section {
  margin-bottom: var(--sp-4);
}

.op-progress-section :deep(.progress-track) {
  margin-bottom: var(--sp-1);
}

.op-progress-text {
  font-size: 12px;
  margin: 0;
}

</style>

<style>
/* Filter popover — unscoped for Teleport */
.filter-popover {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 9999;
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: var(--radius-lg, 12px);
  padding: 0 20px 16px;
  width: 230px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.18);
  -webkit-app-region: no-drag;
}

.filter-popover--placed {
  transform: none;
}

.filter-popover-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0 0;
  margin-bottom: 0;
  cursor: grab;
  user-select: none;
  -webkit-app-region: no-drag;
}

.filter-popover-header:active {
  cursor: grabbing;
}

.filter-popover-close {
  background: none;
  border: none;
  font-size: 18px;
  line-height: 1;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 0 2px;
  opacity: 0.6;
  transition: opacity 0.15s;
}

.filter-popover-close:hover {
  opacity: 1;
}

.filter-popover-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 2px;
}

.filter-popover-hint {
  font-size: 11px;
  color: var(--muted);
  margin: 0 0 12px;
}

.filter-popover-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.filter-label-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.filter-label-row {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
}

.filter-label-name {
  font-size: 12px;
  font-weight: 500;
  flex: 1;
}

.filter-weight-value {
  font-size: 10px;
  color: var(--muted);
  font-weight: 600;
  min-width: 30px;
  text-align: right;
}

.filter-weight-row {
  padding-left: 0;
  margin-top: 2px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.filter-weight-slider {
  width: 100%;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(0, 0, 0, 0.08);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}

.filter-weight-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  border: 2px solid #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  cursor: pointer;
}

.filter-weight-labels {
  display: flex;
  justify-content: space-between;
  font-size: 9px;
  color: var(--muted);
  opacity: 0.7;
}

.filter-popover-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 14px;
  padding-top: 10px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
}

.filter-disable-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-secondary);
  cursor: pointer;
}

.filter-disable-row input[type="checkbox"] {
  accent-color: var(--accent);
}

.filter-reset-btn {
  font-size: 11px;
  padding: 2px 8px;
  color: var(--danger);
}

.filter-popover-note {
  margin: 8px 0 0;
  font-size: 11px;
  color: var(--muted);
  font-style: italic;
}

/* Info popover — unscoped for Teleport */
.nsfw-popover-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.35);
}

.nsfw-popover {
  background: #fff;
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: var(--radius-lg, 12px);
  padding: 14px 16px;
  min-width: 260px;
  max-width: 340px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
}

.nsfw-popover-header {
  margin-bottom: 10px;
}

.nsfw-popover-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--muted);
}

.nsfw-popover-filename {
  font-size: 11px;
  color: var(--text-secondary);
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}

.nsfw-popover-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 0;
}

.nsfw-popover-label {
  font-size: 11px;
  min-width: 110px;
  text-transform: capitalize;
  color: var(--text-secondary);
}

.nsfw-popover-label--exposed {
  font-weight: 600;
  color: var(--text-primary);
}

.nsfw-popover-bar-track {
  flex: 1;
  height: 5px;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.06);
  overflow: hidden;
}

.nsfw-popover-bar-fill {
  display: block;
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}

.bar--high   { background: var(--danger); }
.bar--med    { background: var(--warning); }
.bar--low    { background: var(--yellow); }
.bar--neutral { background: var(--grey, #999); opacity: 0.5; }

.nsfw-popover-conf {
  font-size: 11px;
  font-weight: 600;
  min-width: 32px;
  text-align: right;
  color: var(--text-secondary);
  white-space: nowrap;
}

.nsfw-popover-weighted {
  font-weight: 400;
  color: var(--muted);
  font-size: 10px;
}
</style>
