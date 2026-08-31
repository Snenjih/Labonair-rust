import { describe, expect, it } from "vitest";
import { type PendingConfirmation, resolveCloseAction } from "./closeConfirmation";

describe("resolveCloseAction", () => {
  it("disposes a workspace tab immediately when confirmCloseTerminalTab is off", () => {
    expect(resolveCloseAction({ kind: "workspace", title: "bash" }, false)).toEqual({ type: "dispose" });
  });

  it("queues a terminal confirmation for a workspace tab when the preference is on", () => {
    expect(resolveCloseAction({ kind: "workspace", title: "bash" }, true)).toEqual({
      type: "confirm",
      item: { kind: "terminal", title: "bash" },
    });
  });

  it("queues a save confirmation for an untitled editor tab", () => {
    expect(resolveCloseAction({ kind: "editor", title: "Untitled-1", isUntitled: true }, false)).toEqual({
      type: "confirm",
      item: { kind: "save", title: "Untitled-1" },
    });
  });

  it("queues a dirty confirmation for a dirty (non-untitled) editor tab", () => {
    expect(resolveCloseAction({ kind: "editor", title: "app.ts", dirty: true }, false)).toEqual({
      type: "confirm",
      item: { kind: "dirty", title: "app.ts" },
    });
  });

  it("disposes a clean editor tab immediately", () => {
    expect(resolveCloseAction({ kind: "editor", title: "app.ts" }, false)).toEqual({ type: "dispose" });
  });

  it("disposes non-workspace, non-editor tabs immediately", () => {
    expect(resolveCloseAction({ kind: "sftp", title: "host" }, true)).toEqual({ type: "dispose" });
  });
});

describe("bulk-close confirmation queue ordering", () => {
  // Regression test for the real bug: `handleCloseAll`/`handleCloseOthers`
  // call `handleClose` synchronously in a `.forEach` loop. Before the fix,
  // the three pending-tab slots were single values, so each iteration's
  // `setPendingXTab(...)` overwrote the previous one — only the LAST tab
  // needing confirmation ever got a dialog, and earlier ones were silently
  // left open, unwarned, un-disposed. This simulates that loop against a
  // functional-updater-based queue and asserts every tab needing
  // confirmation is queued, in order, and drains correctly one at a time.
  function simulateBulkClose(
    tabs: Array<{ id: number; kind: "workspace" | "editor" | "sftp"; title: string; dirty?: boolean }>,
    confirmCloseTerminalTab: boolean,
  ) {
    let queue: PendingConfirmation[] = [];
    const disposed: number[] = [];

    for (const t of tabs) {
      const action = resolveCloseAction(t, confirmCloseTerminalTab);
      if (action.type === "dispose") {
        disposed.push(t.id);
      } else {
        // Functional-updater semantics: each push sees the queue state left
        // by the previous iteration of this same synchronous loop.
        queue = [...queue, { ...action.item, id: t.id } as PendingConfirmation];
      }
    }
    return { queue, disposed };
  }

  it("queues every tab needing confirmation during a mixed bulk close, in iteration order", () => {
    const { queue, disposed } = simulateBulkClose(
      [
        { id: 1, kind: "editor", title: "a.ts", dirty: true },
        { id: 2, kind: "sftp", title: "host" }, // disposed immediately, no confirmation
        { id: 3, kind: "workspace", title: "bash" },
        { id: 4, kind: "editor", title: "b.ts", dirty: true },
      ],
      true,
    );

    expect(disposed).toEqual([2]);
    expect(queue).toEqual([
      { kind: "dirty", id: 1, title: "a.ts" },
      { kind: "terminal", id: 3, title: "bash" },
      { kind: "dirty", id: 4, title: "b.ts" },
    ]);
  });

  it("drains the queue one confirmation at a time as each is resolved", () => {
    let queue = simulateBulkClose(
      [
        { id: 1, kind: "editor", title: "a.ts", dirty: true },
        { id: 2, kind: "editor", title: "b.ts", dirty: true },
      ],
      false,
    ).queue;

    expect(queue).toHaveLength(2);
    expect(queue[0]).toEqual({ kind: "dirty", id: 1, title: "a.ts" });

    // Resolving (Confirm or Cancel) shifts the front of the queue, revealing
    // the next dialog — mirrors `CloseDialogs.tsx`'s `shift()`.
    queue = queue.slice(1);
    expect(queue).toHaveLength(1);
    expect(queue[0]).toEqual({ kind: "dirty", id: 2, title: "b.ts" });

    queue = queue.slice(1);
    expect(queue).toHaveLength(0);
  });
});
