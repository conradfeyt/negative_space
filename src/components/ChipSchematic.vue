<script setup lang="ts">
/**
 * ChipSchematic — Apple-style die-shot visualization with live thermal data.
 *
 * Uses the same generator functions as chip-test.html:
 *   genPCore, genECore, genGPU, genNPU, genDisplayEngine,
 *   genMediaEngine, genUnknownBL, genNand
 *
 * Each block type has a precise internal structural layout matching
 * the real Apple Silicon die art from keynote/press images.
 *
 * =========================================================================
 * TODO — Architecture improvements from GPT analysis (see matrix.md)
 * =========================================================================
 *
 * 1. VARIANT MASKS / FUSED-OFF BLOCKS
 *    The die master has a fixed silicon area. Lower-bin SKUs (e.g. M4 Pro
 *    12C/16G vs 14C/20G) fuse off CPU/GPU blocks but the silicon is still
 *    there. We should render ALL blocks up to the master cap and DIM the
 *    disabled ones (reduced opacity / hatched). Currently we only draw
 *    blocks matching the detected count — the die looks smaller than it is.
 *    → Use MASTER_CAPS from matrix.md to know the max block counts.
 *    → Use resolve_layout() match to know which are enabled.
 *    → Enabled blocks: full thermal color. Disabled: ~0.15 opacity gray.
 *
 * 2. RUST-SIDE CHIP DETECTION (resolve_layout matrix)
 *    See /matrix.md for the full Rust match table. Wire it into
 *    thermal.rs so the frontend receives: die_master_id, pkg_type,
 *    enabled_fast, enabled_mid, enabled_slow, enabled_gpu.
 *    The assembly function here should consume that instead of
 *    hardcoding layout assumptions.
 *
 * 3. ULTRA / MULTI-DIE COMPOSITION
 *    Ultra chips = 2x Max die + UltraFusion bridge. M5 Pro/Max use
 *    "Fusion Architecture" (2 dies). The renderer should support:
 *    - Mono: single die (current)
 *    - Ultra2x: left die + mirrored right die + bridge element
 *    - Fusion2x: similar but different bridge style
 *    Don't create separate layouts — compose from the Max die master.
 *
 * 4. PER-MAC-MODEL SENSOR OVERLAYS
 *    Same chip in different Macs exposes different board-level sensors
 *    (fans, battery, SSD, chassis, VRM). Sensor bindings should be
 *    keyed by hw.model (e.g. Mac16,1) not just chip name.
 *    Split into: SoC-level sensors (on-die, chip-specific) vs
 *    board-level sensors (model-specific).
 *
 * 5. CONFIDENCE-BASED SENSOR MAPPING
 *    Not every sensor maps 1:1 to a block. Mapping types:
 *    - direct: sensor is on/adjacent to that block
 *    - zone: thermal region, not exact location
 *    - board: off-SoC component (fan, battery, SSD)
 *    - derived: computed aggregate
 *    This avoids false precision in the thermal visualization.
 *
 * 6. DIE MASTER REDUCTION
 *    12 unique monolithic dies cover all M1-M4 chips:
 *    m1, m1_pro, m1_max, m2, m2_pro, m2_max, m3, m3_pro, m3_max,
 *    m4, m4_pro, m4_max (+ m5, m5_pro_die, m5_max_die)
 *    Ultra = 2x Max composition. Bins within a named chip share the
 *    same die geometry with different fuse masks.
 *    → One layout function per die master, not per marketed chip.
 *    → Assembly proportions should be parameterized, not hardcoded
 *       to M4 Pro dimensions.
 *
 * 7. REMAINING BLOCK TEMPLATES
 *    Some die regions still use placeholder fillBlock rectangles.
 *    Need user-provided HTML table layouts for:
 *    - Memory controllers / SLC
 *    - IO / PCIe / Fabric blocks
 *    - Secure Enclave
 *    - ISP / JPEG block
 * =========================================================================
 */
import { computed } from "vue";
import { TEMP_HOT, TEMP_COOL } from "../utils";
import type { ThermalSensor } from "../types";

const props = defineProps<{
  sensors: ThermalSensor[];
  chipName?: string;
  width?: number;
}>();

// Format chip name: "Apple M4 Pro" → "M4 Pro"
const chipLabel = computed(() => {
  const raw = props.chipName || "";
  return raw.replace(/^Apple\s*/i, "") || "Apple Silicon";
});

const W = computed(() => props.width ?? 340);

// ---------------------------------------------------------------------------
// Temperature → HSL color mapping
// ---------------------------------------------------------------------------
interface HSL { h: number; s: number; l: number; }

// Schematic-specific thresholds for finer die-shot thermal granularity.
// TEMP_HOT (80) and TEMP_COOL (45) come from shared constants; the rest
// are intermediate bands needed only for the chip visualization.
const CHIP_TEMP_EXTREME = 100;
const CHIP_TEMP_VERY_HOT = 90;
const CHIP_TEMP_WARM = 70;
const CHIP_TEMP_MILD = 55;

function tempHSL(t: number): HSL {
  if (t >= CHIP_TEMP_EXTREME) return { h: 5, s: 55, l: 55 };
  if (t >= CHIP_TEMP_VERY_HOT) return { h: 20, s: 50, l: 52 };
  if (t >= TEMP_HOT) return { h: 35, s: 45, l: 50 };
  if (t >= CHIP_TEMP_WARM) return { h: 170, s: 40, l: 50 };
  if (t >= CHIP_TEMP_MILD) return { h: 185, s: 42, l: 50 };
  if (t >= TEMP_COOL) return { h: 195, s: 45, l: 52 };
  return { h: 210, s: 48, l: 55 };
}

function hsl(c: HSL, a: number): string {
  return `hsla(${c.h}, ${c.s}%, ${c.l}%, ${a})`;
}

// Static blue for non-thermal blocks
const m4ProBlue: HSL = { h: 225, s: 55, l: 55 };

// ---------------------------------------------------------------------------
// Core grouping (sensor data → block arrays)
// ---------------------------------------------------------------------------
interface CoreBlock { id: string; temp: number; hsl: HSL; }

function charToOrd(ch: number): number {
  if (ch >= 48 && ch <= 57) return ch - 48;
  if (ch >= 65 && ch <= 90) return ch - 65 + 10;
  if (ch >= 97 && ch <= 122) return ch - 97 + 36;
  return 0;
}

function groupPCores(sensors: ThermalSensor[]): CoreBlock[] {
  const filtered = sensors.filter(s => s.key.startsWith("Tp"));
  if (!filtered.length) return [];
  const groups = new Map<number, number[]>();
  for (const s of filtered) {
    const ord = charToOrd(s.key.charCodeAt(3));
    const bucket = Math.floor(ord / 6);
    if (!groups.has(bucket)) groups.set(bucket, []);
    groups.get(bucket)!.push(s.temp_celsius);
  }
  const sorted = [...groups.entries()].sort((a, b) => a[0] - b[0]);
  let trimmed = sorted;
  if (trimmed.length > 2) {
    const avgSize = trimmed.reduce((sum, [, t]) => sum + t.length, 0) / trimmed.length;
    while (trimmed.length > 2 && trimmed[trimmed.length - 1][1].length < avgSize * 0.35) {
      trimmed = trimmed.slice(0, -1);
    }
  }
  return trimmed.map(([_bucket, temps], i) => {
    const maxT = Math.round(Math.max(...temps) * 10) / 10;
    return { id: `Tp${i}`, temp: maxT, hsl: tempHSL(maxT) };
  });
}

function groupECores(sensors: ThermalSensor[]): CoreBlock[] {
  const filtered = sensors.filter(s => s.key.startsWith("Te"));
  if (!filtered.length) return [];
  return filtered.map((s, i) => {
    const t = Math.round(s.temp_celsius * 10) / 10;
    return { id: `Te${i}`, temp: t, hsl: tempHSL(t) };
  });
}

function groupGPU(sensors: ThermalSensor[]): CoreBlock[] {
  const filtered = sensors.filter(s => s.key.startsWith("Tg"));
  if (!filtered.length) return [];
  const groups = new Map<string, number[]>();
  for (const s of filtered) {
    const pfx3 = s.key.substring(0, 3);
    const ord = charToOrd(s.key.charCodeAt(3));
    const bucket = Math.floor(ord / 3);
    const key = `${pfx3}_${String(bucket).padStart(3, "0")}`;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(s.temp_celsius);
  }
  const sorted = [...groups.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  return sorted.map(([_key, temps], i) => {
    const maxT = Math.round(Math.max(...temps) * 10) / 10;
    return { id: `Tg${i}`, temp: maxT, hsl: tempHSL(maxT) };
  });
}

const pCores = computed(() => groupPCores(props.sensors));
const eCores = computed(() => groupECores(props.sensors));
const gpuClusters = computed(() => groupGPU(props.sensors));

// ---------------------------------------------------------------------------
// SVG helpers (same as chip-test.html)
// ---------------------------------------------------------------------------
function seededRand(seed: number): () => number {
  let s = seed;
  return () => { s = (s * 1103515245 + 12345) & 0x7fffffff; return s / 0x7fffffff; };
}

function svgLine(x1: number, y1: number, x2: number, y2: number, stroke: string, sw: number): string {
  return `<line x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" stroke="${stroke}" stroke-width="${sw}"/>`;
}
function svgRect(x: number, y: number, w: number, h: number, fill?: string, stroke?: string, sw?: number): string {
  return `<rect x="${x.toFixed(1)}" y="${y.toFixed(1)}" width="${w.toFixed(1)}" height="${h.toFixed(1)}" fill="${fill || 'none'}" stroke="${stroke || 'none'}" stroke-width="${sw || 0}"/>`;
}

// Fill a sub-block: STRONG outline + subtle internal detail
function fillBlock(x: number, y: number, w: number, h: number, color: HSL, rand: () => number): string {
  let svg = '';
  svg += svgRect(x, y, w, h,
    hsl(color, 0.06 + rand() * 0.04),
    hsl(color, 0.70 + rand() * 0.15),
    0.8 + rand() * 0.3);
  const hCount = Math.max(1, Math.floor(h / (4 + rand() * 4)));
  for (let i = 1; i <= hCount; i++) {
    const ly = y + (h / (hCount + 1)) * i + (rand() - 0.5) * 0.5;
    svg += svgLine(x + 0.5, ly, x + w - 0.5, ly, hsl(color, 0.10 + rand() * 0.10), 0.2 + rand() * 0.2);
  }
  const vCount = Math.max(1, Math.floor(w / (4 + rand() * 5)));
  for (let i = 1; i <= vCount; i++) {
    const lx = x + (w / (vCount + 1)) * i + (rand() - 0.5) * 0.5;
    svg += svgLine(lx, y + 0.5, lx, y + h - 0.5, hsl(color, 0.08 + rand() * 0.08), 0.2 + rand() * 0.15);
  }
  if (w > 6 && h > 6 && rand() > 0.5) {
    const rw = w * (0.3 + rand() * 0.35);
    const rh = h * (0.25 + rand() * 0.35);
    const rx = x + (w - rw) * (0.1 + rand() * 0.8);
    const ry = y + (h - rh) * (0.1 + rand() * 0.8);
    svg += svgRect(rx, ry, rw, rh, hsl(color, 0.03 + rand() * 0.03), hsl(color, 0.12 + rand() * 0.10), 0.25);
  }
  return svg;
}

// ---------------------------------------------------------------------------
// Block generator functions — identical to chip-test.html
// (imported as string-returning functions for SVG innerHTML)
// ---------------------------------------------------------------------------

// genPCore, genECore, genGPU, genNPU, genDisplayEngine, genMediaEngine, genUnknownBL, genNand
// These are copied verbatim from chip-test.html — they produce SVG markup strings.

function genPCore(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const topBandH = iw * 0.12; const botBandH = iw * 0.12;
  const topY1 = iy; const topY2 = iy + topBandH; const midY = iy + topBandH * 2;
  const botY2end = iy + ih; const botY1 = botY2end - botBandH; const botY2 = botY2end - botBandH * 2;
  const midH = botY2 - midY;
  svg += fillBlock(ix, topY1, iw, topBandH, color, rand);
  svg += fillBlock(ix, topY2, iw, topBandH, color, rand);
  svg += svgLine(x, midY, x + w, midY, hsl(color, 0.80), 1.6);
  const sideW = iw * 0.11; const centerX = ix + sideW * 2; const centerW = iw - sideW * 4;
  const rightX1 = centerX + centerW; const rightX2 = rightX1 + sideW;
  svg += fillBlock(ix, midY, sideW, midH, color, rand);
  svg += fillBlock(ix + sideW, midY, sideW, midH, color, rand);
  svg += fillBlock(rightX1, midY, sideW, midH, color, rand);
  svg += fillBlock(rightX2, midY, sideW, midH, color, rand);
  const thinH = sideW; const fourCellH = midH - thinH * 4; let cry = midY;
  svg += fillBlock(centerX, cry, centerW, thinH, color, rand); cry += thinH;
  svg += fillBlock(centerX, cry, centerW, thinH, color, rand); cry += thinH;
  const cellW = centerW / 4;
  for (let c = 0; c < 4; c++) svg += fillBlock(centerX + c * cellW, cry, cellW, fourCellH, color, rand);
  cry += fourCellH;
  svg += fillBlock(centerX, cry, centerW, thinH, color, rand); cry += thinH;
  svg += fillBlock(centerX, cry, centerW, thinH, color, rand);
  svg += svgLine(x, botY2, x + w, botY2, hsl(color, 0.75), 1.4);
  svg += fillBlock(ix, botY2, iw, botBandH, color, rand);
  svg += fillBlock(ix, botY1, iw, botBandH, color, rand);
  return svg;
}

function genECore(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const outerColW = iw * 0.14; const sideColW = iw * 0.10; const centerW = iw - outerColW * 2 - sideColW * 2;
  const col1X = ix; const col2X = ix + outerColW; const col3X = col2X + sideColW; const col5X = col3X + centerW; const col6X = col5X + sideColW;
  const bandRowH = ih * 0.14; const cellRowH = (ih - bandRowH * 2) / 5;
  const rowY: number[] = []; rowY[0] = iy; rowY[1] = iy + bandRowH;
  for (let i = 2; i <= 6; i++) rowY[i] = rowY[1] + cellRowH * (i - 1);
  const rowEnd = iy + ih; const halfBandW = (iw - outerColW * 2) / 2;
  svg += fillBlock(col1X, rowY[0], outerColW, bandRowH + cellRowH, color, rand);
  svg += fillBlock(col6X, rowY[0], outerColW, bandRowH + cellRowH, color, rand);
  svg += fillBlock(col2X, rowY[0], halfBandW, bandRowH, color, rand);
  svg += fillBlock(col2X + halfBandW, rowY[0], halfBandW, bandRowH, color, rand);
  svg += fillBlock(col2X, rowY[1], sideColW, cellRowH, color, rand);
  svg += fillBlock(col5X, rowY[1], sideColW, cellRowH, color, rand);
  svg += fillBlock(col3X, rowY[1], centerW, cellRowH * 5, color, rand);
  for (let r = 2; r <= 4; r++) {
    svg += fillBlock(col1X, rowY[r], outerColW, cellRowH, color, rand);
    svg += fillBlock(col2X, rowY[r], sideColW, cellRowH, color, rand);
    svg += fillBlock(col5X, rowY[r], sideColW, cellRowH, color, rand);
    svg += fillBlock(col6X, rowY[r], outerColW, cellRowH, color, rand);
  }
  svg += fillBlock(col1X, rowY[5], outerColW, cellRowH + bandRowH, color, rand);
  svg += fillBlock(col6X, rowY[5], outerColW, cellRowH + bandRowH, color, rand);
  svg += fillBlock(col2X, rowY[5], sideColW, cellRowH, color, rand);
  svg += fillBlock(col5X, rowY[5], sideColW, cellRowH, color, rand);
  svg += fillBlock(col2X, rowY[6], halfBandW, rowEnd - rowY[6], color, rand);
  svg += fillBlock(col2X + halfBandW, rowY[6], halfBandW, rowEnd - rowY[6], color, rand);
  return svg;
}

function genGPU(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const colUnit = iw / 12;
  function cX(c: number) { return ix + c * colUnit; }
  const capUnit = 1.0, midUnit = 1.0, divUnit = 0.6, bodyUnit = 1.2;
  const totalUnits = 3 * capUnit * 2 + 3 * midUnit * 2 + 2 * divUnit + 11 * bodyUnit;
  const unitH = ih / totalUnits;
  const capRowH = capUnit * unitH; const midRowH = midUnit * unitH; const divRowH = divUnit * unitH; const bodyRowH = bodyUnit * unitH;
  let cy = iy;
  const capAY: number[] = []; for (let i = 0; i < 3; i++) { capAY.push(cy); cy += capRowH; }
  const midBY: number[] = []; for (let i = 0; i < 3; i++) { midBY.push(cy); cy += midRowH; }
  const divCY = cy; cy += divRowH;
  const bodyDY: number[] = []; for (let i = 0; i < 11; i++) { bodyDY.push(cy); cy += bodyRowH; }
  const divEY = cy; cy += divRowH;
  const midFY: number[] = []; for (let i = 0; i < 3; i++) { midFY.push(cy); cy += midRowH; }
  const capGY: number[] = []; for (let i = 0; i < 3; i++) { capGY.push(cy); cy += capRowH; }
  function drawSideCells3(rowYs: number[], rowH: number) {
    for (let r = 0; r < 3; r++) {
      svg += fillBlock(cX(0), rowYs[r], colUnit * 2, rowH, color, rand);
      svg += fillBlock(cX(10), rowYs[r], colUnit * 2, rowH, color, rand);
    }
  }
  drawSideCells3(capAY, capRowH); svg += fillBlock(cX(2), capAY[0], colUnit * 8, capRowH * 3, color, rand);
  drawSideCells3(midBY, midRowH);
  for (let c = 0; c < 4; c++) svg += fillBlock(cX(2 + c * 2), midBY[0], colUnit * 2, midRowH * 3, color, rand);
  const divCells = [0,3,4,6,8,9]; const divWidths = [3,1,2,2,1,3];
  for (let i = 0; i < 6; i++) svg += fillBlock(cX(divCells[i]), divCY, colUnit * divWidths[i], divRowH, color, rand);
  for (let r = 0; r < 11; r++) { svg += fillBlock(cX(0), bodyDY[r], colUnit * 2, bodyRowH, color, rand); svg += fillBlock(cX(10), bodyDY[r], colUnit * 2, bodyRowH, color, rand); }
  for (let c = 0; c < 4; c++) svg += fillBlock(cX(2 + c * 2), bodyDY[0], colUnit * 2, bodyRowH * 11, color, rand);
  for (let i = 0; i < 6; i++) svg += fillBlock(cX(divCells[i]), divEY, colUnit * divWidths[i], divRowH, color, rand);
  drawSideCells3(midFY, midRowH);
  for (let c = 0; c < 4; c++) svg += fillBlock(cX(2 + c * 2), midFY[0], colUnit * 2, midRowH * 3, color, rand);
  drawSideCells3(capGY, capRowH); svg += fillBlock(cX(2), capGY[0], colUnit * 8, capRowH * 3, color, rand);
  return svg;
}

function genNPU(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const colW = iw / 6; function cXn(c: number) { return ix + c * colW; }
  const halfH = ih / 2; const topY = iy; const botY = iy + halfH; const rowH = halfH / 3;
  svg += fillBlock(cXn(0), topY, colW, halfH, color, rand); svg += fillBlock(cXn(1), topY, colW, halfH, color, rand);
  svg += fillBlock(cXn(4), topY, colW, halfH, color, rand); svg += fillBlock(cXn(5), topY, colW, halfH, color, rand);
  svg += fillBlock(cXn(2), topY, colW * 2, rowH * 2, color, rand);
  svg += fillBlock(cXn(2), topY + rowH * 2, colW * 2, rowH, color, rand);
  svg += fillBlock(cXn(0), botY, colW, halfH, color, rand); svg += fillBlock(cXn(1), botY, colW, halfH, color, rand);
  svg += fillBlock(cXn(4), botY, colW, halfH, color, rand); svg += fillBlock(cXn(5), botY, colW, halfH, color, rand);
  svg += fillBlock(cXn(2), botY, colW * 2, rowH, color, rand);
  svg += fillBlock(cXn(2), botY + rowH, colW * 2, rowH * 2, color, rand);
  return svg;
}

function genDisplayEngine(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const outerW = iw * 0.15; const sideW2 = iw * 0.15; const centerW = iw - outerW * 2 - sideW2 * 2;
  const c1X = ix; const c2X = ix + outerW; const c3X = c2X + sideW2; const c4X = c3X + centerW; const c5X = c4X + sideW2;
  const rowH = ih / 4;
  svg += fillBlock(c3X, iy, centerW, ih, color, rand);
  svg += fillBlock(c2X, iy, sideW2, rowH * 2, color, rand); svg += fillBlock(c4X, iy, sideW2, rowH * 2, color, rand);
  svg += fillBlock(c2X, iy + rowH * 2, sideW2, rowH * 2, color, rand); svg += fillBlock(c4X, iy + rowH * 2, sideW2, rowH * 2, color, rand);
  for (let r = 0; r < 4; r++) { svg += fillBlock(c1X, iy + r * rowH, outerW, rowH, color, rand); svg += fillBlock(c5X, iy + r * rowH, outerW, rowH, color, rand); }
  return svg;
}

function genMediaEngine(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const colUnit = iw / 40; const rowUnit = ih / 32;
  function cX(c: number) { return ix + c * colUnit; } function cW(n: number) { return n * colUnit; }
  function rY(r: number) { return iy + r * rowUnit; } function rH(n: number) { return n * rowUnit; }
  svg += fillBlock(cX(0), rY(0), cW(10), rH(8), color, rand);
  svg += fillBlock(cX(10), rY(0), cW(20), rH(8), color, rand);
  svg += fillBlock(cX(30), rY(0), cW(10), rH(8), color, rand);
  svg += fillBlock(cX(0), rY(8), cW(20), rH(2), color, rand);
  svg += fillBlock(cX(20), rY(8), cW(20), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(10), cW(8), rH(2), color, rand);
  svg += fillBlock(cX(8), rY(10), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(8), rY(11), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(12), rY(10), cW(16), rH(2), color, rand);
  svg += fillBlock(cX(28), rY(10), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(28), rY(11), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(32), rY(10), cW(8), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(12), cW(40), rH(4), color, rand);
  svg += fillBlock(cX(0), rY(16), cW(40), rH(4), color, rand);
  for (let c = 0; c < 10; c++) svg += fillBlock(cX(10 + c * 2), rY(15), cW(2), rH(2), color, rand);
  const midLineY = rY(15) + rH(1);
  svg += svgLine(x, midLineY, cX(10), midLineY, hsl(color, 0.80), 1.4);
  svg += svgLine(cX(30), midLineY, x + w, midLineY, hsl(color, 0.80), 1.4);
  svg += fillBlock(cX(0), rY(20), cW(8), rH(2), color, rand);
  svg += fillBlock(cX(8), rY(20), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(8), rY(21), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(12), rY(20), cW(16), rH(2), color, rand);
  svg += fillBlock(cX(28), rY(20), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(28), rY(21), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(32), rY(20), cW(8), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(22), cW(20), rH(2), color, rand);
  svg += fillBlock(cX(20), rY(22), cW(20), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(24), cW(10), rH(32 - 24), color, rand);
  svg += fillBlock(cX(10), rY(24), cW(20), rH(32 - 24), color, rand);
  svg += fillBlock(cX(30), rY(24), cW(10), rH(32 - 24), color, rand);
  return svg;
}

function genUnknownBL(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const colUnit = iw / 26; const rowUnit = ih / 24;
  function cX(c: number) { return ix + c * colUnit; } function cW(n: number) { return n * colUnit; }
  function rY(r: number) { return iy + r * rowUnit; } function rH(n: number) { return n * rowUnit; }
  svg += fillBlock(cX(0), rY(0), cW(13), rH(2), color, rand);
  svg += fillBlock(cX(13), rY(0), cW(13), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(2), cW(26), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(4), cW(3), rH(16), color, rand);
  svg += fillBlock(cX(23), rY(4), cW(3), rH(16), color, rand);
  svg += fillBlock(cX(3), rY(4), cW(6), rH(2), color, rand);
  svg += fillBlock(cX(9), rY(4), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(9), rY(5), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(17), rY(4), cW(6), rH(2), color, rand);
  svg += fillBlock(cX(3), rY(6), cW(2), rH(2), color, rand); svg += fillBlock(cX(5), rY(6), cW(4), rH(2), color, rand);
  svg += fillBlock(cX(9), rY(6), cW(4), rH(1), color, rand); svg += fillBlock(cX(13), rY(6), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(9), rY(7), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(17), rY(6), cW(4), rH(2), color, rand); svg += fillBlock(cX(21), rY(6), cW(2), rH(2), color, rand);
  svg += fillBlock(cX(3), rY(8), cW(2), rH(8), color, rand); svg += fillBlock(cX(21), rY(8), cW(2), rH(8), color, rand);
  svg += fillBlock(cX(5), rY(8), cW(4), rH(2), color, rand); svg += fillBlock(cX(9), rY(8), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(13), rY(8), cW(4), rH(1), color, rand); svg += fillBlock(cX(9), rY(9), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(17), rY(8), cW(4), rH(2), color, rand);
  svg += fillBlock(cX(5), rY(10), cW(4), rH(2), color, rand); svg += fillBlock(cX(9), rY(10), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(13), rY(10), cW(4), rH(1), color, rand); svg += fillBlock(cX(9), rY(11), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(17), rY(10), cW(4), rH(2), color, rand);
  svg += fillBlock(cX(5), rY(12), cW(4), rH(2), color, rand); svg += fillBlock(cX(9), rY(12), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(9), rY(13), cW(4), rH(1), color, rand); svg += fillBlock(cX(13), rY(13), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(17), rY(12), cW(4), rH(2), color, rand);
  svg += fillBlock(cX(5), rY(14), cW(4), rH(2), color, rand); svg += fillBlock(cX(9), rY(14), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(9), rY(15), cW(4), rH(1), color, rand); svg += fillBlock(cX(13), rY(15), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(17), rY(14), cW(4), rH(2), color, rand);
  svg += fillBlock(cX(3), rY(16), cW(2), rH(2), color, rand); svg += fillBlock(cX(5), rY(16), cW(4), rH(2), color, rand);
  svg += fillBlock(cX(9), rY(16), cW(8), rH(1), color, rand); svg += fillBlock(cX(9), rY(17), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(13), rY(17), cW(4), rH(1), color, rand);
  svg += fillBlock(cX(17), rY(16), cW(4), rH(2), color, rand); svg += fillBlock(cX(21), rY(16), cW(2), rH(2), color, rand);
  svg += fillBlock(cX(3), rY(18), cW(6), rH(2), color, rand); svg += fillBlock(cX(9), rY(18), cW(8), rH(1), color, rand);
  svg += fillBlock(cX(9), rY(19), cW(8), rH(1), color, rand); svg += fillBlock(cX(17), rY(18), cW(6), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(20), cW(26), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(22), cW(13), rH(2), color, rand);
  svg += fillBlock(cX(13), rY(22), cW(13), rH(2), color, rand);
  return svg;
}

function genNand(x: number, y: number, w: number, h: number, color: HSL, seed: number): string {
  const rand = seededRand(seed); let svg = '';
  svg += svgRect(x, y, w, h, hsl(color, 0.08), hsl(color, 0.80), 1.8);
  const bw = 0.9; const ix = x + bw; const iy = y + bw; const iw = w - bw * 2; const ih = h - bw * 2;
  const colUnit = iw / 19; const rowUnit = ih / 4;
  function cX(c: number) { return ix + c * colUnit; } function cW(n: number) { return n * colUnit; }
  function rY(r: number) { return iy + r * rowUnit; } function rH(n: number) { return n * rowUnit; }
  svg += fillBlock(cX(0), rY(0), cW(6), rH(1), color, rand);
  svg += fillBlock(cX(6), rY(0), cW(1), rH(1), color, rand);
  for (let c = 0; c < 5; c++) svg += fillBlock(cX(7 + c), rY(0), cW(1), rH(1), color, rand);
  svg += fillBlock(cX(12), rY(0), cW(1), rH(1), color, rand);
  svg += fillBlock(cX(13), rY(0), cW(6), rH(1), color, rand);
  svg += fillBlock(cX(0), rY(1), cW(7), rH(2), color, rand);
  svg += fillBlock(cX(7), rY(1), cW(5), rH(1), color, rand);
  svg += fillBlock(cX(7), rY(2), cW(5), rH(1), color, rand);
  svg += fillBlock(cX(12), rY(1), cW(7), rH(2), color, rand);
  svg += fillBlock(cX(0), rY(3), cW(6), rH(1), color, rand);
  svg += fillBlock(cX(6), rY(3), cW(1), rH(1), color, rand);
  for (let c = 0; c < 5; c++) svg += fillBlock(cX(7 + c), rY(3), cW(1), rH(1), color, rand);
  svg += fillBlock(cX(12), rY(3), cW(1), rH(1), color, rand);
  svg += fillBlock(cX(13), rY(3), cW(6), rH(1), color, rand);
  return svg;
}

// ---------------------------------------------------------------------------
// Full die assembly — generates complete SVG innerHTML
// ---------------------------------------------------------------------------
const dieSvg = computed(() => {
  const dieW = W.value;
  const dieH = Math.round(dieW * 1.15); // near-square, slightly portrait
  const pad = 3;
  const gap = 1.5;

  let svg = '';
  svg += svgRect(0, 0, dieW, dieH, hsl(m4ProBlue, 0.03), hsl(m4ProBlue, 0.60), 2.0);

  // Header
  const headerH = dieH * 0.065;
  svg += `<text x="${pad + 4}" y="${pad + headerH * 0.72}" fill="${hsl(m4ProBlue, 0.85)}" font-size="${headerH * 0.6}" font-weight="700" font-family="-apple-system, SF Pro Display, Helvetica Neue, sans-serif" letter-spacing="0.5">\uF8FF ${chipLabel.value}</text>`;

  // Layout grid
  const cpuColW = dieW * 0.22;
  const gpuColX = cpuColW;
  const gpuColW = dieW - cpuColW;
  const contentY = headerH;

  const nandH = 16;
  const nandY = dieH - nandH;
  const gpuRegionH = dieH * 0.55;
  const npuRegionY = gpuRegionH;
  const npuRegionH = dieH * 0.08;
  const mediaRegionY = npuRegionY + npuRegionH;
  const mediaRegionH = nandY - mediaRegionY;

  const cpuTotalH = nandY - contentY;
  const pCoreRegionY = contentY;
  const pCoreRegionH = cpuTotalH * 0.62;
  const eCoreRegionY = pCoreRegionY + pCoreRegionH;
  const eCoreRegionH = cpuTotalH * 0.10;
  const miscRegionY = eCoreRegionY + eCoreRegionH;
  const miscRegionH = cpuTotalH - pCoreRegionH - eCoreRegionH;

  // P-cores
  const pCount = pCores.value.length || 10;
  const pCols = 2, pRows = Math.ceil(pCount / 2);
  const pAreaW = cpuColW - pad * 2;
  const pAreaH = pCoreRegionH;
  const pW = (pAreaW - gap) / pCols;
  const pH = (pAreaH - gap * (pRows - 1)) / pRows;
  for (let c = 0; c < pCols; c++) {
    for (let r = 0; r < pRows; r++) {
      const idx = c * pRows + r;
      if (idx >= pCount) continue;
      const core = pCores.value[idx];
      const color = core ? core.hsl : m4ProBlue;
      svg += genPCore(pad + c * (pW + gap), pCoreRegionY + r * (pH + gap), pW, pH, color, idx * 37 + 7);
    }
  }

  // E-cores
  const eCount = eCores.value.length || 4;
  const eAreaW = cpuColW - pad * 2;
  const eH = (eCoreRegionH - gap * (eCount - 1)) / eCount;
  for (let i = 0; i < eCount; i++) {
    const core = eCores.value[i];
    const color = core ? core.hsl : m4ProBlue;
    svg += genECore(pad, eCoreRegionY + i * (eH + gap), eAreaW, eH, color, i * 53 + 200);
  }

  // GPU
  const gCount = gpuClusters.value.length || 20;
  const gpuCols2 = 5, gpuRows2 = Math.ceil(gCount / gpuCols2);
  const gAreaW = gpuColW - pad;
  const gAreaH = gpuRegionH - pad;
  const gW = (gAreaW - gap * (gpuCols2 - 1)) / gpuCols2;
  const gH = (gAreaH - gap * (gpuRows2 - 1)) / gpuRows2;
  for (let c = 0; c < gpuCols2; c++) {
    for (let r = 0; r < gpuRows2; r++) {
      const idx = c * gpuRows2 + r;
      if (idx >= gCount) continue;
      const cluster = gpuClusters.value[idx];
      const color = cluster ? cluster.hsl : m4ProBlue;
      svg += genGPU(gpuColX + c * (gW + gap), pad + r * (gH + gap), gW, gH, color, idx * 41 + 500);
    }
  }

  // NPU — 16 units, 8x2
  const npuCols2 = 8, npuRows2 = 2;
  const nAreaW = gpuColW - pad;
  const nAreaH = npuRegionH - gap * 2;
  const nW = (nAreaW - gap * (npuCols2 - 1)) / npuCols2;
  const nH = (nAreaH - gap) / npuRows2;
  for (let c = 0; c < npuCols2; c++) {
    for (let r = 0; r < npuRows2; r++) {
      svg += genNPU(gpuColX + c * (nW + gap), npuRegionY + gap + r * (nH + gap), nW, nH, m4ProBlue, (c * 2 + r) * 59 + 700);
    }
  }

  // Media Engine + Display Engines
  const meAreaW = gpuColW - pad;
  const meW = meAreaW * 0.55;
  const meX = gpuColX;
  const meY = mediaRegionY + gap * 0.5;
  const meH = mediaRegionH - gap;
  svg += genMediaEngine(meX, meY, meW, meH, m4ProBlue, 1100);

  const deX = meX + meW + gap;
  const deW = meAreaW - meW - gap;
  const deH = meH / 4;
  for (let i = 0; i < 4; i++) {
    svg += genDisplayEngine(deX, meY + i * deH, deW, deH, m4ProBlue, i * 73 + 900);
  }

  // Unknown BL
  svg += genUnknownBL(pad, miscRegionY, cpuColW - pad * 2, miscRegionH, m4ProBlue, 1300);

  // NAND
  const nandCount = 4;
  const nandTotalW = dieW - pad * 2;
  const nandUnitW = (nandTotalW - gap * (nandCount - 1)) / nandCount;
  for (let i = 0; i < nandCount; i++) {
    svg += genNand(pad + i * (nandUnitW + gap), nandY + 1, nandUnitW, nandH - 2, m4ProBlue, i * 111 + 1500);
  }

  return { svg, dieH };
});
</script>

<template>
  <svg
    :width="W"
    :height="dieSvg.dieH"
    :viewBox="`0 0 ${W} ${dieSvg.dieH}`"
    class="chip-schematic"
    v-html="dieSvg.svg"
  />
</template>

<style scoped>
.chip-schematic {
  display: block;
}
</style>
