<script setup lang="ts">
/**
 * SpaceMap — D3 Zoomable Sunburst disk visualization.
 *
 * Based on the canonical D3 zoomable sunburst by Mike Bostock.
 * Key mechanics:
 *   - partition() fills [0, 2*PI] x [0, radius] — no hollow center
 *   - The root node's arc fills the entire inner circle (solid disc)
 *   - Click any arc → zoom in (that arc becomes the inner disc)
 *   - Click the center disc → zoom out to parent
 *   - arcVisible() + labelVisible() control what renders after transition
 */
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import * as d3 from "d3";
import { formatSize } from "../utils";
import {
  diskMapResult,
  diskMapLoading,
  diskMapLoaded,
  diskMapError,
  loadDiskMap,
  hasFullDiskAccess,
  checkFullDiskAccess,
  diskMapCaches,
  diskMapActiveCacheId,
  listDiskMapCaches,
  loadDiskMapCache,
  deleteDiskMapCache,
  loadMostRecentCache,
  enrichDiskNodes,
} from "../stores/scanStore";
import type { DiskNode, CacheMetadata } from "../types";
import GalacticViz from "../components/GalacticViz.vue";
import VoronoiViz from "../components/VoronoiViz.vue";

// ---------------------------------------------------------------------------
// View switcher (Phase 4 shell — only Sunburst active)
// ---------------------------------------------------------------------------
type VizMode = "sunburst" | "galactic" | "voronoi";
const activeViz = ref<VizMode>("sunburst");

// ---------------------------------------------------------------------------
// Color mode toggle: "size" (category) vs "recency" (cold→warm)
// ---------------------------------------------------------------------------
type ColorMode = "size" | "recency";
const colorMode = ref<ColorMode>("size");
const enriching = ref(false);
const enriched = ref(false);

// ---------------------------------------------------------------------------
// Color system: per-subtree distinct hues (like the D3 reference)
// ---------------------------------------------------------------------------
// Each top-level directory gets a unique hue from a well-separated palette.
// Category colors for the legend only
const categoryFills: Record<string, string> = {
  developer: "#43a047",
  media: "#e91e63",
  documents: "#1e88e5",
  applications: "#8e24aa",
  system: "#5c6bc0",
  caches: "#fb8c00",
  docker: "#00acc1",
  other: "#78909c",
};


function getFill(d: d3.HierarchyRectangularNode<DiskNode>): string {
  if (d.data.name === "Free Space") return "rgba(255,255,255,0.06)";
  if (colorMode.value === "recency") {
    return recencyColor(d.data.modified);
  }
  // White with varying opacity — lets the pastel gradient background show through,
  // matching the Voronoi wing-cell aesthetic. Wider range = more alive/iridescent.
  const midAngle = (d.x0 + d.x1) / 2 * 2 * Math.PI;
  const opacity = 0.08 + 0.34 * (0.5 + 0.5 * Math.sin(midAngle * 2.3 + d.depth * 1.7));
  return `rgba(255,255,255,${opacity.toFixed(3)})`;
}

// Recency scale: blue (recent) → warm amber (old).
// Biased toward the "old" end so stale files stand out more.
function recencyColor(modified: number | null | undefined): string {
  if (modified == null) return "#b0bec5";
  const now = Date.now() / 1000;
  const ageDays = (now - modified) / 86400;
  // t=0 → just modified, t=1 → 1 year+ old
  const raw = Math.max(0, Math.min(1, ageDays / 365));
  // Apply a bias curve so things shift toward amber sooner.
  // pow(0.7) pushes midpoint toward old (t=0.5 input → ~0.62 output).
  const t = Math.pow(raw, 0.7);
  // Recent = cool blue, Old = warm amber
  const interp = d3.interpolateRgb("#3b82f6", "#e8960c");
  return interp(t);
}

// ---------------------------------------------------------------------------
// Expand / contract toggle — when expanded, chart fills the content panel
// ---------------------------------------------------------------------------
// Single shared expanded state — carries through when switching viz tabs
const vizExpanded = ref(false);
const expandedChartHeight = ref(520);

function measureExpandedHeight() {
  // In expanded mode the chart is the ONLY thing visible in the content panel.
  // The content panel is 100vh. We subtract a small top margin (drag strip)
  // so the chart fills nearly the entire panel.
  return Math.max(400, window.innerHeight - 16);
}

// ---------------------------------------------------------------------------
// Sunburst state
// ---------------------------------------------------------------------------
const svgRef = ref<SVGSVGElement | null>(null);
const sunburstContainerRef = ref<HTMLDivElement | null>(null);

// Persistent d3.zoom instance — lives outside renderSunburst() so it
// survives re-renders. The zoom handler dynamically selects the current <g>.
let sunburstZoom: d3.ZoomBehavior<SVGSVGElement, unknown> | null = null;
const sunburstHeight = computed(() => (vizExpanded.value ? expandedChartHeight.value : 520));

// Hover / center info
const hoverName = ref("");
const hoverSize = ref(0);
const hoverPct = ref(0);
const hoverPath = ref("");
const isHovering = ref(false);

// Tooltip
const tooltipText = ref("");
const tooltipX = ref(0);
const tooltipY = ref(0);
const tooltipVisible = ref(false);

// Track current zoom root for zoom-out
let zoomCurrent: d3.HierarchyRectangularNode<DiskNode> | null = null;

// Reactive zoom state for the dir list — tracks which node is "zoomed into"
const zoomNode = ref<DiskNode | null>(null);
const zoomChildren = computed<DiskNode[]>(() => {
  const node = zoomNode.value || diskMapResult.value?.root;
  return node?.children || [];
});
const zoomNodeSize = computed(() => zoomNode.value?.size || diskMapResult.value?.root.size || 0);

// Cache dropdown
const showCacheDropdown = ref(false);

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------
const diskUsedPct = computed(() => {
  if (!diskMapResult.value || diskMapResult.value.disk_total === 0) return 0;
  return (diskMapResult.value.disk_used / diskMapResult.value.disk_total) * 100;
});

const rootSize = computed(() => diskMapResult.value?.root.size ?? 0);

/** Center display info — shows hover target or current zoom root */
const centerName = computed(() => isHovering.value ? hoverName.value : (zoomCurrent?.data.name ?? "~"));
const centerSize = computed(() => isHovering.value ? hoverSize.value : (zoomCurrent?.data.size ?? rootSize.value));
const centerPctDisplay = computed(() => {
  if (isHovering.value) return hoverPct.value;
  const total = diskMapResult.value?.disk_total ?? 1;
  return total > 0 ? ((zoomCurrent?.data.size ?? rootSize.value) / total) * 100 : 0;
});
// (centerPathDisplay removed — Reveal button moved to dir list)

function formatAge(seconds: number): string {
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)} min ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} hours ago`;
  return `${Math.floor(seconds / 86400)} days ago`;
}

const cacheBadgeLabel = computed(() => {
  if (!diskMapActiveCacheId.value) return "";
  const cache = diskMapCaches.value.find(
    (c) => c.id === diskMapActiveCacheId.value
  );
  if (!cache) return "Cached";
  return `Cached · ${formatAge(cache.age_seconds)}`;
});

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------
async function openFdaSettings() {
  try { await invoke("open_full_disk_access_settings"); } catch (_) {}
}

async function recheckFda() {
  await checkFullDiskAccess();
}

async function scan() {
  enriched.value = false;
  colorMode.value = "size";
  await loadDiskMap();
}

async function revealInFinder(path: string) {
  if (!path) return;
  try { await invoke("reveal_in_finder", { path }); } catch (_) {}
}

async function toggleRecency() {
  if (colorMode.value === "recency") {
    colorMode.value = "size";
    renderSunburst();
    return;
  }
  colorMode.value = "recency";
  if (!enriched.value) {
    enriching.value = true;
    await enrichDiskNodes();
    enriched.value = true;
    enriching.value = false;
  }
  renderSunburst();
}

async function onCacheSelect(cache: CacheMetadata) {
  showCacheDropdown.value = false;
  enriched.value = false;
  colorMode.value = "size";
  await loadDiskMapCache(cache.id);
}

async function onCacheDelete(cache: CacheMetadata) {
  await deleteDiskMapCache(cache.id);
}

// ---------------------------------------------------------------------------
// D3 Zoomable Sunburst — canonical pattern (Mike Bostock reference)
// ---------------------------------------------------------------------------

/**
 * Recursively group children that are too small to render meaningfully.
 * Two criteria — whichever is more aggressive wins:
 *   1. Size fraction: children below `minFraction` of the total disk are grouped.
 *   2. Count cap: after filtering, keep at most `maxVisible` individual segments.
 * Grouped children are preserved as sub-children of the rollup node so
 * drill-down still reveals them — nothing is discarded.
 */
function groupSmallChildren(node: DiskNode, totalDisk: number, maxVisible = 6, minFraction = 0.005): DiskNode {
  if (node.children.length === 0) return node;

  const minSize = totalDisk * minFraction;

  const processed = node.children.map(c => groupSmallChildren(c, totalDisk, maxVisible, minFraction));

  const visible = processed.filter(c => c.size >= minSize);
  const tooSmall = processed.filter(c => c.size < minSize);

  const kept = visible.slice(0, maxVisible);
  const overflow = visible.slice(maxVisible);

  const rollup = [...tooSmall, ...overflow];
  if (rollup.length === 0) return { ...node, children: kept };

  const rollupSize = rollup.reduce((sum, c) => sum + c.size, 0);
  kept.push({
    name: `${rollup.length} more`,
    path: "",
    size: rollupSize,
    children: rollup,
    expanded: true,
    category: "other",
    modified: null,
  });

  return { ...node, children: kept };
}

function renderSunburst() {
  if (!diskMapResult.value || !svgRef.value) return;

  // Inject a "Free Space" leaf so unused disk space shows as a segment.
  const raw = diskMapResult.value.root;
  const freeBytes = diskMapResult.value.disk_free;
  const dataWithFree: DiskNode = freeBytes > 0
    ? {
        ...raw,
        children: [
          ...raw.children,
          {
            name: "Free Space",
            path: "",
            size: freeBytes,
            children: [],
            expanded: false,
            category: "other",
            modified: null,
          }
        ]
      }
    : raw;

  const totalDisk = diskMapResult.value.disk_total;
  const data = groupSmallChildren(dataWithFree, totalDisk);
  const el = svgRef.value;
  const width = el.clientWidth || 600;
  const height = sunburstHeight.value;
  const radius = Math.min(width, height) / 2;

  // Build hierarchy — sum leaf sizes, sort largest first
  const root = d3
    .hierarchy<DiskNode>(data)
    .sum((d) => (d.children.length === 0 ? d.size : 0))
    .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));

  // Partition: full circle [0, 2*PI], full radius [0, radius]
  d3.partition<DiskNode>().size([2 * Math.PI, radius]).padding(0)(root);

  const partitioned = root as d3.HierarchyRectangularNode<DiskNode>;
  zoomCurrent = partitioned;
  zoomNode.value = partitioned.data;

  // Scales for zoom animation — retargeted on click.
  // Initial domain = full partition range (identity mapping).
  // On zoom, the domain shrinks to the clicked node's extent.
  const xScale = d3.scaleLinear().domain([0, 2 * Math.PI]).range([0, 2 * Math.PI]);
  const yScale = d3.scaleLinear().domain([0, radius]).range([0, radius]);

  // Arc generator — reads through scales so transitions "just work"
  const arc = d3
    .arc<d3.HierarchyRectangularNode<DiskNode>>()
    .startAngle((d) => Math.max(0, Math.min(2 * Math.PI, xScale(d.x0))))
    .endAngle((d) => Math.max(0, Math.min(2 * Math.PI, xScale(d.x1))))
    .padAngle((d) => {
      const angle = Math.abs(xScale(d.x1) - xScale(d.x0));
      // Tiny slivers get no padding (avoids barcode effect)
      if (angle < 0.01) return 0;
      // Wider gaps between arcs for organic breathing room (like Voronoi veins)
      return Math.min(angle / 2, 0.012);
    })
    .padRadius(radius / 2)
    .innerRadius((d) => yScale(d.y0) + 1)  // 1px inner gap for radial separation
    .outerRadius((d) => Math.max(yScale(d.y0), yScale(d.y1) - 2));

  // Visibility: is this arc visible in the current zoom window?
  function arcVisible(d: d3.HierarchyRectangularNode<DiskNode>): boolean {
    const a0 = xScale(d.x0);
    const a1 = xScale(d.x1);
    // Must have some angular extent and be within visible radius
    return a1 > 0.001 && a0 < 2 * Math.PI - 0.001 && yScale(d.y0) < radius;
  }

  // Label visibility: strict. A label shows only if the node is a proper
  // descendant of the zoom target AND its arc has enough visual space.
  function labelVisible(d: d3.HierarchyRectangularNode<DiskNode>): boolean {
    // Must be strictly deeper than the zoom target
    if (zoomCurrent && d.depth <= zoomCurrent.depth) return false;
    // Must actually be within the zoom target's angular range (descendant)
    if (zoomCurrent && (d.x0 < zoomCurrent.x0 || d.x1 > zoomCurrent.x1)) return false;
    const angle = xScale(d.x1) - xScale(d.x0);
    const radialThickness = yScale(d.y1) - yScale(d.y0);
    // Needs real angular extent (>0.18 rad ~10°) and radial room (>20px)
    return angle > 0.18 && radialThickness > 20;
  }

  // Clear previous render
  const svg = d3.select(el);
  svg.selectAll("*").remove();
  svg.attr("viewBox", `${-width / 2} ${-height / 2} ${width} ${height}`);

  // Clip everything to the circle — prevents deep/thin arcs from drawing
  // hairline strokes outside the chart boundary.
  svg.append("defs")
    .append("clipPath")
    .attr("id", "sunburst-clip")
    .append("circle")
    .attr("r", radius);

  const g = svg.append("g").attr("clip-path", "url(#sunburst-clip)");

  // ---- d3.zoom: wheel-to-zoom + drag-to-pan on the SVG ----
  // Created once, persists across re-renders. The zoom handler
  // dynamically queries the current <g> child so it always works
  // even after renderSunburst() clears and re-creates children.
  if (!sunburstZoom) {
    sunburstZoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.3, 8])
      .filter((event: any) => {
        if (event.type === 'wheel') return vizExpanded.value;
        if (event.type === 'dblclick') return false;
        if (event.type === 'mousedown') return vizExpanded.value;
        return vizExpanded.value;
      })
      .on("zoom", (event: any) => {
        // Always select the live <g> element, not a stale closure
        if (!svgRef.value) return;
        const liveG = d3.select(svgRef.value).select("g");
        liveG.attr("transform", event.transform.toString());
        const centerEl = sunburstContainerRef.value?.querySelector('.sunburst-center') as HTMLElement | null;
        if (centerEl) {
          const t = event.transform;
          centerEl.style.transform = `translate(calc(-50% + ${t.x}px), calc(-50% + ${t.y}px)) scale(${t.k})`;
        }
      });
  }
  svg.call(sunburstZoom);
  // Reset transform to identity on each re-render (new content starts centered)
  svg.call(sunburstZoom.transform, d3.zoomIdentity);
  const centerEl = sunburstContainerRef.value?.querySelector('.sunburst-center') as HTMLElement | null;
  if (centerEl) centerEl.style.transform = 'translate(-50%, -50%)';

  // All descendant nodes (skip root — it's the center disc)
  const descendants = partitioned.descendants().filter((d) => d.depth > 0);

  // ---- Center disc (root node): transparent with subtle border ----
  g.append("circle")
    .datum(partitioned)
    .attr("r", () => yScale(partitioned.y1))
    .attr("fill", "rgba(255, 255, 255, 0.18)")
    .attr("stroke", "rgba(12, 16, 42, 0.70)")
    .attr("stroke-width", 0.8)
    .attr("cursor", "pointer")
    .attr("pointer-events", "auto")
    .on("click", () => {
      if (zoomCurrent && zoomCurrent.parent) {
        clicked(zoomCurrent);
      }
    });

  // ---- Arcs for all child nodes ----
  const path = g
    .selectAll<SVGPathElement, d3.HierarchyRectangularNode<DiskNode>>("path")
    .data(descendants)
    .join("path")
    .attr("d", (d) => arc(d) || "")
    .attr("fill", (d) => getFill(d))
    .attr("fill-opacity", (d) => arcVisible(d) ? 1 : 0)
    .attr("stroke", (d) => arcVisible(d) ? "rgba(12, 16, 42, 0.82)" : "none")
    .attr("stroke-width", (d) => {
      if (!arcVisible(d)) return 0;
      const angle = xScale(d.x1) - xScale(d.x0);
      if (angle < 0.02) return 0;
      return d.depth <= 1 ? 0.8 : 0.5;
    })
    .attr("cursor", "pointer")
    .attr("pointer-events", (d) => {
      if (d === zoomCurrent) return "none";
      return arcVisible(d) ? "auto" : "none";
    })
    .on("click", (_event, d) => clicked(d))
    .on("mouseenter", (event, d) => {
      // Suppress hover/tooltip for the current zoom target (it IS the center disc)
      if (d === zoomCurrent) return;
      // Suppress for nodes outside the zoom window (invisible arcs)
      if (!arcVisible(d)) return;

      isHovering.value = true;
      hoverName.value = d.data.name;
      hoverSize.value = d.data.size;
      hoverPath.value = d.data.path;
      const total = diskMapResult.value?.disk_total ?? 1;
      hoverPct.value = total > 0 ? (d.data.size / total) * 100 : 0;

      const svgRect = el.getBoundingClientRect();
      tooltipText.value = `${d.data.name}  ${formatSize(d.data.size)}`;
      tooltipX.value = event.clientX - svgRect.left;
      tooltipY.value = event.clientY - svgRect.top - 28;
      tooltipVisible.value = true;
    })
    .on("mousemove", (event) => {
      if (!tooltipVisible.value) return;
      const svgRect = el.getBoundingClientRect();
      tooltipX.value = event.clientX - svgRect.left;
      tooltipY.value = event.clientY - svgRect.top - 28;
    })
    .on("mouseleave", () => {
      isHovering.value = false;
      tooltipVisible.value = false;
    });

  // ---- Text labels (only on large, prominent arcs) ----
  const label = g
    .selectAll<SVGTextElement, d3.HierarchyRectangularNode<DiskNode>>("text")
    .data(descendants)
    .join("text")
    .attr("pointer-events", "none")
    .attr("text-anchor", "middle")
    .attr("dy", "0.32em")
    .attr("font-size", "10px")
    .attr("font-weight", "500")
    .attr("fill", "rgba(22, 28, 58, 0.85)")
    .attr("stroke", "rgba(255,255,255,0.50)")
    .attr("stroke-width", "1.5px")
    .attr("paint-order", "stroke fill")
    .attr("fill-opacity", (d) => labelVisible(d) ? 1 : 0)
    .attr("stroke-opacity", (d) => labelVisible(d) ? 1 : 0)
    .attr("transform", (d) => labelTransform(d))
    .text((d) => {
      // Fit name to available radial space
      const radial = yScale(d.y1) - yScale(d.y0);
      const maxChars = Math.max(3, Math.floor(radial / 6.5));
      const name = d.data.name;
      return name.length > maxChars ? name.slice(0, maxChars - 1) + ".." : name;
    });

  // ---- Zoom handler (click any arc, or click center to zoom out) ----
  function clicked(p: d3.HierarchyRectangularNode<DiskNode>) {
    // Click current root → zoom out to parent
    const target = p === zoomCurrent && p.parent ? p.parent : p;
    zoomCurrent = target;
    zoomNode.value = target.data;

    // Retarget scales
    xScale.domain([target.x0, target.x1]);
    yScale.domain([target.y0, radius]);

    const t = g.transition().duration(650).ease(d3.easeCubicInOut);

    // Update center disc radius
    g.select("circle")
      .transition(t as any)
      .attr("r", yScale(target.y1));

    // Transition arcs
    path
      .transition(t as any)
      .attrTween("d", (d: any) => () => arc(d) || "")
      .attr("fill-opacity", (d: any) => arcVisible(d) ? 1 : 0)
      .attr("stroke", (d: any) => arcVisible(d) ? "rgba(12, 16, 42, 0.82)" : "none")
      .attr("stroke-width", (d: any) => {
        if (!arcVisible(d)) return 0;
        const angle = xScale(d.x1) - xScale(d.x0);
        if (angle < 0.02) return 0;
        const relDepth = d.depth - target.depth;
        return relDepth <= 1 ? 0.8 : 0.5;
      })
      .attr("pointer-events", (d: any) => {
        // Disable pointer events on the zoom target's arc — let clicks
        // fall through to the center disc circle element underneath
        if (d === zoomCurrent) return "none";
        return arcVisible(d) ? "auto" : "none";
      });

    // Transition labels
    label
      .transition(t as any)
      .attr("fill-opacity", (d: any) => labelVisible(d) ? 1 : 0)
      .attr("stroke-opacity", (d: any) => labelVisible(d) ? 1 : 0)
      .attrTween("transform", (d: any) => () => labelTransform(d));

    // Reset zoom/pan when drilling so the user starts centered
    if (sunburstZoom) {
      svg.transition().duration(650).call(sunburstZoom.transform, d3.zoomIdentity);
      const centerEl = sunburstContainerRef.value?.querySelector('.sunburst-center') as HTMLElement | null;
      if (centerEl) centerEl.style.transform = 'translate(-50%, -50%)';
    }
  }

  function labelTransform(d: d3.HierarchyRectangularNode<DiskNode>): string {
    const midAngle = (xScale(d.x0) + xScale(d.x1)) / 2;
    const midRadius = (yScale(d.y0) + yScale(d.y1)) / 2;
    const angleDeg = midAngle * (180 / Math.PI) - 90;
    const flip = angleDeg > 90 && angleDeg < 270;
    return `rotate(${angleDeg}) translate(${midRadius},0) rotate(${flip ? 180 : 0})`;
  }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
watch(
  () => diskMapResult.value,
  () => {
    if (diskMapResult.value) {
      nextTick(() => renderSunburst());
    }
  }
);

// Re-render sunburst when switching back from galactic/voronoi.
// The sunburst template uses v-if, so the SVG is destroyed when switching away
// and re-created when switching back. We need a nextTick so the new DOM element
// exists before we try to draw into it.
let sunburstResizeObserver: ResizeObserver | null = null;
watch(activeViz, (viz) => {
  // Reset zoom instance when switching away — the SVG is destroyed by v-if,
  // so the zoom needs to be re-created for the new SVG element.
  if (viz !== 'sunburst') sunburstZoom = null;

  if (viz === 'sunburst' && diskMapResult.value) {
    nextTick(() => {
      // Re-render sunburst with correct dimensions (may be expanded)
      if (vizExpanded.value) {
        expandedChartHeight.value = measureExpandedHeight();
      }
      nextTick(() => {
        renderSunburst();
        // Re-attach ResizeObserver to the new SVG parent element
        if (sunburstResizeObserver) sunburstResizeObserver.disconnect();
        if (svgRef.value?.parentElement) {
          sunburstResizeObserver = new ResizeObserver(() => {
            if (diskMapResult.value) renderSunburst();
          });
          sunburstResizeObserver.observe(svgRef.value.parentElement);
        }
      });
    });
  }
});

// Re-render when expand/contract changes (chart height changes).
// We use a double-nextTick so the DOM has updated the layout classes
// (which remove max-width / padding) BEFORE we measure the available space.
watch(vizExpanded, () => {
  // Lock/unlock the parent content panel scroll
  const contentEl = document.querySelector('.content') as HTMLElement | null;
  if (contentEl) {
    contentEl.style.overflow = vizExpanded.value ? 'hidden' : '';
  }

  // If sunburst is active, re-render at the new size
  if (activeViz.value === 'sunburst' && diskMapResult.value) {
    nextTick(() => {
      expandedChartHeight.value = measureExpandedHeight();
      nextTick(() => renderSunburst());
    });
  }
});

onMounted(() => {
  // Fire-and-forget — UI renders immediately, updates reactively when data arrives.
  (async () => {
    await listDiskMapCaches();
    if (!diskMapLoaded.value && !diskMapLoading.value) {
      const loaded = await loadMostRecentCache();
      if (!loaded) {
        await loadDiskMap();
      }
    }
  })();

  // Resize observer — use the shared variable so the activeViz watcher
  // can disconnect and re-attach when the sunburst DOM is recreated.
  if (svgRef.value) {
    sunburstResizeObserver = new ResizeObserver(() => {
      if (diskMapResult.value) renderSunburst();
    });
    sunburstResizeObserver.observe(svgRef.value.parentElement!);
  }
});

// Cleanup: ensure we never leave .content with overflow:hidden when
// navigating away from SpaceMap (e.g. clicking Memory in the sidebar
// while a viz is expanded).
onBeforeUnmount(() => {
  const contentEl = document.querySelector('.content') as HTMLElement | null;
  if (contentEl) contentEl.style.overflow = '';
  if (sunburstResizeObserver) sunburstResizeObserver.disconnect();
});
</script>

<template>
    <div class="spacemap-view" :class="{ 'viz-expanded': vizExpanded }">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Space Map</h2>
          <p class="text-muted">
            Visualize what's using your disk space
          </p>
        </div>
      </div>
    </div>

    <!-- View switcher row — in expanded mode, header-actions sit inline -->
    <div class="viz-switcher-row">
    <div class="viz-switcher">
      <button
        class="viz-btn"
        :class="{ active: activeViz === 'sunburst' }"
        @click="activeViz = 'sunburst'"
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <circle cx="7" cy="7" r="3" stroke="currentColor" stroke-width="1.5"/>
          <circle cx="7" cy="7" r="6" stroke="currentColor" stroke-width="1" stroke-dasharray="2 2"/>
        </svg>
        Sunburst
      </button>
      <button
        class="viz-btn"
        :class="{ active: activeViz === 'galactic' }"
        @click="activeViz = 'galactic'"
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <circle cx="7" cy="7" r="2" fill="currentColor"/>
          <circle cx="3" cy="4" r="1" fill="currentColor" opacity="0.5"/>
          <circle cx="11" cy="5" r="1.5" fill="currentColor" opacity="0.4"/>
          <circle cx="5" cy="11" r="1" fill="currentColor" opacity="0.3"/>
          <circle cx="10" cy="10" r="0.8" fill="currentColor" opacity="0.35"/>
        </svg>
        Galactic
      </button>
      <button
        class="viz-btn"
        :class="{ active: activeViz === 'voronoi' }"
        @click="activeViz = 'voronoi'"
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <polygon points="7,1 12,4 12,10 7,13 2,10 2,4" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <line x1="7" y1="1" x2="7" y2="13" stroke="currentColor" stroke-width="0.8" opacity="0.4"/>
          <line x1="2" y1="4" x2="12" y2="10" stroke="currentColor" stroke-width="0.8" opacity="0.4"/>
        </svg>
        Voronoi
      </button>
    </div>
      <div class="header-actions">
        <!-- Cache badge -->
        <div v-if="cacheBadgeLabel" class="cache-badge-wrapper">
          <button
            class="cache-badge"
            @click="showCacheDropdown = !showCacheDropdown"
          >
            {{ cacheBadgeLabel }}
            <svg width="10" height="6" viewBox="0 0 10 6" fill="none" style="margin-left: 4px">
              <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <div v-if="showCacheDropdown" class="cache-dropdown">
            <div class="cache-dropdown-header">Saved Scans</div>
            <div
              v-for="cache in diskMapCaches"
              :key="cache.id"
              class="cache-dropdown-item"
              :class="{ active: cache.id === diskMapActiveCacheId }"
              @click="onCacheSelect(cache)"
            >
              <div class="cache-item-info">
                <span class="cache-item-time">{{ cache.timestamp }}</span>
                <span class="cache-item-age text-muted">{{ formatAge(cache.age_seconds) }}</span>
              </div>
              <button
                class="cache-item-delete"
                @click.stop="onCacheDelete(cache)"
                title="Delete this scan"
              >
                &times;
              </button>
            </div>
            <div v-if="diskMapCaches.length === 0" class="cache-dropdown-empty">
              No saved scans yet
            </div>
          </div>
        </div>
        <button
          class="btn-primary scan-btn"
          :disabled="diskMapLoading"
          @click="scan"
        >
          <span v-if="diskMapLoading" class="spinner spinner-sm"></span>
          {{ diskMapLoading ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <!-- FDA warning -->
    <div v-if="hasFullDiskAccess === false" class="fda-warning-banner">
      <span class="fda-warning-dot"></span>
      <div class="fda-warning-body">
        <div class="fda-warning-title">
          Limited view -- Full Disk Access shows all directories
        </div>
        <div class="fda-warning-text">
          Without Full Disk Access, Desktop, Documents, Downloads, and media
          folders are not included. Sizes shown may not account for all disk usage.
        </div>
        <div class="fda-warning-actions">
          <button class="btn-fda btn-fda-primary" @click="openFdaSettings">
            Open System Settings
          </button>
          <button class="btn-fda btn-fda-secondary" @click="recheckFda">
            Re-check
          </button>
        </div>
      </div>
    </div>

    <!-- Messages -->
    <div v-if="diskMapError" class="error-message">{{ diskMapError }}</div>

    <!-- Loading -->
    <div v-if="diskMapLoading" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning disk usage... this may take a moment</span>
    </div>

    <!-- Results -->
    <template v-else-if="diskMapResult">
      <!-- Disk usage bar (shared across all viz modes) -->
      <div class="disk-bar-container">
        <div class="disk-bar">
          <div
            class="disk-bar-fill"
            :style="{ width: diskUsedPct + '%' }"
            :class="{ 'disk-bar-warning': diskUsedPct > 80, 'disk-bar-danger': diskUsedPct > 90 }"
          ></div>
        </div>
        <div class="disk-bar-labels">
          <span>{{ formatSize(diskMapResult.disk_used) }} used</span>
          <span>{{ formatSize(diskMapResult.disk_free) }} free</span>
          <span class="text-muted">{{ formatSize(diskMapResult.disk_total) }} total</span>
        </div>
      </div>

      <!-- ============================================================= -->
      <!-- SUNBURST VIEW                                                   -->
      <!-- ============================================================= -->
      <template v-if="activeViz === 'sunburst'">
        <!-- Color mode toggle -->
        <div class="color-toggle-row">
          <div class="color-toggle">
            <button
              class="color-toggle-btn"
              :class="{ active: colorMode === 'size' }"
              @click="colorMode = 'size'; renderSunburst()"
            >
              Size
            </button>
            <button
              class="color-toggle-btn"
              :class="{ active: colorMode === 'recency' }"
              @click="toggleRecency()"
            >
              <span v-if="enriching" class="spinner spinner-xs"></span>
              Recency
            </button>
          </div>
          <span class="color-hint text-muted">Click any arc to zoom in. Click center to zoom out.</span>
          <div class="expand-toggle-group">
            <button
              class="expand-toggle-btn"
              :class="{ active: vizExpanded }"
              @click="vizExpanded = !vizExpanded"
            >
              <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
                <template v-if="!vizExpanded">
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
              {{ vizExpanded ? 'Compact' : 'Expand' }}
            </button>
          </div>
        </div>

        <!-- Sunburst visualization -->
        <div class="sunburst-container" ref="sunburstContainerRef">
          <svg
            ref="svgRef"
            class="sunburst-svg"
            :style="{ width: '100%', height: sunburstHeight + 'px' }"
          ></svg>

          <!-- Center info overlay (over the root disc) -->
          <div class="sunburst-center">
            <div class="center-name">{{ centerName }}</div>
            <div class="center-size">{{ formatSize(centerSize) }}</div>
            <div class="center-pct text-muted">{{ centerPctDisplay.toFixed(1) }}% of disk</div>
          </div>

          <!-- Floating compact button (only visible in expanded mode) -->
          <button
            v-if="vizExpanded"
            class="compact-fab"
            @click="vizExpanded = false"
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

          <!-- Floating tooltip -->
          <div
            v-if="tooltipVisible"
            class="sunburst-tooltip"
            :style="{ left: tooltipX + 'px', top: tooltipY + 'px' }"
          >
            {{ tooltipText }}
          </div>
        </div>

        <!-- Legend -->
        <div class="legend" v-if="colorMode === 'size'">
          <div
            v-for="(color, cat) in categoryFills"
            :key="cat"
            class="legend-item"
          >
            <span class="legend-swatch" :style="{ background: color }"></span>
            <span class="legend-label">{{ cat }}</span>
          </div>
        </div>
        <div class="legend" v-else>
          <div class="recency-legend">
            <span class="recency-label">Recent</span>
            <div class="recency-gradient"></div>
            <span class="recency-label">Old (&gt; 1 year)</span>
            <span class="legend-item">
              <span class="legend-swatch" style="background: #90a4ae"></span>
              <span class="legend-label">Unknown</span>
            </span>
          </div>
        </div>

        <!-- Directory list (synced with sunburst zoom level) -->
        <div class="dir-list">
          <div
            v-for="child in zoomChildren"
            :key="child.path || child.name"
            class="dir-row"
          >
            <div class="dir-row-left">
              <span
                class="dir-swatch"
                :style="{ background: categoryFills[child.category] || categoryFills.other }"
              ></span>
              <div class="dir-info">
                <span class="dir-name">{{ child.name }}</span>
                <span v-if="child.path" class="dir-path mono truncate text-muted">
                  {{ child.path }}
                </span>
              </div>
            </div>
            <div class="dir-row-right">
              <button
                v-if="child.path"
                class="dir-reveal-btn"
                @click="revealInFinder(child.path)"
                title="Reveal in Finder"
              >
                <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
                  <path d="M4.5 1.5H2.5C1.95 1.5 1.5 1.95 1.5 2.5V9.5C1.5 10.05 1.95 10.5 2.5 10.5H9.5C10.05 10.5 10.5 10.05 10.5 9.5V7.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                  <path d="M7 1.5H10.5V5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                  <path d="M10.5 1.5L5.5 6.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                </svg>
              </button>
              <span class="dir-size">{{ formatSize(child.size) }}</span>
              <span class="dir-pct text-muted">
                {{
                  zoomNodeSize > 0
                    ? ((child.size / zoomNodeSize) * 100).toFixed(1)
                    : "0"
                }}%
              </span>
            </div>
          </div>
        </div>
      </template>

      <!-- ============================================================= -->
      <!-- GALACTIC VIEW                                                   -->
      <!-- ============================================================= -->
      <GalacticViz
        v-else-if="activeViz === 'galactic'"
        :data="diskMapResult"
        :expanded="vizExpanded"
        @update:expanded="vizExpanded = $event"
      />

      <!-- ============================================================= -->
      <!-- VORONOI VIEW                                                    -->
      <!-- ============================================================= -->
      <VoronoiViz
        v-else-if="activeViz === 'voronoi'"
        :data="diskMapResult"
        :expanded="vizExpanded"
        @update:expanded="vizExpanded = $event"
      />

    </template>
  </div>
</template>

<style scoped>
.spacemap-view {
  max-width: 740px;
}

/* ---- Expanded mode (shared across all viz types) ----
   Header + switcher stay visible at z-index 10 (above the viz canvas).
   Title stays left, cache badge + scan button pin to the right edge.
   Non-essential UI (disk bar, FDA warning, etc.) hidden. */
.spacemap-view.viz-expanded {
  position: relative;
  max-width: none;
}

.spacemap-view.viz-expanded .view-header {
  position: relative;
  z-index: 10;
}

.spacemap-view.viz-expanded .viz-switcher-row {
  position: relative;
  z-index: 10;
}

.spacemap-view.viz-expanded .viz-switcher {
  position: relative;
  z-index: 10;
}

/* Hide non-essential UI when expanded */
.spacemap-view.viz-expanded .fda-warning-banner,
.spacemap-view.viz-expanded .error-message,
.spacemap-view.viz-expanded .loading-state,
.spacemap-view.viz-expanded .disk-bar-container,
.spacemap-view.viz-expanded .color-toggle-row,
.spacemap-view.viz-expanded .legend,
.spacemap-view.viz-expanded .dir-list {
  display: none;
}

/* ---- Sunburst expanded: canvas behind header ---- */
.spacemap-view.viz-expanded .sunburst-container {
  position: fixed;
  top: 0;
  left: 230px;
  right: 0;
  bottom: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ---- View switcher row ---- */
.viz-switcher-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--sp-4);
}

.viz-switcher {
  display: flex;
  gap: 2px;
  background: var(--glass);
  border-radius: var(--radius-sm);
  padding: 3px;
  width: fit-content;
}

.viz-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, box-shadow 0.15s;
}

.viz-btn.active {
  background: var(--glass-strong);
  color: var(--text);
  box-shadow: var(--shadow-sm);
}

.viz-btn.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* ---- Header actions ---- */
.header-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

/* ---- Cache badge + dropdown ---- */
.cache-badge-wrapper {
  position: relative;
}

.cache-badge {
  display: flex;
  align-items: center;
  padding: 5px 10px;
  border: 1px solid var(--border-divider);
  border-radius: 8px;
  background: var(--glass);
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
  transition: background 0.15s;
  white-space: nowrap;
}

.cache-badge:hover {
  background: var(--glass-hover);
}

.cache-dropdown {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  min-width: 260px;
  background: var(--glass-strong);
  border: 1px solid var(--border-divider);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-md);
  z-index: 100;
  overflow: hidden;
}

.cache-dropdown-header {
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border-divider);
}

.cache-dropdown-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  cursor: pointer;
  transition: background 0.1s;
}

.cache-dropdown-item:hover {
  background: var(--surface-alt);
}

.cache-dropdown-item.active {
  background: var(--accent-light);
}

.cache-item-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.cache-item-time {
  font-size: 12px;
  color: var(--text);
  font-family: var(--font-mono);
}

.cache-item-age {
  font-size: 11px;
}

.cache-item-delete {
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 16px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  line-height: 1;
}

.cache-item-delete:hover {
  color: var(--danger);
  background: var(--danger-tint);
}

.cache-dropdown-empty {
  padding: 12px;
  text-align: center;
  font-size: 12px;
  color: var(--muted);
}

/* ---- Color toggle ---- */
.color-toggle-row {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  margin-bottom: var(--sp-3);
}

.color-hint {
  font-size: 11px;
  flex: 1;
}

/* ---- Expand / contract toggle ---- */
.expand-toggle-group {
  display: inline-flex;
  background: var(--glass);
  border-radius: 8px;
  padding: 2px;
  flex-shrink: 0;
}

.expand-toggle-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 4px 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, box-shadow 0.15s;
}

.expand-toggle-btn:hover {
  color: var(--text);
}

.expand-toggle-btn.active {
  background: var(--glass-strong);
  color: var(--text);
  box-shadow: var(--shadow-sm);
}

/* Floating compact button — fixed bottom-right of expanded chart */
.compact-fab {
  position: fixed;
  bottom: 24px;
  right: 24px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 18px;
  border: 1px solid rgba(0, 0, 0, 0.10);
  border-radius: 12px;
  background: var(--glass-strong);
  color: var(--text);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, box-shadow 0.15s;
  z-index: 1000;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.10);
}

.compact-fab:hover {
  background: rgba(255, 255, 255, 0.95);
  color: var(--text);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12);
}

.color-toggle {
  display: inline-flex;
  gap: 2px;
  background: var(--glass);
  border-radius: 8px;
  padding: 2px;
}

.color-toggle-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, box-shadow 0.15s;
}

.color-toggle-btn.active {
  background: var(--glass-strong);
  color: var(--text);
  box-shadow: var(--shadow-sm);
}

/* ---- Disk usage bar ---- */
.disk-bar-container {
  margin-bottom: var(--sp-4);
}

.disk-bar {
  height: 10px;
  background: var(--border);
  border-radius: 5px;
  overflow: hidden;
  margin-bottom: var(--sp-2);
}

.disk-bar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 5px;
  transition: width 0.5s ease;
}

.disk-bar-fill.disk-bar-warning {
  background: var(--warning);
}

.disk-bar-fill.disk-bar-danger {
  background: var(--danger);
}

.disk-bar-labels {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-secondary);
}

/* ---- Sunburst ---- */
.sunburst-container {
  position: relative;
  margin-bottom: var(--sp-4);
  overflow: hidden;
}

.sunburst-svg {
  display: block;
}

.sunburst-center {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  pointer-events: none;
  text-align: center;
  max-width: 120px;
}

.center-name {
  font-size: 13px;
  font-weight: 700;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 110px;
}

.center-size {
  font-size: 16px;
  font-weight: 700;
  color: var(--text);
}

.center-pct {
  font-size: 11px;
}

/* dir-reveal-btn in the directory list */
.dir-reveal-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  transition: color 0.12s, background 0.12s;
  flex-shrink: 0;
}

.dir-reveal-btn:hover {
  color: var(--accent);
  background: var(--accent-light);
}

/* ---- Tooltip ---- */
.sunburst-tooltip {
  position: absolute;
  padding: 4px 8px;
  background: rgba(30, 30, 40, 0.88);
  color: #fff;
  font-size: 11px;
  font-weight: 500;
  border-radius: 6px;
  pointer-events: none;
  white-space: nowrap;
  z-index: 50;
  transform: translateX(-50%);
  box-shadow: 0 2px 8px rgba(0,0,0,0.2);
}

/* ---- Legend ---- */
.legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-3);
  margin-bottom: var(--sp-4);
  padding: 0 var(--sp-1);
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 5px;
}

.legend-swatch {
  width: 10px;
  height: 10px;
  border-radius: 3px;
  flex-shrink: 0;
}

.legend-label {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: capitalize;
}

.recency-legend {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.recency-label {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.recency-gradient {
  flex: 1;
  max-width: 200px;
  height: 8px;
  border-radius: 4px;
  background: linear-gradient(to right, #3b82f6, #e8960c);
}

/* ---- Directory list ---- */
.dir-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--glass);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  overflow: hidden;
}

.dir-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-3) var(--sp-4);
  transition: background 0.1s;
  border-bottom: 1px solid var(--border-divider);
}

.dir-row:last-child {
  border-bottom: none;
}

.dir-row.clickable {
  cursor: pointer;
}

.dir-row.clickable:hover {
  background: var(--surface-alt);
}

.dir-row-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  min-width: 0;
  flex: 1;
}

.dir-swatch {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  flex-shrink: 0;
}

.dir-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.dir-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.dir-path {
  font-size: 11px;
  max-width: 400px;
}

.dir-row-right {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
  margin-left: var(--sp-4);
}

.dir-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  min-width: 70px;
  text-align: right;
}

.dir-pct {
  font-size: 12px;
  min-width: 40px;
  text-align: right;
}

/* ---- Spinner variants ---- */
.spinner-xs {
  width: 10px !important;
  height: 10px !important;
  border-width: 1.5px !important;
}
</style>
