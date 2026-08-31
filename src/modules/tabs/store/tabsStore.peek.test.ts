import { beforeEach, describe, expect, it } from "vitest";
import type { EditorTab } from "../types";
import { useTabsStore } from "./tabsStore";

function editorTabs(): EditorTab[] {
  return useTabsStore.getState().tabs.filter((t): t is EditorTab => t.kind === "editor");
}

describe("peek editor tabs", () => {
  beforeEach(() => {
    useTabsStore.setState({ tabs: [], activeId: -1, _nextId: 1 });
  });

  it("openFileTab defaults to a peek tab", () => {
    useTabsStore.getState().newTab();
    const id = useTabsStore.getState().openFileTab("/tmp/a.txt");
    expect(editorTabs().find((t) => t.id === id)?.peek).toBe(true);
  });

  it("openFileTab(path, activate, false) opens a permanent (non-peek) tab", () => {
    useTabsStore.getState().newTab();
    const id = useTabsStore.getState().openFileTab("/tmp/a.txt", true, false);
    expect(editorTabs().find((t) => t.id === id)?.peek).toBe(false);
  });

  it("opening a second file while a peek tab is active recycles the same tab id", () => {
    useTabsStore.getState().newTab();
    const firstId = useTabsStore.getState().openFileTab("/tmp/a.txt");
    const secondId = useTabsStore.getState().openFileTab("/tmp/b.txt");

    expect(secondId).toBe(firstId);
    expect(editorTabs()).toHaveLength(1);
    expect(editorTabs()[0]?.path).toBe("/tmp/b.txt");
    expect(useTabsStore.getState().activeId).toBe(firstId);
  });

  it("does not recycle a dirty peek tab — opens a separate tab instead", () => {
    const firstId = useTabsStore.getState().openFileTab("/tmp/a.txt");
    useTabsStore.getState().updateTab(firstId!, { dirty: true });

    const secondId = useTabsStore.getState().openFileTab("/tmp/b.txt");

    expect(secondId).not.toBe(firstId);
    expect(editorTabs()).toHaveLength(2);
  });

  it("switching away from an untouched peek tab auto-closes it", () => {
    const termId = useTabsStore.getState().newTab();
    const peekId = useTabsStore.getState().openFileTab("/tmp/a.txt");
    expect(editorTabs()).toHaveLength(1);

    useTabsStore.getState().setActiveId(termId);

    expect(editorTabs()).toHaveLength(0);
    expect(useTabsStore.getState().tabs.some((t) => t.id === peekId)).toBe(false);
    expect(useTabsStore.getState().activeId).toBe(termId);
  });

  it("switching away from a dirty peek tab does not close it", () => {
    const termId = useTabsStore.getState().newTab();
    const peekId = useTabsStore.getState().openFileTab("/tmp/a.txt");
    useTabsStore.getState().updateTab(peekId!, { dirty: true });

    useTabsStore.getState().setActiveId(termId);

    expect(useTabsStore.getState().tabs.some((t) => t.id === peekId)).toBe(true);
  });

  it("editing a peek tab promotes it (dirty=true clears peek)", () => {
    const id = useTabsStore.getState().openFileTab("/tmp/a.txt");
    useTabsStore.getState().updateTab(id!, { dirty: true, peek: false });

    const tab = editorTabs().find((t) => t.id === id);
    expect(tab?.dirty).toBe(true);
    expect(tab?.peek).toBe(false);

    // Now switching away must not auto-close it.
    const termId = useTabsStore.getState().newTab();
    useTabsStore.getState().setActiveId(termId);
    expect(useTabsStore.getState().tabs.some((t) => t.id === id)).toBe(true);
  });

  it("switching directly to a new peek tab (bypassing setActiveId) still auto-closes the old one", () => {
    const peekId = useTabsStore.getState().openFileTab("/tmp/a.txt");
    // newTab() writes activeId directly, not via setActiveId — the
    // subscription must still catch this transition.
    const termId = useTabsStore.getState().newTab();

    expect(useTabsStore.getState().activeId).toBe(termId);
    expect(useTabsStore.getState().tabs.some((t) => t.id === peekId)).toBe(false);
  });

  it("switching to another already-open editor (permanent) tab still auto-closes the peek tab", () => {
    const permanentId = useTabsStore.getState().openFileTab("/tmp/permanent.txt", true, false);
    const peekId = useTabsStore.getState().openFileTab("/tmp/a.txt");

    useTabsStore.getState().setActiveId(permanentId!);

    expect(useTabsStore.getState().tabs.some((t) => t.id === peekId)).toBe(false);
    expect(useTabsStore.getState().tabs.some((t) => t.id === permanentId)).toBe(true);
    expect(useTabsStore.getState().activeId).toBe(permanentId);
  });
});
