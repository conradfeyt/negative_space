/**
 * useScreenGradient — screen-anchored gradient background system.
 *
 * Queries all monitors via Tauri to compute the full virtual screen bounding
 * box, renders gradient bitmaps covering that entire space, then positions them
 * so the window acts as a viewport. On window move/resize, background-position
 * updates so the gradient stays locked to screen coordinates across all monitors.
 *
 * Two-phase rendering:
 *   Phase 1 — shared blob geometry (positions + sizes) for both gradients.
 *   Phase 2 — paint each bitmap with its own palette + filter settings.
 *     2a: Main (warm) gradient — CSS background on content panel.
 *     2b: Sidebar (cool) gradient — sent to Rust native NSImageView layer.
 */

import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, availableMonitors } from "@tauri-apps/api/window";
import { LogicalPosition } from "@tauri-apps/api/dpi";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PaletteEntry {
  color: [number, number, number];
  a: number;
  w: number;
}

interface BlobGeo {
  cx: number; cy: number;
  rx: number; ry: number;
  alphaScale: number;  // per-blob random multiplier (0.7-1.0)
}

interface Geometry {
  blobs: BlobGeo[];
  prngState: number;
}

type MonitorSnapshot = {
  position: { x: number; y: number };
  size: { width: number; height: number };
  scaleFactor: number;
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Shared blob geometry config — tuned in bg-test.html
const BLOB_SEED = 372057;
const BLOB_COUNT = 108;
const BLOB_SCALE = 1.0;
const BLOB_RMIN = 380 * BLOB_SCALE;
const BLOB_RMAX = 680 * BLOB_SCALE;
const BLOB_JITTER = 0.85;
const BLOB_HOLD = 0.35;              // slight hold so colours survive 70% white overlay

const RENDER_SCALE = 0.20;

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

const DRAG_MODE_KEY = "negative_space_use_custom_js_drag";

// ---------------------------------------------------------------------------
// Pure helpers (no side effects)
// ---------------------------------------------------------------------------

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

  // Pre-blur at low-res before upscaling — 8px here ~ 65px at full scale
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

function useCustomJsDrag(): boolean {
  try {
    const saved = localStorage.getItem(DRAG_MODE_KEY);
    // Default to true: this mode keeps gradients smooth during drag.
    return saved === null ? true : saved !== "false";
  } catch (_) {
    return true;
  }
}

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export function useScreenGradient() {
  // Virtual screen origin — stored for potential fallback CSS positioning.
  // Primary positioning is handled by the native NSImageView layer.
  let _vsOriginX = 0;
  let _vsOriginY = 0;

  // Position polling state
  let positionPollId: number | null = null;
  let nativeBgUpdateInFlight = false;

  // Custom JS drag state
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

  // Monitor watch state
  let monitorWatchIntervalId: ReturnType<typeof setInterval> | null = null;
  let monitorSignature = "";
  let screenAnchorRefreshInFlight = false;
  let screenAnchorRefreshQueued = false;

  // -----------------------------------------------------------------------
  // Background position updates
  // -----------------------------------------------------------------------

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

  // -----------------------------------------------------------------------
  // Custom JS drag (keeps gradient smooth during window drag)
  // -----------------------------------------------------------------------

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

  // -----------------------------------------------------------------------
  // Monitor topology watching
  // -----------------------------------------------------------------------

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

  // -----------------------------------------------------------------------
  // Screen anchor initialisation (renders both gradient bitmaps)
  // -----------------------------------------------------------------------

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

  // -----------------------------------------------------------------------
  // Window drag handler (exposed to App.vue for @mousedown bindings)
  // -----------------------------------------------------------------------

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

  // -----------------------------------------------------------------------
  // Lifecycle — init on mount, cleanup on unmount
  // -----------------------------------------------------------------------

  onMounted(() => {
    void refreshScreenAnchor();
    void startMonitorWatch();
  });

  onUnmounted(() => {
    stopPositionPolling();
    stopCustomJsDrag();
    stopMonitorWatch();
  });

  // -----------------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------------

  return {
    /** Attach to @mousedown on draggable regions (sidebar, drag strips). */
    startDrag,
  };
}
