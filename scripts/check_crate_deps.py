#!/usr/bin/env python3
import json
import sys

meta = json.load(sys.stdin)

# ---------------------------------------------------------------------------
# ALLOW-LIST — workspace-internal deps permitted per crate.
#
# Derived from docs/architecture.md and the v2 migration inventory in
# docs/audits/architecture-inventory.md. Entries marked transitional are
# temporary compatibility edges. They remain explicit so a new edge still
# fails the build and each transitional edge can be removed independently.
# ---------------------------------------------------------------------------
ALLOWED = {
    # bin — depends on the shell + the engines it boots (rule 3 consumer side).
    "labonair": {
        "labonair-shell",
        # Smoke tests exercise the terminal engine and theme conversion
        # directly; these are dev-dependency edges, not runtime bootstrap
        # ownership.
        "labonair-terminal", "labonair-theme",
    },

    # Foundation ------------------------------------------------------------
    # rule 6: leaf below ui-kit, only gpui / gpui-component.
    "labonair-gpui-ext": set(),
    # rule 5: only gpui, gpui-component, theme, gpui-ext.
    "labonair-ui-kit": {"labonair-theme", "labonair-gpui-ext"},
    # UI-free identity contracts shared by commands and keymap. Keeping this
    # below both feature crates prevents a command/keymap dependency cycle.
    "labonair-interaction-contracts": set(),
    # Theme metadata also contributes palette commands through the shared
    # command contract; it does not depend on the palette UI.
    "labonair-theme": {"labonair-command-palette-core"},
    # Background capability owns image persistence and GPUI rendering. It
    # consumes only the filesystem platform service.
    "labonair-background": {"labonair-filesystem", "labonair-theme"},
    # Update manifest/download/install capability; the GPUI updater view stays
    # in shell and consumes this UI-free crate.
    "labonair-updater": {"labonair-filesystem"},
    # Platform service — no GPUI or feature-crate deps. Feature crates may
    # consume it directly; the backend edge is transitional during migration.
    "labonair-filesystem": set(),
    # UI-free SSH capability contracts; transport implementations remain in
    # backend adapters.
    "labonair-ssh": {"labonair-errors"},
    # UI-free SFTP contracts may use opaque SSH session ids only.
    "labonair-sftp": {"labonair-errors", "labonair-ssh"},
    # UI-free transfer lifecycle, event, and worker contracts.
    "labonair-transfers": set(),
    # UI-free shortcut identities, keymap file data/defaults, and conflict
    # resolution. GPUI publication remains in the palette/shell adapters.
    "labonair-keymap": {
        "labonair-interaction-contracts", "labonair-command-palette-core",
    },
    # Keymap presentation is a real sibling boundary: it consumes the
    # keymap-owned snapshot and shared UI/theme contracts, but owns no
    # persistence, resolution, or feature execution.
    "labonair-keymap-ui": {
        "labonair-command-palette-core", "labonair-keymap",
        "labonair-notifications",
        "labonair-theme", "labonair-ui-kit",
    },
    "labonair-command-palette-core": {"labonair-interaction-contracts"},
    # Platform service — secret storage and encryption, without GPUI or
    # feature-module dependencies.
    "labonair-secrets": {"labonair-filesystem"},
    # UI-free notification lifecycle and metadata. This is the only owner of
    # retention, ordering, deduplication, and read state.
    "labonair-notifications-core": set(),
    # UI-free MCP session/grant contracts. The bridge implementation remains
    # an injected backend adapter while workspace consumes only this boundary.
    "labonair-mcp-core": set(),
    # Cross-cutting structured error contract. It contains no workspace
    # dependencies and is shared by capability services during migration.
    "labonair-errors": set(),
    # Hosts owns domain models plus capability-local query code. Secret-bearing
    # writes and MCP side effects remain transitional backend adapters.
    "labonair-hosts": {
        "labonair-errors", "labonair-persistence", "labonair-secrets",
        "labonair-command-palette-core",
    },
    # Shared SQLite lifecycle only. Feature stores own their queries and
    # domain models; this crate must remain UI- and backend-free.
    "labonair-persistence": set(),
    # UI-free Git value types and capability contracts. Implementations stay
    # in backend adapters and are injected by the composition root.
    "labonair-git": {"labonair-command-palette-core"},
    # Credential capability: metadata, secret references, and key material.
    "labonair-credentials": {
        "labonair-persistence", "labonair-secrets",
    },
    # Snippet domain, SQLite store, and execution contracts. Transport
    # implementations are injected by the composition root.
    "labonair-snippets": {
        "labonair-errors", "labonair-persistence",
        "labonair-command-palette-core",
    },
    "labonair-notifications": {
        "labonair-notifications-core",
        "labonair-panel", "labonair-theme", "labonair-ui-kit",
    },
    "labonair-command-palette": {
        "labonair-theme", "labonair-ui-kit",
        "labonair-filesystem",
        "labonair-keymap",
        "labonair-command-palette-core",
        # transitional: palette settings reads move behind a provider contract
        "labonair-settings",
    },

    # Settings track ------------------------------------------------------
    # rule 7: settings-ui renders SettingsContent values and receives only
    # narrow discovery services from the composition root.
    # T19-004: the generated field grid + navigation is built directly off
    # `labonair-settings-content::SettingsContent`/`areas::AREAS` and the
    # layered `labonair-settings::SettingsStore` global — the old
    # `PreferencesStore`/`GlobalPreferences` bridge stays for modules not yet
    # migrated onto the `Settings` trait (see `store.rs`'s doc comment).
    "labonair-settings-ui": {
        "labonair-theme", "labonair-ui-kit", "labonair-gpui-ext",
        "labonair-notifications", "labonair-command-palette",
        "labonair-settings", "labonair-settings-content", "labonair-filesystem",
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
        "labonair-hosts",
        "labonair-terminal", "labonair-editor",
        "labonair-git",
        "labonair-ai", "labonair-settings", "labonair-settings-json",
        "labonair-filesystem",
        "labonair-ssh", "labonair-sftp", "labonair-transfers",
        "labonair-background", "labonair-mcp-core",
        "labonair-command-palette-core", "labonair-keymap",
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
        "labonair-command-palette-core",
        "labonair-keymap",
        "labonair-keymap-ui",
        "labonair-workspace", "labonair-settings-ui", "labonair-panel",
        "labonair-panel-explorer", "labonair-panel-scm",
        "labonair-panel-git-graph", "labonair-panel-snippets",
        "labonair-panel-ai", "labonair-terminal", "labonair-backend",
        "labonair-settings", "labonair-filesystem", "labonair-ssh",
        "labonair-sftp", "labonair-transfers", "labonair-transfers-ui",
        "labonair-background", "labonair-mcp-core", "labonair-persistence",
        "labonair-updater",
        # Provider metadata contracts are assembled here; feature behavior
        # remains in the owning crates and is not implemented by this root.
        "labonair-editor", "labonair-git", "labonair-hosts",
        "labonair-hosts-ui", "labonair-snippets",
    },

    # Transfer presentation — owns the statusbar dropdown, while lifecycle
    # state and worker contracts remain in `labonair-transfers`.
    "labonair-transfers-ui": {
        "labonair-theme", "labonair-ui-kit", "labonair-transfers",
    },

    # Panels — rule 2 (+ §8.4: explorer/snippets/ai may pull workspace).
    # Each panel crate depends on `labonair-panel` to `impl Panel` (T17-001);
    # the contracts crate is a leaf (only gpui / gpui-ext), so no cycle.
    "labonair-panel-explorer": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-workspace",
        # transitional: settings reads move behind a feature settings contract
        "labonair-settings",
        "labonair-filesystem",
    },
    "labonair-panel-scm": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-git",
        # transitional: editor and settings contracts are extracted in Phase 7
        "labonair-editor", "labonair-settings",
    },
    "labonair-panel-git-graph": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-git",
    },
    "labonair-panel-snippets": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-notifications", "labonair-hosts", "labonair-snippets",
        "labonair-persistence", "labonair-workspace",
    },
    # [deviation] panel-ai also pulls command-palette (slash-command model)
    # and editor (composer buffer) — accepted, still no panel-* / shell edge.
    "labonair-panel-ai": {
        "labonair-theme", "labonair-ui-kit", "labonair-panel",
        "labonair-command-palette", "labonair-backend", "labonair-editor",
        "labonair-ai", "labonair-workspace",
    },

    # Host access — rule 9: not a panel crate; no workspace / shell / panel*.
    # [deviation] also pulls notifications for user-visible feedback.
    # Host definitions stay in the Hosts-owned store; Settings is deliberately
    # absent so the management surface cannot create a second write path.
    "labonair-hosts-ui": {
        "labonair-theme", "labonair-ui-kit", "labonair-notifications",
        "labonair-hosts", "labonair-credentials", "labonair-persistence",
        "labonair-secrets", "labonair-snippets", "labonair-ssh",
        "labonair-errors",
    },

    # Engines — rule 4: no UI dep.
    # [deviation] labonair-terminal pulls labonair-theme (leaf token crate)
    # for its ANSI palette; a deeper engine/renderer split is future work
    # (see docs/perf-baseline.md). It must reach nothing else.
    "labonair-terminal": {"labonair-theme", "labonair-command-palette-core"},
    "labonair-editor": {
        "labonair-command-palette-core", "labonair-interaction-contracts",
    },
    # Backend contains only concrete platform adapters. Settings migrations are
    # owned by `labonair-settings::legacy_migrations`.
    "labonair-backend": {
        "labonair-filesystem", "labonair-secrets",
        "labonair-errors", "labonair-hosts", "labonair-persistence",
        "labonair-credentials", "labonair-snippets", "labonair-git",
        "labonair-ssh", "labonair-sftp", "labonair-transfers",
        "labonair-mcp-core",
    },
    "labonair-ai": {"labonair-filesystem"},

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
        "labonair-command-palette-core", "labonair-interaction-contracts",
    },
}

# UI crates the engines (backend/ai/editor) must not reach, even transitively.
UI_CRATES = {
    "labonair-gpui-ext", "labonair-ui-kit", "labonair-theme",
    "labonair-background",
    "labonair-notifications", "labonair-command-palette",
    "labonair-workspace", "labonair-shell", "labonair-settings-ui",
    "labonair-keymap-ui",
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
    f"no untracked boundary violations. Transitional edges remain documented "
    f"in docs/audits/architecture-inventory.md."
)
