<script lang="ts">
  /**
   * A draggable resize handle for a sidebar.
   *
   * The parent (Sidebar / MetadataPanel) places this component on the
   * edge that should be draggable. The handle is invisible by default
   * (4px wide) and reveals a subtle highlight on hover. Dragging
   * updates the width state, which the parent applies to itself.
   *
   * Usage:
   *   <ResizeHandle
   *     side="left" | "right"
   *     width={$sidebarWidth}
   *     onResize={(w) => sidebarWidth.set(w)}
   *     min={240} max={480}
   *   />
   */
  let {
    side = "left",
    width,
    onResize,
    min = 220,
    max = 480,
  }: {
    /** Which edge this handle sits on. "left" = sidebar on the left,
     *  "right" = metadata panel on the right. Affects cursor & direction. */
    side?: "left" | "right";
    /** Current width in pixels (read by parent). */
    width: number;
    onResize: (newWidth: number) => void;
    min?: number;
    max?: number;
  } = $props();

  let dragging = $state(false);
  let startX = 0;
  let startWidth = 0;

  function onPointerDown(e: PointerEvent) {
    e.preventDefault();
    dragging = true;
    startX = e.clientX;
    startWidth = width;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    // For "right" panel, moving the cursor to the LEFT makes the panel
    // wider; for "left" panel, moving the cursor to the RIGHT makes the
    // panel wider.
    const delta = e.clientX - startX;
    const next =
      side === "right"
        ? startWidth - delta
        : startWidth + delta;
    const clamped = Math.max(min, Math.min(max, next));
    onResize(clamped);
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div
  role="separator"
  aria-orientation="vertical"
  aria-valuenow={Math.round(width)}
  aria-valuemin={min}
  aria-valuemax={max}
  class="resize-handle"
  class:resize-handle-right={side === "right"}
  class:resize-handle-active={dragging}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  title="Drag to resize"
></div>

<style>
  .resize-handle {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 5px;
    cursor: ew-resize;
    z-index: 10;
    user-select: none;
    touch-action: none;
  }

  .resize-handle-right {
    left: 0;
  }

  .resize-handle:not(.resize-handle-right) {
    right: 0;
  }

  .resize-handle::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 1px;
    background: transparent;
    transition: background 0.15s;
  }

  .resize-handle:hover::before,
  .resize-handle-active::before {
    background: var(--color-accent);
  }

  .resize-handle:hover,
  .resize-handle-active {
    /* subtle hit area */
    background: rgba(109, 40, 217, 0.04);
  }
</style>
