import { describe, expect, it } from "vitest";
import type { EditorTab, HomeTab, SftpTab, Tab, WorkspaceTab } from "../types";
import { buildGroupedRenderPlan, groupNameFor, isBlockedCrossGroupDrag, pathKeyFor } from "./tabGrouping";

function makeWorkspaceTab(id: number, cwd: string | undefined): WorkspaceTab {
  return {
    id,
    kind: "workspace",
    title: "term",
    activePaneId: "pane-1",
    layout: { type: "pane", id: "pane-1" },
    sessions: { "pane-1": { id: "pane-1", kind: "local", title: "term", cwd } },
  };
}

function makeEditorTab(id: number, path: string): EditorTab {
  return { id, kind: "editor", title: "file", path, dirty: false };
}

function makeSftpTab(id: number, hostId: string, remotePath: string | undefined): SftpTab {
  return { id, kind: "sftp", title: "sftp", hostId, remotePath };
}

function makeHomeTab(id: number): HomeTab {
  return { id, kind: "home", title: "Home" };
}

describe("pathKeyFor", () => {
  it("reads the active pane's cwd for a workspace tab", () => {
    expect(pathKeyFor(makeWorkspaceTab(1, "/home/user/project"))).toBe("/home/user/project");
  });

  it("is undefined for a workspace tab with no cwd yet", () => {
    expect(pathKeyFor(makeWorkspaceTab(1, undefined))).toBeUndefined();
  });

  it("reads the containing folder for an editor tab, not the file itself", () => {
    expect(pathKeyFor(makeEditorTab(1, "/home/user/project/file.ts"))).toBe("/home/user/project");
  });

  it("reads the remote path for an sftp tab", () => {
    expect(pathKeyFor(makeSftpTab(1, "host-1", "/var/www"))).toBe("/var/www");
  });

  it("is undefined for a home tab", () => {
    expect(pathKeyFor(makeHomeTab(1))).toBeUndefined();
  });
});

describe("groupNameFor", () => {
  it("keeps the last two path segments", () => {
    expect(groupNameFor("/home/user/Developer/active/Labonair")).toBe("active/Labonair");
  });

  it("falls back to the single segment when there's only one", () => {
    expect(groupNameFor("/tmp")).toBe("tmp");
  });

  it("falls back to '/' for the root path", () => {
    expect(groupNameFor("/")).toBe("/");
  });
});

describe("buildGroupedRenderPlan", () => {
  it("clusters tabs sharing a path key under one header, at its first occurrence", () => {
    const tabs: Tab[] = [
      makeWorkspaceTab(1, "/repo"),
      makeEditorTab(2, "/other/file.ts"),
      makeSftpTab(3, "host-1", "/repo"),
    ];
    const pathKeys = new Map(tabs.map((t) => [t.id, pathKeyFor(t)]));

    const plan = buildGroupedRenderPlan(tabs, pathKeys);

    expect(plan).toEqual([
      { kind: "header", key: "/repo", name: "repo" },
      { kind: "tab", tab: tabs[0], groupKey: "/repo", groupSize: 2 },
      { kind: "tab", tab: tabs[2], groupKey: "/repo", groupSize: 2 },
      { kind: "tab", tab: tabs[1], groupKey: undefined, groupSize: 0 },
    ]);
  });

  it("leaves tabs with a unique path ungrouped", () => {
    const tabs: Tab[] = [makeWorkspaceTab(1, "/a"), makeEditorTab(2, "/b")];
    const pathKeys = new Map(tabs.map((t) => [t.id, pathKeyFor(t)]));

    expect(buildGroupedRenderPlan(tabs, pathKeys)).toEqual([
      { kind: "tab", tab: tabs[0], groupKey: undefined, groupSize: 0 },
      { kind: "tab", tab: tabs[1], groupKey: undefined, groupSize: 0 },
    ]);
  });

  it("leaves tabs with no path (e.g. home) ungrouped", () => {
    const tabs: Tab[] = [makeHomeTab(1)];
    const pathKeys = new Map(tabs.map((t) => [t.id, pathKeyFor(t)]));

    expect(buildGroupedRenderPlan(tabs, pathKeys)).toEqual([
      { kind: "tab", tab: tabs[0], groupKey: undefined, groupSize: 0 },
    ]);
  });

  it("groups even a single tab per folder when minGroupSize is 1", () => {
    const tabs: Tab[] = [makeWorkspaceTab(1, "/a"), makeEditorTab(2, "/b/file.ts")];
    const pathKeys = new Map(tabs.map((t) => [t.id, pathKeyFor(t)]));

    expect(buildGroupedRenderPlan(tabs, pathKeys, 1)).toEqual([
      { kind: "header", key: "/a", name: "a" },
      { kind: "tab", tab: tabs[0], groupKey: "/a", groupSize: 1 },
      { kind: "header", key: "/b", name: "b" },
      { kind: "tab", tab: tabs[1], groupKey: "/b", groupSize: 1 },
    ]);
  });

  it("climbs a nested editor file up into an already-open ancestor project folder", () => {
    const tabs: Tab[] = [
      makeWorkspaceTab(1, "/Users/niklas/Developer/active/Labonair/app"),
      makeEditorTab(2, "/Users/niklas/Developer/active/Labonair/app/src/app/App.tsx"),
    ];
    const pathKeys = new Map(tabs.map((t) => [t.id, pathKeyFor(t)]));

    expect(buildGroupedRenderPlan(tabs, pathKeys)).toEqual([
      { kind: "header", key: "/Users/niklas/Developer/active/Labonair/app", name: "Labonair/app" },
      {
        kind: "tab",
        tab: tabs[0],
        groupKey: "/Users/niklas/Developer/active/Labonair/app",
        groupSize: 2,
      },
      {
        kind: "tab",
        tab: tabs[1],
        groupKey: "/Users/niklas/Developer/active/Labonair/app",
        groupSize: 2,
      },
    ]);
  });

  it("does not let two nested workspace tabs merge — ancestor climbing is editor/AI-diff only", () => {
    const tabs: Tab[] = [makeWorkspaceTab(1, "/repo"), makeWorkspaceTab(2, "/repo/sub")];
    const pathKeys = new Map(tabs.map((t) => [t.id, pathKeyFor(t)]));

    expect(buildGroupedRenderPlan(tabs, pathKeys)).toEqual([
      { kind: "tab", tab: tabs[0], groupKey: undefined, groupSize: 0 },
      { kind: "tab", tab: tabs[1], groupKey: undefined, groupSize: 0 },
    ]);
  });
});

describe("isBlockedCrossGroupDrag", () => {
  it("blocks when both tabs are grouped but under different keys", () => {
    expect(isBlockedCrossGroupDrag("/repo-a", "/repo-b")).toBe(true);
  });

  it("allows when both tabs share the same group", () => {
    expect(isBlockedCrossGroupDrag("/repo", "/repo")).toBe(false);
  });

  it("allows when either tab is ungrouped", () => {
    expect(isBlockedCrossGroupDrag(undefined, "/repo")).toBe(false);
    expect(isBlockedCrossGroupDrag("/repo", undefined)).toBe(false);
    expect(isBlockedCrossGroupDrag(undefined, undefined)).toBe(false);
  });
});
