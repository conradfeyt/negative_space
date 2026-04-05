/**
 * Shared zoom/pan interaction composable.
 *
 * Handles the common drag-state-machine (mousedown → move → up with threshold
 * to distinguish clicks from drags) and wheel-zoom gating (expanded-only,
 * preventDefault, zoom-toward-cursor factor). The actual coordinate transforms
 * differ between SVG (VoronoiViz) and Canvas (GalacticViz), so they are
 * delegated to callbacks supplied by each consumer.
 */

export interface ZoomPanOptions {
  /** Minimum scale factor (default 0.3). */
  minScale?: number;
  /** Maximum scale factor (default 8). */
  maxScale?: number;
  /** Pixel movement threshold to distinguish a click from a drag (default 4). */
  dragThreshold?: number;
  /** Zoom-in multiplier per wheel tick (default 1.06). */
  zoomInFactor?: number;
  /** Zoom-out multiplier per wheel tick (default 0.94). */
  zoomOutFactor?: number;
}

export interface ZoomPanCallbacks {
  /**
   * Called when the user scrolls the wheel while expanded.
   * Receives the WheelEvent, the clamped new scale, and the zoom factor.
   * The component is responsible for applying the scale to its own
   * coordinate system.
   */
  onZoom: (e: WheelEvent, newScale: number, currentScale: number) => void;

  /**
   * Called on every mousemove once the drag threshold has been exceeded.
   * Receives the current MouseEvent and the pixel delta from drag start.
   */
  onPan: (e: MouseEvent, pixelDx: number, pixelDy: number) => void;

  /**
   * Called when a drag begins (threshold just exceeded).  Optional.
   */
  onDragStart?: () => void;

  /**
   * Called when a drag ends (mouseup after a real drag).  Optional.
   */
  onDragEnd?: () => void;
}

export interface ZoomPanState {
  /** Whether a drag is currently active (threshold exceeded). */
  readonly dragging: boolean;
  /** Whether the last mousedown–mouseup was a drag (not a click). */
  readonly didDrag: boolean;
  /** Current scale value. */
  scale: number;
}

export interface ZoomPanHandlers {
  onWheel: (e: WheelEvent, expanded: boolean) => void;
  onMouseDown: (e: MouseEvent, expanded: boolean) => void;
  onMouseMove: (e: MouseEvent) => void;
  onMouseUp: () => void;
  onMouseLeave: () => void;
  /** Reset scale to 1 and clear drag state. Returns the old scale. */
  resetZoom: () => number;
  /** Readable state — not reactive (mutated via plain lets for perf). */
  state: ZoomPanState;
}

/**
 * Create zoom/pan event handlers that delegate coordinate math to callbacks.
 *
 * Usage:
 * ```ts
 * const { onWheel, onMouseDown, onMouseMove, onMouseUp, state } = useZoomPan(
 *   { minScale: 0.3, maxScale: 8 },
 *   { onZoom, onPan, onDragStart, onDragEnd },
 * );
 * ```
 */
export function useZoomPan(
  options: ZoomPanOptions = {},
  callbacks: ZoomPanCallbacks,
): ZoomPanHandlers {
  const {
    minScale = 0.3,
    maxScale = 8,
    dragThreshold = 4,
    zoomInFactor = 1.06,
    zoomOutFactor = 0.94,
  } = options;

  // Internal mutable state — kept as plain lets for performance (no Vue reactivity).
  let scale = 1;
  let mouseIsDown = false; // true between mousedown and mouseup (before threshold)
  let isDragging = false;  // true once drag threshold exceeded
  let didDrag = false;     // sticky flag — true if drag happened in this gesture
  let dragStartClientX = 0;
  let dragStartClientY = 0;

  // Exposed state object — properties are updated in-place.
  const state: ZoomPanState = {
    get dragging() { return isDragging; },
    get didDrag() { return didDrag; },
    get scale() { return scale; },
    set scale(v: number) { scale = v; },
  };

  function onWheel(e: WheelEvent, expanded: boolean): void {
    if (!expanded) return;
    e.preventDefault();

    const factor = e.deltaY > 0 ? zoomOutFactor : zoomInFactor;
    const newScale = Math.max(minScale, Math.min(maxScale, scale * factor));
    const oldScale = scale;
    scale = newScale;
    callbacks.onZoom(e, newScale, oldScale);
  }

  function onMouseDown(e: MouseEvent, expanded: boolean): void {
    if (!expanded) return;
    isDragging = false; // not yet — threshold not exceeded
    didDrag = false;
    dragStartClientX = e.clientX;
    dragStartClientY = e.clientY;
    mouseIsDown = true;
  }

  function onMouseMove(e: MouseEvent): void {
    if (!mouseIsDown) return;

    const pixelDx = e.clientX - dragStartClientX;
    const pixelDy = e.clientY - dragStartClientY;

    if (!isDragging) {
      if (Math.abs(pixelDx) + Math.abs(pixelDy) > dragThreshold) {
        isDragging = true;
        didDrag = true;
        callbacks.onDragStart?.();
      } else {
        return; // below threshold — not a drag yet
      }
    }

    callbacks.onPan(e, pixelDx, pixelDy);
  }

  function onMouseUp(): void {
    if (isDragging) {
      callbacks.onDragEnd?.();
    }
    isDragging = false;
    mouseIsDown = false;
  }

  function onMouseLeave(): void {
    onMouseUp();
  }

  function resetZoom(): number {
    const old = scale;
    scale = 1;
    isDragging = false;
    didDrag = false;
    mouseIsDown = false;
    return old;
  }

  return { onWheel, onMouseDown, onMouseMove, onMouseUp, onMouseLeave, resetZoom, state };
}
