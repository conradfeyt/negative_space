<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { BUILD_NUMBER } from "./buildNumber";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, availableMonitors } from "@tauri-apps/api/window";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { useRouter, useRoute } from "vue-router";
import { checkFullDiskAccess, hasFullDiskAccess as _hasFullDiskAccess, domainStatus, checkDockerInstalled, dockerInstalled, restoreAllCaches, checkIntelligence } from "./stores/scanStore";
import type { DomainStatus } from "./stores/scanStore";

interface NavItem {
  id: string;
  label: string;
  icon: string | string[];
  section?: string;
}

const router = useRouter();
const route = useRoute();

// SVG path data for sidebar icons — simple, rounded, system-adjacent.
// Monochrome stroke icons, consistent 24x24 viewBox.
const icons = {
  dashboard:    "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-4 0v-6a1 1 0 011-1h2a1 1 0 011 1v6m-6 0h6",
  largeFiles:   "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z",
  caches:       "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10",
  logs:         "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2",
  docker:       [
    "M18 9.1V5a2 2 0 0 0-4 0",
    "M18 5a2 2 0 0 1 4 0",
    "M6 9.7 3.9 8.4C2.7 7.7 2 6.4 2 5V3c2 0 4 2 4 2s2-2 4-2v2c0 1.4-.7 2.7-1.9 3.4l-3.8 2.4A5 5 0 0 0 7 20h12c1.7 0 3-1.3 3-3v-3c0-2.8-2.2-5-5-5-2.7 0-5.1 1.4-6.4 3.6L9.7 14A2 2 0 0 1 6 13Z",
    "M15 15h.01",
  ],
  apps:         "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4",
  trash:        "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16",
  browsers:     "M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9",
  duplicates:   "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z",
  vault:        "M21 8V5a2 2 0 00-2-2H5a2 2 0 00-2 2v3m18 0v11a2 2 0 01-2 2H5a2 2 0 01-2-2V8m18 0H3m7 4h4",
  spaceMap:     "M11 3.055A9.001 9.001 0 1020.945 13H11V3.055z M20.488 9H15V3.512A9.025 9.025 0 0120.488 9z",
  maintenance:  "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
  security:     "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z",
  memory:       "M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z",
  packages:     "M20 7l-8-4-8 4m16 0v10l-8 4-8-4V7m16 0l-8 4-8-4m8 4v10",
  cpu:          "M6 18V6h12v12H6zM9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3M18 9h3m-3 6h3",
  vitals:       "M3 12h4l3-9 4 18 3-9h4",
  thermal:      "M12 9V3m0 6a3 3 0 100 6 3 3 0 000-6zm-1-6h2v6h-2V3zM12 21a6 6 0 01-4-10.46V4a4 4 0 118 0v6.54A6 6 0 0112 21z",
  settings:     "M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4",
};

const navSections: { label: string; items: NavItem[] }[] = [
  {
    label: "Overview",
    items: [
      { id: "dashboard", label: "Dashboard", icon: icons.dashboard },
    ],
  },
  {
    label: "Cleanup",
    items: [
      { id: "large-files", label: "Large Files", icon: icons.largeFiles },
      { id: "caches", label: "Caches", icon: icons.caches },
      { id: "logs", label: "Logs", icon: icons.logs },
      { id: "docker", label: "Docker", icon: icons.docker },
      { id: "apps", label: "Apps", icon: icons.apps },
      { id: "trash", label: "Trash", icon: icons.trash },
      { id: "browsers", label: "Browsers", icon: icons.browsers },
      { id: "duplicates", label: "Duplicates", icon: icons.duplicates },
      { id: "vault", label: "Vault", icon: icons.vault },
    ],
  },
  {
    label: "Analysis",
    items: [
      { id: "space-map", label: "Space Map", icon: icons.spaceMap },
      { id: "cpu", label: "CPU", icon: icons.cpu },
      { id: "memory", label: "Memory", icon: icons.memory },
      { id: "packages", label: "Packages", icon: icons.packages },
    ],
  },
  {
    label: "System",
    items: [
      { id: "thermal", label: "Thermal", icon: icons.thermal },
      { id: "maintenance", label: "Maintenance", icon: icons.maintenance },
      { id: "security", label: "Security", icon: icons.security },
      { id: "settings", label: "Settings", icon: icons.settings },
    ],
  },
];

// Map sidebar nav IDs to domain keys in domainStatus
const navToDomain: Record<string, string> = {
  "large-files": "largeFiles",
  "caches":      "caches",
  "logs":        "logs",
  "docker":      "docker",
  "apps":        "apps",
  "trash":       "trash",
  "browsers":    "browsers",
  "security":    "security",
};

// Get the domain scan status for a nav item (returns undefined for items without domains)
function getDomainStatus(navId: string): DomainStatus | undefined {
  const key = navToDomain[navId];
  if (!key) return undefined;
  return domainStatus.value[key]?.status;
}

// Domains where item count is more meaningful than total size
const countBasedDomains = new Set(["largeFiles", "duplicates"]);

// Format a scan-result badge for a completed domain
function getDomainBadge(navId: string): string | null {
  const key = navToDomain[navId];
  if (!key) return null;
  const info = domainStatus.value[key];
  if (!info || info.status !== "done") return null;
  if (countBasedDomains.has(key)) {
    return info.itemCount > 0 ? String(info.itemCount) : null;
  }
  return info.totalSize > 0 ? formatBadgeSize(info.totalSize) : null;
}

// Compact size format for badges: "4.2G", "800M", "12K"
function formatBadgeSize(bytes: number): string {
  if (bytes === 0) return "0";
  const units = ["B", "K", "M", "G", "T"];
  const k = 1024;
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), units.length - 1);
  const val = bytes / Math.pow(k, i);
  return (i === 0 ? val.toFixed(0) : val.toFixed(val < 10 ? 1 : 0)) + units[i];
}

// FDA gate state
const fdaChecked = ref(false);
const fdaDismissed = ref(false);

function navigateTo(navId: string) {
  router.push({ name: navId });
}

async function openSystemSettings() {
  try {
    await invoke("open_full_disk_access_settings");
  } catch (_) {}
}

async function recheckFda() {
  await checkFullDiskAccess();
}

function skipFda() {
  fdaDismissed.value = true;
}

const showApp = () => import.meta.env.DEV || _hasFullDiskAccess.value || fdaDismissed.value;

// --- Screen-anchored gradient background ---
// Queries ALL monitors via Tauri to compute the full virtual screen bounding
// box, renders a gradient bitmap covering that entire space, then positions it
// so the window acts as a viewport. On window move/resize, background-position
// updates so the gradient stays locked to screen coordinates across all monitors.


// Cached from init — the virtual screen origin (top-left of bounding box in
// logical px). Scale factor is read live from window.devicePixelRatio since
// it changes when the window moves between monitors with different DPIs.
// Virtual screen origin — stored for potential fallback CSS positioning.
// Primary positioning is handled by the native NSImageView layer.
let _vsOriginX = 0;
let _vsOriginY = 0;



// --- Gradient rendering: two-phase approach ---
// Phase 1 generates blob geometry (positions + sizes) shared by both main
// and sidebar gradients so the shapes correlate across the boundary.
// Phase 2 paints each bitmap with its own palette + filter settings.

interface PaletteEntry {
  color: [number, number, number];
  a: number;
  w: number;
}

interface BlobGeo {
  cx: number; cy: number;
  rx: number; ry: number;
  alphaScale: number;  // per-blob random multiplier (0.7–1.0)
}

interface Geometry {
  blobs: BlobGeo[];
  prngState: number;
}

// Shared blob geometry config — tuned in bg-test.html
const BLOB_SEED = 372057;
const BLOB_COUNT = 108;
const BLOB_SCALE = 1.0;
const BLOB_RMIN = 380 * BLOB_SCALE;
const BLOB_RMAX = 680 * BLOB_SCALE;
const BLOB_JITTER = 0.85;
const BLOB_HOLD = 0.35;              // slight hold so colours survive 70% white overlay

// Main gradient palette
const mainPalette: PaletteEntry[] = [
  { color: [230, 80, 140],  a: 0.75, w: 3 },  // hot pink
  { color: [50, 190, 180],  a: 0.85, w: 2 },  // teal
  { color: [140, 80, 200],  a: 0.85, w: 2 },  // purple
  { color: [240, 160, 80],  a: 0.65, w: 2 },  // orange — slightly lower to avoid dominance
  { color: [100, 200, 140], a: 0.70, w: 1 },  // green
  { color: [235, 120, 160], a: 0.75, w: 1 },  // rose
  { color: [80, 140, 220],  a: 0.85, w: 1 },  // blue
  { color: [200, 100, 180], a: 0.70, w: 1 },  // magenta
];

// Sidebar gradient palette — cool-toned frosted glass
const sidebarPalette: PaletteEntry[] = [
  { color: [40, 120, 140],  a: 0.85, w: 3 },  // deep teal
  { color: [70, 90, 150],   a: 0.80, w: 2 },  // slate blue
  { color: [140, 120, 170], a: 1.00, w: 2 },  // lavender
  { color: [160, 120, 140], a: 1.00, w: 1 },  // dusty rose
  { color: [90, 110, 140],  a: 0.75, w: 2 },  // cool gray
  { color: [50, 150, 160],  a: 0.80, w: 2 },  // dark aqua
];

/**
 * Phase 1: Generate blob geometry — deterministic positions + sizes.
 * Called ONCE and shared by both main and sidebar renders so shapes match.
 */
function generateBlobGeometry(w: number, h: number): Geometry {
  // Seeded PRNG (Park-Miller) for deterministic placement
  let s = BLOB_SEED;
  function rand() { s = (s * 16807) % 2147483647; return s / 2147483647; }

  const aspect = w / h;
  const cols = Math.round(Math.sqrt(BLOB_COUNT * aspect));
  const rows = Math.max(1, Math.round(BLOB_COUNT / cols));
  const cellW = w / cols;
  const cellH = h / rows;

  const blobs: BlobGeo[] = [];
  for (let r = 0; r < rows && blobs.length < BLOB_COUNT; r++) {
    for (let c = 0; c < cols && blobs.length < BLOB_COUNT; c++) {
      const cx = (c + 0.5) * cellW + (rand() - 0.5) * cellW * BLOB_JITTER;
      const cy = (r + 0.5) * cellH + (rand() - 0.5) * cellH * BLOB_JITTER;
      const rx = BLOB_RMIN + rand() * (BLOB_RMAX - BLOB_RMIN);
      const ry = (BLOB_RMIN + rand() * (BLOB_RMAX - BLOB_RMIN)) * (0.7 + rand() * 0.3);
      const alphaScale = 0.7 + rand() * 0.3;
      blobs.push({ cx, cy, rx, ry, alphaScale });
    }
  }

  return { blobs, prngState: s };
}

/**
 * Phase 2: Paint blobs onto a canvas using a specific palette + filter.
 * Renders at low resolution (RENDER_SCALE) then upscales to full size —
 * bilinear interpolation + pre-blur eliminates blob edges completely.
 */
const RENDER_SCALE = 0.20;

function paintBitmap(
  w: number, h: number,
  geometry: Geometry,
  pal: PaletteEntry[],
  baseFill: string,
  _blur: number,
  saturate: number,
  brightness: number,
): string {
  const rw = Math.max(1, Math.round(w * RENDER_SCALE));
  const rh = Math.max(1, Math.round(h * RENDER_SCALE));
  const canvas = document.createElement('canvas');
  canvas.width = rw;
  canvas.height = rh;
  const ctx = canvas.getContext('2d')!;

  ctx.fillStyle = baseFill;
  ctx.fillRect(0, 0, rw, rh);

  function drawBlob(cx: number, cy: number, rx: number, ry: number, color: string, alpha: number) {
    const scx = cx * RENDER_SCALE, scy = cy * RENDER_SCALE;
    const srx = rx * RENDER_SCALE, sry = ry * RENDER_SCALE;
    ctx.save();
    ctx.translate(scx, scy);
    ctx.scale(1, sry / srx);
    const grad = ctx.createRadialGradient(0, 0, 0, 0, 0, srx);
    grad.addColorStop(0, color);
    if (BLOB_HOLD > 0) grad.addColorStop(BLOB_HOLD, color);
    grad.addColorStop(1, 'transparent');
    ctx.globalAlpha = alpha;
    ctx.fillStyle = grad;
    ctx.fillRect(-srx, -srx, srx * 2, srx * 2);
    ctx.restore();
  }

  let s = geometry.prngState;
  function rand() { s = (s * 16807) % 2147483647; return s / 2147483647; }

  const weightedPalette: PaletteEntry[] = [];
  for (const p of pal) {
    for (let i = 0; i < p.w; i++) weightedPalette.push(p);
  }
  const unshuffled = weightedPalette.length > 0 ? weightedPalette : [...pal];
  const colorList = [...unshuffled];
  for (let i = colorList.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [colorList[i], colorList[j]] = [colorList[j], colorList[i]];
  }

  for (let i = 0; i < geometry.blobs.length; i++) {
    const b = geometry.blobs[i];
    const p = colorList[i % colorList.length];
    drawBlob(b.cx, b.cy, b.rx, b.ry, `rgb(${p.color[0]},${p.color[1]},${p.color[2]})`, p.a * b.alphaScale);
  }

  // Pre-blur at low-res before upscaling — 8px here ≈ 65px at full scale
  const preBlur = document.createElement('canvas');
  preBlur.width = rw; preBlur.height = rh;
  const pbCtx = preBlur.getContext('2d')!;
  pbCtx.filter = 'blur(8px)';
  pbCtx.drawImage(canvas, 0, 0);

  // Upscale — bilinear interpolation makes everything perfectly smooth
  const out = document.createElement('canvas');
  out.width = w; out.height = h;
  const oCtx = out.getContext('2d')!;
  oCtx.filter = `saturate(${saturate}) brightness(${brightness})`;
  oCtx.drawImage(preBlur, 0, 0, w, h);

  return out.toDataURL('image/jpeg', 0.92);
}

/**
 * Update background-position so the bitmap region under the window matches
 * its screen position. winX/winY are logical coords of the window's top-left.
 * We subtract the virtual screen origin so coordinates map to bitmap space.
 */


// --- Background position updates ---
// The main gradient inside the content panel is CSS-rendered and needs JS
// position updates via CSS variables.
let positionPollId: number | null = null;
let nativeBgUpdateInFlight = false;
let customDragActive = false;
let customDragStartScreenX = 0;
let customDragStartScreenY = 0;
let customDragStartWindowX = 0;
let customDragStartWindowY = 0;
let customDragPendingX: number | null = null;
let customDragPendingY: number | null = null;
let customDragSetPosInFlight = false;
let customDragMoveHandler: ((e: MouseEvent) => void) | null = null;
let customDragUpHandler: ((e: MouseEvent) => void) | null = null;
let customDragSetPosErrorCount = 0;
const DRAG_MODE_KEY = "negative_space_use_custom_js_drag";
let monitorWatchIntervalId: ReturnType<typeof setInterval> | null = null;
let monitorSignature = "";
let screenAnchorRefreshInFlight = false;
let screenAnchorRefreshQueued = false;

function updateBgPositionFromLogicalWindowPos(el: HTMLElement, lx: number, ly: number) {
  const bx = -(Math.round(lx) - _vsOriginX);
  const by = -(Math.round(ly) - _vsOriginY);
  el.style.setProperty('--bg-pos-x', `${bx}px`);
  el.style.setProperty('--bg-pos-y', `${by}px`);
}

function startPositionPolling() {
  if (positionPollId !== null) return;
  const appWindow = getCurrentWindow();
  const el = document.querySelector('#app') as HTMLElement;
  if (!el) return;

  function poll() {
    appWindow.outerPosition().then((pos) => {
      const dpr = window.devicePixelRatio || 2;
      const lx = pos.x / dpr;
      const ly = pos.y / dpr;
      updateBgPositionFromLogicalWindowPos(el, lx, ly);

      // Keep the native sidebar gradient in sync at rAF cadence.
      // One in-flight invoke avoids queue buildup under load.
      if (!nativeBgUpdateInFlight) {
        nativeBgUpdateInFlight = true;
        invoke('update_native_background_position', {
          windowX: lx,
          windowY: ly,
        })
          .catch(() => { /* native sync is best-effort */ })
          .finally(() => {
            nativeBgUpdateInFlight = false;
          });
      }
    });
    positionPollId = requestAnimationFrame(poll);
  }
  positionPollId = requestAnimationFrame(poll);
}

function stopPositionPolling() {
  if (positionPollId !== null) {
    cancelAnimationFrame(positionPollId);
    positionPollId = null;
  }
  nativeBgUpdateInFlight = false;
}

function stopCustomJsDrag() {
  customDragActive = false;
  customDragPendingX = null;
  customDragPendingY = null;
  customDragSetPosInFlight = false;
  customDragSetPosErrorCount = 0;
  if (customDragMoveHandler) {
    window.removeEventListener('mousemove', customDragMoveHandler);
    customDragMoveHandler = null;
  }
  if (customDragUpHandler) {
    window.removeEventListener('mouseup', customDragUpHandler);
    customDragUpHandler = null;
  }
}

function flushCustomJsDragPosition() {
  if (!customDragActive) return;
  if (customDragSetPosInFlight) return;
  if (customDragPendingX === null || customDragPendingY === null) return;
  const x = customDragPendingX;
  const y = customDragPendingY;
  customDragPendingX = null;
  customDragPendingY = null;
  customDragSetPosInFlight = true;

  getCurrentWindow()
    .setPosition(new LogicalPosition(Math.round(x), Math.round(y)))
    .catch((err) => {
      customDragSetPosErrorCount += 1;
      if (customDragSetPosErrorCount <= 3) {
        console.error('[custom-js-drag] setPosition failed:', err);
      }
    })
    .finally(() => {
      customDragSetPosInFlight = false;
      if (customDragPendingX !== null && customDragPendingY !== null) {
        flushCustomJsDragPosition();
      }
    });
}

function useCustomJsDrag(): boolean {
  try {
    const saved = localStorage.getItem(DRAG_MODE_KEY);
    // Default to true: this mode keeps gradients smooth during drag.
    return saved === null ? true : saved !== "false";
  } catch (_) {
    return true;
  }
}

type MonitorSnapshot = {
  position: { x: number; y: number };
  size: { width: number; height: number };
  scaleFactor: number;
};

function signatureFromMonitors(monitors: MonitorSnapshot[]): string {
  const normalized = monitors
    .map((m) => ({
      x: m.position.x,
      y: m.position.y,
      w: m.size.width,
      h: m.size.height,
      s: m.scaleFactor,
    }))
    .sort((a, b) => {
      if (a.x !== b.x) return a.x - b.x;
      if (a.y !== b.y) return a.y - b.y;
      if (a.w !== b.w) return a.w - b.w;
      if (a.h !== b.h) return a.h - b.h;
      return a.s - b.s;
    });
  return JSON.stringify(normalized);
}

async function refreshScreenAnchor() {
  if (screenAnchorRefreshInFlight) {
    screenAnchorRefreshQueued = true;
    return;
  }
  screenAnchorRefreshInFlight = true;
  try {
    await initScreenAnchor();
  } finally {
    screenAnchorRefreshInFlight = false;
    if (screenAnchorRefreshQueued) {
      screenAnchorRefreshQueued = false;
      void refreshScreenAnchor();
    }
  }
}

async function startMonitorWatch() {
  // Prime signature on startup so we only react to true topology changes.
  try {
    monitorSignature = signatureFromMonitors(await availableMonitors());
  } catch (_) {
    monitorSignature = "";
  }

  monitorWatchIntervalId = setInterval(async () => {
    try {
      const next = signatureFromMonitors(await availableMonitors());
      if (next !== monitorSignature) {
        monitorSignature = next;
        void refreshScreenAnchor();
      }
    } catch (_) {
      // Ignore transient monitor-query failures.
    }
  }, 1500);
}

function stopMonitorWatch() {
  if (monitorWatchIntervalId !== null) {
    clearInterval(monitorWatchIntervalId);
    monitorWatchIntervalId = null;
  }
}

async function initScreenAnchor() {
  // Query all monitors to compute the virtual screen bounding box.
  // IMPORTANT: Tauri reports monitor position in *global display coords*
  // (which on macOS are already in logical points for AppKit), and size in
  // physical pixels. Each monitor may have a different scale factor, so we
  // must use each monitor's own sf to convert its size to logical pixels.
  const monitors = await availableMonitors();
  let minX = 0, minY = 0, maxX = 3840, maxY = 2160;

  if (monitors.length > 0) {
    minX = Infinity; minY = Infinity; maxX = -Infinity; maxY = -Infinity;
    for (const m of monitors) {
      // position.x/y are in global display points (logical on macOS)
      // size.width/height are in physical pixels — divide by THIS monitor's sf
      const lx = m.position.x;
      const ly = m.position.y;
      const lw = m.size.width / m.scaleFactor;
      const lh = m.size.height / m.scaleFactor;
      minX = Math.min(minX, lx);
      minY = Math.min(minY, ly);
      maxX = Math.max(maxX, lx + lw);
      maxY = Math.max(maxY, ly + lh);
    }
  }

  // Virtual screen dimensions in logical pixels
  _vsOriginX = minX;
  _vsOriginY = minY;
  const vsW = Math.round(maxX - minX);
  const vsH = Math.round(maxY - minY);


  // Pre-render both gradient bitmaps covering the entire virtual screen.
  // Phase 1: shared blob geometry (identical positions + sizes)
  const geometry = generateBlobGeometry(vsW, vsH);

  // Phase 2a: Main gradient (warm palette) — stays in CSS (needs border-radius clipping)
  const mainUrl = paintBitmap(vsW, vsH, geometry, mainPalette,
    '#b8a8cc', 120, 6.0, 1.10);

  // Phase 2b: Sidebar gradient (cool palette) — sent to Rust for native layer
  const sidebarUrl = paintBitmap(vsW, vsH, geometry, sidebarPalette,
    '#1e2e3e', 80, 2.0, 0.85);

  // Set main gradient as CSS background on the content panel (webview-rendered).
  // Jitter through the 70% white overlay is imperceptible.
  const bgEl = document.querySelector('#app') as HTMLElement;
  if (bgEl) {
    bgEl.style.setProperty('--gradient-bg', `url(${mainUrl})`);
    bgEl.style.setProperty('--screen-w', `${vsW}px`);
    bgEl.style.setProperty('--screen-h', `${vsH}px`);
  }

  // Send sidebar gradient to Rust to create a native NSImageView behind the
  // webview. This is positioned by the window compositor for zero-lag tracking.
  const sidebarB64 = sidebarUrl.split(',')[1];
  try {
    await invoke('set_native_background', {
      sidebarJpeg: sidebarB64,
      originX: minX,
      originY: minY,
      screenW: vsW,
      screenH: vsH,
    });
  } catch (e) {
    console.error('Failed to set native background:', e);
    // Fallback: set sidebar gradient as CSS background too
    if (bgEl) {
      bgEl.style.setProperty('--sidebar-gradient-bg', `url(${sidebarUrl})`);
    }
  }

  // Start rAF polling for the main gradient position (CSS-rendered in content panel).
  // The sidebar gradient doesn't need this — it's a native layer.
  startPositionPolling();
}

function startDrag(e: MouseEvent) {
  // Only drag on primary button, and not when clicking interactive elements
  if (e.button !== 0) return;
  e.preventDefault();
  const tag = (e.target as HTMLElement).tagName.toLowerCase();
  if (["button", "a", "input", "select", "textarea"].includes(tag)) return;
  if (!useCustomJsDrag()) {
    getCurrentWindow().startDragging();
    return;
  }

  const appWindow = getCurrentWindow();
  const appEl = document.querySelector('#app') as HTMLElement | null;
  const startDpr = window.devicePixelRatio || 2;

  appWindow.outerPosition().then((pos) => {
    customDragActive = true;
    customDragStartScreenX = e.screenX;
    customDragStartScreenY = e.screenY;
    // Keep everything in logical coordinates for setPosition(LogicalPosition).
    customDragStartWindowX = pos.x / startDpr;
    customDragStartWindowY = pos.y / startDpr;
    customDragPendingX = null;
    customDragPendingY = null;
    customDragSetPosInFlight = false;
    customDragSetPosErrorCount = 0;

    customDragMoveHandler = (moveEv: MouseEvent) => {
      if (!customDragActive) return;
      const dx = moveEv.screenX - customDragStartScreenX;
      const dy = moveEv.screenY - customDragStartScreenY;
      const nextX = customDragStartWindowX + dx;
      const nextY = customDragStartWindowY + dy;
      customDragPendingX = nextX;
      customDragPendingY = nextY;
      if (appEl) {
        updateBgPositionFromLogicalWindowPos(appEl, nextX, nextY);
      }
      flushCustomJsDragPosition();
    };

    customDragUpHandler = () => {
      stopCustomJsDrag();
    };

    window.addEventListener('mousemove', customDragMoveHandler);
    window.addEventListener('mouseup', customDragUpHandler);
  });
}

onMounted(async () => {
  try {
    await checkFullDiskAccess();
  } catch (_) { /* FDA check may fail in dev mode — proceed anyway */ }
  fdaChecked.value = true;
  // Fire-and-forget: don't block render waiting for these
  void refreshScreenAnchor();
  void startMonitorWatch();
  void checkDockerInstalled();
  void restoreAllCaches();
  void checkIntelligence();
});

onUnmounted(() => {
  stopPositionPolling();
  stopCustomJsDrag();
  stopMonitorWatch();
});
</script>

<template>
  <!-- FDA Setup Gate -->
  <div v-if="fdaChecked && !showApp()" class="fda-gate">
    <div class="fda-drag-strip" @mousedown="startDrag"></div>
    <button class="btn-gate-skip" @click="skipFda">Skip for now</button>
    <div class="fda-gate-content">
      <div class="fda-gate-icon">
        <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
          <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
        </svg>
      </div>
      <h1>Welcome to Negativ_</h1>
      <p class="fda-gate-subtitle">
        For the most thorough scan, Negativ_ needs Full Disk Access.
      </p>
      <p class="fda-gate-detail">
        Without it, protected folders like Desktop, Documents, and Downloads
        will be skipped. You can always grant access later in Settings.
      </p>

      <div class="fda-gate-steps">
        <div class="fda-step">
          <span class="fda-step-num">1</span>
          <span>Click <strong>Open System Settings</strong> below</span>
        </div>
        <div class="fda-step">
          <span class="fda-step-num">2</span>
          <span>Find <strong>Full Disk Access</strong> in the list</span>
        </div>
        <div class="fda-step">
          <span class="fda-step-num">3</span>
          <span>Toggle <strong>Negativ_</strong> on</span>
        </div>
        <div class="fda-step">
          <span class="fda-step-num">4</span>
          <span>Come back and click <strong>Re-check</strong></span>
        </div>
      </div>

      <div class="fda-gate-actions">
        <button class="btn-gate-primary" @click="openSystemSettings">
          Open System Settings
        </button>
        <button class="btn-gate-secondary" @click="recheckFda">
          Re-check Access
        </button>
      </div>
    </div>
  </div>

  <!-- Main App — sidebar gradient is full-bleed, content panel inset -->
  <div v-else-if="fdaChecked" class="app-layout">
    <!-- Sidebar nav — sits on the native NSImageView gradient layer (behind webview) -->
    <aside class="sidebar" @mousedown="startDrag">
      <div class="sidebar-header" @mousedown="startDrag">
        <h1 class="app-title">Negativ_</h1>
      </div>

      <nav class="sidebar-nav">
        <template v-for="(section, sIdx) in navSections" :key="section.label">
          <div v-if="sIdx > 0" class="nav-divider"></div>
          <span class="nav-section-label">{{ section.label }}</span>
          <button
            v-for="item in section.items"
            v-show="item.id !== 'docker' || dockerInstalled !== false"
            :key="item.id"
            :class="['nav-item', { active: route.name === item.id }]"
            @click="navigateTo(item.id)"
          >
            <svg class="nav-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <template v-if="Array.isArray(item.icon)">
                <path v-for="(d, pi) in item.icon" :key="pi" :d="d"/>
              </template>
              <path v-else :d="item.icon"/>
            </svg>
            <span class="nav-label">{{ item.label }}</span>
            <!-- Domain scan status badge -->
            <span v-if="getDomainStatus(item.id) === 'scanning'" class="nav-badge nav-badge-scanning">
              <span class="spinner-xs"></span>
            </span>
            <span v-else-if="getDomainBadge(item.id)" class="nav-badge nav-badge-result">
              {{ getDomainBadge(item.id) }}
            </span>
            <span v-else-if="getDomainStatus(item.id) === 'done'" class="nav-badge nav-badge-done">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </span>
            <span v-else-if="getDomainStatus(item.id) === 'error'" class="nav-badge nav-badge-error">
              <span class="nav-badge-dot"></span>
            </span>
          </button>
        </template>
      </nav>

      <div class="sidebar-footer">
        <span class="version-tag">v0.1.0 (build {{ BUILD_NUMBER }})</span>
      </div>
    </aside>

    <!-- Content panel — inset with rounded corners, main gradient inside -->
    <main class="content-panel">
      <div class="content">
        <div class="content-drag-strip" @mousedown="startDrag"></div>
        <router-view v-slot="{ Component }">
          <KeepAlive>
            <component :is="Component" />
          </KeepAlive>
        </router-view>
      </div>
    </main>
  </div>

  <!-- Loading -->
  <div v-else class="fda-gate">
    <div class="fda-drag-strip" @mousedown="startDrag"></div>
    <div class="fda-gate-content">
      <div class="spinner spinner-centered"></div>
      <p style="color: var(--muted); font-size: 14px;">Checking permissions...</p>
    </div>
  </div>
</template>

<style scoped>
/* ==========================================================================
   FDA Gate — frosted glass onboarding
   ========================================================================== */
.fda-gate {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  position: relative;
}

.fda-drag-strip {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 48px;
}

.btn-gate-skip {
  position: absolute;
  top: 52px;
  right: 24px;
  background: rgba(255, 255, 255, 0.4);
  border: 1px solid var(--glass-border);
  color: var(--muted);
  font-size: 13px;
  font-weight: 500;
  padding: 7px 18px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}

.btn-gate-skip:hover {
  background: rgba(255, 255, 255, 0.65);
  color: var(--text-secondary);
}

.fda-gate-content {
  max-width: 480px;
  text-align: center;
  padding: 48px;
  background: var(--glass);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-lg);
}

.fda-gate-icon {
  color: var(--accent);
  margin-bottom: 24px;
  opacity: 0.85;
}

.fda-gate-content h1 {
  font-size: 26px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: -0.5px;
  margin-bottom: 8px;
}

.fda-gate-subtitle {
  font-size: 15px;
  color: var(--text-secondary);
  line-height: 1.55;
  margin-bottom: 6px;
}

.fda-gate-detail {
  font-size: 13px;
  color: var(--muted);
  line-height: 1.6;
  margin-bottom: 32px;
}

.fda-gate-steps {
  text-align: left;
  margin-bottom: 32px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.fda-step {
  display: flex;
  align-items: center;
  gap: 14px;
  font-size: 14px;
  color: var(--text);
}

.fda-step-num {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--accent-light);
  color: var(--accent-deep);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.fda-gate-actions {
  display: flex;
  gap: 10px;
  justify-content: center;
}

.btn-gate-primary {
  padding: 12px 28px;
  font-size: 14px;
  font-weight: 600;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--accent);
  color: white;
  cursor: pointer;
  transition: background 0.2s, box-shadow 0.2s, transform 0.2s;
  box-shadow: 0 2px 8px rgba(59, 199, 232, 0.25);
}

.btn-gate-primary:hover {
  background: var(--accent-hover);
  box-shadow: 0 4px 12px rgba(59, 199, 232, 0.3);
  transform: translateY(-0.5px);
}

.btn-gate-secondary {
  padding: 12px 28px;
  font-size: 14px;
  font-weight: 600;
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.4);
  color: var(--text);
  cursor: pointer;
  transition: background 0.2s;
}

.btn-gate-secondary:hover {
  background: rgba(255, 255, 255, 0.65);
}

/* ==========================================================================
   App Layout — sidebar gradient is full-bleed (#app::before),
   content panel is inset with rounded corners
   ========================================================================== */
.app-layout {
  position: relative;
  height: 100vh;
  overflow: hidden;
}

/* ==========================================================================
   Sidebar — nav text sits directly on the full-bleed sidebar gradient.
   No backdrop-filter needed — the sidebar gradient IS the background.
   ========================================================================== */
.sidebar {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  width: 230px;
  background: transparent;
  display: flex;
  flex-direction: column;
  user-select: none;
  z-index: 3;
}

.sidebar-header {
  padding: 48px 22px 18px;
}

.app-title {
  font-size: 17px;
  font-weight: 700;
  color: var(--sidebar-text);
  letter-spacing: -0.3px;
}

.sidebar-nav {
  flex: 1;
  padding: 4px 10px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

/* Section labels */
.nav-section-label {
  display: block;
  font-size: 10px;
  font-weight: 600;
  color: var(--sidebar-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  padding: 12px 12px 5px;
}

.nav-divider {
  height: 1px;
  background: var(--sidebar-divider);
  margin: 4px 12px;
}

/* Nav items */
.nav-item {
  display: flex;
  align-items: center;
  gap: 11px;
  width: 100%;
  padding: 8px 12px;
  border-radius: 10px;
  background: transparent;
  border: none;
  cursor: pointer;
  transition: background 0.15s ease, box-shadow 0.15s ease;
  text-align: left;
}

.nav-item:hover {
  background: var(--sidebar-hover);
}

/* Active pill — aqua-to-mint gradient */
.nav-item.active {
  background: linear-gradient(135deg, rgba(59, 199, 232, 0.60) 0%, rgba(59, 199, 232, 0.40) 100%);
  box-shadow: 0 1px 6px rgba(59, 199, 232, 0.20);
}

.nav-icon {
  flex-shrink: 0;
  color: rgba(255, 255, 255, 0.75);
  transition: color 0.15s;
}

.nav-item:hover .nav-icon {
  color: var(--sidebar-text);
}

.nav-item.active .nav-icon {
  color: #ffffff;
}

.nav-label {
  font-size: 13px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.75);
  line-height: 1.3;
  transition: color 0.15s;
}

.nav-item:hover .nav-label {
  color: var(--sidebar-text);
}

.nav-item.active .nav-label {
  color: var(--sidebar-text);
  font-weight: 600;
}

/* Nav status badges — appear at right edge of nav items */
.nav-badge {
  margin-left: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.nav-badge-scanning {
  opacity: 0.85;
}

/* Override spinner-xs color to match sidebar white text */
.nav-badge-scanning :deep(.spinner-xs) {
  border-color: rgba(255, 255, 255, 0.15);
  border-top-color: rgba(255, 255, 255, 0.8);
  width: 12px;
  height: 12px;
}

.nav-item.active .nav-badge-scanning :deep(.spinner-xs) {
  border-color: rgba(255, 255, 255, 0.2);
  border-top-color: #ffffff;
}

.nav-badge-result {
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 0.02em;
  padding: 1px 6px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.18);
  color: rgba(255, 255, 255, 0.85);
  line-height: 1.4;
}

.nav-item.active .nav-badge-result {
  background: rgba(255, 255, 255, 0.22);
  color: rgba(255, 255, 255, 0.95);
}

.nav-badge-done {
  color: var(--success);
  opacity: 0.8;
}

.nav-badge-error .nav-badge-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--danger);
  opacity: 0.8;
}

.sidebar-footer {
  padding: 14px 22px;
  border-top: 1px solid var(--sidebar-divider);
}

.version-tag {
  font-size: 11px;
  color: #ffffff;
  font-family: var(--font-mono);
  opacity: 0.7;
}

/* ==========================================================================
   Content panel — inset from edges, rounded corners.
   The sidebar gradient wraps around this panel (visible in gutters).
   Main (warm) gradient shows through via ::before, white overlay on top.
   ========================================================================== */
.content-panel {
  position: absolute;
  top: 8px;
  right: 8px;
  bottom: 8px;
  left: 238px; /* 230px sidebar + 8px gap */
  border-radius: 12px;
  overflow: hidden;
  z-index: 2;
}

/* Main (warm) gradient — inside content panel, screen-anchored via
   background-position. Jitter is imperceptible through the white overlay. */
.content-panel::before {
  content: '';
  position: absolute;
  inset: -20px;
  background-image: var(--gradient-bg, none);
  background-size: var(--screen-w, 3840px) var(--screen-h, 2160px);
  background-position:
    calc(var(--bg-pos-x, 0px) + 20px - 238px)
    calc(var(--bg-pos-y, 0px) + 20px - 8px);
  background-repeat: no-repeat;
  z-index: 0;
}

/* White frosted overlay on top of main gradient for readability */
.content-panel::after {
  content: '';
  position: absolute;
  inset: 0;
  background: rgba(255, 255, 255, 0.70);
  z-index: 1;
}

.content {
  position: relative;
  z-index: 2;
  height: 100%;
  overflow-y: auto;
  padding: 0 40px 40px;
}

/* Invisible drag region at top of content area */
.content-drag-strip {
  height: 48px;
  flex-shrink: 0;
}

/* View transitions removed — opacity animations cause white flash on transparent
   Tauri windows when composited over blur filters. Instant swap is cleanest. */

.spinner-centered { margin: 0 auto var(--sp-4); }
</style>
