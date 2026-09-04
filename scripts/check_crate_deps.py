#!/usr/bin/env python3
import json
import sys

meta = json.load(sys.stdin)

# ---------------------------------------------------------------------------
# ALLOW-LIST — workspace-internal deps permitted per crate.
#
# Derived 1:1 from docs/architecture.md §3 + §8.4. Each entry is annotated with
# the rule it follows. Edges marked "[deviation]" are pre-existing, accepted,
# and recorded in the architecture doc / perf-baseline; they are listed so the
# check still catches *new* regressions.
# ---------------------------------------------------------------------------
ALLOWED = {
    # bin — depends on the shell + the engines it boots (rule 3 consumer side).
    "labonair": {
        "labonair-shell", "labonair-terminal", "labonair-editor",
        "labonair-backend", "labonair-ai", "labonair-theme",
        "labonair-settings",
    },

    # Foundation ------------------------------------------------------------
    # rule 6: leaf below ui-kit, only gpui / gpui-component.
    "labonair-gpui-ext": set(),
    # rule 5: only gpui, gpui-component, theme, gpui-ext.
    "labonair-ui-kit": {"labonair-theme", "labonair-gpui-ext"},
    # extended theme crate — a leaf token crate, no workspace deps.
    "labonair-theme": set(),
    "labonair-notifications": {
        "labonair-theme", "labonair-ui-kit", "labonair-gpui-ext",
    },
    "labonair-command-palette": {
        "labonair-theme", "labonair-ui-kit", "labonair-gpui-ext",
        "labonair-backend",
    },

    # Settings track ------------------------------------------------------
    # rule 7: settings-ui depends on settings + ui-kit (+ hosts-ui later,
    # T19-010).
    # [deviation] workspace / command-palette / ai edges are pre-Phase-18
    # couplings kept until the settings track fully replaces them.
    # labonair-panel: T18-007's Personalization pane reads/writes the
    # StatusItemRegistry / PanelRegistry contracts (StatusSide, DockPosition,
    # …) directly, same as `labonair-workspace` already does.
    # T19-004: the generated field grid + navigation is built directly off
    # `labonair-settings-content::SettingsContent`/`areas::AREAS` and the
    # layered `labonair-settings::SettingsStore` global — the old
    # `PreferencesStore`/`GlobalPreferences` bridge stays for modules not yet
    # migrated onto the `Settings` trait (see `store.rs`'s doc comment).
    "labonair-settings-ui": {
        "labonair-theme", "labonair-ui-kit", "labonair-gpui-ext",
        "labonair-notifications", "labonair-command-palette",
        "labonair-workspace", "labonair-panel", "labonair-backend", "labonair-ai",
        "labonair-settings", "labonair-settings-content",
        # T19-008: surgical `keymap.json` edits reuse T19-005's tree-sitter
        # JSON editor.
        "labonair-settings-json",
    },

    # Workspace track --------------------------------------------------
    # rule 1: contracts crate — NO workspace-track dep at all.
    "labonair-panel": {"labonair-gpui-ext"},
    # rule 3 + §8.4: workspace owns the tab-view entities, so it pulls
    # hosts-ui and panel-git-graph (acyclic — neither depends back on it).
    # T19-002: ThemeSettings/TerminalSettings real consumers
    # (workspace.rs::reduce_motion, views/terminal.rs opacity/copy-on-select/
    # right-click-pastes) pull the typed settings store directly.
    # T19-006: the code-editor view's settings.json schema-hover helper
    # (`views/editor.rs::update_hover`) calls
    # `labonair_settings_json::json_path_at_offset` directly to resolve the
    # key path under the mouse — a leaf crate (`labonair-settings-json`),
    # no cycle.
    "labonair-workspace": {
        "labonair-theme", "labonair-ui-kit", "labonair-gpui-ext",
        "labonair-notifications", "labonair-command-palette",
        "labonair-panel", "labonair-panel-git-graph", "labonair-hosts-ui",
        "labonair-terminal", "labonair-editor", "labonair-backend",
        "labonair-ai", "labonair-settings", "labonair-settings-json",
    },
    # rule 3: the only crate that knows every concrete panel type — it also
    # touches the `labonair-panel` contracts crate to register them (T17-001).
    # T19-008: shell also depends on `labonair-settings` directly — it owns
    # the concrete `menu::` GPUI Actions, so it's the only crate that can
    # turn a merged `keymap.json` into real `gpui::KeyBinding`s / watch the
    # file live.
    "labonair-shell": {
        "labonair-theme", "labonair-ui-kit", "labonair-gpui-ext",
        "labonair-notifications", "labonair-command-palette",
        "labonair-workspace", "labonair-settings-ui", "labonair-panel",
        "labonair-panel-explorer", "labonair-panel-scm",
        "labonair-panel-git-graph", "labonair-panel-snippets",
        "labonair-panel-ai", "labonair-terminal", "labonair-backend",
        "labonair-settings",
    },

    # Panels — rule 2 (+ §8.4: explorer/snippets/ai may pull workspace).
    # Each panel crate depends on `labonair-panel` to `impl Panel` (T17-001);
    # the contracts crate is a leaf (only gpui / gpui-ext), so no cycle.
    "labonair-panel-explorer": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-backend", "labonair-workspace",
    },
    "labonair-panel-scm": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-backend",
    },
    "labonair-panel-git-graph": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-backend",
    },
    "labonair-panel-snippets": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-backend", "labonair-workspace",
    },
    # [deviation] panel-ai also pulls command-palette (slash-command model)
    # and editor (composer buffer) — accepted, still no panel-* / shell edge.
    "labonair-panel-ai": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-command-palette", "labonair-backend", "labonair-editor",
        "labonair-ai", "labonair-workspace",
    },

    # Host access — rule 9: not a panel crate; no workspace / shell / panel*.
    # [deviation] also pulls notifications for toast feedback.
    "labonair-hosts-ui": {
        "labonair-theme", "labonair-ui-kit", "labonair-notifications",
        "labonair-backend",
    },

    # Engines — rule 4: no UI dep.
    # [deviation] labonair-terminal pulls labonair-theme (leaf token crate)
    # for its ANSI palette; a deeper engine/renderer split is future work
    # (see docs/perf-baseline.md). It must reach nothing else.
    "labonair-terminal": {"labonair-theme"},
    "labonair-editor": set(),
    # [deviation, T19-001, docs/architecture.md §8.15] labonair-backend
    # depends on labonair-settings-content for the
    # `impl From<&SettingsContent> for Preferences` bridge — a pure,
    # non-UI leaf crate (no cycle: labonair-settings-content never depends
    # back on labonair-backend).
    "labonair-backend": {"labonair-settings-content"},
    "labonair-ai": {"labonair-backend"},

    # Settings track (T19-001) — pure data model, no GPUI/UI/backend deps.
    "labonair-settings-content": {"labonair-settings-macros"},
    "labonair-settings-macros": set(),
    # Settings track (T19-005) — surgical `settings.json` text edits via a
    # real `tree-sitter-json` syntax tree. A leaf: only `tree-sitter`/
    # `tree-sitter-json`/`serde_json` (external), no workspace deps.
    "labonair-settings-json": set(),
    # Settings track (T19-002) — the layered SettingsStore. Depends on the
    # pure data model + its own derive-macro crate; `gpui` is used (Store as
    # a Global + App/AsyncApp access) but that's an external dep, not a
    # workspace edge, so it doesn't show up here. No UI crate, no backend.
    # T19-005 added `labonair-settings-json` for the surgical write path;
    # T19-006 reuses it (`find_value_range`/`json_path_at_offset`) for
    # schema-validation error positions.
    "labonair-settings": {
        "labonair-settings-content", "labonair-settings-macros",
        "labonair-settings-json",
    },
}

# UI crates the engines (backend/ai/editor) must not reach, even transitively.
UI_CRATES = {
    "labonair-gpui-ext", "labonair-ui-kit", "labonair-theme",
    "labonair-notifications", "labonair-command-palette",
    "labonair-workspace", "labonair-shell", "labonair-settings-ui",
    "labonair-hosts-ui", "labonair-panel", "labonair-panel-explorer",
    "labonair-panel-scm", "labonair-panel-git-graph",
    "labonair-panel-snippets", "labonair-panel-ai",
}
PANEL_CRATES = {
    "labonair-panel-explorer", "labonair-panel-scm",
    "labonair-panel-git-graph", "labonair-panel-snippets",
    "labonair-panel-ai",
}

# ---------------------------------------------------------------------------
# Build the workspace-internal adjacency from `cargo metadata`.
# ---------------------------------------------------------------------------
ws_members = set()
graph = {}
for pkg in meta["packages"]:
    name = pkg["name"]
    if not name.startswith("labonair"):
        continue
    ws_members.add(name)
    deps = sorted({
        d["name"] for d in pkg["dependencies"]
        if d["name"].startswith("labonair") and d["name"] != name
    })
    graph[name] = deps

errors = []

# Every workspace member must have an ALLOWED entry (keeps the list honest).
for name in sorted(ws_members):
    if name not in ALLOWED:
        errors.append(
            f"{name}: no ALLOW-LIST entry in scripts/check-crate-deps.sh — "
            f"add one citing the docs/architecture.md §3 rule it follows."
        )

# 1. Per-crate allow-list check.
for name, deps in sorted(graph.items()):
    allowed = ALLOWED.get(name, set())
    for dep in deps:
        if dep not in allowed:
            errors.append(
                f"{name} depends on {dep} — forbidden by docs/architecture.md "
                f"§3. Allowed workspace deps for {name}: "
                f"{sorted(allowed) or '(none)'}."
            )

# Transitive reachability (memoised DFS over the workspace subgraph).
_reach_cache = {}
def reaches(src):
    if src in _reach_cache:
        return _reach_cache[src]
    seen = set()
    stack = list(graph.get(src, []))
    while stack:
        n = stack.pop()
        if n in seen:
            continue
        seen.add(n)
        stack.extend(graph.get(n, []))
    _reach_cache[src] = seen
    return seen

# 2. Acyclicity (rule 8) — a crate must not reach itself.
for name in sorted(graph):
    if name in reaches(name):
        errors.append(
            f"{name} is part of a dependency cycle — docs/architecture.md §3 "
            f"rule 8 requires an acyclic crate graph."
        )

# 3. Transitive must-not-reach invariants.
#    Panel crates may transitively reach labonair-panel-git-graph *via*
#    labonair-workspace (§8.4: workspace owns that tab-view entity, acyclic) —
#    that indirection is sanctioned. What is forbidden: a *direct* panel→panel
#    edge (API coupling, rule 2) and reaching labonair-shell by any path
#    (§3 warning: "panel-ai must not, via workspace, land back at shell").
for name in sorted(PANEL_CRATES & ws_members):
    if "labonair-shell" in reaches(name):
        errors.append(
            f"{name} transitively reaches labonair-shell — forbidden by "
            f"docs/architecture.md §3 rule 2 / warning."
        )
    direct_panels = (PANEL_CRATES & set(graph.get(name, []))) - {name}
    if direct_panels:
        errors.append(
            f"{name} directly depends on another panel crate "
            f"{sorted(direct_panels)} — forbidden by docs/architecture.md §3 "
            f"rule 2 (panel crates never depend on each other)."
        )

if "labonair-panel" in ws_members:
    bad = {"labonair-workspace", "labonair-shell"} & reaches("labonair-panel")
    if bad:
        errors.append(
            f"labonair-panel transitively reaches {sorted(bad)} — forbidden by "
            f"docs/architecture.md §3 rule 1 (contracts crate breaks the cycle)."
        )

for engine in ("labonair-backend", "labonair-ai", "labonair-editor"):
    if engine not in ws_members:
        continue
    bad = UI_CRATES & reaches(engine)
    if bad:
        errors.append(
            f"{engine} transitively reaches UI crate(s) {sorted(bad)} — "
            f"forbidden by docs/architecture.md §3 rule 4."
        )

if "labonair-ui-kit" in ws_members:
    forbidden_for_ui_kit = {
        "labonair-workspace", "labonair-shell", "labonair-notifications",
        "labonair-command-palette", "labonair-settings-ui",
        "labonair-hosts-ui",
    } | PANEL_CRATES
    bad = forbidden_for_ui_kit & reaches("labonair-ui-kit")
    if bad:
        errors.append(
            f"labonair-ui-kit transitively reaches {sorted(bad)} — forbidden "
            f"by docs/architecture.md §3 rule 5."
        )

if errors:
    print("crate dependency check FAILED:\n", file=sys.stderr)
    for e in errors:
        print(f"  ✗ {e}", file=sys.stderr)
    print(
        f"\n{len(errors)} violation(s). See docs/architecture.md §3.",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"crate dependency check OK — {len(graph)} workspace crates, "
    f"{sum(len(v) for v in graph.values())} internal edges, acyclic, "
    f"all rules in docs/architecture.md §3 satisfied."
)
