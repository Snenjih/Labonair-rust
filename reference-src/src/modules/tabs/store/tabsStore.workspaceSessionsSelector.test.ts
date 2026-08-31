import { beforeEach, describe, expect, it } from "vitest";
import { selectSessionStableWorkspaceTabs, useTabsStore } from "./tabsStore";

describe("selectSessionStableWorkspaceTabs", () => {
  beforeEach(() => {
    useTabsStore.setState({ tabs: [], activeId: -1, _nextId: 1 });
  });

  it("returns the same tab reference across a cwd-only update", () => {
    const id = useTabsStore.getState().newTab();
    const tab = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id)!;
    const sessionId = Object.keys(tab.sessions)[0]!;

    const before = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id);
    useTabsStore.getState().updatePaneSessionCwd(id, sessionId, "/some/new/path");
    const after = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id);

    expect(after).toBe(before);
  });

  it("returns a new reference when splitPane adds a session", () => {
    const id = useTabsStore.getState().newTab();
    const before = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id);

    useTabsStore.getState().splitPane(id, "vertical");
    const after = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id);

    expect(after).not.toBe(before);
    expect(Object.keys(after!.sessions)).toHaveLength(2);
  });

  it("returns a new reference when closePane removes a session from a multi-pane tab", () => {
    const id = useTabsStore.getState().newTab();
    useTabsStore.getState().splitPane(id, "vertical");
    const splitTab = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id)!;
    const [firstPaneId] = Object.keys(splitTab.sessions);

    const before = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id);
    useTabsStore.getState().closePane(id, firstPaneId!);
    const after = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === id);

    expect(after).not.toBe(before);
    expect(Object.keys(after!.sessions)).toHaveLength(1);
  });

  it("omits a tab after closeTab removes it (and its cache entry) entirely", () => {
    const first = useTabsStore.getState().newTab();
    const second = useTabsStore.getState().newTab();

    useTabsStore.getState().closeTab(second);
    const remaining = selectSessionStableWorkspaceTabs(useTabsStore.getState());

    expect(remaining.map((t) => t.id)).toEqual([first]);
  });

  it("omits a tab after closePane closes its last remaining pane", () => {
    const first = useTabsStore.getState().newTab();
    const second = useTabsStore.getState().newTab();
    const secondTab = selectSessionStableWorkspaceTabs(useTabsStore.getState()).find((t) => t.id === second)!;

    useTabsStore.getState().closePane(second, secondTab.activePaneId);
    const remaining = selectSessionStableWorkspaceTabs(useTabsStore.getState());

    expect(remaining.map((t) => t.id)).toEqual([first]);
  });
});
