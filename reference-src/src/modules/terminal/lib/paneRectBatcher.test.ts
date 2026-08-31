import { describe, expect, it, vi } from "vitest";
import { PaneRectBatcher, type PaneRect } from "./paneRectBatcher";

function makeFakeFrame() {
  let pending: (() => void) | null = null;
  let nextHandle = 1;
  const requestFrame = vi.fn((cb: () => void) => {
    pending = cb;
    return nextHandle++;
  });
  const cancelFrame = vi.fn(() => {
    pending = null;
  });
  const runFrame = () => {
    const cb = pending;
    pending = null;
    cb?.();
  };
  return { requestFrame, cancelFrame, runFrame };
}

describe("PaneRectBatcher", () => {
  it("measures a pane only once even if scheduled twice before a flush", () => {
    const { requestFrame, cancelFrame, runFrame } = makeFakeFrame();
    const measure = vi.fn((): PaneRect => ({ x: 0, y: 0, w: 100, h: 100 }));
    const onFlush = vi.fn();
    const batcher = new PaneRectBatcher(measure, onFlush, requestFrame, cancelFrame);

    batcher.schedule("a");
    batcher.schedule("a");
    runFrame();

    expect(measure).toHaveBeenCalledTimes(1);
    expect(measure).toHaveBeenCalledWith("a");
  });

  it("batches multiple panes scheduled before a flush into one onFlush call", () => {
    const { requestFrame, cancelFrame, runFrame } = makeFakeFrame();
    const rects: Record<string, PaneRect> = {
      a: { x: 0, y: 0, w: 10, h: 10 },
      b: { x: 10, y: 0, w: 10, h: 10 },
    };
    const measure = vi.fn((paneId: string) => rects[paneId] ?? null);
    const onFlush = vi.fn();
    const batcher = new PaneRectBatcher(measure, onFlush, requestFrame, cancelFrame);

    batcher.schedule("a");
    batcher.schedule("b");
    runFrame();

    expect(onFlush).toHaveBeenCalledTimes(1);
    const updates = onFlush.mock.calls[0][0] as Map<string, PaneRect>;
    expect(updates.get("a")).toEqual(rects.a);
    expect(updates.get("b")).toEqual(rects.b);
  });

  it("excludes a pane from the flush payload when its rect is unchanged", () => {
    const { requestFrame, cancelFrame, runFrame } = makeFakeFrame();
    const rect: PaneRect = { x: 0, y: 0, w: 10, h: 10 };
    const measure = vi.fn(() => rect);
    const onFlush = vi.fn();
    const batcher = new PaneRectBatcher(measure, onFlush, requestFrame, cancelFrame);

    batcher.schedule("a");
    runFrame();
    expect(onFlush).toHaveBeenCalledTimes(1);

    onFlush.mockClear();
    batcher.schedule("a");
    runFrame();

    expect(onFlush).not.toHaveBeenCalled();
  });

  it("dispose() cancels a pending scheduled flush so it never fires", () => {
    const { requestFrame, cancelFrame, runFrame } = makeFakeFrame();
    const measure = vi.fn((): PaneRect => ({ x: 0, y: 0, w: 10, h: 10 }));
    const onFlush = vi.fn();
    const batcher = new PaneRectBatcher(measure, onFlush, requestFrame, cancelFrame);

    batcher.schedule("a");
    batcher.dispose();
    expect(cancelFrame).toHaveBeenCalledTimes(1);

    runFrame();
    expect(onFlush).not.toHaveBeenCalled();
    expect(measure).not.toHaveBeenCalled();
  });

  it("only schedules one pending frame across multiple schedule() calls", () => {
    const { requestFrame, cancelFrame } = makeFakeFrame();
    const measure = vi.fn((): PaneRect => ({ x: 0, y: 0, w: 10, h: 10 }));
    const onFlush = vi.fn();
    const batcher = new PaneRectBatcher(measure, onFlush, requestFrame, cancelFrame);

    batcher.schedule("a");
    batcher.schedule("b");
    batcher.schedule("c");

    expect(requestFrame).toHaveBeenCalledTimes(1);
  });
});
