#!/usr/bin/env python3
"""Validate dependency boundaries declared in the architecture manifest."""

from __future__ import annotations

import json
import sys

from architecture_model import load_manifest, manifest_graph


# UI-free engines must not reach these crates, even transitively.
UI_CRATES = {
    "labonair-gpui-ext", "labonair-ui-kit", "labonair-theme",
    "labonair-background", "labonair-notifications",
    "labonair-command-palette", "labonair-workspace", "labonair-shell",
    "labonair-settings-ui", "labonair-keymap-ui", "labonair-hosts-ui",
    "labonair-panel", "labonair-panel-explorer", "labonair-panel-scm",
    "labonair-panel-git-graph", "labonair-panel-snippets",
}
PANEL_CRATES = {
    "labonair-panel-explorer", "labonair-panel-scm",
    "labonair-panel-git-graph", "labonair-panel-snippets",
}


meta = json.load(sys.stdin)
manifest = load_manifest()
allowed = manifest_graph(manifest)

# ---------------------------------------------------------------------------
# Build the workspace-internal adjacency from cargo metadata.
# ---------------------------------------------------------------------------
ws_members = set()
graph = {}
for package in meta["packages"]:
    name = package["name"]
    if not name.startswith("labonair"):
        continue
    ws_members.add(name)
    graph[name] = {
        dependency["name"]
        for dependency in package["dependencies"]
        if dependency["name"].startswith("labonair")
        and dependency["name"] != name
    }

errors = []

# Every workspace member must have an explicit manifest entry.
for name in sorted(ws_members):
    if name not in allowed:
        errors.append(
            f"{name}: no architecture manifest entry — add the owner, role, "
            "layer, and allowed edges in docs/architecture/graph.toml."
        )

# 1. Per-crate manifest check.
for name, dependencies in sorted(graph.items()):
    declared = allowed.get(name, set())
    for dependency in sorted(dependencies - declared):
        errors.append(
            f"{name} depends on {dependency} — forbidden by the architecture "
            f"manifest. Allowed workspace deps: {sorted(declared) or '(none)'}. "
        )
    for dependency in sorted(declared - dependencies):
        errors.append(
            f"{name} declares {dependency} in the architecture manifest, but "
            "cargo metadata does not contain that edge."
        )


# Transitive reachability (memoised DFS over the workspace subgraph).
_reach_cache = {}


def reaches(source):
    if source in _reach_cache:
        return _reach_cache[source]
    seen = set()
    stack = list(graph.get(source, []))
    while stack:
        name = stack.pop()
        if name in seen:
            continue
        seen.add(name)
        stack.extend(graph.get(name, []))
    _reach_cache[source] = seen
    return seen


# 2. Acyclicity — a crate must not reach itself.
for name in sorted(graph):
    if name in reaches(name):
        errors.append(
            f"{name} is part of a dependency cycle — the architecture policy "
            "requires an acyclic crate graph."
        )

# 3. Transitive must-not-reach invariants.
# Panel crates may reach panel-git-graph through workspace because Workspace
# owns that tab-view entity. Direct panel-to-panel coupling remains forbidden,
# and no panel may reach the shell.
for name in sorted(PANEL_CRATES & ws_members):
    if "labonair-shell" in reaches(name):
        errors.append(
            f"{name} transitively reaches labonair-shell — panels must not "
            "depend on the composition root."
        )
    direct_panels = (PANEL_CRATES & graph.get(name, set())) - {name}
    if direct_panels:
        errors.append(
            f"{name} directly depends on another panel crate "
            f"{sorted(direct_panels)} — panel crates must remain independent."
        )

if "labonair-panel" in ws_members:
    bad = {"labonair-workspace", "labonair-shell"} & reaches("labonair-panel")
    if bad:
        errors.append(
            f"labonair-panel transitively reaches {sorted(bad)} — the panel "
            "contract crate must remain below the composition and workspace roots."
        )

for engine in ("labonair-ai", "labonair-editor"):
    if engine not in ws_members:
        continue
    bad = UI_CRATES & reaches(engine)
    if bad:
        errors.append(
            f"{engine} transitively reaches UI crate(s) {sorted(bad)} — "
            "UI-free engines must not depend on presentation crates."
        )

if "labonair-ui-kit" in ws_members:
    forbidden_for_ui_kit = {
        "labonair-workspace", "labonair-shell", "labonair-notifications",
        "labonair-command-palette", "labonair-settings-ui",
        "labonair-hosts-ui", "labonair-theme",
    } | PANEL_CRATES
    bad = forbidden_for_ui_kit & reaches("labonair-ui-kit")
    if bad:
        errors.append(
            f"labonair-ui-kit transitively reaches {sorted(bad)} — the UI kit "
            "must remain below product modules."
        )

if errors:
    print("crate dependency check FAILED:", file=sys.stderr)
    for error in errors:
        print(f"  ✗ {error}", file=sys.stderr)
    print(f"\n{len(errors)} violation(s). See docs/architecture/graph.toml.", file=sys.stderr)
    sys.exit(1)

print(
    f"crate dependency check OK — {len(graph)} workspace crates, "
    f"{sum(len(dependencies) for dependencies in graph.values())} internal edges, "
    "acyclic, no untracked boundary violations. Transitional edges remain "
    "documented in docs/audits/architecture-inventory.md."
)
