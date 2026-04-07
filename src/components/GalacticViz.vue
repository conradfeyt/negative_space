<script setup lang="ts">
/**
 * GalacticViz — Navigable star-field with orbital physics.
 *
 * Data mapping:
 *   Stars   = top-level directories (size → star size/brightness)
 *   Planets = depth-1 children (orbit their star)
 *   Moons   = depth-2 children (orbit their planet)
 *
 * Navigation:
 *   Drag to pan, scroll to zoom, click system to center+zoom
 *
 * Visual:
 *   - Stars: white-hot cores, diffraction flares, size proportional to bytes
 *   - Planets: colored dots orbiting on rings, size proportional to bytes
 *   - Moons: tiny dots orbiting planets
 *   - Orbits animate slowly over time
 *   - Background twinkling star dust
 *   - Nebula haze behind large systems
 */
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from "vue";
import * as d3 from "d3";
import { formatSize } from "../utils";
import type { DiskNode, DiskMapResult } from "../types";
import { useZoomPan } from "../composables/useZoomPan";

const props = defineProps<{
  data: DiskMapResult | null;
  expanded: boolean;
}>();

const emit = defineEmits<{
  (e: "update:expanded", value: boolean): void;
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------
const palette = [
  { h: 220, s: 55, l: 35 },
  { h: 35,  s: 60, l: 38 },
  { h: 170, s: 45, l: 32 },
  { h: 340, s: 45, l: 38 },
  { h: 50,  s: 50, l: 36 },
  { h: 265, s: 40, l: 35 },
  { h: 190, s: 50, l: 30 },
  { h: 15,  s: 55, l: 38 },
  { h: 95,  s: 35, l: 32 },
  { h: 300, s: 35, l: 34 },
];

// ---------------------------------------------------------------------------
// Types — true orbital hierarchy
// ---------------------------------------------------------------------------
interface Moon {
  name: string;
  path: string;
  size: number;
  radius: number;        // visual radius
  orbitRadius: number;    // distance from planet center
  orbitSpeed: number;     // radians per second
  orbitPhase: number;     // starting angle
  hsl: { h: number; s: number; l: number };
}

interface Planet {
  name: string;
  path: string;
  size: number;
  radius: number;
  orbitRadius: number;    // distance from star center
  orbitSpeed: number;
  orbitPhase: number;
  hsl: { h: number; s: number; l: number };
  moons: Moon[];
}

interface StarSystem {
  name: string;
  path: string;
  size: number;
  starRadius: number;     // visual core radius
  flareLength: number;
  hsl: { h: number; s: number; l: number };
  // Position in world space
  wx: number;
  wy: number;
  planets: Planet[];
  // How much space this system occupies (outermost orbit)
  systemRadius: number;
  twinklePhase: number;
  twinkleSpeed: number;
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------
let systems: StarSystem[] = [];
const systemsReady = ref(false); // reactive flag for template bindings
let bgStars: { x: number; y: number; s: number; b: number; t: number }[] = [];
let width = 0;
let height = 0;
let dpr = 1;
let animFrame = 0;
let time = 0;

// Camera — pan/zoom in world coordinates.
// camZoom stays as a plain local variable because it's referenced in ~50 places
// including smoothPanTo animation. The composable handles drag state and wheel
// gating; camZoom is updated directly in the onZoom callback.
let camX = 0;    // world X at canvas center
let camY = 0;
let camZoom = 1; // 1 = default, >1 = zoomed in

// Animation control — pause when window hidden or component not visible
let paused = false;

function onVisibilityChange() {
  if (document.hidden) {
    paused = true;
    cancelAnimationFrame(animFrame);
  } else {
    if (paused && systems.length > 0) {
      paused = false;
      animFrame = requestAnimationFrame(render);
    }
  }
}

// Drag start camera snapshot (set in onDragStart callback)
let camStartX = 0;
let camStartY = 0;

// Hover
let hoveredBody: { name: string; size: number; x: number; y: number; type: string } | null = null;

// ---------------------------------------------------------------------------
// Zoom/pan composable — handles drag state machine and wheel gating.
// Canvas-specific coordinate math is in the callbacks.
// ---------------------------------------------------------------------------
const zoomPan = useZoomPan(
  { minScale: 0.15, maxScale: 12, dragThreshold: 3 },
  {
    onZoom(e, newScale, _oldScale) {
      const canvas = canvasRef.value;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;

      // Zoom toward cursor position
      const [wx, wy] = screenToWorld(mx, my);
      // Adjust camera so the world point under cursor stays fixed
      camX = wx - (mx - width / 2) / newScale;
      camY = wy - (my - height / 2) / newScale;
      camZoom = newScale;
    },
    onPan(_e, pixelDx, pixelDy) {
      // pixelDx/pixelDy are total delta from drag start (client coords).
      const dx = pixelDx / camZoom;
      const dy = pixelDy / camZoom;
      camX = camStartX - dx;
      camY = camStartY - dy;
      const canvas = canvasRef.value;
      if (canvas) canvas.style.cursor = "grabbing";
      hoveredBody = null;
    },
    onDragStart() {
      camStartX = camX;
      camStartY = camY;
      const canvas = canvasRef.value;
      if (canvas) canvas.style.cursor = "grabbing";
    },
    onDragEnd() {
      const canvas = canvasRef.value;
      if (canvas) canvas.style.cursor = "grab";
    },
  },
);

// Expand / compact — emits to parent, parent handles layout.
// Scale camZoom proportionally so the scene fills the new canvas size.
function toggleExpand() {
  const oldSize = Math.min(width, height) || 1;
  emit("update:expanded", !props.expanded);
  nextTick(() => {
    resize();
    const newSize = Math.min(width, height) || 1;
    camZoom *= newSize / oldSize;
    zoomPan.state.scale = camZoom;
  });
}

// Selected system info panel
const selectedSystem = ref<StarSystem | null>(null);
const totalDiskUsed = ref(0);

// Computed: sorted planets for the info panel (largest first, top 15)
const selectedSystemPlanets = computed(() => {
  if (!selectedSystem.value) return [];
  return [...selectedSystem.value.planets]
    .sort((a, b) => b.size - a.size)
    .slice(0, 15);
});

// Computed: legend items showing what star sizes mean
const legendItems = computed(() => {
  if (!systemsReady.value || systems.length === 0) return [];
  const sorted = [...systems].sort((a, b) => b.size - a.size);
  // Pick 3 representative entries: largest, median, smallest
  const picks: StarSystem[] = [];
  if (sorted.length >= 1) picks.push(sorted[0]);
  if (sorted.length >= 3) picks.push(sorted[Math.floor(sorted.length / 2)]);
  if (sorted.length >= 2) picks.push(sorted[sorted.length - 1]);
  return picks.map((s) => ({
    label: `${s.name} (${formatSize(s.size)})`,
    px: Math.max(4, Math.min(14, s.starRadius * 0.8)),
  }));
});

// ---------------------------------------------------------------------------
// Build star systems from DiskNode tree
// ---------------------------------------------------------------------------
function buildSystems(root: DiskNode): StarSystem[] {
  const tops = (root.children || []).filter(
    (c) => c.size > 0 && (c.path || !c.name.includes("other"))
  );

  // Scale star radius by size. Library=166GB should be big, .npm=tiny.
  const sizes = tops.map((t) => t.size);
  const maxSize = Math.max(...sizes) || 1;
  const minSize = Math.min(...sizes) || 1;
  const starScale = d3.scalePow().exponent(0.35).domain([minSize, maxSize]).range([3, 18]).clamp(true);

  // Planet scale (relative to their siblings)
  const allChildSizes: number[] = [];
  for (const t of tops) {
    for (const c of t.children || []) if (c.size > 0) allChildSizes.push(c.size);
  }
  const childMin = Math.min(...allChildSizes) || 1;
  const childMax = Math.max(...allChildSizes) || 1;
  const planetScale = d3.scalePow().exponent(0.35).domain([childMin, childMax]).range([1.5, 7]).clamp(true);

  // Place systems in a scattered layout — larger systems get more space
  // Use a force-like layout: pack circles by system radius
  const result: StarSystem[] = [];

  tops.forEach((top, ti) => {
    const pal = palette[ti % palette.length];
    const sr = starScale(top.size);

    // Build planets
    const childNodes = (top.children || []).filter(
      (c) => c.size > 0 && (c.path || !c.name.includes("other"))
    );

    const planets: Planet[] = [];
    let orbitDist = sr + 15; // first orbit starts outside the star

    childNodes.forEach((child, ci) => {
      const pr = planetScale(child.size);
      const orbitR = orbitDist + pr + 5;
      orbitDist = orbitR + pr + 8; // gap between orbits

      // Build moons from depth-2
      const moonNodes = (child.children || []).filter(
        (m) => m.size > 0 && (m.path || !m.name.includes("other"))
      ).slice(0, 5); // max 5 moons per planet

      const moons: Moon[] = [];
      let moonOrbit = pr + 4;
      moonNodes.forEach((mn) => {
        const mr = Math.max(0.8, pr * 0.2 + Math.random() * 0.5);
        moonOrbit += mr + 2;
        moons.push({
          name: mn.name,
          path: mn.path,
          size: mn.size,
          radius: mr,
          orbitRadius: moonOrbit,
          orbitSpeed: 0.08 + Math.random() * 0.12,
          orbitPhase: Math.random() * Math.PI * 2,
          hsl: { h: pal.h, s: pal.s * 0.6, l: pal.l + 5 + Math.random() * 10 },
        });
        moonOrbit += mr + 2;
      });

      planets.push({
        name: child.name,
        path: child.path,
        size: child.size,
        radius: pr,
        orbitRadius: orbitR,
        orbitSpeed: 0.015 + (0.03 / (ci + 1)), // inner planets orbit faster
        orbitPhase: Math.random() * Math.PI * 2,
        hsl: { h: pal.h + (ci * 15 - 20), s: Math.max(30, pal.s + (Math.random() - 0.5) * 15), l: pal.l + (Math.random() - 0.5) * 8 },
        moons,
      });
    });

    const sysRadius = orbitDist + 10;

    result.push({
      name: top.name,
      path: top.path,
      size: top.size,
      starRadius: sr,
      flareLength: sr * 4,
      hsl: { h: pal.h, s: pal.s * 0.5, l: 15 },
      wx: 0,
      wy: 0,
      planets,
      systemRadius: sysRadius,
      twinklePhase: Math.random() * Math.PI * 2,
      twinkleSpeed: 0.3 + Math.random() * 0.4,
    });
  });

  // -------------------------------------------------------------------
  // Gravity-based force layout — runs at build time (200 iterations)
  // -------------------------------------------------------------------
  result.sort((a, b) => b.size - a.size);

  // Seed positions on a tight spiral
  if (result.length > 0) {
    result[0].wx = 0;
    result[0].wy = 0;
    let angle = 0;
    for (let i = 1; i < result.length; i++) {
      angle += 2.4;
      const r = (result[i].systemRadius + result[i - 1].systemRadius) * 0.5;
      result[i].wx = Math.cos(angle) * r * (i * 0.6);
      result[i].wy = Math.sin(angle) * r * (i * 0.6);
    }
  }

  const totalMass = result.reduce((s, r) => s + r.size, 0) || 1;

  for (let iter = 0; iter < 200; iter++) {
    const temp = 1.0 - iter / 200;
    const dt = 0.3 * (0.2 + temp * 0.8);

    for (let i = 0; i < result.length; i++) {
      let fx = 0;
      let fy = 0;
      const a = result[i];
      const massA = a.size / totalMass;

      const distToCenter = Math.sqrt(a.wx * a.wx + a.wy * a.wy) + 1;
      const gravStrength = 2.0 * massA;
      fx -= (a.wx / distToCenter) * gravStrength;
      fy -= (a.wy / distToCenter) * gravStrength;

      for (let j = 0; j < result.length; j++) {
        if (i === j) continue;
        const b = result[j];
        const dx = a.wx - b.wx;
        const dy = a.wy - b.wy;
        const dist = Math.sqrt(dx * dx + dy * dy) + 0.1;
        const minDist = a.systemRadius + b.systemRadius + 12;

        if (dist < minDist) {
          const overlap = (minDist - dist) / minDist;
          const repel = overlap * overlap * 8;
          fx += (dx / dist) * repel;
          fy += (dy / dist) * repel;
        } else {
          const massB = b.size / totalMass;
          const attract = 0.3 * massA * massB / (dist * 0.02 + 1);
          fx -= (dx / dist) * attract;
          fy -= (dy / dist) * attract;
        }
      }

      a.wx += fx * dt * 20;
      a.wy += fy * dt * 20;
    }
  }

  // Re-center on center of mass
  const comX = result.reduce((s, r) => s + r.wx * r.size, 0) / totalMass;
  const comY = result.reduce((s, r) => s + r.wy * r.size, 0) / totalMass;
  for (const sys of result) {
    sys.wx -= comX;
    sys.wy -= comY;
  }

  return result;
}

// ---------------------------------------------------------------------------
// Background stars
// ---------------------------------------------------------------------------
function genBgStars() {
  bgStars = [];
  // Generate in a large world-space area
  const extent = 3000;
  const count = 2000;
  for (let i = 0; i < count; i++) {
    bgStars.push({
      x: (Math.random() - 0.5) * extent * 2,
      y: (Math.random() - 0.5) * extent * 2,
      s: Math.random() < 0.93 ? 0.3 + Math.random() * 0.4 : 0.7 + Math.random() * 0.5,
      b: 0.15 + Math.random() * 0.4,
      t: Math.random() * Math.PI * 2,
    });
  }
}

// ---------------------------------------------------------------------------
// Coordinate transforms
// ---------------------------------------------------------------------------
function worldToScreen(wx: number, wy: number): [number, number] {
  return [
    (wx - camX) * camZoom + width / 2,
    (wy - camY) * camZoom + height / 2,
  ];
}

function screenToWorld(sx: number, sy: number): [number, number] {
  return [
    (sx - width / 2) / camZoom + camX,
    (sy - height / 2) / camZoom + camY,
  ];
}

// ---------------------------------------------------------------------------
// Flare drawing
// ---------------------------------------------------------------------------
function drawFlare(
  ctx: CanvasRenderingContext2D,
  sx: number, sy: number,
  len: number, color: string, alpha: number
) {
  if (len < 2) return;
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.lineWidth = 1;
  ctx.lineCap = "round";

  // Vertical
  const v = ctx.createLinearGradient(sx, sy - len, sx, sy + len);
  v.addColorStop(0, "transparent"); v.addColorStop(0.35, color);
  v.addColorStop(0.5, color); v.addColorStop(0.65, color);
  v.addColorStop(1, "transparent");
  ctx.strokeStyle = v;
  ctx.beginPath(); ctx.moveTo(sx, sy - len); ctx.lineTo(sx, sy + len); ctx.stroke();

  // Horizontal
  const h = ctx.createLinearGradient(sx - len, sy, sx + len, sy);
  h.addColorStop(0, "transparent"); h.addColorStop(0.35, color);
  h.addColorStop(0.5, color); h.addColorStop(0.65, color);
  h.addColorStop(1, "transparent");
  ctx.strokeStyle = h;
  ctx.beginPath(); ctx.moveTo(sx - len, sy); ctx.lineTo(sx + len, sy); ctx.stroke();

  ctx.restore();
}

// ---------------------------------------------------------------------------
// Main render
// ---------------------------------------------------------------------------
function render() {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  time += 0.016;

  ctx.save();
  ctx.scale(dpr, dpr);

  // Clear to transparent — lets native vibrancy show through
  ctx.clearRect(0, 0, width, height);

  // Draw each star system
  for (const sys of systems) {
    const [cx, cy] = worldToScreen(sys.wx, sys.wy);

    // Cull if too far off screen
    const screenR = sys.systemRadius * camZoom;
    if (cx + screenR < -50 || cx - screenR > width + 50 ||
        cy + screenR < -50 || cy - screenR > height + 50) continue;

    const twinkle = 0.85 + 0.15 * Math.sin(time * sys.twinkleSpeed + sys.twinklePhase);

    // Nebula haze — soft tinted wash behind large systems
    if (screenR > 30) {
      const nebulaR = sys.systemRadius * 0.8 * camZoom;
      const neb = ctx.createRadialGradient(cx, cy, 0, cx, cy, nebulaR);
      neb.addColorStop(0, `hsla(${sys.hsl.h}, 15%, 75%, 0.06)`);
      neb.addColorStop(0.4, `hsla(${sys.hsl.h}, 10%, 80%, 0.03)`);
      neb.addColorStop(1, "transparent");
      ctx.fillStyle = neb;
      ctx.beginPath(); ctx.arc(cx, cy, nebulaR, 0, 2 * Math.PI); ctx.fill();
    }

    // Disk usage % arc — thin ring around the star showing share of total disk
    const arcCoreR = sys.starRadius * camZoom;
    if (arcCoreR > 2 && totalDiskUsed.value > 0) {
      const pct = sys.size / totalDiskUsed.value;
      const arcR = arcCoreR + 5 * Math.min(camZoom, 2);
      const arcWidth = Math.max(1.5, 2.5 * Math.min(camZoom, 2));
      const arcAlpha = Math.min(0.5, 0.2 + camZoom * 0.1);

      // Background ring (full circle, dim)
      ctx.strokeStyle = `hsla(${sys.hsl.h}, 10%, 75%, ${arcAlpha * 0.35})`;
      ctx.lineWidth = arcWidth;
      ctx.beginPath();
      ctx.arc(cx, cy, arcR, 0, 2 * Math.PI);
      ctx.stroke();

      // Filled arc (proportional to disk %)
      ctx.strokeStyle = `hsla(${sys.hsl.h}, 50%, 30%, ${arcAlpha})`;
      ctx.lineWidth = arcWidth;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.arc(cx, cy, arcR, -Math.PI / 2, -Math.PI / 2 + pct * 2 * Math.PI);
      ctx.stroke();
      ctx.lineCap = "butt";

      // Percentage text (only if large enough to read)
      if (arcCoreR > 5) {
        const pctText = `${(pct * 100).toFixed(1)}%`;
        ctx.textAlign = "center";
        ctx.font = `500 ${Math.max(7, Math.min(9, 8 * camZoom))}px -apple-system, BlinkMacSystemFont, sans-serif`;
        ctx.fillStyle = `hsla(${sys.hsl.h}, 30%, 30%, ${arcAlpha * 0.7})`;
        ctx.fillText(pctText, cx, cy - arcCoreR - 8 * Math.min(camZoom, 2));
      }
    }

    // Orbit rings
    if (screenR > 15) {
      for (const planet of sys.planets) {
        const orbitScreenR = planet.orbitRadius * camZoom;
        if (orbitScreenR < 3) continue;
        ctx.strokeStyle = `hsla(${sys.hsl.h}, 10%, 45%, ${Math.min(0.3, 0.1 * camZoom)})`;
        ctx.lineWidth = 0.7;
        ctx.beginPath();
        ctx.arc(cx, cy, orbitScreenR, 0, 2 * Math.PI);
        ctx.stroke();
      }
    }

    // Star flares — dark cross spikes
    const flareLen = sys.flareLength * camZoom * twinkle;
    if (flareLen > 2) {
      const fc = `hsla(${sys.hsl.h}, 15%, 20%, ${0.2 * twinkle})`;
      drawFlare(ctx, cx, cy, flareLen, fc, 0.3 * twinkle);
      // Diagonal secondary flares
      ctx.save();
      ctx.translate(cx, cy);
      ctx.rotate(Math.PI / 4);
      drawFlare(ctx, 0, 0, flareLen * 0.45 * twinkle, fc, 0.12 * twinkle);
      ctx.restore();
    }

    // Star shadow halo — soft dark aura
    const haloR = sys.starRadius * 3 * camZoom;
    if (haloR > 2) {
      const halo = ctx.createRadialGradient(cx, cy, sys.starRadius * 0.3 * camZoom, cx, cy, haloR);
      halo.addColorStop(0, `hsla(${sys.hsl.h}, 20%, 15%, ${0.18 * twinkle})`);
      halo.addColorStop(0.5, `hsla(${sys.hsl.h}, 15%, 30%, ${0.04 * twinkle})`);
      halo.addColorStop(1, "transparent");
      ctx.fillStyle = halo;
      ctx.beginPath(); ctx.arc(cx, cy, haloR, 0, 2 * Math.PI); ctx.fill();
    }

    // Star core — dark solid center
    const coreR = sys.starRadius * camZoom;
    if (coreR > 0.5) {
      const core = ctx.createRadialGradient(cx, cy, 0, cx, cy, coreR);
      core.addColorStop(0, `hsla(${sys.hsl.h}, 25%, 8%, ${0.92 * twinkle})`);
      core.addColorStop(0.4, `hsla(${sys.hsl.h}, 20%, 18%, ${0.85 * twinkle})`);
      core.addColorStop(0.75, `hsla(${sys.hsl.h}, 15%, 30%, ${0.45 * twinkle})`);
      core.addColorStop(1, `hsla(${sys.hsl.h}, 10%, 50%, 0.0)`);
      ctx.fillStyle = core;
      ctx.beginPath(); ctx.arc(cx, cy, coreR, 0, 2 * Math.PI); ctx.fill();
    }

    // Star label — always visible with name + size
    if (coreR > 1.5) {
      const labelY = cy + coreR + 14 * Math.min(camZoom, 1.5);
      const labelFontSize = Math.max(10, Math.min(13, 11 * camZoom));
      const sizeFontSize = Math.max(8, Math.min(11, 9 * camZoom));
      const labelAlpha = Math.min(0.85, 0.5 + camZoom * 0.15);
      const sizeAlpha = Math.min(0.55, 0.3 + camZoom * 0.1);

      ctx.textAlign = "center";

      // Name
      ctx.font = `600 ${labelFontSize}px -apple-system, BlinkMacSystemFont, sans-serif`;
      ctx.fillStyle = `hsla(${sys.hsl.h}, 20%, 15%, ${labelAlpha})`;
      ctx.fillText(sys.name, cx, labelY);

      // Size (below name)
      ctx.font = `400 ${sizeFontSize}px -apple-system, BlinkMacSystemFont, sans-serif`;
      ctx.fillStyle = `hsla(${sys.hsl.h}, 15%, 35%, ${sizeAlpha})`;
      ctx.fillText(formatSize(sys.size), cx, labelY + sizeFontSize + 3);
    }

    // Planets
    for (const planet of sys.planets) {
      const angle = planet.orbitPhase + time * planet.orbitSpeed;
      const px = sys.wx + Math.cos(angle) * planet.orbitRadius;
      const py = sys.wy + Math.sin(angle) * planet.orbitRadius;
      const [psx, psy] = worldToScreen(px, py);
      const pr = planet.radius * camZoom;

      if (pr < 0.3) continue;
      if (psx < -20 || psx > width + 20 || psy < -20 || psy > height + 20) continue;

      // Vector line: star → planet
      const lineAlpha = Math.min(0.2, 0.06 + camZoom * 0.04);
      ctx.strokeStyle = `hsla(${sys.hsl.h}, 10%, 35%, ${lineAlpha})`;
      ctx.lineWidth = 0.5;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(psx, psy);
      ctx.stroke();

      // Planet small flare
      if (pr > 2) {
        const pfc = `hsla(${planet.hsl.h}, ${planet.hsl.s * 0.5}%, ${planet.hsl.l}%, 0.1)`;
        drawFlare(ctx, psx, psy, pr * 2, pfc, 0.15);
      }

      // Planet halo
      if (pr > 1) {
        const ph = ctx.createRadialGradient(psx, psy, pr * 0.2, psx, psy, pr * 2);
        ph.addColorStop(0, `hsla(${planet.hsl.h}, ${planet.hsl.s}%, ${planet.hsl.l}%, 0.1)`);
        ph.addColorStop(1, "transparent");
        ctx.fillStyle = ph;
        ctx.beginPath(); ctx.arc(psx, psy, pr * 2, 0, 2 * Math.PI); ctx.fill();
      }

      // Planet core — dark solid
      const pc = ctx.createRadialGradient(psx, psy, 0, psx, psy, pr);
      pc.addColorStop(0, `hsla(${planet.hsl.h}, ${planet.hsl.s}%, ${Math.max(10, planet.hsl.l - 10)}%, 0.9)`);
      pc.addColorStop(0.5, `hsla(${planet.hsl.h}, ${planet.hsl.s}%, ${planet.hsl.l}%, 0.8)`);
      pc.addColorStop(1, `hsla(${planet.hsl.h}, ${planet.hsl.s * 0.7}%, ${planet.hsl.l + 15}%, 0.0)`);
      ctx.fillStyle = pc;
      ctx.beginPath(); ctx.arc(psx, psy, pr, 0, 2 * Math.PI); ctx.fill();

      // Planet label with name + size (visible when zoomed enough)
      if (pr > 2.5) {
        const plAlpha = Math.min(0.7, 0.25 + camZoom * 0.12);
        const plFontSize = Math.max(8, Math.min(11, 9 * camZoom));

        ctx.textAlign = "center";
        ctx.font = `500 ${plFontSize}px -apple-system, BlinkMacSystemFont, sans-serif`;
        ctx.fillStyle = `hsla(${planet.hsl.h}, 15%, 25%, ${plAlpha})`;
        ctx.fillText(planet.name, psx, psy + pr + 11);

        // Size below name (slightly more faded)
        if (pr > 3.5) {
          ctx.font = `400 ${Math.max(7, plFontSize - 1)}px -apple-system, BlinkMacSystemFont, sans-serif`;
          ctx.fillStyle = `hsla(${planet.hsl.h}, 10%, 40%, ${plAlpha * 0.65})`;
          ctx.fillText(formatSize(planet.size), psx, psy + pr + 11 + plFontSize + 1);
        }
      }

      // Moons
      for (const moon of planet.moons) {
        const ma = moon.orbitPhase + time * moon.orbitSpeed;
        const mx = px + Math.cos(ma) * moon.orbitRadius;
        const my = py + Math.sin(ma) * moon.orbitRadius;
        const [msx, msy] = worldToScreen(mx, my);
        const mr = moon.radius * camZoom;

        if (mr < 0.3) continue;
        if (msx < -10 || msx > width + 10 || msy < -10 || msy > height + 10) continue;

        // Vector line: planet → moon
        const moonLineAlpha = Math.min(0.15, 0.04 + camZoom * 0.03);
        ctx.strokeStyle = `hsla(${planet.hsl.h}, 8%, 40%, ${moonLineAlpha})`;
        ctx.lineWidth = 0.35;
        ctx.beginPath();
        ctx.moveTo(psx, psy);
        ctx.lineTo(msx, msy);
        ctx.stroke();

        // Moon orbit ring
        if (mr > 0.8) {
          const moonOrbitR = moon.orbitRadius * camZoom;
          ctx.strokeStyle = `hsla(${moon.hsl.h}, 8%, 45%, 0.15)`;
          ctx.lineWidth = 0.4;
          ctx.beginPath(); ctx.arc(psx, psy, moonOrbitR, 0, 2 * Math.PI); ctx.stroke();
        }

        // Moon dot
        ctx.fillStyle = `hsla(${moon.hsl.h}, ${moon.hsl.s}%, ${Math.max(15, moon.hsl.l - 10)}%, 0.6)`;
        ctx.beginPath(); ctx.arc(msx, msy, Math.max(mr, 0.4), 0, 2 * Math.PI); ctx.fill();
      }
    }
  }

  // Hover tooltip
  if (hoveredBody) {
    ctx.font = "500 11px -apple-system, BlinkMacSystemFont, sans-serif";
    ctx.textAlign = "left";
    ctx.textBaseline = "middle";
    ctx.fillStyle = "rgba(20, 25, 35, 0.85)";
    ctx.fillText(hoveredBody.name, hoveredBody.x + 12, hoveredBody.y - 6);
    ctx.font = "400 10px -apple-system, BlinkMacSystemFont, sans-serif";
    ctx.fillStyle = "rgba(60, 70, 90, 0.55)";
    ctx.fillText(`${formatSize(hoveredBody.size)}  (${hoveredBody.type})`, hoveredBody.x + 12, hoveredBody.y + 8);

    // ring
    ctx.strokeStyle = "rgba(40, 50, 70, 0.3)";
    ctx.lineWidth = 1;
    ctx.beginPath(); ctx.arc(hoveredBody.x, hoveredBody.y, 8, 0, 2 * Math.PI); ctx.stroke();
  }

  ctx.restore();
  if (!paused) {
    animFrame = requestAnimationFrame(render);
  }
}

// ---------------------------------------------------------------------------
// Hit test — check planets and stars at current zoom
// ---------------------------------------------------------------------------
function hitTest(sx: number, sy: number): { name: string; size: number; x: number; y: number; type: string; systemName: string } | null {
  // Check planets (and moons) first, then stars
  for (const sys of systems) {
    // Planets
    for (const planet of sys.planets) {
      const angle = planet.orbitPhase + time * planet.orbitSpeed;
      const px = sys.wx + Math.cos(angle) * planet.orbitRadius;
      const py = sys.wy + Math.sin(angle) * planet.orbitRadius;
      const [psx, psy] = worldToScreen(px, py);
      const pr = Math.max(planet.radius * camZoom, 5);
      if ((sx - psx) ** 2 + (sy - psy) ** 2 <= (pr + 5) ** 2) {
        return { name: planet.name, size: planet.size, x: psx, y: psy, type: "planet", systemName: sys.name };
      }

      // Moons
      for (const moon of planet.moons) {
        const ma = moon.orbitPhase + time * moon.orbitSpeed;
        const mx = px + Math.cos(ma) * moon.orbitRadius;
        const my = py + Math.sin(ma) * moon.orbitRadius;
        const [msx, msy] = worldToScreen(mx, my);
        const mr = Math.max(moon.radius * camZoom, 4);
        if ((sx - msx) ** 2 + (sy - msy) ** 2 <= (mr + 4) ** 2) {
          return { name: moon.name, size: moon.size, x: msx, y: msy, type: "moon", systemName: sys.name };
        }
      }
    }

    // Star
    const [csx, csy] = worldToScreen(sys.wx, sys.wy);
    const sr = Math.max(sys.starRadius * camZoom, 8);
    if ((sx - csx) ** 2 + (sy - csy) ** 2 <= (sr + 6) ** 2) {
      return { name: sys.name, size: sys.size, x: csx, y: csy, type: "star", systemName: sys.name };
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Mouse / touch handlers — delegate drag/wheel to composable, keep
// component-specific hover and click-to-zoom logic here.
// ---------------------------------------------------------------------------
function onMouseMove(e: MouseEvent) {
  // Let the composable handle drag detection and panning
  zoomPan.onMouseMove(e);

  // If composable is handling a drag, skip hover detection
  if (zoomPan.state.dragging) return;

  const canvas = canvasRef.value;
  if (!canvas) return;
  const rect = canvas.getBoundingClientRect();
  const mx = e.clientX - rect.left;
  const my = e.clientY - rect.top;

  const hit = hitTest(mx, my);
  hoveredBody = hit;
  canvas.style.cursor = hit ? "pointer" : "grab";
}

function onMouseDown(e: MouseEvent) {
  // Always allow drag/click — GalacticViz supports pan in compact mode too.
  // Only wheel zoom is gated by expanded.
  zoomPan.onMouseDown(e, true);
}

function onMouseUp(e: MouseEvent) {
  const wasDrag = zoomPan.state.didDrag;
  zoomPan.onMouseUp();

  if (!wasDrag) {
    const canvas = canvasRef.value;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    // Click — zoom into / center on clicked body
    const hit = hitTest(mx, my);
    if (hit) {
      const sys = systems.find((s) => s.name === hit.systemName);
      if (sys) selectedSystem.value = sys;

      if (hit.type === "star" && sys) {
        // Zoom to show the full star system
        smoothPanTo(sys.wx, sys.wy, Math.min(3, 800 / sys.systemRadius));
      } else if (hit.type === "planet") {
        // Zoom tight onto the planet's current position
        const [wx, wy] = screenToWorld(hit.x, hit.y);
        smoothPanTo(wx, wy, Math.min(10, camZoom * 2.5));
      } else if (hit.type === "moon") {
        // Zoom tight onto the moon
        const [wx, wy] = screenToWorld(hit.x, hit.y);
        smoothPanTo(wx, wy, Math.min(12, camZoom * 3));
      }
    } else {
      // Clicked empty space — dismiss info panel
      selectedSystem.value = null;
    }
  }
}

function onMouseLeave() {
  zoomPan.onMouseLeave();
  hoveredBody = null;
}

function onWheel(e: WheelEvent) {
  zoomPan.onWheel(e, props.expanded);
}

// Smooth pan animation
let panAnim: number | null = null;
function smoothPanTo(wx: number, wy: number, zoom: number) {
  if (panAnim) cancelAnimationFrame(panAnim);
  const startX = camX, startY = camY, startZ = camZoom;
  const dur = 600;
  const t0 = performance.now();

  function step(now: number) {
    const t = Math.min(1, (now - t0) / dur);
    const ease = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2; // ease in-out quad
    camX = startX + (wx - startX) * ease;
    camY = startY + (wy - startY) * ease;
    camZoom = startZ + (zoom - startZ) * ease;
    if (t < 1) panAnim = requestAnimationFrame(step);
    else { panAnim = null; zoomPan.state.scale = camZoom; }
  }
  panAnim = requestAnimationFrame(step);
}

// ---------------------------------------------------------------------------
// Resize / build
// ---------------------------------------------------------------------------
function resize() {
  const el = containerRef.value;
  const cv = canvasRef.value;
  if (!el || !cv) return;
  dpr = window.devicePixelRatio || 1;
  width = el.clientWidth;
  height = el.clientHeight;
  cv.width = width * dpr;
  cv.height = height * dpr;
  cv.style.width = `${width}px`;
  cv.style.height = `${height}px`;
}

function build() {
  if (!props.data || !canvasRef.value || !containerRef.value) return;
  resize();
  systems = buildSystems(props.data.root);
  systemsReady.value = systems.length > 0;
  genBgStars();

  // Store total disk used for percentage calculations
  totalDiskUsed.value = props.data.disk_used;

  // Center camera on the largest system
  if (systems.length > 0) {
    camX = systems[0].wx;
    camY = systems[0].wy;
    // Tighter initial zoom — show just the top 3-4 systems readable on first load
    // Instead of fitting ALL systems, fit just the largest system's neighborhood
    const topSystems = systems.slice(0, Math.min(4, systems.length));
    const extent = Math.max(
      ...topSystems.map((s) => Math.max(
        Math.abs(s.wx - systems[0].wx) + s.systemRadius,
        Math.abs(s.wy - systems[0].wy) + s.systemRadius
      ))
    );
    // Use 1.8 divisor instead of 2.5 for tighter framing, and raise the max from 1.5 to 2.2
    camZoom = Math.min(2.2, Math.min(width, height) / (extent * 1.8));
    zoomPan.state.scale = camZoom;
  }

  cancelAnimationFrame(animFrame);
  animFrame = requestAnimationFrame(render);
}

let resizeObs: ResizeObserver | null = null;

onMounted(() => {
  if (props.data) nextTick(() => build());
  if (containerRef.value) {
    resizeObs = new ResizeObserver(() => { resize(); });
    resizeObs.observe(containerRef.value);
  }
  document.addEventListener("visibilitychange", onVisibilityChange);
});

onUnmounted(() => {
  paused = true;
  cancelAnimationFrame(animFrame);
  if (panAnim) cancelAnimationFrame(panAnim);
  if (resizeObs) resizeObs.disconnect();
  document.removeEventListener("visibilitychange", onVisibilityChange);
});

watch(() => props.data, () => { if (props.data) nextTick(() => build()); });

// When expanded state changes externally (e.g. tab switch while expanded),
// scale zoom proportionally so the scene fills the new canvas size.
watch(() => props.expanded, () => {
  const oldSize = Math.min(width, height) || 1;
  nextTick(() => {
    resize();
    const newSize = Math.min(width, height) || 1;
    camZoom *= newSize / oldSize;
    zoomPan.state.scale = camZoom;
  });
});
</script>

<template>
  <div class="galactic-container" :class="{ expanded }" ref="containerRef">
    <canvas
      ref="canvasRef"
      class="galactic-canvas"
      @mousemove="onMouseMove"
      @mousedown="onMouseDown"
      @mouseup="onMouseUp"
      @mouseleave="onMouseLeave"
      @wheel="onWheel"
    ></canvas>

    <!-- Expand / Compact button — top-right -->
    <button class="galactic-expand-btn" @click="toggleExpand">
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <template v-if="!expanded">
          <path d="M8.5 1.5H12.5V5.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M5.5 12.5H1.5V8.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M12.5 1.5L8.5 5.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <path d="M1.5 12.5L5.5 8.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </template>
        <template v-else>
          <path d="M12.5 5.5H8.5V1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M1.5 8.5H5.5V12.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M8.5 5.5L12.5 1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <path d="M5.5 8.5L1.5 12.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </template>
      </svg>
      {{ expanded ? 'Compact' : 'Expand' }}
    </button>

    <!-- Floating compact FAB — only in expanded mode -->
    <button
      v-if="expanded"
      class="galactic-compact-fab"
      @click="toggleExpand"
      title="Exit expanded view"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M12.5 5.5H8.5V1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M1.5 8.5H5.5V12.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M8.5 5.5L12.5 1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        <path d="M5.5 8.5L1.5 12.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
      Compact
    </button>

    <!-- Size legend — bottom-left corner -->
    <div class="galactic-legend" v-if="systemsReady">
      <div class="legend-title">Star size = disk usage</div>
      <div class="legend-row" v-for="item in legendItems" :key="item.label">
        <span class="legend-dot" :style="{ width: item.px + 'px', height: item.px + 'px' }"></span>
        <span class="legend-label">{{ item.label }}</span>
      </div>
    </div>

    <!-- Info panel — shown when a star system is clicked -->
    <div class="galactic-info-panel" v-if="selectedSystem">
      <div class="info-header">
        <span class="info-name">{{ selectedSystem.name }}</span>
        <button class="info-close" @click="selectedSystem = null">&times;</button>
      </div>
      <div class="info-total">{{ formatSize(selectedSystem.size) }}</div>
      <div class="info-pct" v-if="totalDiskUsed > 0">
        {{ ((selectedSystem.size / totalDiskUsed) * 100).toFixed(1) }}% of used disk
      </div>
      <div class="info-divider"></div>
      <div class="info-planets-label">Contents</div>
      <div class="info-planet-list">
        <div
          class="info-planet-row"
          v-for="planet in selectedSystemPlanets"
          :key="planet.name"
        >
          <span class="info-planet-dot" :style="{ background: `hsl(${planet.hsl.h}, ${planet.hsl.s}%, ${planet.hsl.l}%)` }"></span>
          <span class="info-planet-name">{{ planet.name }}</span>
          <span class="info-planet-size">{{ formatSize(planet.size) }}</span>
          <span class="info-planet-pct">{{ ((planet.size / selectedSystem!.size) * 100).toFixed(1) }}%</span>
        </div>
      </div>
    </div>

    <div class="galactic-hint">
      Scroll to zoom. Drag to pan. Click a star to center on it.
    </div>
  </div>
</template>

<style scoped>
.galactic-container {
  position: relative;
  width: 100%;
  height: 600px;
  border-radius: var(--radius-md);
  overflow: hidden;
  cursor: grab;
}

/* Expanded: fill the entire content panel behind header/switcher (z-index 1).
   SpaceMap.vue gives .view-header and .viz-switcher z-index: 10 so they
   stay interactive above this canvas. */
.galactic-container.expanded {
  position: fixed;
  top: 0;
  left: 230px; /* sidebar width */
  right: 0;
  bottom: 0;
  height: auto;
  max-width: none;
  border-radius: 0;
  z-index: 1;
}

/* Expand / Compact button — top-right corner */
.galactic-expand-btn {
  position: absolute;
  top: 12px;
  right: 12px;
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 12px;
  background: var(--glass);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  z-index: 10;
}

.galactic-expand-btn:hover {
  background: var(--glass-strong);
  color: var(--text);
}

/* Floating compact FAB — fixed position so it's always visible above all UI */
.galactic-compact-fab {
  position: fixed;
  bottom: 24px;
  right: 24px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 18px;
  background: var(--glass-strong);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 12px;
  color: var(--text);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
  transition: box-shadow 0.15s;
  z-index: 1000;
}

.galactic-compact-fab:hover {
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.12);
}

/* Hide the small expand button when expanded (FAB takes over) */
.galactic-container.expanded .galactic-expand-btn {
  display: none;
}

.galactic-canvas {
  display: block;
  width: 100%;
  height: 100%;
}

/* Size legend — bottom-left */
.galactic-legend {
  position: absolute;
  bottom: 40px;
  left: 16px;
  padding: 10px 14px;
  background: var(--glass);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 8px;
  pointer-events: none;
  min-width: 120px;
}

.legend-title {
  font-size: 9px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.5);
  letter-spacing: 0.8px;
  text-transform: uppercase;
  margin-bottom: 8px;
}

.legend-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 5px;
}

.legend-row:last-child {
  margin-bottom: 0;
}

.legend-dot {
  border-radius: 50%;
  background: radial-gradient(circle, rgba(20,20,30,0.85) 0%, rgba(50,55,70,0.3) 80%, transparent 100%);
  flex-shrink: 0;
}

.legend-label {
  font-size: 10px;
  color: rgba(40, 45, 60, 0.6);
  white-space: nowrap;
}

/* Info panel — top-right overlay */
.galactic-info-panel {
  position: absolute;
  top: 16px;
  right: 16px;
  width: 220px;
  max-height: calc(100% - 60px);
  overflow-y: auto;
  padding: 16px;
  background: var(--glass);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  pointer-events: auto;
}

.info-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.info-name {
  font-size: 14px;
  font-weight: 600;
  color: rgba(20, 25, 35, 0.88);
  letter-spacing: 0.2px;
}

.info-close {
  background: none;
  border: none;
  color: rgba(60, 65, 80, 0.45);
  font-size: 18px;
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
}

.info-close:hover {
  color: rgba(20, 25, 35, 0.75);
}

.info-total {
  font-size: 22px;
  font-weight: 300;
  color: rgba(20, 30, 50, 0.75);
  letter-spacing: -0.5px;
  margin-bottom: 2px;
}

.info-pct {
  font-size: 11px;
  color: rgba(60, 70, 90, 0.45);
  margin-bottom: 12px;
}

.info-divider {
  height: 1px;
  background: rgba(0, 0, 0, 0.06);
  margin-bottom: 10px;
}

.info-planets-label {
  font-size: 9px;
  font-weight: 600;
  color: rgba(60, 65, 80, 0.45);
  letter-spacing: 0.8px;
  text-transform: uppercase;
  margin-bottom: 8px;
}

.info-planet-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.info-planet-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
}

.info-planet-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
  opacity: 0.8;
}

.info-planet-name {
  color: rgba(30, 35, 50, 0.7);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.info-planet-size {
  color: rgba(50, 55, 70, 0.55);
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}

.info-planet-pct {
  color: rgba(80, 85, 100, 0.4);
  font-size: 10px;
  min-width: 36px;
  text-align: right;
  flex-shrink: 0;
}

.galactic-hint {
  position: absolute;
  bottom: 14px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 11px;
  color: rgba(80, 85, 100, 0.3);
  pointer-events: none;
  white-space: nowrap;
  letter-spacing: 0.3px;
}
</style>
