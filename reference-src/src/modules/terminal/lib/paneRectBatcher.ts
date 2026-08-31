// Coalesces per-pane ResizeObserver ticks into at most one measurement +
// one state flush per animation frame. Without this, WorkspacePane's
// per-pane observers each force a synchronous layout read and a React state
// update on every single tick, which compounds across every open pane in
// every mounted (even inactive) workspace tab during a sidebar/split drag.
// Dependency-free (no DOM/xterm imports) so it stays unit testable without
// pulling in @testing-library/react, which this repo doesn't have — same
// rationale as PtyResizeQueue in resizeQueue.ts.

export type PaneRect = { x: number; y: number; w: number; h: number };

function rectsEqual(a: PaneRect, b: PaneRect): boolean {
  return a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h;
}

export class PaneRectBatcher {
  private readonly measure: (paneId: string) => PaneRect | null;
  private readonly onFlush: (updates: Map<string, PaneRect>) => void;
  private readonly requestFrame: (cb: () => void) => number;
  private readonly cancelFrame: (handle: number) => void;

  private dirty = new Set<string>();
  private lastRects = new Map<string, PaneRect>();
  private frameHandle: number | null = null;

  constructor(
    measure: (paneId: string) => PaneRect | null,
    onFlush: (updates: Map<string, PaneRect>) => void,
    requestFrame: (cb: () => void) => number = (cb) => requestAnimationFrame(cb),
    cancelFrame: (handle: number) => void = (handle) => cancelAnimationFrame(handle),
  ) {
    this.measure = measure;
    this.onFlush = onFlush;
    this.requestFrame = requestFrame;
    this.cancelFrame = cancelFrame;
  }

  schedule(paneId: string): void {
    this.dirty.add(paneId);
    if (this.frameHandle !== null) return;
    this.frameHandle = this.requestFrame(() => this.flush());
  }

  dispose(): void {
    if (this.frameHandle !== null) {
      this.cancelFrame(this.frameHandle);
      this.frameHandle = null;
    }
    this.dirty.clear();
  }

  private flush(): void {
    this.frameHandle = null;
    const paneIds = this.dirty;
    this.dirty = new Set();

    const updates = new Map<string, PaneRect>();
    for (const paneId of paneIds) {
      const rect = this.measure(paneId);
      if (!rect) continue;
      const last = this.lastRects.get(paneId);
      if (last && rectsEqual(last, rect)) continue;
      this.lastRects.set(paneId, rect);
      updates.set(paneId, rect);
    }

    if (updates.size > 0) this.onFlush(updates);
  }
}
