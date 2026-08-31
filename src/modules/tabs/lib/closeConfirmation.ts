import type { Tab } from "../types";

/** One tab awaiting a close-confirmation dialog. Kept as a FIFO queue (not a
 *  single value) so a bulk close (Close All / Close Others / Close All By
 *  Kind) confirms every tab that needs it, one dialog at a time, instead of
 *  a later tab's confirmation silently overwriting an earlier one's. */
export type PendingConfirmation =
  | { kind: "terminal"; id: number; title: string }
  | { kind: "save"; id: number; title: string }
  | { kind: "dirty"; id: number; title: string };

/** Pure decision logic for `handleClose`, kept in its own dependency-light
 *  module (not `useTabManagement.ts`, which transitively pulls in the whole
 *  terminal module) so bulk-close queueing behavior is unit-testable
 *  without heavy Zustand/editor-ref/xterm-addon mocking. Given a tab and the
 *  current `confirmCloseTerminalTab` preference, decides whether the tab
 *  can be disposed immediately or needs to be queued for confirmation first. */
export function resolveCloseAction(
  tab: Pick<Tab, "kind" | "title"> & { isUntitled?: boolean; dirty?: boolean; remoteSyncFailed?: boolean },
  confirmCloseTerminalTab: boolean,
): { type: "dispose" } | { type: "confirm"; item: Omit<PendingConfirmation, "id"> } {
  if (tab.kind === "workspace") {
    if (confirmCloseTerminalTab) return { type: "confirm", item: { kind: "terminal", title: tab.title } };
    return { type: "dispose" };
  }
  if (tab.kind === "editor" && tab.isUntitled) {
    return { type: "confirm", item: { kind: "save", title: tab.title } };
  }
  // A failed remote push means the local temp file's `dirty` flag has
  // already cleared even though the remote copy is still out of sync — a
  // tab in this state must be treated the same as a plain dirty tab so
  // closing it doesn't silently discard that fact.
  if (tab.kind === "editor" && (tab.dirty || tab.remoteSyncFailed)) {
    return { type: "confirm", item: { kind: "dirty", title: tab.title } };
  }
  return { type: "dispose" };
}
