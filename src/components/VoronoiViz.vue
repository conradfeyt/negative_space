<script setup lang="ts">
/**
 * VoronoiViz — Organic circular Voronoi treemap visualization.
 *
 * Inspired by leaf vein structures and biological cell patterns:
 *   - Circular clipping boundary (like the world population treemap reference)
 *   - Thick organic "vein" borders between parent cells, thinner veins for children
 *   - Cells are inset from their polygon edges to create visible gap channels
 *   - Color families per top-level directory: parent hue with lighter children
 *   - Smooth animated transitions when drilling in/out
 *   - Labels scale with cell area
 *
 * Uses d3-voronoi-treemap for weighted cell computation and SVG for crisp rendering.
 */
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from "vue";
import * as d3 from "d3";
// @ts-ignore — no type declarations for this package
import { voronoiTreemap } from "d3-voronoi-treemap";
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

const containerRef = ref<HTMLDivElement | null>(null);
const svgRef = ref<SVGSVGElement | null>(null);

// ---------------------------------------------------------------------------
// Mode toggle: "consolidated" (single circle) vs "clusters" (multi-circle)
// ---------------------------------------------------------------------------
type VoronoiMode = "consolidated" | "clusters";
const voronoiMode = ref<VoronoiMode>("consolidated");

// ---------------------------------------------------------------------------
// Zoom/pan — direct DOM manipulation on a <g> wrapper.
// NO Vue reactivity in the zoom loop to avoid re-diffing hundreds of paths.
// Uses shared composable for drag state machine and wheel gating;
// SVG-specific coordinate math lives here.
// ---------------------------------------------------------------------------
const zoomGroupRef = ref<SVGGElement | null>(null);
let zoomX = 0;
let zoomY = 0;
let zoomStartX = 0;
let zoomStartY = 0;
let zoomDebounce = 0;

function applyZoomTransform() {
  if (!zoomGroupRef.value) return;
  zoomGroupRef.value.setAttribute(
    "transform",
    `translate(${zoomX},${zoomY}) scale(${zoomPan.state.scale})`
  );
}

// During active zoom, hide labels and thin veins for performance.
// Re-show them after a short debounce when zooming stops.
function setZoomingClass(active: boolean) {
  if (!svgRef.value) return;
  if (active) {
    svgRef.value.classList.add("zooming");
  } else {
    svgRef.value.classList.remove("zooming");
  }
}

const zoomPan = useZoomPan(
  { minScale: 0.3, maxScale: 8, dragThreshold: 4 },
  {
    onZoom(e, newScale, oldScale) {
      const rect = containerRef.value?.getBoundingClientRect();
      if (!rect) return;
      // Cursor position in SVG element space (0..width, 0..height)
      const mx = (e.clientX - rect.left) / rect.width * width;
      const my = (e.clientY - rect.top) / rect.height * height;

      // Zoom toward cursor: adjust translation so the point under cursor stays fixed
      zoomX = mx - (mx - zoomX) * (newScale / oldScale);
      zoomY = my - (my - zoomY) * (newScale / oldScale);

      setZoomingClass(true);
      applyZoomTransform();

      // Debounce: re-show labels after zoom stops
      clearTimeout(zoomDebounce);
      zoomDebounce = window.setTimeout(() => setZoomingClass(false), 150);
    },
    onPan(_e, pixelDx, pixelDy) {
      const rect = containerRef.value?.getBoundingClientRect();
      if (!rect) return;
      // Convert pixel delta to SVG coordinate delta.
      // The <g> transform is `translate(tx,ty) scale(s)` — translate operates in
      // the untransformed SVG coordinate system, so no zoomScale adjustment needed.
      // The ratio (width / rect.width) converts screen pixels → SVG units.
      const dx = pixelDx / rect.width * width;
      const dy = pixelDy / rect.height * height;
      zoomX = zoomStartX + dx;
      zoomY = zoomStartY + dy;
      applyZoomTransform();
    },
    onDragStart() {
      // Snapshot current translate for drag delta calculation
      zoomStartX = zoomX;
      zoomStartY = zoomY;
      // Hide labels during pan for smoother performance
      setZoomingClass(true);
      containerRef.value?.classList.add("panning");
    },
    onDragEnd() {
      setZoomingClass(false);
      containerRef.value?.classList.remove("panning");
    },
  },
);

function onVoronoiWheel(e: WheelEvent) {
  zoomPan.onWheel(e, props.expanded);
}

function onVoronoiMouseDown(e: MouseEvent) {
  zoomPan.onMouseDown(e, props.expanded);
}

function onVoronoiMouseMove(e: MouseEvent) {
  zoomPan.onMouseMove(e);
}

function onVoronoiMouseUp() {
  zoomPan.onMouseUp();
}

function resetCamera() {
  zoomPan.resetZoom();
  zoomX = 0;
  zoomY = 0;
  applyZoomTransform();
  setZoomingClass(false);
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------
let width = 0;
let height = 0;
let cx = 0;  // center x of the circle
let cy = 0;  // center y
let radius = 0;

// Reactive geometry for the consolidated voronoi boundary
const substrateGeom = ref({ cx: 0, cy: 0, r: 0 });
// SVG path string for the organic boundary shape (consolidated mode)
const organicBoundaryPath = ref('');

// Navigation stack — drill into children, back out to parents
const breadcrumbs = ref<{ name: string; node: DiskNode }[]>([]);
const currentNode = ref<DiskNode | null>(null);
const hoveredCellId = ref<string | null>(null);
const hoveredCell = ref<{ name: string; size: number; pct: number; x: number; y: number } | null>(null);

// Animation state — we keep "previous" cells and lerp to "target" cells
const animating = ref(false);
let animFrameId = 0;

// ---------------------------------------------------------------------------
// Color system — Fog + Aqua palette, hue families per parent
// ---------------------------------------------------------------------------
// 10 well-separated hues that feel calm and premium.
// Deliberately avoiding pure primaries — these are muted, sophisticated.
// White membrane cells — varying opacity by position so each cell has a
// different translucency, like a real dragonfly wing. App background shows through.
function wingCellFill(centX: number, centY: number, idx: number, circleCx: number, circleCy: number): string {
  const angle = Math.atan2(centY - circleCy, centX - circleCx);
  const opacity = 0.12 + 0.30 * (0.5 + 0.5 * Math.sin(angle * 2.1 + idx * 1.37));
  return `rgba(255,255,255,${opacity.toFixed(3)})`;
}





// The "vein" color — outer ring stroke color.
const VEIN_COLOR = "rgba(15, 18, 45, 0.90)";

// ---------------------------------------------------------------------------
// Cell interface
// ---------------------------------------------------------------------------
interface VoronoiCell {
  id: string;
  name: string;
  path: string;
  size: number;
  pct: number;
  polygon: [number, number][];       // raw polygon from d3-voronoi-treemap
  insetPolygon: [number, number][];   // polygon shrunk inward to create gap channel
  fill: string;
  labelX: number;
  labelY: number;
  showLabel: boolean;
  showSizeLabel: boolean;
  fontSize: number;
  area: number;
  node: DiskNode;
  children: VoronoiCell[];
  parentIndex: number;  // index of parent in top-level for color inheritance
}

const cells = ref<VoronoiCell[]>([]);

// Flattened leaf cells for the consolidated view gap mask (child cells if subdivided, else parent)
const consolidatedLeafCells = computed<VoronoiCell[]>(() => {
  const leaves: VoronoiCell[] = [];
  for (const cell of cells.value) {
    if (cell.children.length > 0) {
      leaves.push(...cell.children);
    } else {
      leaves.push(cell);
    }
  }
  return leaves;
});

// ---------------------------------------------------------------------------
// Cluster mode data — each cluster is one circle with its own voronoi
// ---------------------------------------------------------------------------
interface ClusterCircle {
  id: string;
  name: string;            // directory name or "Other"
  size: number;            // total bytes
  node: DiskNode;          // the DiskNode (or synthetic for "Other")
  cx: number;              // center x in SVG coords
  cy: number;              // center y
  r: number;               // radius
  colorIndex: number;      // hue family index
  cells: VoronoiCell[];    // voronoi cells within this circle
  organicPath: string;     // SVG path for the organic boundary
}

const clusterCircles = ref<ClusterCircle[]>([]);

// How many top-level dirs get their own circle (rest go to "Other")
const MAX_CLUSTERS = 4;

// Previous cells for animation interpolation
let prevCells: VoronoiCell[] = [];

// ---------------------------------------------------------------------------
// Generate circular clipping polygon (N-gon approximation of a circle)
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Organic blob polygon — layered sine perturbations give a natural irregular
// boundary instead of a perfect circle. Perturbation kept small so the polygon
// remains simple (non-self-intersecting) and the voronoi treemap stays valid.
// ---------------------------------------------------------------------------
function circlePolygon(cx: number, cy: number, r: number, n: number = 96): [number, number][] {
  const pts: [number, number][] = [];
  for (let i = 0; i < n; i++) {
    const a = (2 * Math.PI * i) / n;
    pts.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
  }
  return pts;
}


// Convert a polygon to an SVG path string (for use in <path d="...">)
function polyToPath(poly: [number, number][]): string {
  if (poly.length === 0) return '';
  return poly.map((p, i) => `${i === 0 ? 'M' : 'L'}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ') + ' Z';
}

// ---------------------------------------------------------------------------
// Inset a polygon toward its centroid to create gap channels
// ---------------------------------------------------------------------------
/**
 * Inset a polygon by moving each edge inward by a fixed pixel amount.
 * This creates uniform-width channels regardless of cell size — like the
 * leaf-vein reference where veins have consistent thickness.
 *
 * Algorithm: for each edge, compute its inward-facing normal, offset both
 * endpoints, then intersect consecutive offset edges to get new vertices.
 * Falls back to centroid-scaling if the geometry degenerates.
 */
function insetPolygon(poly: [number, number][], amount: number): [number, number][] {
  if (poly.length < 3 || amount <= 0) return poly;

  const n = poly.length;

  // Safety: don't inset more than would collapse the polygon.
  // Use a tighter clamp (0.28) to prevent degenerate shapes on narrow cells.
  const area = Math.abs(polygonArea(poly));
  const avgRadius = Math.sqrt(area / Math.PI);
  const safeAmount = Math.min(amount, avgRadius * 0.28);

  if (safeAmount < 0.5) return poly;

  // Compute inward normals for each edge
  // We need the polygon to be in a consistent winding order.
  // If area is negative the polygon is clockwise; flip normal direction.
  const sign = polygonArea(poly) > 0 ? 1 : -1;

  // For each edge i→i+1, compute offset line
  const offsets: { px: number; py: number; nx: number; ny: number }[] = [];

  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n;
    const dx = poly[j][0] - poly[i][0];
    const dy = poly[j][1] - poly[i][1];
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 0.001) {
      // Degenerate edge — use a dummy normal
      offsets.push({ px: poly[i][0], py: poly[i][1], nx: 0, ny: 0 });
      continue;
    }
    // Inward normal (perpendicular, pointing inside)
    const nx = (-dy / len) * sign;
    const ny = (dx / len) * sign;
    // Offset the edge start point by the normal * amount
    offsets.push({
      px: poly[i][0] + nx * safeAmount,
      py: poly[i][1] + ny * safeAmount,
      nx: dx, ny: dy, // edge direction for intersection
    });
  }

  // Intersect consecutive offset edges to find new vertices
  const result: [number, number][] = [];
  const [ccx, ccy] = polygonCentroid(poly);

  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n;

    // Compute offset start point of edge j (next edge after the shared vertex)
    const jNext = (j + 1) % n;
    const djx = poly[jNext][0] - poly[j][0];
    const djy = poly[jNext][1] - poly[j][1];
    const djLen = Math.sqrt(djx * djx + djy * djy);
    const njx = djLen > 0.001 ? (-djy / djLen) * sign : 0;
    const njy = djLen > 0.001 ? (djx / djLen) * sign : 0;
    const e2startX = poly[j][0] + njx * safeAmount;
    const e2startY = poly[j][1] + njy * safeAmount;

    // Intersect: P1 + t*D1 = P2 + s*D2
    const d1x = offsets[i].nx;
    const d1y = offsets[i].ny;
    const d2x = offsets[j].nx;
    const d2y = offsets[j].ny;
    const denom = d1x * d2y - d1y * d2x;

    if (Math.abs(denom) < 0.001) {
      // Parallel edges — use midpoint of the two offset start points
      result.push([
        (offsets[i].px + e2startX) / 2,
        (offsets[i].py + e2startY) / 2,
      ]);
    } else {
      const t = ((e2startX - offsets[i].px) * d2y - (e2startY - offsets[i].py) * d2x) / denom;
      const ix = offsets[i].px + t * d1x;
      const iy = offsets[i].py + t * d1y;

      // Sanity check: intersection shouldn't be too far from centroid
      const distFromCenter = Math.sqrt((ix - ccx) ** 2 + (iy - ccy) ** 2);
      if (distFromCenter > avgRadius * 2) {
        // Degenerate — fallback to scaled point
        const scale = Math.max(0.4, 1 - safeAmount / avgRadius);
        result.push([
          ccx + (poly[j][0] - ccx) * scale,
          ccy + (poly[j][1] - ccy) * scale,
        ]);
      } else {
        result.push([ix, iy]);
      }
    }
  }

  // Final safety: if inset polygon self-intersects or has negative area,
  // fall back to simple centroid scaling
  const insetArea = Math.abs(polygonArea(result));
  if (insetArea < area * 0.05 || insetArea > area * 1.5) {
    const scale = Math.max(0.4, 1 - safeAmount / avgRadius);
    return poly.map(([x, y]) => [
      ccx + (x - ccx) * scale,
      ccy + (y - ccy) * scale,
    ] as [number, number]);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Smooth/round polygon corners using corner-cutting with quadratic Bezier.
//
// For each vertex, we cut back along both adjacent edges by a fraction
// (roundFraction) and insert a Q-curve through the original vertex direction.
// This NEVER overshoots the polygon boundary — the rounded path stays
// strictly inside the convex hull of each corner's two edges.
// ---------------------------------------------------------------------------
function smoothPolygonPath(poly: [number, number][], roundFraction: number = 0.28): string {
  if (poly.length < 3) return "";

  const n = poly.length;
  let d = "";

  for (let i = 0; i < n; i++) {
    const prev = poly[(i - 1 + n) % n];
    const curr = poly[i];
    const next = poly[(i + 1) % n];

    // Distances to neighbors
    const dPrev = Math.sqrt((curr[0] - prev[0]) ** 2 + (curr[1] - prev[1]) ** 2);
    const dNext = Math.sqrt((next[0] - curr[0]) ** 2 + (next[1] - curr[1]) ** 2);

    // Cut distance: fraction of the shorter adjacent edge, capped
    const cut = Math.min(dPrev, dNext) * roundFraction;

    // Points along edges where the curve starts and ends
    // (pulling back from the vertex toward each neighbor)
    const startX = curr[0] + (prev[0] - curr[0]) * (cut / dPrev);
    const startY = curr[1] + (prev[1] - curr[1]) * (cut / dPrev);
    const endX = curr[0] + (next[0] - curr[0]) * (cut / dNext);
    const endY = curr[1] + (next[1] - curr[1]) * (cut / dNext);

    if (i === 0) {
      d += `M${startX.toFixed(1)},${startY.toFixed(1)} `;
    } else {
      d += `L${startX.toFixed(1)},${startY.toFixed(1)} `;
    }
    // Quadratic bezier with the original vertex as control point
    d += `Q${curr[0].toFixed(1)},${curr[1].toFixed(1)} ${endX.toFixed(1)},${endY.toFixed(1)} `;
  }

  return d + "Z";
}

// ---------------------------------------------------------------------------
// Build the Voronoi treemap
// ---------------------------------------------------------------------------
function buildVoronoi(node: DiskNode, animate: boolean = true) {
  if (!containerRef.value) return;

  const el = containerRef.value;
  width = el.clientWidth;
  height = el.clientHeight;

  // Rectangle fills the container with some padding
  const padding = 20;
  cx = width / 2;
  cy = height / 2;
  radius = Math.min(width, height) / 2 - padding;
  substrateGeom.value = { cx, cy, r: radius };

  if (radius < 50) return;

  // Filter children to those with size > 0
  const allChildren = (node.children || []).filter(
    (c) => c.size > 0 && (c.path || !c.name.includes("other"))
  );

  if (allChildren.length === 0) {
    cells.value = [];
    return;
  }

  // Merge tiny children (below 0.5% of parent) into a single "other" bucket.
  // This eliminates the visual noise of many tiny slivers packed together.
  const parentSize = node.size || allChildren.reduce((s, c) => s + c.size, 0);
  const minSize = parentSize * 0.005;
  const bigChildren = allChildren.filter(c => c.size >= minSize);
  const smallChildren = allChildren.filter(c => c.size < minSize);

  let children = bigChildren;
  if (smallChildren.length > 0) {
    const otherSize = smallChildren.reduce((s, c) => s + c.size, 0);
    const otherNode: DiskNode = {
      name: `${smallChildren.length} other items`,
      path: "",
      size: otherSize,
      expanded: false,
      children: smallChildren,
      category: "other",
      modified: null,
    };
    children = [...bigChildren, otherNode];
  }

  // Also cap max children for readability (too many cells = visual soup)
  if (children.length > 20) {
    const kept = children.slice(0, 18).sort((a, b) => b.size - a.size);
    const rest = children.slice(18);
    const restSize = rest.reduce((s, c) => s + c.size, 0);
    const restNode: DiskNode = {
      name: `${rest.length} other items`,
      path: "",
      size: restSize,
      expanded: false,
      children: rest,
      category: "other",
      modified: null,
    };
    children = [...kept, restNode];
  }

  // Build d3 hierarchy — FLAT (one level only).
  // Each child gets a single cell proportional to its total size.
  // No grandchildren subdivision — that only happens when you drill in.
  // This eliminates the messy overlapping child overlay issue.
  const rootData = {
    name: node.name,
    children: children.map((c) => ({
      name: c.name,
      value: c.size,
      originalNode: c,
    })),
  };

  const hierarchy = d3
    .hierarchy(rootData)
    .sum((d: any) => d.value || 0);

  // Create voronoi treemap with rectangular boundary
  const clipPoly: [number, number][] = [
    [padding, padding],
    [width - padding, padding],
    [width - padding, height - padding],
    [padding, height - padding],
  ];
  organicBoundaryPath.value = polyToPath(clipPoly);

  const treemap = voronoiTreemap()
    .clip(clipPoly)
    .maxIterationCount(150)
    .convergenceRatio(0.001);

  treemap(hierarchy);

  // Inset amount — the visible vein channel between two neighbors is 2× the inset.
  // Larger inset = wider dark gap (substrate shows through).
  const CELL_INSET = 0.7;

  // Save previous cells for animation
  prevCells = [...cells.value];

  // Extract cells — one per child, no grandchild subdivision
  const totalSize = node.size || 1;
  const result: VoronoiCell[] = [];

  hierarchy.children?.forEach((child: any, i: number) => {
    const poly = child.polygon as [number, number][];
    if (!poly || poly.length < 3) return;

    const inset = insetPolygon(poly, CELL_INSET);
    const cent = polygonCentroid(inset);
    const area = Math.abs(polygonArea(inset));
    const effectiveSize = Math.sqrt(area);

    result.push({
      id: `${i}`,
      name: child.data.name,
      path: child.data.originalNode?.path || "",
      size: child.data.originalNode?.size || child.value,
      pct: ((child.data.originalNode?.size || child.value) / totalSize) * 100,
      polygon: poly,
      insetPolygon: inset,
      fill: wingCellFill(cent[0], cent[1], i, cx, cy),
      labelX: cent[0],
      labelY: cent[1],
      showLabel: effectiveSize > 55,
      showSizeLabel: effectiveSize > 80,
      fontSize: Math.max(11, Math.min(18, effectiveSize * 0.13)),
      area,
      node: child.data.originalNode,
      children: [],
      parentIndex: i,
    });
  });

  // ---------------------------------------------------------------------------
  // Nested subdivision — run a child voronoi inside each parent cell
  // ---------------------------------------------------------------------------
  const CHILD_INSET = 0.8;
  for (const parentCell of result) {
    const grandchildren = (parentCell.node?.children || [])
      .filter((c) => c.size > 0)
      .sort((a, b) => b.size - a.size)
      .slice(0, 20);

    if (grandchildren.length < 2 || parentCell.insetPolygon.length < 3) continue;

    const childData = {
      name: parentCell.name,
      children: grandchildren.map((c) => ({ name: c.name, value: c.size, originalNode: c })),
    };
    const childHierarchy = d3.hierarchy(childData).sum((d: any) => d.value || 0);

    try {
      voronoiTreemap()
        .clip(parentCell.insetPolygon)
        .maxIterationCount(80)
        .convergenceRatio(0.005)(childHierarchy);
    } catch (_) {
      continue;
    }

    const parentSize = parentCell.size || 1;
    childHierarchy.children?.forEach((child: any, ci: number) => {
      const poly = child.polygon as [number, number][];
      if (!poly || poly.length < 3) return;
      const inset = insetPolygon(poly, CHILD_INSET);
      const cent = polygonCentroid(inset);
      const area = Math.abs(polygonArea(inset));
      const effSize = Math.sqrt(area);
      parentCell.children.push({
        id: `${parentCell.id}-c${ci}`,
        name: child.data.name,
        path: child.data.originalNode?.path || "",
        size: child.data.originalNode?.size || child.value,
        pct: ((child.data.originalNode?.size || child.value) / parentSize) * 100,
        polygon: poly,
        insetPolygon: inset,
        fill: wingCellFill(cent[0], cent[1], ci, cx, cy),
        labelX: cent[0],
        labelY: cent[1],
        showLabel: effSize > 35,
        showSizeLabel: effSize > 55,
        fontSize: Math.max(8, Math.min(13, effSize * 0.11)),
        area,
        node: child.data.originalNode,
        children: [],
        parentIndex: parentCell.parentIndex,
      });
    });
  }

  // ---------------------------------------------------------------------------
  // Post-processing: suppress labels that are too close to the circle edge
  // or that overlap other labels. This runs after all cells are built so we
  // can consider every label position together.
  // ---------------------------------------------------------------------------
  const edgeMargin = radius * 0.08; // labels must be this far inside the circle

  // Collect all label positions for overlap detection
  interface LabelRect {
    x: number; y: number;
    halfW: number; halfH: number;
    area: number;
    cell: VoronoiCell;
  }
  const labelRects: LabelRect[] = [];

  for (const cell of result) {
    if (cell.showLabel) {
      const distFromCenter = Math.sqrt((cell.labelX - cx) ** 2 + (cell.labelY - cy) ** 2);
      if (distFromCenter > radius - edgeMargin) {
        cell.showLabel = false;
        cell.showSizeLabel = false;
      }
    }
    if (cell.showLabel) {
      const charW = cell.fontSize * 0.55;
      const nameLen = Math.min(cell.name.length, maxChars(Math.sqrt(cell.area), cell.fontSize));
      const halfW = (nameLen * charW) / 2;
      const halfH = cell.showSizeLabel ? cell.fontSize * 1.2 : cell.fontSize * 0.6;
      labelRects.push({ x: cell.labelX, y: cell.labelY, halfW, halfH, area: cell.area, cell });
    }
  }

  // Sort by area descending — larger labels win collisions
  labelRects.sort((a, b) => b.area - a.area);

  // Simple O(n^2) overlap check — fine for <100 labels
  for (let i = 0; i < labelRects.length; i++) {
    if (!labelRects[i].cell.showLabel) continue;
    for (let j = i + 1; j < labelRects.length; j++) {
      if (!labelRects[j].cell.showLabel) continue;
      const a = labelRects[i];
      const b = labelRects[j];
      // AABB overlap test with a small padding
      const pad = 4;
      if (
        Math.abs(a.x - b.x) < (a.halfW + b.halfW + pad) &&
        Math.abs(a.y - b.y) < (a.halfH + b.halfH + pad)
      ) {
        // Smaller label loses
        labelRects[j].cell.showLabel = false;
        labelRects[j].cell.showSizeLabel = false;
      }
    }
  }

  if (animate && prevCells.length > 0) {
    animateTo(result);
  } else {
    cells.value = result;
  }
}

// ---------------------------------------------------------------------------
// Build the "Clusters" layout — multiple circles, one per major directory
// ---------------------------------------------------------------------------
/**
 * Split root children into N largest + "Other", compute circle sizes
 * proportional to byte size, pack them using d3.packSiblings, and run
 * a voronoi treemap inside each circle.
 */
function buildClusters(node?: DiskNode) {
  if (!containerRef.value || !props.data) return;

  const el = containerRef.value;
  width = el.clientWidth;
  height = el.clientHeight;

  const targetNode = node || currentNode.value || props.data.root;
  const children = (targetNode.children || [])
    .filter((c) => c.size > 0)
    .sort((a, b) => b.size - a.size);

  if (children.length === 0) {
    clusterCircles.value = [];
    return;
  }

  // Split: top N get their own circle, rest are grouped into "Other"
  const topChildren = children.slice(0, MAX_CLUSTERS);
  const restChildren = children.slice(MAX_CLUSTERS);

  // Build cluster entries
  interface ClusterEntry {
    name: string;
    size: number;
    node: DiskNode;
    colorIndex: number;
  }

  const entries: ClusterEntry[] = topChildren.map((c, i) => ({
    name: c.name,
    size: c.size,
    node: c,
    colorIndex: i,
  }));

  // "Other" bucket — create a synthetic DiskNode
  if (restChildren.length > 0) {
    const otherSize = restChildren.reduce((s, c) => s + c.size, 0);
    const otherNode: DiskNode = {
      name: `${restChildren.length} other items`,
      path: "",
      size: otherSize,
      expanded: false,
      children: restChildren,
      category: "other",
      modified: null,
    };
    entries.push({
      name: otherNode.name,
      size: otherSize,
      node: otherNode,
      colorIndex: MAX_CLUSTERS,
    });
  }

  // Circle radii proportional to sqrt(size) — area ∝ size
  const totalSize = entries.reduce((s, e) => s + e.size, 0);
  const availableR = Math.min(width, height) / 2 - 30;

  // d3.packSiblings expects objects with an `r` property.
  // We scale radii so the total area fits nicely in the container.
  // Total area of packed circles ≈ π * Σ(ri²). We want this to be roughly
  // 60-70% of the container area for breathing room.
  const targetArea = Math.PI * availableR * availableR * 0.65;
  const rawRadii = entries.map((e) => Math.sqrt(e.size / totalSize));
  const rawArea = rawRadii.reduce((s, r) => s + Math.PI * r * r, 0);
  const scaleFactor = Math.sqrt(targetArea / rawArea);

  const packInput = entries.map((_e, i) => ({
    r: rawRadii[i] * scaleFactor,
    idx: i,
  }));

  // d3.packSiblings arranges circles with no overlap, centered near origin
  const packed = d3.packSiblings(packInput);

  // Find bounding box of packed circles — include label space below each circle
  const LABEL_SPACE = 40; // px reserved below each circle for name + size labels
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (const p of packed) {
    minX = Math.min(minX, p.x - p.r);
    maxX = Math.max(maxX, p.x + p.r);
    minY = Math.min(minY, p.y - p.r);
    maxY = Math.max(maxY, p.y + p.r + LABEL_SPACE);
  }
  const packW = maxX - minX;
  const packH = maxY - minY;

  // Scale so the packed cluster fits the container with breathing room
  const fitScale = Math.min(
    (width - 40) / packW,
    (height - 40) / packH,
    1.0
  );
  const offsetX = width / 2 - (minX + maxX) / 2 * fitScale;
  const offsetY = height / 2 - (minY + maxY) / 2 * fitScale;

  // Build a voronoi treemap inside each circle
  const circles: ClusterCircle[] = [];

  for (const p of packed) {
    const entry = entries[p.idx];
    const circleCx = p.x * fitScale + offsetX;
    const circleCy = p.y * fitScale + offsetY;
    const circleR = p.r * fitScale;

    if (circleR < 20) continue; // too small to render

    // Build voronoi for this cluster's children
    const clusterChildren = (entry.node.children || [])
      .filter((c) => c.size > 0)
      .sort((a, b) => b.size - a.size);

    let clusterCells: VoronoiCell[] = [];
    const clipPoly = circlePolygon(circleCx, circleCy, circleR, 96);

    if (clusterChildren.length > 0) {
      // Build d3 hierarchy for this cluster
      const clusterData = {
        name: entry.name,
        children: clusterChildren.slice(0, 30).map((c) => ({
          name: c.name,
          value: c.size,
          originalNode: c,
        })),
      };

      const hierarchy = d3
        .hierarchy(clusterData)
        .sum((d: any) => d.value || 0);

      const treemap = voronoiTreemap()
        .clip(clipPoly)
        .maxIterationCount(80)
        .convergenceRatio(0.005);

      try {
        treemap(hierarchy);
      } catch (_) {
        // voronoi-treemap can occasionally fail on degenerate inputs
        continue;
      }

      // Inset — creates dark gap channels between cells
      const CLUSTER_INSET = 0.7;

      const clusterTotalSize = entry.size || 1;

      hierarchy.children?.forEach((child: any, ci: number) => {
        const poly = child.polygon as [number, number][];
        if (!poly || poly.length < 3) return;

        const inset = insetPolygon(poly, CLUSTER_INSET);
        const cent = polygonCentroid(inset);
        const area = Math.abs(polygonArea(inset));
        const effectiveSize = Math.sqrt(area);

        clusterCells.push({
          id: `cluster-${p.idx}-${ci}`,
          name: child.data.name,
          path: child.data.originalNode?.path || "",
          size: child.data.originalNode?.size || child.value,
          pct: ((child.data.originalNode?.size || child.value) / clusterTotalSize) * 100,
          polygon: poly,
          insetPolygon: inset,
          fill: wingCellFill(cent[0], cent[1], ci, circleCx, circleCy),
          labelX: cent[0],
          labelY: cent[1],
          showLabel: effectiveSize > 50,
          showSizeLabel: effectiveSize > 75,
          fontSize: Math.max(8, Math.min(14, effectiveSize * 0.12)),
          area,
          node: child.data.originalNode,
          children: [],
          parentIndex: entry.colorIndex,
        });
      });

      // Edge-clip labels that are too close to this circle's boundary
      const edgeMargin = circleR * 0.10;
      for (const cell of clusterCells) {
        if (!cell.showLabel) continue;
        const dist = Math.sqrt((cell.labelX - circleCx) ** 2 + (cell.labelY - circleCy) ** 2);
        if (dist > circleR - edgeMargin) {
          cell.showLabel = false;
          cell.showSizeLabel = false;
        }
      }
    }

    circles.push({
      id: `cluster-${p.idx}`,
      name: entry.name,
      size: entry.size,
      node: entry.node,
      cx: circleCx,
      cy: circleCy,
      r: circleR,
      colorIndex: entry.colorIndex,
      cells: clusterCells,
      organicPath: polyToPath(clipPoly),
    });
  }

  clusterCircles.value = circles;
}

// ---------------------------------------------------------------------------
// Animation — morph from prevCells to targetCells over ~400ms
// ---------------------------------------------------------------------------
function animateTo(targetCells: VoronoiCell[]) {
  const duration = 400;
  const start = performance.now();
  animating.value = true;

  // Create a simple opacity-based transition
  // (Full polygon morphing requires matching cell counts which is complex)
  // Instead: fade out old, fade in new with a scale effect
  const fadePhase = () => {
    const elapsed = performance.now() - start;
    const t = Math.min(1, elapsed / duration);
    if (t < 1) {
      animFrameId = requestAnimationFrame(fadePhase);
    } else {
      animating.value = false;
    }

    // Just set final cells — CSS transition handles the visual smoothness
    if (t > 0.05) {
      cells.value = targetCells;
    }
  };

  // Brief delay then swap with CSS transitions handling the visual
  cells.value = [];
  requestAnimationFrame(() => {
    setTimeout(() => {
      cells.value = targetCells;
      animating.value = false;
    }, 50);
  });
}

// ---------------------------------------------------------------------------
// Polygon helpers
// ---------------------------------------------------------------------------
function polygonCentroid(poly: [number, number][]): [number, number] {
  let ccx = 0, ccy = 0;
  for (const [x, y] of poly) { ccx += x; ccy += y; }
  return [ccx / poly.length, ccy / poly.length];
}

function polygonArea(poly: [number, number][]): number {
  let area = 0;
  const n = poly.length;
  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n;
    area += poly[i][0] * poly[j][1];
    area -= poly[j][0] * poly[i][1];
  }
  return area / 2;
}

// Truncate long names to fit cell
function truncateName(name: string, maxLen: number): string {
  if (name.length <= maxLen) return name;
  return name.slice(0, maxLen - 1) + "\u2026";
}

// Compute max chars based on cell size and font size
function maxChars(cellSize: number, fontSize: number): number {
  const charWidth = fontSize * 0.55;
  return Math.max(3, Math.floor((cellSize * 0.8) / charWidth));
}

// ---------------------------------------------------------------------------
// Navigation — drill in / out
// ---------------------------------------------------------------------------
function drillInto(cell: VoronoiCell) {
  // Suppress drill-down if the user just finished panning (drag, not click)
  if (zoomPan.state.didDrag) return;

  if (!cell.node || !cell.node.children || cell.node.children.length === 0) return;

  if (currentNode.value) {
    breadcrumbs.value.push({ name: currentNode.value.name, node: currentNode.value });
  }
  currentNode.value = cell.node;
  resetCamera();
  buildVoronoi(cell.node, true);
}

/** Drill into a cluster circle — stays in clusters mode, showing that node's children as new clusters */
function drillIntoCluster(cluster: ClusterCircle) {
  if (zoomPan.state.didDrag) return;
  if (!cluster.node.children || cluster.node.children.length === 0) return;

  // Push current node onto breadcrumbs so navigateBack works
  if (currentNode.value) {
    breadcrumbs.value.push({ name: currentNode.value.name, node: currentNode.value });
  }
  currentNode.value = cluster.node;
  resetCamera();
  buildClusters(cluster.node);
}

function navigateBack() {
  resetCamera();
  if (breadcrumbs.value.length > 0) {
    const prev = breadcrumbs.value.pop()!;
    currentNode.value = prev.node;
    if (voronoiMode.value === "clusters") {
      buildClusters(prev.node);
    } else {
      buildVoronoi(prev.node, true);
    }
  } else if (props.data) {
    currentNode.value = props.data.root;
    if (voronoiMode.value === "clusters") {
      buildClusters(props.data.root);
    } else {
      buildVoronoi(props.data.root, true);
    }
  }
}

// Can we go back? True when we've drilled into at least one level
const canGoBack = computed(() => breadcrumbs.value.length > 0);

// Current location label: the name of the node we're viewing
const locationLabel = computed(() => currentNode.value?.name || "/");

// ---------------------------------------------------------------------------
// Expand / compact
// ---------------------------------------------------------------------------
function toggleExpand() {
  const wasExpanded = props.expanded;
  emit("update:expanded", !wasExpanded);
  nextTick(() => {
    // Always reset camera when toggling — especially important when collapsing
    // so the graph doesn't stay zoomed/panned from the expanded session
    resetCamera();
    if (voronoiMode.value === "clusters") {
      buildClusters();
    } else if (currentNode.value) {
      buildVoronoi(currentNode.value, false);
    }
  });
}

// ---------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------
function onCellEnter(cell: VoronoiCell, event: MouseEvent) {
  hoveredCellId.value = cell.id;
  hoveredCell.value = {
    name: cell.name,
    size: cell.size,
    pct: cell.pct,
    x: event.clientX,
    y: event.clientY,
  };
}

function onCellMove(event: MouseEvent) {
  if (hoveredCell.value) {
    hoveredCell.value.x = event.clientX;
    hoveredCell.value.y = event.clientY;
  }
}

function onCellLeave() {
  hoveredCellId.value = null;
  hoveredCell.value = null;
}

// ---------------------------------------------------------------------------
// Computed: tooltip position relative to container
// ---------------------------------------------------------------------------
const tooltipStyle = computed(() => {
  if (!hoveredCell.value || !containerRef.value) return { display: "none" };
  const rect = containerRef.value.getBoundingClientRect();
  const x = hoveredCell.value.x - rect.left + 14;
  const y = hoveredCell.value.y - rect.top - 10;
  return {
    left: `${x}px`,
    top: `${y}px`,
    display: "flex",
  };
});

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
let resizeObs: ResizeObserver | null = null;

function build() {
  if (!props.data) return;
  currentNode.value = props.data.root;
  breadcrumbs.value = [];
  if (voronoiMode.value === "clusters") {
    buildClusters(props.data.root);
  } else {
    buildVoronoi(props.data.root, false);
  }
}



onMounted(() => {
  if (props.data) nextTick(() => build());
  if (containerRef.value) {
    resizeObs = new ResizeObserver(() => {
      nextTick(() => {
        if (voronoiMode.value === "clusters") {
          buildClusters(currentNode.value || undefined);
        } else if (currentNode.value) {
          buildVoronoi(currentNode.value!, false);
        }
      });
    });
    resizeObs.observe(containerRef.value);
  }
});

onUnmounted(() => {
  if (resizeObs) resizeObs.disconnect();
  if (animFrameId) cancelAnimationFrame(animFrameId);
});

watch(() => props.data, () => { if (props.data) nextTick(() => build()); });

// Re-build when switching between consolidated and clusters mode
watch(voronoiMode, () => {
  resetCamera();
  breadcrumbs.value = [];
  if (props.data) {
    currentNode.value = props.data.root;
    nextTick(() => build());
  }
});
</script>

<template>
  <div
    class="voronoi-container"
    :class="{ expanded, animating }"
    ref="containerRef"
    @mousemove="(e: MouseEvent) => { onVoronoiMouseMove(e); onCellMove(e); }"
    @wheel="onVoronoiWheel"
    @mousedown="onVoronoiMouseDown"
    @mouseup="onVoronoiMouseUp"
    @mouseleave="onVoronoiMouseUp"
  >
    <!-- Mode toggle: Consolidated / Clusters -->
    <div class="voronoi-mode-toggle" v-if="props.data">
      <button
        class="voronoi-mode-btn"
        :class="{ active: voronoiMode === 'consolidated' }"
        @click="voronoiMode = 'consolidated'"
      >Consolidated</button>
      <button
        class="voronoi-mode-btn"
        :class="{ active: voronoiMode === 'clusters' }"
        @click="voronoiMode = 'clusters'"
      >Clusters</button>
    </div>

    <!-- ================================================================= -->
    <!-- CONSOLIDATED MODE — single circle (existing)                       -->
    <!-- ================================================================= -->
    <svg v-if="voronoiMode === 'consolidated'" ref="svgRef" class="voronoi-svg" :viewBox="`0 0 ${width} ${height}`" :width="width" :height="height">
      <defs>
        <clipPath id="voronoi-circle-clip">
          <path :d="organicBoundaryPath" />
        </clipPath>
        <!-- Gap mask: white inside boundary, black where leaf cells are → dark only in gaps -->
        <mask id="gap-mask">
          <path :d="organicBoundaryPath" fill="white" />
          <path v-for="leaf in consolidatedLeafCells" :key="'gm-'+leaf.id"
            :d="smoothPolygonPath(leaf.insetPolygon)" fill="black" />
        </mask>
        <filter id="cell-glow" x="-10%" y="-10%" width="120%" height="120%">
          <feGaussianBlur in="SourceAlpha" stdDeviation="3" result="blur" />
          <feFlood flood-color="rgba(2, 117, 244, 0.4)" result="color" />
          <feComposite in="color" in2="blur" operator="in" result="shadow" />
          <feMerge>
            <feMergeNode in="shadow" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <g ref="zoomGroupRef">
        <!-- Dark gap layer: organic boundary, cells punched out by mask -->
        <path :d="organicBoundaryPath" fill="rgba(12, 16, 42, 0.90)" mask="url(#gap-mask)" />
        <!-- Organic outline ring -->
        <path :d="organicBoundaryPath" fill="none" :stroke="VEIN_COLOR" stroke-width="1" class="voronoi-outer-ring" />

        <g clip-path="url(#voronoi-circle-clip)">
          <!-- Leaf cell fills — child cells if subdivided, else parent cell -->
          <template v-for="cell in cells" :key="'pg-' + cell.id">
            <template v-if="cell.children.length > 0">
              <path v-for="child in cell.children" :key="'cf-' + child.id"
                :d="smoothPolygonPath(child.insetPolygon)" :fill="child.fill"
                stroke="none" class="voronoi-parent-fill" />
            </template>
            <path v-else
              :d="smoothPolygonPath(cell.insetPolygon)" :fill="cell.fill"
              stroke="none" class="voronoi-parent-fill"
              :class="{ hovered: hoveredCellId === cell.id }" />
          </template>

          <!-- Interactive hit areas — always on parent cell polygon -->
          <path v-for="cell in cells" :key="'ph-' + cell.id"
            :d="smoothPolygonPath(cell.insetPolygon)" fill="transparent"
            class="voronoi-cell-hit"
            :filter="hoveredCellId === cell.id ? 'url(#cell-glow)' : undefined"
            @mouseenter="onCellEnter(cell, $event)" @mouseleave="onCellLeave"
            @click="drillInto(cell)" />

          <!-- Child labels (subcells) -->
          <template v-for="cell in cells" :key="'chlg-' + cell.id">
            <template v-for="child in cell.children" :key="'chl-' + child.id">
              <text v-if="child.showLabel" :x="child.labelX"
                :y="child.labelY - (child.showSizeLabel ? child.fontSize * 0.35 : 0)"
                :font-size="child.fontSize" class="voronoi-label voronoi-child-label">
                {{ truncateName(child.name, maxChars(Math.sqrt(child.area), child.fontSize)) }}
              </text>
              <text v-if="child.showSizeLabel" :x="child.labelX"
                :y="child.labelY + child.fontSize * 0.7" :font-size="child.fontSize * 0.75"
                class="voronoi-label voronoi-size-label">
                {{ formatSize(child.size) }}
              </text>
            </template>
          </template>

          <!-- Parent labels — shown above subcells -->
          <template v-for="cell in cells" :key="'plg-' + cell.id">
            <text v-if="cell.showLabel" :x="cell.labelX"
              :y="cell.labelY - (cell.showSizeLabel ? cell.fontSize * 0.4 : 0)"
              :font-size="cell.fontSize" class="voronoi-label voronoi-parent-label">
              {{ truncateName(cell.name, maxChars(Math.sqrt(cell.area), cell.fontSize)) }}
            </text>
            <text v-if="cell.showSizeLabel" :x="cell.labelX"
              :y="cell.labelY + cell.fontSize * 0.75" :font-size="cell.fontSize * 0.7"
              class="voronoi-label voronoi-size-label voronoi-parent-size">
              {{ formatSize(cell.size) }}
            </text>
          </template>
        </g>
      </g>
    </svg>

    <!-- ================================================================= -->
    <!-- CLUSTERS MODE — multiple circles                                   -->
    <!-- ================================================================= -->
    <svg v-else-if="voronoiMode === 'clusters'" ref="svgRef" class="voronoi-svg"
      :viewBox="`0 0 ${width} ${height}`" :width="width" :height="height">
      <defs>
        <!-- Per-cluster clip paths and gap masks -->
        <clipPath v-for="circle in clusterCircles" :key="'clip-' + circle.id"
          :id="'cluster-clip-' + circle.id">
          <circle :cx="circle.cx" :cy="circle.cy" :r="circle.r" />
        </clipPath>
        <mask v-for="circle in clusterCircles" :key="'cgm-' + circle.id"
          :id="'cgap-mask-' + circle.id">
          <circle :cx="circle.cx" :cy="circle.cy" :r="circle.r" fill="white" />
          <path v-for="cell in circle.cells" :key="'cgmc-'+cell.id"
            :d="smoothPolygonPath(cell.insetPolygon)" fill="black" />
        </mask>
        <filter id="cell-glow-cluster" x="-10%" y="-10%" width="120%" height="120%">
          <feGaussianBlur in="SourceAlpha" stdDeviation="3" result="blur" />
          <feFlood flood-color="rgba(2, 117, 244, 0.4)" result="color" />
          <feComposite in="color" in2="blur" operator="in" result="shadow" />
          <feMerge>
            <feMergeNode in="shadow" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <g ref="zoomGroupRef">
        <!-- Render each cluster circle -->
        <g v-for="circle in clusterCircles" :key="circle.id" class="cluster-group">
          <!-- Dark gap layer for this cluster -->
          <circle :cx="circle.cx" :cy="circle.cy" :r="circle.r" fill="rgba(12, 16, 42, 0.90)" :mask="`url(#cgap-mask-${circle.id})`" />

          <!-- Clipped cells -->
          <g :clip-path="`url(#cluster-clip-${circle.id})`">
            <!-- White membrane cells — varying opacity -->
            <path v-for="cell in circle.cells" :key="'cpf-' + cell.id"
              :d="smoothPolygonPath(cell.insetPolygon)" :fill="cell.fill"
              stroke="none"
              class="voronoi-parent-fill"
              :class="{ hovered: hoveredCellId === cell.id }" />

            <!-- Hit areas -->
            <path v-for="cell in circle.cells" :key="'cph-' + cell.id"
              :d="smoothPolygonPath(cell.insetPolygon)" fill="transparent"
              class="voronoi-cell-hit"
              :filter="hoveredCellId === cell.id ? 'url(#cell-glow-cluster)' : undefined"
              @mouseenter="onCellEnter(cell, $event)" @mouseleave="onCellLeave"
              @click="drillIntoCluster(circle)" />

            <!-- Labels -->
            <template v-for="cell in circle.cells" :key="'cpl-' + cell.id">
              <text v-if="cell.showLabel" :x="cell.labelX"
                :y="cell.labelY - (cell.showSizeLabel ? cell.fontSize * 0.35 : 0)"
                :font-size="cell.fontSize" class="voronoi-label voronoi-child-label">
                {{ truncateName(cell.name, maxChars(Math.sqrt(cell.area), cell.fontSize)) }}
              </text>
              <text v-if="cell.showSizeLabel" :x="cell.labelX"
                :y="cell.labelY + cell.fontSize * 0.7" :font-size="cell.fontSize * 0.75"
                class="voronoi-label voronoi-size-label">
                {{ formatSize(cell.size) }}
              </text>
            </template>
          </g>

          <!-- Circle outline -->
          <circle :cx="circle.cx" :cy="circle.cy" :r="circle.r" fill="none" :stroke="VEIN_COLOR" stroke-width="1" class="voronoi-outer-ring" />

          <!-- Cluster label: name + size below circle -->
          <text :x="circle.cx" :y="circle.cy + circle.r + 18"
            text-anchor="middle" class="cluster-label-name">
            {{ circle.name }}
          </text>
          <text :x="circle.cx" :y="circle.cy + circle.r + 32"
            text-anchor="middle" class="cluster-label-size">
            {{ formatSize(circle.size) }}
          </text>
        </g>
      </g>
    </svg>

    <!-- Back button + location label -->
    <div class="voronoi-nav" v-if="(voronoiMode === 'consolidated' && cells.length > 0) || (voronoiMode === 'clusters' && canGoBack)">
      <button
        class="voronoi-back-btn"
        :class="{ disabled: !canGoBack }"
        :disabled="!canGoBack"
        @click="navigateBack"
        title="Go back"
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
          <path d="M10 3L5 8L10 13" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <span class="voronoi-location">{{ locationLabel }}</span>
    </div>

    <!-- Cursor-following tooltip -->
    <div class="voronoi-tooltip" :style="tooltipStyle" v-if="hoveredCell">
      <span class="tooltip-name">{{ hoveredCell.name }}</span>
      <span class="tooltip-size">{{ formatSize(hoveredCell.size) }}</span>
      <span class="tooltip-pct">{{ hoveredCell.pct.toFixed(1) }}%</span>
    </div>

    <!-- Expand / Compact button -->
    <button class="voronoi-expand-btn" @click="toggleExpand">
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

    <!-- Floating compact FAB (expanded mode only) -->
    <button
      v-if="expanded"
      class="voronoi-compact-fab"
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

    <!-- Empty state -->
    <div v-if="!props.data" class="voronoi-empty">
      <p class="voronoi-empty-title">No scan data</p>
      <p class="voronoi-empty-sub">Run a disk scan to see the Voronoi treemap.</p>
    </div>

    <div class="voronoi-hint" v-if="voronoiMode === 'clusters' && clusterCircles.length > 0">
      Click a cluster to explore
    </div>
    <div class="voronoi-hint" v-else-if="voronoiMode === 'consolidated' && cells.length > 0 && !canGoBack">
      Click a cell to explore
    </div>
  </div>
</template>

<style scoped>
.voronoi-container {
  position: relative;
  width: 100%;
  height: 600px;
  border-radius: var(--radius-md);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.voronoi-container.expanded {
  position: fixed;
  top: 0;
  left: 230px;
  right: 0;
  bottom: 0;
  height: auto;
  max-width: none;
  border-radius: 0;
  z-index: 1;
  cursor: grab;
  user-select: none;
}

/* While actively dragging, show closed-hand cursor */
.voronoi-container.expanded.panning {
  cursor: grabbing;
}

.voronoi-svg {
  display: block;
}

/* During active zoom/pan, hide labels and hit areas for smooth performance.
   The .zooming class is toggled via direct DOM manipulation with a debounce. */
.voronoi-svg.zooming .voronoi-label,
.voronoi-svg.zooming .voronoi-cell-hit {
  display: none;
}

/* ---------------------------------------------------------------------------
   Background circle — the dark "vein" substrate visible through gaps
   --------------------------------------------------------------------------- */
.voronoi-bg-circle {
  /* smooth anti-aliased edge */
}

.voronoi-outer-ring {
  opacity: 0.7;
}

/* ---------------------------------------------------------------------------
   Cell fills — parent and child layers
   --------------------------------------------------------------------------- */
.voronoi-parent-fill {
  transition: filter 0.2s ease;
}

.voronoi-parent-fill.hovered {
  filter: brightness(1.08) saturate(0.92);
}

.voronoi-child-fill {
  transition: filter 0.2s ease;
}

.voronoi-child-fill.hovered {
  opacity: 0.85;
  filter: brightness(1.12);
}

/* Interactive hit area — invisible but captures mouse events */
.voronoi-cell-hit {
  cursor: pointer;
  transition: filter 0.15s;
}

/* ---------------------------------------------------------------------------
   Animation — cells fade in on drill
   --------------------------------------------------------------------------- */
.voronoi-container.animating .voronoi-parent-fill,
.voronoi-container.animating .voronoi-child-fill {
  animation: cellFadeIn 0.35s ease-out;
}

@keyframes cellFadeIn {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

/* ---------------------------------------------------------------------------
   Labels
   --------------------------------------------------------------------------- */
.voronoi-label {
  font-family: var(--font-sans);
  text-anchor: middle;
  pointer-events: none;
  dominant-baseline: middle;
  letter-spacing: -0.2px;
}

.voronoi-parent-label {
  font-weight: 600;
  fill: rgba(22, 28, 58, 0.90);
  paint-order: stroke fill;
  stroke: rgba(255, 255, 255, 0.55);
  stroke-width: 2.5px;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.voronoi-child-label {
  font-weight: 500;
  fill: rgba(22, 28, 58, 0.82);
  paint-order: stroke fill;
  stroke: rgba(255, 255, 255, 0.50);
  stroke-width: 2px;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.voronoi-size-label {
  font-weight: 400;
  fill: rgba(22, 28, 58, 0.58);
  paint-order: stroke fill;
  stroke: rgba(255, 255, 255, 0.40);
  stroke-width: 1.5px;
  stroke-linecap: round;
  stroke-linejoin: round;
  font-variant-numeric: tabular-nums;
}

.voronoi-parent-size {
  fill: rgba(22, 28, 58, 0.65);
}

/* ---------------------------------------------------------------------------
   Navigation — back button + location label
   --------------------------------------------------------------------------- */
.voronoi-nav {
  position: absolute;
  top: 12px;
  left: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px 4px 4px;
  background: var(--glass);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  font-size: 12px;
  z-index: 10;
  transition: filter 0.15s;
}

/* In expanded mode, push back button below the header + viz switcher
   so it doesn't overlap the title/tabs area at the top.
   The content area has: 48px drag strip + ~55px header + ~42px switcher row
   + margins ≈ 180px from the viewport top. The voronoi container starts
   at top:0 so we use the same offset. */
.voronoi-container.expanded .voronoi-nav {
  top: 180px;
  left: 52px;
}

.voronoi-back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  transition: background 0.12s, transform 0.12s;
}

.voronoi-back-btn:hover:not(.disabled) {
  background: rgba(0, 0, 0, 0.06);
}

.voronoi-back-btn:active:not(.disabled) {
  transform: scale(0.92);
}

.voronoi-back-btn.disabled {
  color: var(--muted);
  opacity: 0.35;
  cursor: default;
}

.voronoi-location {
  font-weight: 550;
  color: var(--text);
  letter-spacing: -0.2px;
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ---------------------------------------------------------------------------
   Cursor-following tooltip
   --------------------------------------------------------------------------- */
.voronoi-tooltip {
  position: absolute;
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 6px 12px;
  background: var(--glass-strong);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  font-size: 11px;
  pointer-events: none;
  z-index: 20;
  box-shadow: var(--shadow-sm);
  white-space: nowrap;
  transform: translateY(-100%);
}

.tooltip-name {
  font-weight: 600;
  color: var(--text);
}

.tooltip-size {
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.tooltip-pct {
  color: var(--muted);
  font-size: 10px;
}

/* ---------------------------------------------------------------------------
   Expand button
   --------------------------------------------------------------------------- */
.voronoi-expand-btn {
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

.voronoi-expand-btn:hover {
  background: var(--glass-strong);
  color: var(--text);
}

.voronoi-compact-fab {
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

.voronoi-compact-fab:hover {
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.12);
}

.voronoi-container.expanded .voronoi-expand-btn {
  display: none;
}

/* ---------------------------------------------------------------------------
   Empty state
   --------------------------------------------------------------------------- */
.voronoi-empty {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
}

.voronoi-empty-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.voronoi-empty-sub {
  font-size: 13px;
  color: var(--muted);
}

/* ---------------------------------------------------------------------------
   Hint
   --------------------------------------------------------------------------- */
.voronoi-hint {
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

/* ---------------------------------------------------------------------------
   Mode toggle: Consolidated / Clusters
   --------------------------------------------------------------------------- */
.voronoi-mode-toggle {
  position: absolute;
  top: 12px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 2px;
  background: var(--glass);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  padding: 3px;
  z-index: 10;
}

.voronoi-mode-btn {
  padding: 4px 14px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, box-shadow 0.15s;
}

.voronoi-mode-btn.active {
  background: var(--glass-strong);
  color: var(--text);
  box-shadow: var(--shadow-sm);
}

.voronoi-mode-btn:hover:not(.active) {
  color: var(--text);
}

/* ---------------------------------------------------------------------------
   Cluster labels (below each circle)
   --------------------------------------------------------------------------- */
.cluster-label-name {
  font-family: var(--font-sans);
  font-size: 13px;
  font-weight: 650;
  fill: var(--text);
  letter-spacing: -0.2px;
}

.cluster-label-size {
  font-family: var(--font-sans);
  font-size: 11px;
  font-weight: 400;
  fill: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

/* Cluster circles are clickable */
.cluster-group {
  cursor: pointer;
}

.cluster-group:hover .voronoi-outer-ring {
  stroke: var(--accent);
  stroke-width: 3;
  transition: stroke 0.15s, stroke-width 0.15s;
}
</style>
