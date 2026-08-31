import type { Tab, WorkspaceTab } from "../types";

// --- Sidebar tab grouping by folder ---

/** The raw path a tab is "in", with no host component — grouping matches
 *  purely on this string, so a local and a remote tab sharing the same
 *  folder land in the same category (see `remoteHostLabelFor` in
 *  `tabUtils.tsx` for the separate per-tab "Remote" badge that disambiguates
 *  them within a group). `undefined` for kinds with no folder concept.
 *
 *  Editor/AI-diff tabs resolve to the file's *containing folder*, not the
 *  file itself — grouping for these is project-based, not file-based: two
 *  files in the same folder land together, and (see `buildGroupedRenderPlan`)
 *  a file nested deeper can still climb up into an already-open ancestor
 *  folder's category instead of spinning up its own. */
export function pathKeyFor(t: Tab): string | undefined {
  if (t.kind === "workspace") {
    const wt = t as WorkspaceTab;
    return wt.sessions[wt.activePaneId]?.cwd;
  }
  if (t.kind === "editor") return dirnameOf(t.remoteHostId ? (t.remotePath ?? t.path) : t.path);
  if (t.kind === "ai-diff") return dirnameOf(t.path);
  if (t.kind === "sftp") return t.remotePath;
  if (t.kind === "git-graph" || t.kind === "commit-diff") return t.repositoryPath;
  if (t.kind === "git-diff") return t.repoRoot;
  return undefined; // "preview" (URL, not a path) and "home" have no folder.
}

/** Directory containing `filePath` (POSIX `dirname`). `undefined` if there's
 *  no directory component to speak of. */
function dirnameOf(filePath: string): string | undefined {
  const idx = filePath.lastIndexOf("/");
  if (idx < 0) return undefined;
  return idx === 0 ? "/" : filePath.slice(0, idx);
}

const FILE_BASED_KINDS = new Set<Tab["kind"]>(["editor", "ai-diff"]);

/** Closest ancestor of `key` found among `allKeys` — a strict ancestor,
 *  i.e. `key` starts with `candidate + "/"` — picking the longest (most
 *  specific) match. Used only for file-based tabs (editor/AI-diff): lets a
 *  deeply nested file join an already-open project folder (a workspace's
 *  cwd, an SFTP path, a git repo root, or another file's folder) instead of
 *  always grouping strictly by its own immediate directory. */
function closestAncestorKey(key: string, allKeys: string[]): string | undefined {
  let best: string | undefined;
  for (const candidate of allKeys) {
    if (candidate !== key && key.startsWith(`${candidate}/`)) {
      if (best === undefined || candidate.length > best.length) best = candidate;
    }
  }
  return best;
}

/** Compact group header name — last 2 path segments, e.g.
 *  "/home/user/Developer/active/Labonair" → "active/Labonair". Two distinct
 *  full paths can coincidentally produce the same short name (e.g. same
 *  project folder name on two different hosts) — a cosmetic ambiguity in
 *  the header text only; the grouping itself still keys on the full path,
 *  so it's never actually merged. */
export function groupNameFor(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts.length ? parts.slice(-2).join("/") : "/";
}

export type RenderPlanEntry =
  | { kind: "header"; key: string; name: string }
  | { kind: "tab"; tab: Tab; groupKey: string | undefined; groupSize: number };

/** Builds the sidebar's render order: tabs sharing a `pathKeys` value with
 *  at least `minGroupSize` tabs total are clustered into a header + block,
 *  emitted at the position their key first appears in `tabs`' original order
 *  (stable, no separate group-ordering rule). Every other tab renders
 *  individually, in place, with `groupKey: undefined`.
 *
 *  `minGroupSize` defaults to 2 — the "stacking" behavior where a folder
 *  only gets a header once a second tab lands in it. Passing 1 switches to
 *  "eager" grouping, where every tab with a resolvable path gets its own
 *  header immediately, even alone (see `sidebarGroupSingleTabs` setting). */
export function buildGroupedRenderPlan(
  tabs: Tab[],
  pathKeys: Map<number, string | undefined>,
  minGroupSize = 2,
): RenderPlanEntry[] {
  // File-based tabs (editor/AI-diff) resolve to their own containing folder
  // by default, but climb up to an already-open ancestor folder when one
  // exists — everything else (workspace cwd, SFTP path, git repo root) keeps
  // its exact key untouched, so terminal/SFTP/git grouping is unaffected.
  const allKeys = Array.from(
    new Set(Array.from(pathKeys.values()).filter((k): k is string => k !== undefined)),
  );
  const effectiveKeys = new Map<number, string | undefined>();
  for (const t of tabs) {
    const raw = pathKeys.get(t.id);
    if (raw === undefined) effectiveKeys.set(t.id, undefined);
    else if (FILE_BASED_KINDS.has(t.kind)) effectiveKeys.set(t.id, closestAncestorKey(raw, allKeys) ?? raw);
    else effectiveKeys.set(t.id, raw);
  }

  const counts = new Map<string, number>();
  for (const t of tabs) {
    const key = effectiveKeys.get(t.id);
    if (key) counts.set(key, (counts.get(key) ?? 0) + 1);
  }

  const plan: RenderPlanEntry[] = [];
  const emitted = new Set<string>();

  for (const t of tabs) {
    const key = effectiveKeys.get(t.id);
    const isGrouped = key !== undefined && (counts.get(key) ?? 0) >= minGroupSize;

    if (!isGrouped) {
      plan.push({ kind: "tab", tab: t, groupKey: undefined, groupSize: 0 });
      continue;
    }
    if (emitted.has(key)) continue;
    emitted.add(key);
    const groupSize = counts.get(key) ?? 0;
    plan.push({ kind: "header", key, name: groupNameFor(key) });
    for (const member of tabs) {
      if (effectiveKeys.get(member.id) === key)
        plan.push({ kind: "tab", tab: member, groupKey: key, groupSize });
    }
  }

  return plan;
}

/** A drag is only blocked when both tabs belong to an actual rendered group
 *  (`groupKey` set, i.e. threshold already met) and those groups differ —
 *  reordering among ungrouped tabs (the common case: most tabs have a
 *  unique folder) stays unrestricted. */
export function isBlockedCrossGroupDrag(
  fromGroupKey: string | undefined,
  toGroupKey: string | undefined,
): boolean {
  return fromGroupKey !== undefined && toGroupKey !== undefined && fromGroupKey !== toGroupKey;
}
