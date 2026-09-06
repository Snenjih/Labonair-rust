# Handshake — Session State (Labonair-rust Port)

Authored by: GPUI-native port of Labonair (formerly Tauri v2 + React 19 → now pure Rust/GPUI).

> This file is the authoritative continuity doc for the **port** project. This is a **hard fork** — fully standalone, no link/symlink/submodule to any external Labonair repo. The old web-app source is a frozen read-only copy at `reference-src/` inside this repo and is the only reference. Do not mistake the old git history/tech for the current target.

## Current Session: 2026-09-06 (Remove path-bookmarks feature)

Ad-hoc request (not a ROADMAP task; user asked to remove the bookmarks system
"vollständig" without breaking other systems). Done in one commit.

**Removed:** `crates/backend/src/modules/bookmarks/` (model + `bookmarks.json`),
`crates/panel-explorer/src/bookmarks.rs` (`BookmarksView` / `BookmarkEvent`),
`BookmarksStatusItem`, `BookmarksModal`, `toggle_bookmarks` /
`sync_bookmarks_modal` / `handle_bookmark_event`, `CommandId::OpenPathBookmarks`,
`ShortcutId::BookmarksOpen` + `ShortcutGroup::Bookmarks`, the `bookmarks::Open`
keymap entries, the Explorer "Bookmark Path" context-menu item, and the 7
`bookmarks*` workspace settings (content struct, `Preferences`, schema, pages,
project overrides, `default.json`, v1→v2 migration, content-bridge). Dropped the
`("bookmarks","bookmarks")` entry from the statusbar-item id migration `ID_MAP`;
affected migration/placement tests reworded to use `jump-hosts`.

**Kept deliberately:** `IconName::Bookmark` + `bookmark.svg` (hosts "pin to top"
marker), `Workspace::known_hosts()` (command-palette host choices — comment
fixed), `Workspace::open_sftp_tab()` (other callers). Also removed the now-unused
`ShellPanels.explorer` field. `bookmarks.json` on disk is left untouched.

Gates green on this machine: `cargo fmt --check`, `cargo clippy --all-targets -D
warnings`, `cargo test` (0 failed).

---

## Previous Session: 2026-09-06 (Zed-parity icon system — full Lucide UI set + Zed icon-theme model)

Ad-hoc request (not a ROADMAP task; user: "einfach machen, keine Task erstellen").
All four gates green on this machine: `cargo fmt --check`, `cargo check
--workspace --all-targets`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` (0 failed). `cargo test -p labonair --test
smoke` green. App launches; chrome icons render (verified by screenshot, since
discarded). `scripts/check-crate-deps.sh` still reports the **3 pre-existing**
panel-explorer/panel-scm → settings/editor violations (present on HEAD with this
work stashed — not introduced here; no new `labonair-*` edges added).

**Two strictly separated icon systems, mirroring Zed** (`docs/architecture.md`
§8.19 rewritten):

1. **UI / chrome icons** — `labonair_ui_kit::IconName` regenerated to Zed's
   full `snake_case` set (~297 variants, one per `crates/shell/assets/icons/*.svg`
   — those SVGs were *already vendored* in the repo, so this was mostly a code
   change) + a 9-file `// + Labonair addition` block (`house`, `shield`,
   `square`, `palette`, `panel_{left,top,bottom}`, `arrow_down_up`,
   `circle_check`). Back-compat **alias assoc-consts** on `impl IconName`
   (`Search = MagnifyingGlass`, `X = Close`, `Minus = Dash`, `Refresh =
   RotateCw`, `Zap = BoltOutlined`, `Globe = Public`, `Type = Font`, …) keep
   every existing call site compiling — no sed across the 15 consumer crates
   (all uses are expression position; no `match` on `IconName` outside ui-kit).
   Deleted 33 obsolete kebab SVGs, renamed 5 kebab→snake.
2. **File / folder icons** — `crates/theme/src/icon_theme.rs` `IconThemeContent`
   moved to Zed's **two-level shape**: `file_stems`/`file_suffixes` → *icon key*
   → `file_icons` map → asset path (`icons/file_icons/rust.svg`). `directory` /
   `chevron` / new `named_directory_icons` are direct paths. New resolver
   `file_icon_path` / `directory_icon_path` / `chevron_icon_path` (return
   `&str` asset path; hard-fallback `icons/file_icons/file.svg`, never blank).
   `DEFAULT_FILE_STEMS` / `DEFAULT_FILE_SUFFIXES` / new `DEFAULT_FILE_ICONS`
   transcribed 1:1 from Zed's `FILE_STEMS_BY_ICON_KEY` /
   `FILE_SUFFIXES_BY_ICON_KEY` / `FILE_ICONS`; `assets/icon_themes/labonair.json`
   regenerated (`REGEN_BUILTIN_ICON_THEME=1`). `load_user_icon_themes` now also
   accepts a Zed-style family file (`{ name, author, themes: [...] }`).

Render path:
- `labonair_ui_kit::icon` — removed `file_icon` / `folder_icon` / `glyph_icon` /
  `chevron_icon` + the ~90-ext match table; added `file_icon_path` /
  `folder_icon_path` / `chevron_icon_path` / `icon_for_path` (→ `SharedString`
  path) + `svg_path(path, color)`.
- `TreeRow` / `GitChangeRow` gained `icon_path(Option<SharedString>)` /
  `chevron_path(…)` that win over the `IconName` setters.
- `panel-explorer` (tree + root row), `workspace/views/sftp.rs`,
  `settings-ui/panes/themes.rs` (icon-theme preview) resolve through
  `ThemeStore::icon_theme()`.
- `crates/shell/src/assets.rs` — `ICONS` hand-list replaced by `rust-embed`
  (`EmbeddedAssets`, new workspace dep `rust-embed = "8"`); new tests:
  `every_icon_variant_has_an_asset`, `no_dangling_ui_icon`,
  `builtin_icon_theme_paths_all_resolve`.

Not done / deviations from the approved plan:
- **No new `crates/icons` crate** — kept `IconName` in `ui-kit`, assets in
  `shell` (both already existed as a 2-crate split); avoids crate-graph +
  `check_crate_deps.py` churn.
- **Tab-bar / command-palette icons stay category glyphs** (Zed does the same
  on editor tabs) — not wired to per-file `file_icon_path`.
- License: `assets/icons/LICENSES` covers Lucide (ISC); Zed's `file_icons/*.svg`
  provenance noted in `docs/architecture.md` §8.19 (revisit if GPL-only).

Next task: resume the ROADMAP — **T20-007** (`theme_settings` metric layer) is
the next open item after Phase 19.

## Current Session: 2026-09-06 (App icon port from reference-src)

Ad-hoc request (not a ROADMAP task), committed on its own so it does not
entangle the parallel settings-ui work in the tree. **Not built this session**
(user asked to hold off on `cargo` runs while other work is in flight) — gates
still to run: `fmt --check`, `clippy -p labonair -D warnings`, `check -p labonair`.

Done — the reference Tauri app icon is now the port's app icon:
- **Vendored** `reference-src/src-tauri/icons/` → `crates/app/assets/app-icon/`
  (`icon.icns`, `icon.ico`, `32x32.png`, `128x128.png`, `128x128@2x.png`,
  `256x256@2x.png`). The 1024² master was dropped (nothing references it; the
  `.icns` is the packaging master).
- **macOS Dock / cmd-tab shim** `crates/app/src/dock_icon.rs`: `set_dock_icon()`
  loads `256x256@2x.png` via `NSData::with_bytes` → `NSImage::initWithData`
  (`mtm.alloc::<NSImage>()`) → `NSApplication::setApplicationIconImage`. Called
  first thing in the `Application::run` closure in `main.rs`. No-op on non-macOS.
  Needed because GPUI 0.2.2 has no runtime app-icon API and the project runs
  un-bundled.
- **New macOS deps on `crates/app`** (`[target.'cfg(target_os = "macos")']`):
  `objc2 = "0.6"`, `objc2-app-kit = "0.3"` (features `NSApplication`,
  `NSResponder`, `NSImage`), `objc2-foundation = "0.3"` (feature `NSData`) —
  all already in the lock tree via `gpui`/`muda`, only the feature set is
  re-declared.
- **`[package.metadata.bundle]`** in `crates/app/Cargo.toml` (cargo-bundle),
  mirroring the reference `tauri.conf.json` `bundle` block: identifier
  `com.labonair.app`, category `public.app-category.developer-tools`, the six
  vendored icons, `minimum_system_version = "10.15"`. A `cargo bundle` build
  picks up `icon.icns` for Finder / Dock / the standard About panel.

Not done / next: run the three gates above once the tree is free; consider a
`cargo bundle` smoke on macOS. In-app raster logo surfaces from the reference
(`app-icon.png` in the settings General section, terminal block empty state,
AI mini-window) were left alone — those screens have no 1:1 equivalent in the
port yet and adding them would be speculative UI.

## Current Session: 2026-09-06 (Settings UI-optimization pass — sidebar sub-nav, chrome trim)

Ad-hoc UI pass (not a ROADMAP task) on `crates/settings-ui/`. All four gates green
on this machine (`fmt --check`, `clippy --workspace --all-targets -D warnings`,
`test -p labonair-settings-ui` 37 ok / `-p labonair-ui-kit` 41 ok, `check --workspace`).

Done:
- **Two-level sidebar** (`view.rs`): each top-level `AREAS` row now has a
  disclosure chevron; expanding lists the page's section labels as sub-level
  scroll anchors. Row click = navigate + expand (`go_to_area`), chevron =
  `toggle_area_expanded`, sub-entry = `go_to_section` → `scroll_to_section`
  consumed in `render_generated_body`. New state: `expanded_areas`,
  `scroll_to_section`; new helper `section_labels_for_area`.
- **Static section headers** (`panes/generic.rs::render_section_header`): plain
  muted+semibold label with hairline, no disclosure/collapse. `render_jump_bar`
  (the image-#4 chip row) deleted; `toggle_section` removed.
- **Reset control**: icon-only `IconName::Refresh`, `Ghost`/`IconXs`, `c.muted`
  (was `Link`/"↺ reset", accent-coloured).
- **Header** (`render_header`): no in-window close button, left-padded clear of
  the traffic lights, "Edit in config.json" moved to the right edge.
- **Sidebar surface**: new `Palette` fields `sidebar` / `sidebar_fg` /
  `sidebar_border` (`ui-kit/src/palette.rs`) from `Theme::sidebar.*`; rail now
  painted on `--sidebar`, distinct from the `--card` content area.
- **Schema warnings suppressed**: only hard type/enum errors banner now;
  `TODO(notification-system)` in `view.rs` for routing unknown-/legacy-key
  warnings through the notification center (next focus after settings).
- Deviation from `docs/settings-guidelines.md` rule 1 recorded in
  `docs/architecture.md` §8.3 ("Deviation (UI-optimization pass, post-T21)").

Not done / next: per-field visual match to Zed (image #6) is only partial
(reset unified; spacing untouched); notification-system work for the suppressed
warnings. Changes uncommitted.

## Current Session: 2026-09-06 (Zed-parity redesign Phase 5 — accessibility, keyboard traversal, performance & contrast evidence)

### Phase 5 done (`docs/ui-comparison-zed-sidebar-status-bar.md` §13 Phase 5 / §14)

Polish + evidence only — no redesign, changes surgical. All four gates green on this
machine (`fmt --check`, `clippy --workspace --all-targets -D warnings`,
`test --workspace` — 50 `test result: ok.` lines, 0 failed, 0 environmental
failures).

**Keyboard traversal / focus model**
- `crates/workspace/src/status_bar.rs`: `StatusBar` gained a `FocusHandle`,
  `key_context("StatusBar")`, `track_focus`, and an `on_key_down` that maps
  **Left / Right Arrow → `window.focus_prev()` / `focus_next()`** and Escape →
  `window.blur()`. This is the status-bar toolbar/tab-group arrow loop that
  Phase 1 explicitly deferred (§6.4). Focus enters the bar by Tab (its controls
  are tab stops).
- `crates/shell/src/status_items.rs`: `simple_bar_button` now sets `.tab_index(0)`
  so the right-cluster info buttons join the same tab-group loop.
  `DockPanelButtons` already had `.tab_index(0)` + a tooltip carrying the
  resolved shortcut (Phase 1), so per-dock buttons were already reachable —
  Phase 5 only closed the *bar-level* arrow traversal.
- Explorer tree, Git change list, Changes/History tabs, context menus and the
  commit editor already carry their focus handles / `key_context` / keydown
  handlers from Phases 2–4; audit found no missing focus model there.

**Roles / names / toggled+expanded state / focus restoration**
- GPUI 0.2.2 has **no ARIA-role API**. Semantics are encoded with the mechanisms
  it does have: `key_context`, tab stops, focus-visible styling via the
  `Density::focus_indicator` token, and per-button tooltips that include the
  configured shortcut. A typed `TODO(a11y)` marks where a real toolbar/button
  role + accessible-name API would attach (`status_bar.rs`).
- Toggled state: panel buttons already render the active/pressed channel
  (`icon_toggle_button(pressed)`); expandable Explorer/SCM rows already carry the
  `expanded` flag in their row data. Menus/modals restore focus to the invoker
  through the existing `context_menu` dismiss + workspace focus handle.
- Only `explorer` (`⌘B`) and `ai` (`⌘I`) have a bound shortcut in
  `command-palette/src/keybind.rs`; the other panel buttons correctly show the
  title alone (no shortcut is *configured*, so none is invented).

**Perf spans (new hot paths from Phases 2–4)**
- `panel-explorer`: `explorer_flatten` (on `flatten_rows`), `explorer_decorate`
  (`decorate_rows`), `explorer_fold_chains` (`fold_chains`),
  `explorer_viewport_build` (inside the `uniform_list` render closure, records
  `built` vs `total`).
- `panel-scm`: `scm_flatten_flat` (`flatten_git`), `scm_flatten_tree`
  (`flatten_git_tree`), `scm_viewport_build` (inside the `uniform_list` closure).
- `workspace`: `render` span with `view = "project_diff"` on
  `ProjectDiffView::render`.
- Reproduce: `RUST_LOG=labonair::perf=trace cargo run`, then scroll the Explorer
  / open Source Control on a large repo. `*_flatten*` fires once per model
  change; `*_viewport_build` fires per frame with `built` == visible rows (never
  the total).

**Perf benchmark tests (§14 "no row-count-linear render regression")**
- `panel-explorer` `flatten_on_a_large_tree_is_bounded_and_geometry_stable`:
  20 000 synthetic rows → `flatten_rows` is 1:1 (no fan-out), `decorate_rows`
  never changes the row count, exactly one row each carries selection /
  active-file (decorations don't smear).
- `panel-explorer` `virtual_list_builds_only_the_viewport_not_the_whole_model`:
  replays the `uniform_list` render closure's work over a 48-row window at
  three scroll offsets in a 20 000-row model and asserts touched-row count == 48
  every time.
- `panel-scm` `flatten_git_on_a_large_change_set_is_bounded_and_viewport_render_is_not`:
  5 000 changes → `flatten_git` len == N+1, every file appears once in both flat
  and tree flatteners, viewport replay touches 40 rows regardless of offset.

**Theme contrast validation**
- New `crates/theme/src/contrast.rs`: promoted the previously test-private
  `luminance`/`contrast` helpers to a public API
  (`relative_luminance`, `contrast_ratio`, `composite_over`), re-exported from
  `labonair_theme`. `tokens.rs` now reuses `contrast_ratio` (surgical — its
  existing body-text AA test is unchanged).
- New test `redesign_surfaces_meet_contrast_in_every_builtin_theme`: iterates
  `Theme::light()` + `Theme::dark()` (the built-in set — `assets/themes/labonair.json`
  is regenerated from these) and checks every new surface — focus indicator,
  right-edge active bar, sticky-ancestor hairline, indent guides, tri-state
  checkbox (incl. the 0.4-opacity partial fill via `composite_over`),
  `GitChangeRow` status tints, drop-target ring — against the background it
  actually sits on. Collects *all* violations before failing.
- **Known deviation (light theme):** the reference's `--ring` and `--primary`
  are a light gold (`--ring: oklch(79.68% 0.1298 82.18)`, identical in `:root`
  and `.dark`). Gold on the near-white light background is inherently ~1.75:1,
  so the light-theme focus ring / active bar / drop ring do **not** reach the
  3:1 UI-component AA floor. The **dark** theme (macOS-first default) meets every
  real threshold. Changing the tokens would violate Critical Rule 3, so the
  light floors are pinned to the reference's achieved values — the test guards
  against a regression *below* the reference and documents the gap. Custom
  themes are validated at import time by reusing the same public helpers.

**Density-mode + narrow-dock validation**
- `ui-kit` `tree_row::…::builds_at_every_density_and_deep_indent` +
  `git_change_row::…::builds_at_every_density_and_deep_indent`: build the rows at
  compact / default / comfortable density (0.85 / 1.0 / 1.15) and depths up to 20,
  with a long label. Rows are `w_full` + ellipsised, so a minimum-width dock
  cannot force horizontal overflow — asserted structurally (build tests, not
  pixel snapshots). `density.rs` already pins hairline/focus at fixed 1–2px
  across densities; re-asserted here (`focus_indicator == px(2.0)`).

**§14 acceptance status:** every "Quality gates" bullet passes. Status-bar /
Explorer / Source Control functional bullets were met by Phases 1–4 and are
preserved; Phase 5 adds the keyboard-traversal, evidence and contrast bullets.
The single **partial** item is light-theme focus/active/drop contrast vs the 3:1
AA floor (reference-token limitation, documented above; dark theme fully meets
it).

---

## Zed-parity redesign — phases 1–5 complete

| Phase | Commit | Scope |
|---|---|---|
| 1 | `dd15f45` | Per-dock `DockPanelButtons` status items; generic dock header removed; overlaid resize handle + double-click reset; `StatusBar`. |
| 2 | `8280b16` | `ui-kit` `TreeRow` / `GitChangeRow` / `Density`; Explorer + SCM virtualized (`uniform_list`). |
| 3 | `1b9855e` | Explorer content-first: compact root row, project-wide search, sticky ancestors, indent guides, auto-reveal, single-child folding, Git decorations, preview vs permanent open. |
| 4 | `719634d` | Git IA: Changes/History tabs, tri-state checkbox staging, workspace-level `ProjectDiffView`, adaptive commit composer, derived-state enums. |
| 5 | _this commit_ | Accessibility + keyboard traversal (status-bar arrow loop), perf spans + large-project benchmark tests, theme contrast validation, density/narrow-dock build tests. |

**Acceptance (§14):** Quality gates — all pass (`fmt --check`,
`clippy --workspace --all-targets -D warnings`, `test --workspace`, no
row-count-linear render regression proven by the new benchmarks, no main-thread
I/O introduced). Status bar & docks / Explorer / Source Control — met.
**One partial:** light-theme gold focus/active/drop-target contrast does not
reach the 3:1 non-text AA floor because the reference's `--ring` / `--primary`
tokens are a light gold (Critical Rule 3 forbids changing them); the dark theme
meets every threshold.

## Previous Session: 2026-09-06 (Zed-parity redesign Phase 4 — Git information architecture)

### Phase 4 done (`docs/ui-comparison-zed-sidebar-status-bar.md` §13 Phase 4)

- **Panel navigation**: panel-owned `Changes | History` tab bar (`segmented_control`,
  `PanelMode`). History = commit list via `git_get_log` (100), read-only (Git Graph
  is untouched).
- **Adaptive Changes header** (`render_changes_header`): repo-level tri-state
  checkbox (`stage_all`/`unstage_all`), `View Diff` + `{n} files`, view-options
  `⋮` menu (Flat/Tree, Collapse/Expand all), adaptive `Stage All`/`Unstage All`,
  overflow `▾` menu (Refresh, Stashes…, Discard All…, Clean… — destructive items
  `.destructive()` + routed through an in-panel confirm bar `PendingConfirm`).
- **Change rows**: section- and repo-level tri-state checkboxes added on top of
  the existing file-level ones (`stage_section`, `stage_paths`/`unstage_paths`
  run one sequential op). `GitListEntry::Directory` added; new pure
  `flatten_git_tree` (tree) alongside `flatten_git` (flat) over one model —
  setting `scmFileTree` (default `false`), runtime toggle in the view menu.
  Files keep a muted full-path `secondary`; `short_path` is now the basename +
  `dir_prefix` helper.
- **Diff flow**: inline 280px viewer removed. New neutral
  `labonair_panel::ProjectDiffRequest` (`repo_root`, `session_id`,
  `Vec<ProjectDiffFile>`, `selected`, `mode`). `GitPanelView` is
  `EventEmitter<ScmEvent>`; `select_file` / `View Diff` emit
  `ScmEvent::OpenProjectDiff`. Shell (`bootstrap.rs`) subscribes and calls
  `Workspace::open_project_diff` which opens/focuses the single `TabKind::GitDiff`
  tab backed by new `crates/workspace/src/views/project_diff.rs`
  (`ProjectDiffView`: file rail + unified/split hunks + per-hunk stage/unstage
  through the backend). Idempotent (same pattern as `open_git_graph_tab`).
  Unified-diff parsing moved to `labonair_editor::unified` (shared by panel-scm
  re-export + workspace).
- **Repo footer + commit composer**: `render_branch_bar` → `render_footer`
  (branch, ahead/behind, op-progress text, one `⋯` repo menu holding
  Fetch/Pull/Push/Publish + separated `Force Push…` (destructive, confirmed) +
  Branches & Tags… + Stashes…). Hand-rolled commit field → editor-backed
  `InputState` composer (`ensure_commit_input`, ⌘↵ commits); adaptive button
  label from `CommitMode` (`Commit`/`Commit Tracked`/`Amend`), `DisabledReason`
  in tooltip, title-length (>72) warning without an error state, `Amend`
  checkbox, `⤢` in-panel expand. `commit_focus` field removed.
- **Derived-state enums** (§12.5): `CommitMode`, `RepoOperation`
  (`Idle`/`Fetching`/`Pulling`/`Pushing`/`Mutating`), `DisabledReason` +
  pure `derive_commit_mode` / `derive_disabled_reason`. `run_op_kind(kind, …)`
  tags each async op; fetch/pull/push carry their identity.
- **Settings**: `scmFileTree` (`FileManagerContent`, default `false`) + `ScmSettings`
  concrete + schema/pages row + migrate_v2 + default.json.
- **Tests**: commit-mode + disabled-reason matrices; tree↔flat same file set +
  Directory tri-state + collapse; `resolve_selection` (Project Diff focus /
  re-point). Editor `unified` module carries the moved hunk-parsing tests.
- **Deferred / deviations**: History rows read-only (no per-commit diff);
  commit "expand" is in-panel, not a separate modal editor window; view-options
  menu covers tree/flat + collapse only (grouping/sorting not added — model is
  flat status buckets, path-sorted); directory-collapse key is not
  section-scoped (collapsing `src` hides `src/*` in every section).

### Phase 3 done (`docs/ui-comparison-zed-sidebar-status-bar.md` §13 Phase 3)

### Phase 3 done (`docs/ui-comparison-zed-sidebar-status-bar.md` §13 Phase 3)

- **Toolbar → compact root row** (`panel_explorer.rs` render): the 5 permanent
  icons collapse to `root identity + Search + one `…` overflow`. Overflow menu =
  New File / New Folder / Refresh / Show-Hide Hidden Files (all still in the
  tree context menu too). Search reachable via its toggle/shortcut.
- **Project-wide search** (`bounded_fs_search`, `spawn_search_walk`): typing a
  query kicks a bounded (`depth ≤ 8`, `≤ 4000` dir reads) blocking FS walk on
  `background_executor`; hits stream into `search_hits` and merge (dedup by
  path) with the loaded-row matches in `search_rows`. Transient "Searching
  project…" hint; overlay + hits cleared when search closes.
- **Sticky ancestors** (`sticky_ancestor_indices`, pure): open-dir chain for the
  first visible row (derived from the `UniformListScrollHandle` offset ÷ row
  height), pinned above the list with a bottom hairline. Setting
  `explorerStickyAncestors` (default **on**).
- **Indent guides**: `TreeRow::indent_guides(bool)` draws 1px vertical guides
  per depth as an absolute overlay (no row-height change). Setting
  `explorerIndentGuides` (default **on**).
- **Active-file auto-reveal** (`reveal_active_file`, `reveal_target_index`):
  `cx.observe(&workspace)` → when `explorerAutoRevealActiveFile` (default
  **off**) is set, expand+load the file's ancestors, mark it (`active_file`
  channel), queue a `scroll_to_item(Center)` serviced in `render`.
- **Single-child folding** (`fold_chains`, pure, reversible): compresses
  `a/b/c` chains into one row labelled `a/b/c` keeping the real deepest path.
  Setting `explorerFoldSingleChildDirs` (default **off**). Skipped during
  search.
- **Git decorations** (`parse_git_porcelain`, `poll_git`): local roots poll
  `git status --porcelain` off-thread on root change + watcher drain; merged
  into `ExplorerRowData` at `decorate_rows` as a label tint + trailing status
  letter (no row-height change). Remote roots degrade gracefully (no
  decorations, same geometry). Setting `explorerGitDecorations` (default
  **on**). **Diagnostics**: typed `DiagnosticSeverity` hook + empty provider +
  TODO — no in-repo diagnostic source yet.
- **Preview vs permanent open**: explicit — single-click / `Space` = preview
  (peek tab), double-click / `Enter` = permanent. Directories toggle on either.
- **Settings**: 5 new `FileManagerContent` fields + `default.json` + `schema.rs`
  rows + `pages.rs` "Explorer tree" group + new concrete `ExplorerSettings`
  (`crates/settings/src/concrete.rs`, registered). Density-neutral defaults
  preserve the current look except guides/sticky which default on for parity.
- Tests: `sticky_ancestor_indices`, `fold_chains` (compress + multi-child
  no-op + purity), `reveal_target_index`, `decorate_rows`,
  `parse_git_porcelain`, `bounded_fs_search` (deep unloaded hit + depth cap);
  `ExplorerSettings` registration. Gates green: `fmt --check`,
  `clippy --workspace --all-targets -D warnings`, `check --workspace`,
  `test -p labonair-panel-explorer -p labonair-ui-kit -p labonair-settings
  -p labonair-settings-content -p labonair-settings-ui -p labonair-backend`.
- **Deferred**: diagnostics source (hook only); a shared git-status provider
  across Explorer + SCM (Explorer runs its own local `git` poll for now); the
  recursive search walk is bounded (see caps above), not exhaustive.

## Previous Session: 2026-09-05 (Zed-parity redesign Phase 2 — dense rows + virtualization)

### Phase 2 done (`docs/ui-comparison-zed-sidebar-status-bar.md` §13 Phase 2)

- **New ui-kit primitives** (`crates/ui-kit/src/`): `density.rs` (`Density` —
  semantic tokens: tree row 24 / section header 22 / control 22 / row gap 4 /
  status-bar pad 4 / panel header 28 / dock resize target 6, all ×density;
  hairline + focus-indicator fixed at 1–2px), `tree_row.rs` (`TreeRow` /
  `TreeRowState` — flat full-width, no radius, independent selected/marked/
  focused/active_file/cut/drag_source/drop_target channels, ≤2px right-edge
  indicator, path tooltip, `.extra()` drag/drop escape hatch), `git_change_row.rs`
  (`GitChangeRow` / `StageState` tri-state checkbox independent of the semantic
  status tint, hover-revealed actions via `group_hover`). `ListItem` untouched.
- **Explorer virtualized**: pure `flatten_rows(rows, expanded, selection, cut,
  drop_target) -> Vec<ExplorerRowData>` (§12.4, no FS access), rendered through
  `uniform_list` + free `explorer_row_element`; every handler goes via
  `view.update(..)`. All behaviour preserved (multi-select, kbd nav, preview/
  open, internal+external DnD, cut/copy/paste, create/rename/delete/duplicate/
  bookmark, hidden filter, context menus, confirm flows, local+remote roots,
  lazy load, Loading/Error/LoadMore/PendingCreate/Rename pseudo-rows now all at
  the uniform 24px row height). Search now walks `TreeModel::all_loaded_rows()`
  (every loaded dir, not just expanded/on-screen); still bounded by lazy load.
- **SCM virtualized**: pure `flatten_git(sections, collapsed, selected) ->
  Vec<GitListEntry>` (`SectionHeader` / `File` / `EmptyState`; `aggregate_stage`
  tri-state) + `uniform_list` + free `git_list_element` rendering `File` rows as
  `GitChangeRow`. `render_section` / `row_action` removed; backend
  stage/unstage/discard/select wrapped, not rewritten. Inline diff, branches,
  stashes, tags, commit form all unchanged.
- Tests: ui-kit density/tree_row/git_change_row build tests; explorer flatten
  (depth/expansion/loading/selection across ranges, cut+drop-target, search
  spans collapsed subtrees); scm `aggregate_stage` + `flatten_git`
  (headers/files/aggregate staging, collapse, empty state).
- Gates green: `fmt --check`, `clippy --workspace --all-targets -D warnings`,
  `test -p labonair-ui-kit -p labonair-panel-explorer -p labonair-panel-scm
  -p labonair-theme` (39 / 12 / 19 / 63).
- **Deferred (Phase 3/4/5):** sticky ancestors, indent guides, auto-reveal,
  auto-fold, Changes/History tabs, tree/flat Git grouping (`Directory` entry),
  Project Diff, a11y/perf audit. `GitListEntry::{Directory,Loading,Error}` from
  §12.5 not added (would be dead code this phase).

## Previous Session: 2026-09-05 (T21-001 — render-path profiling & frame hygiene, in progress)

### Visual parity audit vs reference-src + hover-fill fix (committed)

- Performed a full visual-design comparison of `reference-src/src/styles/globals.css`
  and the component chrome against the GPUI port. Findings: the **token layer is a
  genuine 1:1 port** (all oklch colors incl. ANSI-24, radii 3–13px + window 12,
  animation durations/easings, font sizes 13/14/13). Component-layer gaps cluster
  into four themes: (1) **border opacity** — the reference uses `border-border/40..60`
  everywhere; the port renders full-strength `border` (titlebar, statusbar, explorer
  header, SCM bar); (2) **shadows dead code** — `Theme.shadows` tokens are never applied;
  popovers use GPUI's hardcoded `shadow_lg()`; (3) **text metrics** — buttons lack
  `font-medium`, tabs 12 vs 14px, explorer rows 14 vs 13px; (4) **explorer deltas** —
  indent base 8 vs 6, hover/selected fill `border` vs reference `accent`, content-height
  header vs fixed 32px. Also: no `Badge` primitive ported; `indicator.tsx` does not
  exist in the reference (dots are inline `size-1.5`).
- Fixed one slice of gap (4) in-flight: `ListItem` gained `hover_style(f)`
  (custom hover fill without the GPUI "hover style already set" panic); host list
  row (accent-border hover) and SCM file rows (border hover / accent selected) migrated
  onto it. Committed `864fae0`.
- Gates: `fmt --check`, `check -p ui-kit/hosts-ui/panel-scm`, `clippy --all-targets
  -D warnings`, `test` (19+16+34 passed) all green.
- **Not acted on:** the remaining parity gaps are real but are not formal roadmap tasks.
  Awaiting a decision whether to log/fix them as work or continue the roadmap.

### Zed sidebar/status-bar source comparison

- Added `docs/ui-comparison-zed-sidebar-status-bar.md`, a source-level comparison
  of Labonair and the pinned Zed snapshot for dock chrome, status-bar panel
  switching, Explorer/Project Panel, and Source Control/Git Panel.
- The report includes exact source anchors, design diagnosis, a clean-room
  module/trait/state map, phased implementation order, tests, and acceptance
  criteria. It does not change product behavior.
- Direct source copying is not proposed: the inspected Zed `workspace`,
  `project_panel`, `git_ui`, and `ui` crates are GPL-3.0-or-later while
  Labonair is Apache-2.0. The plan independently reimplements observable UI
  patterns on Labonair's existing APIs.
- **Next remains:** complete the graphical macOS evidence required by T21-001;
  the UI redesign report is planning input and does not supersede roadmap order.

### Explorer sidebar visual alignment

- `crates/panel-explorer/src/panel_explorer.rs` now mirrors the reference
  Explorer header: folder glyph and stronger root label, search/new-file/
  new-folder/refresh/hidden-files control order, and compact matching icon
  sizes. The obsolete collapse control was removed.
- Added the native, focusable 28 px "Search files…" strip with clear action
  and a case-insensitive filter over the lazily loaded tree rows; this keeps
  the existing filesystem loading model intact and provides the reference's
  empty-results state.
- Verification: `cargo fmt --check`, `cargo check -p labonair-panel-explorer`,
  `cargo clippy -p labonair-panel-explorer --all-targets -- -D warnings`, and
  `cargo test -p labonair-panel-explorer` (9 passed) are green.

**T21-001 is in progress.** The code-side render hygiene and opt-in profiling
surface are implemented, but the task must remain open until its graphical
profiling evidence is captured on macOS.

- **Configuration filename refactor (2026-09-05):** the shared user
  configuration is now `~/.config/labonair/config.json`. Startup adopts an
  existing `labonair-settings.json` only when `config.json` is absent, before
  settings loading and schema migration. Every native reader/writer, raw JSON
  command, watcher, user-visible diagnostic, and related test now uses the
  new name. `cargo fmt --check`, `cargo check --workspace --all-targets`, and
  `cargo clippy --workspace --all-targets -- -D warnings` pass. The focused
  filename-adoption test passes. Full backend/settings test packages are
  blocked here by pre-existing macOS Keychain/file-watcher sandbox failures.

- `labonair::perf` trace spans now cover shell/titlebar/workspace/status-bar,
  Explorer/SCM/Git Graph/Snippets/AI panels, settings recomputation, active
  theme recomputation, and live-snapshot recomputation. Enable with
  `RUST_LOG=labonair::perf=trace cargo run`.
- Workspace skips empty deferred-operation queues; `WorkspaceLiveBridge`
  publishes a snapshot only if it actually changed. The new regression test
  `identical_snapshot_is_an_idle_noop` passes.
- `docs/perf-baseline.md` documents expected render ownership and the exact
  host-side trace/heap capture procedure. No made-up frame or allocation data
  was recorded on this headless runner.
- Verification: `cargo fmt --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings` are green. The
  full workspace suite reaches one expected sandbox restriction: the
  `labonair-ai::client::tests::end_to_end_streams_openai_sse_over_http` test
  cannot bind a loopback socket there (`Operation not permitted`). The same
  test passes when run outside the sandbox (1 passed), confirming the project
  test itself is green.
- **Next:** finish T21-001 by taking 10-second idle plus tab-switch and
  terminal-typing traces/heap samples on a graphical macOS host, add the
  measured values, rerun the full test suite where loopback binding is allowed,
  then mark the task done. T21-002 follows after that.

## Last Session: 2026-09-05 (T20-007 — `theme_settings` metric layer)

**T20-007 done.** UI density, font scales, corner-radius scale and
reduce-motion are now a coherent **metric layer** that scales the active
`Theme` at runtime — Zed's `theme_settings` pattern. `ActiveTheme` = colour
(from the `ThemeRegistry`) + metric (from settings), recomputed only on a real
change of either.

- **`crates/theme/src/theme_settings.rs`** (new, ~330 lines incl. 5 tests):
  `UiDensity` (`Compact` ×0.85 / `Default` ×1.0 / `Comfortable` ×1.15,
  `from_str_or_default`/`as_str`/`ALL`); `ThemeMetrics { ui_font_family,
  ui_font_size, ui_line_height, buffer_font_family, buffer_font_size,
  buffer_line_height, density, corner_radius_scale, reduce_motion }` (default =
  historical `Typography` defaults, so a store with no `set_metrics` renders
  as before) + `space(px)` / `scaled_radius(base)`; `ActiveTheme { colors:
  Theme, metrics }` with cached `radius` (`corner_radius_scale`d) + `animation`
  (durations → `Duration::ZERO` when `reduce_motion`); `GlobalActiveTheme`.
- **`RadiusScale::scaled(factor)`** — new (tokens.rs); every corner value ×f,
  `window` fixed.
- **`ThemeStore`** gained `metrics` + cached `active` fields, `metrics()`,
  `active_theme()`, `set_metrics()`, private `rebuild_active()` (called from
  `rebuild_custom` + the 4 preview/clear paths + `new`). `radius()` /
  `animation()` now return the metric-scaled values. `init_theme` installs a
  `cx.observe(&store)` that mirrors `active` into `GlobalActiveTheme` (fires on
  notify only — never per frame). New gpui test
  `active_theme_recomputes_on_colour_and_on_metric_change` (note: `cx.observe`
  effects flush at the end of a `cx.update` block, not between statements —
  the test splits updates to see the global sync).
- **`labonair-ui-kit`** — `theme.rs`: `UiTheme::metrics()` (default
  `ThemeMetrics::default()`, overridden on `ThemeStore` → live); `ThemeStore`'s
  `UiTheme::radius()` now returns `active_theme().radius()`. New
  `ActiveThemeExt` (`cx.active_theme()` off `GlobalActiveTheme`), exported +
  in prelude. `Palette` gained `density: f32` + `Palette::space(px) -> Pixels`;
  `radius` is now the scaled scale. Nine Palette-primitives (`button`,
  `checkbox`, `context_menu`, `banner`, `kbd`, `number_field`, `segmented`,
  `select`, `toggle`) route every spacing/size `px(..)` through `space()`.
  Kept literal: `text_size`, icon glyph `.size()`, 1px hairlines.
  Exceptions (bare-colour param, no density channel; deferred): `list`,
  `disclosure`, `indicator`, `divider`.
- **`AppearanceContent`** — new `buffer_font_family` / `buffer_font_size` /
  `buffer_line_height` / `ui_density` / `corner_radius_scale` (+ `default.json`,
  `migrate_v2::appearance_from` `..Default::default()` — no legacy `Preferences`
  key). `appCornerRadius` (px) retained as `_legacy`: a non-default value →
  `cornerRadiusScale = px / 5` once, read-time, in `ThemeSettings::from_settings`.
- **`ThemeSettings`** (`concrete.rs`) — new `ui_font_*`/`buffer_*`/`ui_density`/
  `corner_radius_scale` accessors + the legacy-migration test.
- **`labonair-settings-ui`** — `apply.rs`: `theme_metrics_from_settings(cx)` +
  `apply_theme_metrics(theme, cx)`; `apply_prefs_to_theme` now also pushes
  metrics. `schema.rs`: 5 new `field!` rows (`uiDensity` = `Select`,
  `cornerRadiusScale`/`bufferLineHeight` = `Float`, `bufferFontSize` = `Int`,
  `bufferFontFamily` = `FontFamily`). `pages.rs`: Appearance groups gain the
  buffer-font rows + a new "Density & Motion" group.
- **`crates/shell/src/bootstrap.rs`** — `cx.observe_global::<SettingsStore>` →
  `apply_theme_metrics` (generated `appearance` fields bypass
  `apply_prefs_to_theme`).
- **`crates/workspace/src/workspace.rs`** — tab-in animation reads
  `reduce_motion` + timings off `active_theme()`; clamps a zero `dur_base` to
  10µs (GPUI `with_animation` divides by the duration).
- **Deviations** (all in `docs/architecture.md` §8.20): `ActiveTheme.colors` is
  `Theme` by value not `Arc`; `GlobalActiveTheme` is a mirror; metrics live in
  the `appearance` area (no new `theme` area), `ThemeSettings` name kept;
  `uiDensity` is a generated `Select` not a bespoke `SegmentedControl`;
  primitive migration limited to `Palette`-taking primitives;
  editor/terminal font-pref consolidation deferred (`buffer_*` seeded at editor
  defaults, terminal keeps `FontOverrides`).
- **Gates:** `fmt --check`, `check --workspace --all-targets`, `clippy
  --workspace --all-targets -D warnings`, `test --workspace` (906 passed /
  0 failed), `scripts/check-crate-deps.sh` (24 crates, acyclic) — all green.
  `labonair-theme` gains **no** new dependency.
- **Not `cargo run`-verified** (headless) — live density/font/radius/motion
  visual check open (same accepted gap as T20-002..006).
- **Next task: first task of Phase 20** (`tasks/phase-20-perf-signoff/`) —
  T21-001 (render-path profiling & frame hygiene). Phase 19 (UI-Kit &
  Theme-System) is now complete.

## Previous Session: 2026-09-05 (T20-006 — Icon themes, JSON, switchable)

**T20-006 done.** The hard-coded `file_icon` extension table is now a
swappable **JSON icon theme** (embedded built-in + user `*.json`), runtime-
switchable with fs-watch live-reload, mirroring the T20-005 theme registry.

- **`crates/theme/src/icon_theme.rs`** (new, ~640 lines incl. 6 tests):
  `IconThemeContent { name, author?, file_stems, file_suffixes,
  directory:{collapsed,expanded}, chevron:{collapsed,expanded}, default_file }`
  (`serde` only — **no `JsonSchema`**, same zero-dep-leaf reasoning as
  T20-005). `file_glyph(name)` = `file_stems` (whole name, case-folded) →
  **longest** dot-suffix (`archive.tar.gz` → `tar.gz` before `gz`) →
  `default_file`. `IconThemeRegistry`: `builtin()` (embedded
  `assets/icon_themes/labonair.json`), `load_user_icon_themes(dir)` (skip
  broken → `Vec<String>` warnings), `list() -> Vec<IconThemeMeta>`,
  `get(id)` / `contains(id)`, `builtin_theme()`. Ids: file stem; `"default"`
  = built-in. `DEFAULT_FILE_STEMS` / `DEFAULT_FILE_SUFFIXES` are the 1:1
  transcription of the old `file_icon` arms + the single source for the
  embedded asset (regen: `REGEN_BUILTIN_ICON_THEME=1 cargo test -p
  labonair-theme builtin_icon`).
- **`crates/ui-kit/src/icon.rs`** — `icon_enum!` gains `IconName::from_glyph_id`.
  New `glyph_icon(id)` (unknown → `IconName::File` + one-time `eprintln`),
  `icon_for_path(theme, name, is_dir, is_expanded)`, `chevron_icon(theme,
  is_expanded)`. `file_icon` / `folder_icon` kept as thin wrappers over a
  `LazyLock<IconThemeRegistry>` built-in. Old hard-coded `file_icon` body
  moved to `#[cfg(test)] legacy_file_icon` for the equality test
  (`builtin_icon_theme_matches_legacy_file_icon`: every single-segment key).
- **`ThemeStore`** gained `icon_registry` + `active_icon_theme` +
  `icon_registry()`, `list_icon_themes()`, `active_icon_theme_id()`,
  `icon_theme() -> &IconThemeContent`, `set_active_icon_theme(id) -> Result`,
  `reload_user_icon_themes(dir) -> Vec<String>`. 1 new gpui test
  (activate + unknown-id + live-reload-fallback).
- **`appearance.icon_theme`** — new `Preferences` (`iconTheme`, default
  `"default"`) + `AppearanceContent` field; wired through `content_bridge.rs`,
  `migrate_v2.rs` (+ its accounting test + `default.json`).
- **`labonair-settings-ui`** — `apply.rs`: `apply_prefs_to_theme` also
  reloads the icon registry + `set_active_icon_theme(p.icon_theme)`; new
  `reload_icon_theme_registry`, `user_icon_themes_dir`, `icon_theme_choices`.
  Themes pane: `render_icon_theme_picker` (SegmentedControl over installed
  icon themes + a live 6-glyph preview row resolved through the *selected*
  theme + Import / Open-folder), `set_icon_theme` / `import_icon_theme[_from]`.
  `refresh_themes` also refreshes icon themes. `SettingsView` gained
  `icon_theme_files` + `active_icon_theme_id`.
- **`crates/shell/src/bootstrap.rs`** — second `watch_dir(user_icon_themes_dir(),
  …)` → `reload_icon_theme_registry`.
- **Call sites** — `panel-explorer` tree row + `workspace/views/sftp.rs` row
  resolve glyphs (file, folder, **and** chevron for explorer) through
  `ThemeStore::icon_theme()` / `icon_for_path`. Tab-title / command-palette
  don't call `file_icon` directly — nothing to migrate there.
- **Deviations (documented `docs/architecture.md §8.19`, task file):** no
  `JsonSchema` derive; `.env*` prefix rule narrowed to explicit `.env*`
  stems; `d.ts` + `tar.gz`-family multi-segment suffix keys added.
- **Gates:** `fmt --check`, `check --workspace --all-targets`, `clippy
  --workspace --all-targets -D warnings`, `test --workspace` (898 passed,
  0 failed), `scripts/check-crate-deps.sh` (24 crates, acyclic) — all green.
  `labonair-theme` gains **no** new dependency.
- **Not `cargo run`-verified** (headless) — user visual check of live icon-
  theme switching / broken-file-ignored is open (same accepted gap as
  T20-002..005).
- **Next task: T20-007** (`theme_settings` layer; `tasks/phase-19-ui-kit/`).

## Previous Session: 2026-09-05 (T20-005 — ThemeRegistry + JSON theme families)

**T20-005 done.** The T02-003 "one built-in light/dark + optionally one
imported custom theme" model is now a **registry of theme families**, each
with named light/dark variants, from embedded + user JSON, runtime-switchable
with fs-watch live-reload.

- **`crates/theme/src/registry.rs`** (new, ~440 lines incl. 7 tests):
  `ThemeFamilyContent { name, author?, themes: [ThemeVariantContent { name,
  appearance: "light"|"dark", colors: BTreeMap<token,String> }] }` —
  `from_json` accepts the new array shape **and** the legacy `ThemeFile`
  (`variants` map) via `impl From<&ThemeFile>`, so existing user files still
  load. `ThemeRegistry`: `builtin()` (embedded `assets/themes/labonair.json`),
  `load_user_themes(dir)` (skip broken → `Vec<String>` warnings, never empty),
  `list() -> Vec<ThemeMeta { family_id, family, variant_name, appearance,
  builtin }>`, `get(id)` / `resolve(id, appearance)` →
  `Result<Theme, ThemeNotFoundError>`, `resolve_family_variant(family,
  appearance, Option<variant>)` for the per-mode override, `family_of(id)`.
  Ids: `"<file stem>/<variant name>"`; `"default"` = built-in family id
  (`BUILTIN_FAMILY_ID`, kept as the historical `appTheme` sentinel). Omitted
  tokens inherit the same-appearance built-in default via the existing
  `import::set_token` (made `pub(crate)`).
- **`assets/themes/labonair.json`** — full-color both variants, generated from
  `tokens.rs` via `ThemeFamilyContent::from_themes`; regen with
  `REGEN_BUILTIN_THEME=1 cargo test -p labonair-theme builtin_json`. Equality
  test asserts ±1/255 per channel vs `Theme::light()/dark()`.
- **`ThemeStore`** gained `registry`, `active_family`, `registry_variant`
  fields + `registry()`, `list_themes()`, `active_theme_id()`,
  `set_active_theme(id) -> Result`, `set_registry_variant(Option<name>)`,
  `registry_variant()`, `reload_user_themes(dir) -> Vec<String>`,
  `preview_registry_theme(id)`. `reresolve_custom()` now also resolves
  `active_family` through the registry into the same `custom`/`custom_base`
  slot the legacy import path uses (so mode-follow + font-overrides keep
  working). Legacy `set_custom_theme` / `import_theme_file_variant` /
  `clear_custom_theme` now clear `active_family` so the two paths don't fight.
  2 new gpui tests (registry activate+mode-follow+fallback; live swap + vanished
  family → built-in).
- **`labonair-settings` `watch_dir(cx, dir, cb)`** — new sibling of
  `watch_file`, filters events on the `.json` extension, `create_dir_all`s the
  dir. Exported from `settings.rs`.
- **`labonair-settings-ui`** — `apply.rs`: `apply_prefs_to_theme` now reloads
  the registry from `themes_dir()` + `set_active_theme(p.app_theme)` +
  `apply_stored_theme_variant` (override keyed by **family id** now, was theme
  id); new public `reload_theme_registry` + `user_themes_dir`. `theme_choices`
  / `scan_themes` → one `ThemeEntry` per non-builtin registry variant
  (`id = "<stem>/<variant>"`, `name = "<family> — <variant>"`), `"default"`
  first. `preview_app_theme` → `ThemeStore::preview_registry_theme`.
  `panes/themes.rs`: `refresh_themes(cx)` (was no-arg) also reloads the store
  registry; `activate_theme`/`delete_theme`/`import_theme_from`/
  `apply_stored_variant`/`set_theme_variant`/`render_variant_picker` rewired
  onto `set_active_theme` / `set_registry_variant` / `registry().family_of` /
  `registry().family_variants`. `read_theme_file_in` + its `ThemeFile` import
  are now `#[cfg(test)]` (only the test module still uses them).
- **`crates/shell/src/bootstrap.rs`** — after `apply_prefs_to_theme`, a
  `labonair_settings::watch_dir(user_themes_dir(), …)` → `reload_theme_registry`
  so a dropped/edited/removed `*.json` re-resolves the active theme with no
  restart. `labonair-theme` gains **no** `notify` dep.
- **Deviations (documented `docs/architecture.md §8.18`, task file):** no
  `JsonSchema` derive on `ThemeFamilyContent` (zero-dep leaf crate);
  `set_active_theme` does not write `appearance.app_theme` itself (persistence
  stays in settings-ui — `labonair-theme` ↛ `labonair-settings`).
- **Gates:** `fmt --check`, `check --workspace --all-targets`, `clippy
  --workspace --all-targets -D warnings`, `test --workspace` (0 failures),
  `scripts/check-crate-deps.sh` (24 crates, acyclic) — all green.
- **Not `cargo run`-verified** (headless) — user visual check of live theme
  switching / System-appearance variant pick / broken-file-ignored is open.
- **Next task: T20-006** (Icon-Themes — JSON, switchable; `tasks/phase-19-ui-kit/`).

## Previous Session: 2026-09-05 (T20-004 — Component Gallery / debug window)

**T20-004 done.** New debug-only component gallery: a hand-maintained page
(not a `Component`-trait registry) showing every `labonair-ui-kit` primitive
across variants/sizes/states, in its own window, with a live
System/Light/Dark switch.

- **`crates/ui-kit/src/gallery.rs`** (`#[cfg(any(debug_assertions, feature =
  "gallery"))]`) — `struct Gallery: Render` + `open_gallery_window(cx)`
  (`cx.open_window` + `gpui_component::Root`, dedupes via a `GalleryWindowRef`
  global, same shape as `settings-ui`'s `open_settings_window`). Sections:
  Button (6 variants × 4 sizes + icon sizes + disabled), ToggleButton (icon +
  labelled variant/size grid + disabled), Checkbox (4 states + a stateful
  one), ListItem (plain/icon/selected+trailing-kbd/disabled), Disclosure
  (interactive open/close), SegmentedControl (stateful + outline/solid ×
  Xs/Sm/Md), NumberField (mid/at-min/at-max/float-no-track/disabled), Select
  (`select_trigger` closed+open — the `select_popover` overlay needs a live
  anchor, exercised in Settings), Banner (all 5 severities), Kbd
  (single/multi chord + hint), ContextMenu (permanently-open via new
  `menu_card_preview`), Divider + Indicator. Header documents that hover/press
  states are only live in the window (GPUI can't force `:hover`).
- **`crates/ui-kit/src/context_menu.rs`** — added
  `menu_card_preview(c, items)` (`#[cfg(any(debug_assertions, feature =
  "gallery"))]`), a thin pub wrapper over the private `menu_card` so the
  gallery can render a menu inline with no backdrop/anchor.
- **`labonair-ui-kit` `gallery` feature** (`[]`); forwarded
  `labonair-shell` → `labonair-app` (`gallery = ["…/gallery"]`) so
  `cargo check --features gallery` exercises the release path. No new
  crate-graph edge (`scripts/check-crate-deps.sh` still green).
- **`crates/command-palette/src/palette.rs`** — `CommandId::OpenComponentGallery`
  + `ACTION_NAMES` row `"debug::OpenComponentGallery"` + `ALL_COMMAND_IDS`
  test entry; `COMMANDS` row `Debug: Open Component Gallery` gated
  `#[cfg(debug_assertions)]` (section "Application").
- **`crates/shell/src/commands.rs`** — `#[cfg(debug_assertions)]`
  `r.register(CommandId::OpenComponentGallery, …)` → `open_gallery_window`.
  No menu.rs change (palette-row only, like the other `Debug:` ids that have
  no menu entry).
- **`docs/architecture.md` §8.17** — access + release-exclusion + the
  honest hover-state caveat.
- Gates: `fmt --check`, `check --workspace --all-targets`,
  `clippy --workspace --all-targets -D warnings`, `test --workspace`
  (0 failures), `check --features gallery`, `check --release` (+`--features
  gallery`), `check-crate-deps.sh` — all green.
- **Screenshots gap** (task instr. 7 / one acceptance box): no PR workflow
  and no headless GPUI screenshot path — same accepted gap as T20-002/003.
- **Next task: T20-005** (`ThemeRegistry` + JSON theme families,
  `tasks/phase-19-ui-kit/`). When it lands, extend the gallery's theme
  switch to list every registered family (already noted in the header text).

## Previous Session: 2026-09-05 (T20-003 — View-Migration Wave 2: Hosts, Snippets, AI, SFTP, Git-Graph, Settings-UI)

**T20-003 done**, 16 commits, gates green throughout (final: `3c4bcc1`).
Migrated the rest of the app's hand-rolled buttons/lists/menus/fields onto
`labonair-ui-kit` primitives, completing the app-wide UI-vocabulary goal
Phase 19 set out for: `a4fda76` hosts-ui, `b620b4e` panel-snippets, five
panel-ai commits (`3fbf8bd` session/agent menu rows, `7e1122e` composer
popover rows, `85c2b0c` ModelPicker, `baeb812` plan-mode + tool-call
actions, `d96095a` header trigger pills), `b031120` SFTP view, `489b366`
transfer queue, `25a6ca1` git-graph chrome, four settings-ui commits
(`e754773` sidebar/header, `a3a4f01` keymap editor, `4187a7f` custom panes,
`4719c62` documented-exception annotations), `7b5513a` shell updater
buttons, and `3c4bcc1` the final close-out audit.

- **Close-out audit (`3c4bcc1`)** was a dedicated pass (per the task's step 6):
  grepped every `.on_click(`-bearing file outside `labonair-ui-kit` for
  anything still un-migrated after the per-view commits above. Migrated:
  `panel-explorer/bookmarks.rs` (rows → `ListItem`), `shell/status_items.rs`
  (`simple_bar_button` rebuilt on `icon_toggle_button`; bell/clear-all/
  breadcrumb-collapse), `notifications/notifications.rs` (toast action
  button), `workspace/views/preview.rs` (open/reload/external buttons),
  `panel-scm/panel_scm.rs` (diff-header/stage-hunk/branch-toggle buttons),
  `workspace/workspace.rs` (`loading_btn` on the SSH loading screen, tab-strip
  "+", tab-close confirm dialog), `hosts-ui/hosts.rs` (import/export rows,
  quick-connect card, remove-tunnel trigger), `panel-ai/panel_ai.rs`
  (session-toggle trigger).
- **Documented exceptions** (no ui-kit primitive fits without a visual/
  behavioral regression — same bar as `4719c62`'s settings-ui exceptions):
  titlebar's non-standard 26px account trigger, search-overlay's
  arbitrary-glyph mini-buttons, command-palette's result row/breadcrumb
  pill/mode chip, the workspace tab row itself, the dock-tab strip, SFTP's
  click-to-edit path bar, SCM's commit-message box, and panel-ai's
  code-block Run/Copy links (no `Palette` in scope at that call site).
- All four gates (`fmt --check`, `check --workspace --all-targets`,
  `clippy --workspace --all-targets -- -D warnings`, `test --workspace`)
  pass after every commit; final full-workspace run green (92 unit tests +
  doctests, 0 failures).
- Task's instruction #7 ("Vorher/Nachher-Screenshots je View im PR") is not
  applicable here — there is no PR workflow in play this session (direct
  commits to `master`, per repo convention) and no way to capture GPUI
  screenshots headlessly; same gap existed and was accepted for T20-002.
- **Next task: T20-004** (Component-Gallery, `tasks/phase-19-ui-kit/`).

## Previous Session: 2026-09-05 (T20-002 — View-Migration Wave 1: Explorer, SCM, Editor, Terminal)

**T20-002 done**, 3 commits (`bf059cf` Explorer, `e96198d` SCM, `f23c611`
Editor/Terminal), gates green after each. Migrated the four views named in
the task from hand-rolled `div()` chrome to `labonair-ui-kit` primitives —
no behavior change, `cargo test --workspace` green throughout.

- **`ui_kit::ListItem` gained a generic `.extra(f: FnOnce(Stateful<Div>) ->
  Stateful<Div>)` escape hatch** (`crates/ui-kit/src/list.rs`) — applied last,
  after every other builder step, so a row can still carry drag-source/
  drop-target/right-click-menu behavior (or a one-off background override)
  that `ListItem` has no named method for, without the call site falling
  back to a bespoke `div()`. This is what let the explorer file-tree row and
  the SCM file/branch/tag/stash rows move onto `ListItem` while keeping their
  existing DnD (explorer) and selected/hover color split (SCM: accent when
  selected vs. border on hover — two different colors, which `ListItem`'s
  single `selected_fill` doesn't natively support).
- **`crates/panel-explorer/src/panel_explorer.rs`**: 5 toolbar icon buttons +
  the show-hidden toggle → `button`/`icon_toggle_button`; clip-banner
  Paste/Clear + delete-confirm Cancel/Delete + the "Load more…" link →
  `button`; the tree row (chevron+icon+name, drag source, drop target,
  right-click menu, selected/cut states) → `ListItem` + `.extra()`. The
  right-click menu was already on `ui_kit::context_menu` from an earlier
  pass.
- **`crates/panel-scm/src/panel_scm.rs`**: `tool_btn`/`row_action` now
  delegate to `ui_kit::button` internally (so the ~14 call sites across the
  action bar, branch bar, and commit/tag/stash forms picked up the shared
  chrome without a per-site edit); 3 collapsible section headers (Staged/
  Changes/Untracked via `render_section`, Remote branches, Tags, Stashes) →
  `disclosure`; the "Checkout after create" toggle → `checkbox`; the file/
  branch/tag/stash rows → `ListItem` + `.extra()` (trailing action buttons
  as an `h_stack`). **Deliberately left alone**: the panel's `text_field`/
  `on_field_key` hand-rolled keystroke-routing fields (branch/tag/stash/
  commit-message input) — this predates `ui_kit::InputState` and migrating
  it is a behavioral rework (focus model, key routing), not a primitive
  swap; flagged as a follow-up, not silently dropped.
- **`crates/workspace/src/views/editor.rs`**: the external-file-change
  conflict banner (the exact call site `banner.rs`'s own doc comment names
  as a T20-002 target) → `banner(Severity::Warning)` + `button` — this also
  fixed a live Critical-Rule-3 gap (the banner was computing its own warning
  tint via `theme.status_warning()` inline rather than going through the
  shared severity→token mapping). The gutter/cursor/selection text-render
  core was correctly out of scope and untouched.
- **`crates/workspace/src/views/terminal.rs`**: no changes — its right-click
  menu was already migrated to `ui_kit::context_menu` in an earlier pass, and
  no other hand-rolled button/list/menu/field markup exists in the view's
  chrome (the "shell exited" strip is a passive one-line status note, not a
  button/list/menu, so it was left as a plain `div()`).
- **Next task: T20-003** (View-Migration Wave 2, `tasks/phase-19-ui-kit/`).

## Previous Session: 2026-09-05 (T20-001 — `ui-kit` primitive set, first task of Phase 19)

**T20-001 done.** `labonair-ui-kit` grew from 5 files / ~880 lines (`button`,
`context_menu`, `icon`, `text_field`, `popover`) into a real design-system
crate: 21 modules / ~3040 lines, 29 unit tests, one shared token type
(`Palette`), and a `prelude`.

- **`Palette` — the one styling parameter** (`crates/ui-kit/src/palette.rs`).
  A `Copy` snapshot of the `labonair-theme` tokens (21 `Hsla` + `RadiusScale`),
  built once per render with `Palette::from_theme(theme)`. It exists because a
  view's `render` cannot hold a `&ThemeStore` borrow across `cx.listener(..)`
  — which is exactly why `settings-ui` (`view.rs`) and `hosts-ui` (`hosts.rs`)
  each already carried a *private* `Palette` struct with the same six fields.
  Both are deleted; both now use `labonair_ui_kit::Palette`.
  **Deviation from the task's literal wording:** to actually satisfy the
  "einheitliche API" criterion rather than document an exception, the three
  pre-existing primitives (`button`, `context_menu`, `popover`) were switched
  from `&impl UiTheme` to `Palette` too — 21 mechanical call sites across
  `workspace`, `shell`, `hosts-ui`, `panel-*`. `UiTheme` itself gained the
  missing accessors (`background`/`accent`/`accent_foreground`/`destructive`/
  `primary_foreground`/`status_error`/`status_warning`/`status_info`), each a
  1:1 mirror of `ThemeStore`'s inherent method of the same name.
- **New primitives** (all token-bound, all with a doc comment naming the
  `reference-src/src/components/ui/*.tsx` cva source *and* the Zed
  counterpart): `stack.rs` (`v_stack`/`h_stack`), `number_field.rs`
  (`NumberField` + pure `step_value`), `select.rs` (`select_trigger`/
  `select_popover` + pure `selected_label`), `checkbox.rs`, `segmented.rs`
  (`SegmentedControl`, `Outline`/`Solid` variants), `toggle.rs`
  (`icon_toggle_button`/`toggle_base`), `indicator.rs`, `banner.rs`
  (`Severity::{Note,Info,Success,Warning,Error}`), `kbd.rs` (`kbd`/`kbd_row`/
  `keybinding_hint`), `palette.rs`, `test_support.rs`. The three files a
  previous session had left on disk uncommitted (`divider.rs`,
  `disclosure.rs`, `list.rs`) were reviewed, wired in, and finished:
  `ListItem` gained `on_click` + `cursor_pointer`, unused imports dropped, and
  their doc comments — which claimed migrations that did not exist yet — were
  made true by actually doing those migrations.
- **`ContextMenu` extended**: `MenuItem::keybind([..])` renders a right-aligned
  `Kbd` chip row (radix `ContextMenuShortcut`); the card body was factored into
  a shared `menu_card` so the new **`popover_menu`** (the `anchored().
  snap_to_window()` + `deferred` sibling, i.e. radix `DropdownMenu`) reuses the
  exact same item renderer. Sub-menus / separators / icons / disabled /
  `checked` already existed.
- **Migrated call sites** (Anweisung #7 — one genuine site per primitive, *not*
  a flood migration; T20-002/003 do the rest). Full inventory table with the
  ✓ markers is now `docs/architecture.md` §8.16:
  * `settings-ui/panes/generic.rs` — `render_section_header` → `disclosure`
    (real chevron icons, the `▸`/`▾` ASCII arrows are gone);
    `FieldControl::Int` **and** `Float` → `number_field`;
    `FieldControl::Select` + `FontFamily` → `select_trigger`;
    `render_dropdown` → `select_popover`. The private `step_btn`,
    `slider_track`, `bump_int`, `bump_float` helpers in `view.rs` are deleted,
    replaced by one `set_float_field` store write (the clamping now lives in
    the primitive).
  * `settings-ui/view.rs` — the JSON syntax-error and schema-validation
    banners → `banner(Severity::…)`. **These hardcoded `gpui::red()` /
    `gpui::yellow()`** — a live Critical Rule 3 violation, now on
    `theme.status.error`/`.warning`. `render_search_results` → `list_header` /
    `ListItem` / `list_separator`.
  * `settings-ui/panes/themes.rs` — Installed/Community tabs and the theme
    variant picker → `segmented_control`.
  * `hosts-ui/hosts.rs` — the SSH-config import list and the host export list
    → `checkbox(..).box_only()`; the per-host reachability dot → `indicator`.
  * `shell/status_items.rs` — the statusbar panel-toggle cluster →
    `icon_toggle_button`.
  * `shell/titlebar.rs` — the hand-rolled absolutely-positioned account
    dropdown → `popover_menu` with real `MenuItem`s (`Settings…` carries its
    live `⌘,` binding via `MenuItem::keybind`).
  * `workspace/workspace.rs` — tab context-menu "Close" → `MenuItem::keybind`
    resolved through the user's `keymap.json` overrides
    (`KeybindDisplay` + `effective_keys(ShortcutId::TabClose)`); the
    connection-log and unsaved-tab dots → `indicator`; `h_stack`.
  * `command-palette/palette.rs` — the private `kbd` fn deleted; result-row
    keys → `kbd`, all five footer hints → `keybinding_hint`.
  * `panel-ai/panel_ai.rs` — `MdBlock::Rule` → `divider`; the AI/Shell composer
    toggle → `toggle_base` (same pressed/hover contract as the statusbar
    toggles instead of a bespoke 9px pill).
  * `workspace/views/preview.rs` (`MdBlock::Rule`) and `views/sftp.rs` (pane
    splitter) → `divider`.
- **Deliberately not built** (task Notizen: "nur Primitives, für die es ≥2
  reale Call-Sites gibt"): `Table` — neither the host list nor the transfer
  queue is a column grid, both are card/row layouts; `Tab`/`TabBar` — the only
  real tab *bar* is `Workspace::render_tab_bar` (drag-reorder, close buttons,
  kind indicators, per-tab context menus), the other "tab" strips are
  segmented controls; extracting a `TabBar` from one bespoke call site would be
  speculative, so it folds into T20-002's workspace migration. `ToggleButton`
  (labelled convenience) and `Indicator::outline` were written and then removed
  again once no second call site appeared. All three decisions are recorded in
  `docs/architecture.md` §8.16.
- **`gpui-component` deliberately not wrapped** for these. 0.5.1 ships
  `checkbox`, `divider`, `kbd`, `select`, `tab`, … but each styles itself from
  *its own* `cx.theme()` global, which `crates/app/src/main.rs` never syncs to
  `labonair-theme` (verified in the vendored source under `~/.cargo`) — a thin
  wrapper would silently bypass our tokens. It stays in use only where the
  behaviour is the hard part: `InputState`/`Input`, `Badge`, `Switch`,
  `Tooltip`.
- **Gates**: `cargo fmt --all --check` / `cargo check --workspace
  --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` all
  green; `cargo test --workspace` → **878 passed, 0 failed** (was 854 at
  T19-010; `crates/ui-kit` alone now contributes 29). Run
  `cargo metadata --format-version 1 --no-deps | python3
  scripts/check_crate_deps.py` — passes
  (24 crates, 103 internal edges, acyclic — no new crate edges: every migrated
  crate already depended on `labonair-ui-kit`).

**Current state**: branch `master`, committed on top of `d95b8f3`. Files
touched: `crates/ui-kit/src/*` (11 new modules + `button.rs`,
`context_menu.rs`, `popover.rs`, `theme.rs`, `ui_kit.rs`, plus the three
previously-uncommitted files), `crates/settings-ui/src/{view.rs,panes/
{generic.rs,themes.rs}}`, `crates/hosts-ui/src/hosts.rs`,
`crates/shell/src/{titlebar.rs,status_items.rs}`,
`crates/workspace/src/{workspace.rs,status_bar.rs,views/{sftp.rs,terminal.rs,
preview.rs}}`, `crates/command-palette/src/palette.rs`,
`crates/panel-{ai,explorer,git-graph,scm,snippets}/src/*.rs`,
`docs/architecture.md` (new §8.16), the task file. The unrelated
`zed-refrence/zed` submodule-pointer change was left unstaged on purpose.

**Next**: `T20-002` — View-Migration Welle 1
(`tasks/phase-19-ui-kit/T20-002-view-migration-wave-1.md`). No blockers.

## Previous Session: 2026-09-04 (T19-010 — Settings › Hosts, final task of Phase 18)

**T19-010 done — Phase 18 (Settings-System Zed-Style, T19-000..T19-010) is
now fully complete.** Host/credential management gets its permanent home: a
top-level Settings category "Hosts" (`AreaMeta` already existed from
T19-001), a peer of "Themes"; the interim `TabKind::Hosts` tab from T17-009
is gone.

- **Settings pane** — new `crates/settings-ui/src/panes/hosts.rs`: `labonair
  -settings-ui` gained a dependency on `labonair-hosts-ui` (new allow-list
  edge in `scripts/check_crate_deps.py`, both directions annotated). Per the
  task's own Notizen ("nicht neu bauen"), the existing, mature
  `HostManagerView` (T16-008/T07-*: list/edit form/jump-hosts/tunnels/
  SSH-config import-export — all already built) is embedded **verbatim**,
  not rebuilt. `pages.rs` gained a `hosts` `AreaKind::Custom` arm with two
  `SubPage`s (`ssh-config`, `availability`); the main page serves both the
  task's `hosts/list` and `hosts/edit` deep-links (`HostManagerView` already
  toggles list ↔ edit form internally — there was never two components to
  give two slugs to). `hosts/ssh-config` adds two buttons that call new
  `pub fn open_import_dialog`/`open_export_dialog` wrappers on
  `HostManagerView`. `hosts/availability` renders `connections.*` polling
  fields as normal generated `SettingField`s via a new
  `pages::HOSTS_AVAILABILITY_GROUPS` + the existing `render_field_groups`
  helper (Notizen: "Custom-Body heißt nicht 'keine generierten Felder'").
  `pages.rs`'s `navigate_to_slug` logic was extracted into a pure
  `resolve_slug(pages, slug)` helper so deep-link resolution is unit-testable
  without constructing a full `SettingsView` — tests added for `hosts` and
  `hosts/ssh-config`.
- **The shared `HostManagerView` entity, not a new one** — `Workspace`
  gained `pub fn host_manager(&self) -> Entity<HostManagerView>`, threaded
  through `SettingsDeps`/`set_settings_deps`/`SettingsView::new` into the
  Settings window. Because it's the *exact same* entity `Workspace` already
  built (own connect flows, `known_hosts`, `recent_hosts`), an edit made in
  Settings is live everywhere with zero extra sync code, and clicking
  Connect/SFTP inside the embedded view fires the same `HostManagerEvent`
  `Workspace` already subscribes to — satisfying the task's
  `on_open_ssh`/`on_open_sftp` callback requirement without adding a new
  `settings-ui -> workspace` data-flow edge.
- **Single write path — `labonair_hosts_ui::apply_host_change`** (new
  `crates/hosts-ui/src/apply.rs`, new `labonair-settings`/
  `labonair-settings-content` deps on `labonair-hosts-ui`, also added to the
  allow-list). Design note/deviation from the task's literal wording:
  `HostManagerView`'s pre-existing SQLite CRUD (`hosts::db::hosts_create`/
  `hosts_update`/...) already correctly writes secrets into
  `backend::modules::secrets` (never JSON) — that machinery is **not**
  duplicated or moved. What's new is projecting the resulting *non-secret*
  host list into `SettingsContent.hosts.entries` + `credential_ref` (new
  public `labonair_backend::modules::hosts::credential_ref(app, id)`), from
  exactly one function, called from exactly the two places
  (`HostManagerView::reload`/`reload_list_only`) that refresh the host list
  after *any* mutation (create/update/duplicate/delete/reorder all funnel
  through one of them) — so there is still only one write path into
  `hosts.entries`, it just sits at the read-refresh boundary rather than
  inside each individual mutation call, which sidesteps needing to thread a
  freshly-created host's id back out of a fire-and-forget async call. Full
  integration test (`apply_host_change_writes_non_secret_fields_and_no_secret`)
  creates a real host via `hosts_create` (so the password really lands in
  the secret store), builds a real `SettingsStore` against a temp file
  (`SettingsStore::new` made `pub`, T19-010, for exactly this — it was
  previously only reachable inside the `labonair-settings` crate's own
  tests), calls `apply_host_change`, and asserts the on-disk JSON contains
  the host's name/address but never the plaintext password, plus
  `credential_ref` is set correctly in the merged content.
- **`TabKind::Hosts` / `open_host_manager` / `CommandId::OpenHostManager`
  removed**, replaced by `Workspace::open_host_settings` +
  `CommandId::OpenHostSettings` / native menu action `OpenHostSettings`.
  Since `Workspace` cannot depend on `labonair-settings-ui` (already depends
  on `labonair-workspace`, so the reverse edge would cycle),
  `open_host_settings` calls a shell-installed closure
  (`Workspace::set_open_host_settings_hook`, mirrors the pre-existing
  `set_dock_persist_hook` pattern exactly) that calls
  `labonair_settings_ui::open_settings_window(Some("hosts"), cx)`; wired in
  `crates/shell/src/bootstrap.rs` next to `set_settings_deps`. The `＋▾`
  "All hosts…" item and the status-bar hosts button now call
  `open_host_settings`; `NewSshTab`/`NewSftpTab`/`NewQuickSsh`/
  `NewSshConnection` were **repointed to the command-palette's Hosts connect
  page** (they used to all share `open_host_manager`, which conflated
  connect and manage) — `Cmd+Shift+N` (`NewSshConnection`) is unchanged
  behaviorally, just no longer routed through the now-deleted function name.
  Six `TabKind::Hosts` match arms across `workspace.rs`/`tabs.rs` removed
  (compiler-guided, all exhaustive matches).
- **Migration hookup already done by T19-009** — `migrate_hosts_to_settings`
  was already called from `crates/app/src/main.rs` at startup; nothing to
  wire for this task beyond confirming it (read the file first per the
  task's own instruction).
- **Deviations from the task's literal wording** (judgment calls made to
  avoid a much larger, riskier rewrite of a mature, already-tested 4000-line
  component; documented in code comments and reported explicitly): (1)
  `hosts/list`/`hosts/edit` are not two separately-slugged bodies — they're
  the same embedded `HostManagerView`, which already does master/detail
  internally; only `ssh-config`/`availability` got real `SubPage` entries.
  (2) `known_hosts`/`recent_hosts` in `workspace.rs` were **not** rewired to
  read from `hosts.entries` — they still read the shared `HostManagerView`'s
  SQLite-backed list directly (unchanged runtime behavior); `apply_host_change`
  mirrors that same list into `hosts.entries` on every refresh, so the two
  are always in sync, but "same known_hosts source" is true via that shared
  entity/mirror rather than literally the same storage backend.
- **Gates**: `cargo fmt --check` / `cargo check --workspace --all-targets` /
  `cargo clippy --workspace --all-targets -- -D warnings` all green;
  `cargo test --workspace` → 854 passed, 0 failed. `cargo tree -p
  labonair-settings-ui` shows the `labonair-hosts-ui` edge; `cargo tree -p
  labonair-hosts-ui` shows no `labonair-workspace`/`-shell`/`-panel*` edge;
  `scripts/check_crate_deps.py` passes (24 crates, 103 edges, acyclic).

**Current state**: branch `master`. **Phase 18 (Settings-System Zed-Style)
is fully complete: T19-000 through T19-010 all `✅ Done`.** Files touched:
`crates/settings-content/src/hosts.rs` (untouched, already correct from
T19-001), `crates/settings-ui/src/{pages.rs,view.rs,window.rs,panes/{mod.rs,
hosts.rs,generic.rs}}`, `crates/hosts-ui/src/{apply.rs (new),hosts.rs,
hosts_ui.rs,Cargo.toml}`, `crates/settings-ui/Cargo.toml`,
`crates/settings/src/store.rs` (`SettingsStore::new` made `pub`),
`crates/backend/src/modules/hosts/mod.rs` (`credential_ref` helper),
`crates/workspace/src/{workspace.rs,tabs.rs}`, `crates/shell/src/
{bootstrap.rs,commands.rs,menu.rs,status_items.rs}`,
`crates/command-palette/src/palette.rs`, `scripts/check_crate_deps.py`.

**Next**: `T20-001` — starts **Phase 19 "UI-Kit"**, a new task-numbering
phase (`tasks/phase-19-ui-kit/T20-001-ui-kit-primitive-set.md`). No
blockers.

## Previous Session: 2026-09-04 (T19-009 — settings migrator: preferences/editor/mcp → SettingsContent, keybinds → keymap.json, SQLite hosts → hosts.entries)

**T19-009 done.** New `crates/backend/src/modules/settings/migrate_v2.rs`:
one-time, idempotent `migrate_settings_v1_to_v2(dir) -> SettingsV2Outcome`
(`AlreadyMigrated`/`NothingToMigrate`/`Migrated { keybinds_migrated }`) that
transforms the legacy `labonair-settings.json` (separate `preferences`/
`editor`/`mcp` top-level keys) into the flat T19-001 `SettingsContent` area
layout (`general`/`appearance`/`terminal`/`editor`/`fileManager`/
`connections`/`workspace`/`ai`/`mcp`/`personalization`), plus a separate
`migrate_hosts_to_settings(dir, hosts, app) -> HostsV2Outcome` that hydrates
the SQLite-backed hosts (`backend::modules::hosts`) into `hosts.entries`.

- **Detection/idempotency**: `schemaVersion: 2` present → no-op; no legacy
  `"preferences"` key present → no-op (either already-v2 or genuinely
  empty); otherwise migrate. `.bak` copy before any write (T18-006
  precedent). Old `preferences`/`editor`/`mcp` raw blobs preserved verbatim
  under `preferences_legacy`/`editor_legacy`/`mcp_legacy` — nothing is ever
  silently dropped. Fields with no `SettingsContent` destination
  (`preferences.barLayoutMigrated`, old `"editor"`'s `number`/`expandtab`/
  `tabstop`/`shiftwidth`) land under `_migratedUnknown.*` instead. Only the
  touched top-level keys are read-merge-written (`serde_json::Map` merge,
  not a blind rewrite) — `statusBarItemPlacements`/`barItemPlacements_legacy`
  (T18-006) survive untouched, confirmed by a test.
- **Field-mapping table**: every one of `Preferences`' 170 fields is a
  hand-written `Some(p.field)` assignment in one of ten per-area builder
  functions (`general_from`/`appearance_from`/`terminal_from`/`editor_from`/
  `file_manager_from`/`connections_from`/`workspace_from`/`ai_from`/
  `mcp_from`/`personalization_from` — literally `content_bridge.rs`'s
  existing reverse-direction `From<&SettingsContent> for Preferences`
  inverted field-by-field, since every `SettingsContent` area struct already
  uses the *same* camelCase field names as `Preferences`). Exhaustiveness is
  proven, not asserted: `tests::every_preferences_field_is_accounted_for`
  diffs `serde_json::to_value(Preferences::default())`'s JSON keys against a
  static `mapped`/`SKIPPED_PREFERENCES_FIELDS`/`UNKNOWN_PREFERENCES_FIELDS`
  union — a field silently falling off the table fails the test immediately
  (caught one typo-class bug for free: this test passed clean on the first
  full run, which is itself a decent signal the hand-transcription was
  accurate). `hmLayout`/`hmSort`/`hmCardScale` move into the `hosts` area
  (merged via `merge_object`, never clobbering a prior
  `migrate_hosts_to_settings` run's `hosts.entries`); `dockLayout`/
  `sidebar*` move into `workspace`; `mcpBridge*` mirror fields are *not*
  re-copied — the new `mcp` area is built from the old separate `"mcp"` key
  (`McpPrefs`), which `Preferences`' own doc comment already calls
  authoritative.
- **Keybinds → `keymap.json`**: `preferences.keybinds` was fully *removed*
  from the `Preferences` struct by T19-008 (a gotcha this task hit directly
  — deserializing an old file into today's `Preferences` silently drops the
  unknown field), so the migrator reads `keybinds` off the *raw* JSON
  `Value` before ever parsing into `Preferences`. A self-contained
  `SLUG_TO_ACTION: &[(&str, &str)]` table (30 entries, hand-verified against
  `crates/command-palette/src/keybind.rs`'s `shortcut_slug` +
  `palette.rs`'s `ACTION_NAMES`) maps old `ShortcutId` slugs
  (`"tab.new"`) to new `CommandId::action_name()` strings
  (`"tab::NewTerminal"`) *without* `labonair-backend` importing
  `labonair-command-palette` — that dependency edge already runs the other
  way (`command-palette -> backend`), so backend importing it back would be
  a cycle the crate-graph script forbids. Every migrated binding lands in
  one `{ "context": null, "bindings": {...} }` block (no per-command default
  context table exists pre-T19-008; matches the shipped default-keymap
  assets' own predominant "most bindings are global" shape). Empty-string
  override (old "unbind") → `null` value, keyed by a `"unbind:<action>"`
  placeholder since there's no real keystroke to key it by. Unknown slugs
  are dropped (logged), not fatal. No overrides → no `keymap.json` file
  written at all (per the task's own instruction).
- **SQLite hosts → `hosts.entries`**: `host_to_entry` is a pure transform
  (`Host` -> `labonair_settings_content::hosts::HostEntry`);
  `credential_ref_for` is the only impure part (reads, never writes, the
  secret store). **Documented scope reduction** since T19-010 hasn't landed:
  this codebase's actual "OS Keychain equivalent" the CLAUDE.md architecture
  overview refers to is `backend::modules::secrets` (`SecretsState`, an
  AES-256-GCM/plain-JSON file store with a `service::account` composite key)
  — the `keyring` crate is a real workspace dep but is only used by
  `crates/ai` for something else entirely, hosts have never used it.
  `credential_ref` is set to `"secrets:labonair-app:<host.id>"` (direct
  host password/passphrase, if present) or `"secrets:labonair-cred:<credential_id>"`
  (a reusable SSH-key `Credential`, if the host references one with an
  actual stored secret) — **no secret is copied or moved**, this is purely a
  reference alongside the key the secret already lives under; a
  `hosts_entries_serialize_without_secrets`-style test confirms none of
  `password`/`privateKey`/`secret`/`passphrase` (nor the literal test
  secret) ever appear in the written JSON. `tags`/`tunnels` (opaque SQLite
  `TEXT` columns with no backend-enforced JSON shape — nothing in
  `hosts/db.rs` parses them either) are parsed best-effort (JSON array, with
  a comma-split fallback for tags) rather than inventing a schema; a parse
  failure is logged and yields an empty list, never a panic — the SQLite row
  itself is completely untouched regardless, so nothing is actually lost,
  only the settings.json projection is momentarily incomplete until T19-010
  nails the real shape. Idempotent via a `hostsMigrated: true` top-level
  marker (empty host list still sets the marker so a `NothingToMigrate` list
  doesn't get re-scanned every launch).
- **Bootstrap wiring** (`crates/app/src/main.rs`): both migrations run
  synchronously in `main()`, **before** `labonair_settings::init(cx)` —
  required, since `SettingsStore` reads the very same
  `labonair-settings.json` file at the very same `config_dir()` path and
  would otherwise cache an all-defaults tree for one more launch on an old
  file. `migrate_hosts_to_settings` needs `hosts_get_all(&backend.db).await`
  (async), so it runs inside a `runtime.block_on(...)` right there in
  `main()`, before the GPUI `Application::run` closure. `migrate_bar_item_placements`
  (T18-006) and `keymap_loader::reload_and_apply` (T19-008) are unchanged
  and still run later inside `bootstrap()`/`AppShell::new` — safe, since
  neither touches the area keys this migration owns.
- 10 new unit tests in `migrate_v2.rs` (full migration + count-match,
  keybind migration incl. unbind + unknown-slug drop, no-keybinds → no file,
  idempotency, already-v2 untouched, empty → nothing-to-migrate, partial
  file, host migration + secret-free JSON + idempotency, empty-hosts
  marker). All four gates green: `cargo fmt --check`, `cargo check
  --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace` (0 failures workspace-wide).
  `scripts/check_crate_deps.py` reports 24 crates / 100 edges, acyclic, no
  new edge needed (`labonair-backend` already depended on
  `labonair-settings-content`).
- **Not done / deferred**: a manual `cargo run` smoke test against a real
  pre-rework `labonair-settings.json` + SQLite `labonair.db` (task's own
  Anweisung #8) — no such fixture was available in this (headless) session;
  `full_migration_moves_every_field_and_counts_match` covers the same
  transform path automatically instead. Noted as `[~]` in the task file
  rather than silently checked off.
- **State**: branch `master`, commit before this session was `09755a5`
  (T19-008). Files touched this session: new
  `crates/backend/src/modules/settings/migrate_v2.rs`;
  `crates/backend/src/modules/settings/mod.rs` (module registration);
  `crates/app/src/main.rs` (migration call sites before
  `labonair_settings::init`); `tasks/phase-18-settings-core/T19-009-settings-migrator.md`
  (status + acceptance-criteria checkboxes); this file.
- **Next**: T19-010 (Settings › Hosts page) — note `crates/hosts-ui` already
  exists (from T16-008's panel-crate split, the existing Host-Manager panel
  UI, not a T19-010 scaffold) and is unrelated to this task; T19-010 still
  starts from zero for the actual Settings-UI hosts page. T19-010 is also
  where `tags`/`tunnels` parsing in `migrate_v2.rs` should get tightened
  once the real Host-Manager-UI JSON shape for those two columns is nailed
  down (currently best-effort, see above).
- **Blockers**: none.

## Last Session: 2026-09-04 (T19-008 — keymap.json with contexts + chords)

**T19-008 done.** The flat, hardcoded `ShortcutId` enum + `preferences.keybinds`
`BTreeMap<slug,String>` override blob is replaced by an editable, layered
`keymap.json` with **contexts** (`"context": "Editor"`) and **chords**
(`"cmd-k cmd-s"`), following the Zed `keymap_file.rs` blueprint referenced in
the task. `ShortcutId`/`SHORTCUTS` (in `crates/command-palette/src/keybind.rs`)
survive only as a display/label source and as the origin of the generated
default-keymap assets — they no longer resolve any actual key binding.

- **`crates/settings/src/keymap.rs`** (new) — pure parse/merge/validate model,
  deliberately decoupled from `CommandId`/`labonair-command-palette` per the
  crate-graph rules (`scripts/check_crate_deps.py`): `KeymapFile`/`KeymapBlock`
  (`context: Option<String>`, ordered `bindings: Vec<(String, Option<String>)>`
  — `None` value = `"key": null` unbind), `parse_keymap_jsonc` (JSONC via
  `jsonc-parser`, already a `labonair-settings` dep), `KeybindSource`
  (`Default`/`BaseKeymap`/`User`), `merge_keymaps(&[(KeybindSource,
  &KeymapFile)]) -> Vec<EffectiveBinding>` (later layers override earlier ones
  keyed by normalized `(context, keystrokes)`; `null` removes a prior entry),
  `validate_keymap(file, known_actions: &BTreeSet<&str>, text)` (unknown
  action → `Error` with a best-effort line number via
  `labonair_settings_json::line_col_at`; same chord in the *same* context
  twice → `Warning`; same chord in *different* contexts → no warning, per the
  task's own acceptance criterion). 10 unit tests: chord parsing round-trip
  via `gpui::Keystroke::parse` on each whitespace-split token, context
  predicate eval via `gpui::KeyBindingContextPredicate::parse(...).eval(...)`
  (both a matching and non-matching `KeyContext`), Default→User merge +
  `null`-unbind, unknown-action error line, context-non-conflict vs.
  same-context conflict, and a reload-proxy test (parse+merge twice with two
  text snapshots, assert the effective set changes).
- **`crates/settings/assets/keymaps/default-{macos,linux}.json`** (new) —
  generated from the pre-T19-008 `ShortcutId`/`SHORTCUTS` table crossed with
  the new `CommandId::action_name()` strings (see below); most bindings sit
  under a `"context": null` block (always active, preserving today's
  behavior of zero context-filtering), a handful get a real context where
  `commands.rs`'s registered `CommandContext` for that command matches one of
  the 20 existing `.key_context("...")` string literals (e.g. `"Editor"`,
  `"Terminal"`). Includes one literal chord demo:
  `"cmd-k cmd-s"`/`"ctrl-k ctrl-s"` → `"zed::OpenKeymap"` (new
  `CommandId::OpenKeymapJson` command, opens `keymap.json` as an editor tab
  the same way `OpenSettingsJson`/`OpenProjectSettings` already do).
- **`crates/command-palette/src/palette.rs`** — `CommandId::action_name()` /
  `::from_action_name()` (all ~70 variants, `<namespace>::<Name>` strings,
  e.g. `tab::NewTerminal`, `command_palette::Toggle`), `known_action_names()`
  helper, new `CommandId::OpenKeymapJson` variant. `keybind.rs` gained a
  `KeybindDisplay` GPUI global so the palette/status-bar can render the
  *merged* effective binding for a command instead of going through the old
  `ShortcutId`-keyed `effective_binding`/`KeybindMap` machinery.
- **`crates/shell/src/keymap_loader.rs`** (new) — `include_str!`s the
  platform default asset, reads user `keymap.json` (sibling of
  `labonair_settings::user_settings_path()`; missing file = empty, not an
  error), validates against `known_action_names()`, merges
  `[(Default, &default), (User, &user)]`, keeps the **last known-good**
  parsed user keymap on a broken file (falls back + logs via `tracing`,
  never crashes — "kaputte Datei → Banner + letzte gute Keymap"), and drives
  a debounced fs-watch on `keymap.json` (same `notify`/`notify-debouncer-mini`
  pattern as `crates/settings/src/watch.rs`) that re-parses/re-merges/
  re-applies on change.
- **`crates/shell/src/menu.rs`** — old `apply_keybinds(cx, &KeybindMap)` +
  `bindings()` (keyed off `ShortcutId` via a `rebind!` macro, always
  `context: None`) replaced by `apply_keymap(cx, &[EffectiveBinding])` +
  `action_for(name: &str) -> Option<Box<dyn Action>>` (one arm per action
  name → concrete `menu::` GPUI `Action` struct) + `bindings_from_keymap`
  (pre-validates every keystroke token via `Keystroke::parse` and any context
  string via `KeyBindingContextPredicate::parse` *before* calling the
  panicking `KeyBinding::new`, so unvalidated user JSON can never actually
  panic it — CLAUDE.md rule 6, no `unwrap()` on predictable errors). The
  handful of genuinely OS-reserved, non-rebindable accelerators (Cmd+,` /
  Cmd+Shift+N / Cmd+Q / Cmd+M / Cmd+H / Ctrl+Cmd+F / Cmd+S) stay hardcoded
  exactly as before, outside keymap.json, per `RESERVED_ACCELERATORS`'s old
  intent. `bootstrap.rs` now calls `keymap_loader::load_effective()` +
  `menu::apply_keymap` at startup and wires the fs-watch spawn;
  `settings-ui/src/window.rs`'s `KeybindApplyHook` signature updated to
  `fn(&mut App)` (re-triggers a full reload+reapply after a pane-driven
  rebind, instead of threading a `KeybindMap` through).
- **`crates/settings-ui/src/keymap_edit.rs`** (new) — surgical `keymap.json`
  writer built on `labonair-settings-json`'s tree-sitter editor
  (`append_top_level_array_value_in_json_text` /
  `replace_top_level_array_value_in_json_text`): rebind (append to an
  existing matching-context block or create a new one), unbind (`null`),
  overwrite-on-conflict, reset-one/reset-all (removes the user block/entry).
  4 pure unit tests.
- **`crates/settings-ui/src/panes/shortcuts.rs`** rebuilt — lists every
  command with its effective binding (from `keymap_loader`'s merged set),
  source badge (Default/User), and context; Edit/Reset/Reset-all/"Open
  keymap.json" wired to `keymap_edit.rs` + `CommandId::OpenKeymapJson`.
  **Deliberately reduced scope** (documented in code comments, not a gate
  blocker per the task's own priority order): the recording UI still commits
  a single keystroke at a time (chords are fully supported by the
  parse/merge/bind model — `"cmd-k cmd-s"` binds and dispatches correctly
  today — just not yet produced by this UI), and both conflict detection in
  the pane and the command-palette's binding display are context-agnostic
  (first-match) rather than fully focus-context-filtered.
- **`crates/backend/src/modules/settings/preferences.rs`** +
  **`content_bridge.rs`** — the legacy `Preferences.keybinds:
  BTreeMap<String,String>` field and its content-bridge passthrough are
  removed entirely (the acceptance criterion "`apply_keybinds` +
  `preferences.keybinds`-Blob sind entfernt" — migration path noted as
  T19-009 per the task, but there was nothing left to migrate since no
  `SettingsContent` field ever existed for it; the blob is just gone).
- **Gates**: `cargo fmt --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace` all green (re-verified independently after the implementing
  fork reported green) — 0 test failures across the whole workspace.
  `scripts/check-crate-deps.sh` also green (no new `labonair-*` crate was
  created; keymap logic lives inside the existing `labonair-settings` /
  `labonair-shell` / `labonair-settings-ui` crates, so no new graph edges).
- **Current state**: branch `master`, commit will follow this handshake
  update (see commit message `feat(settings): T19-008 keymap file with
  contexts`). **Next**: T19-009 (Settings-Migrator).
- **Blockers/notes for next session**: none blocking. Two intentional
  follow-up polish items live in `shortcuts.rs`/palette comments (chord
  *recording* UI, focus-context-filtered conflict/display) — pick up
  whenever a Shortcuts-pane polish task comes up; they don't affect
  correctness of the underlying keymap.json model, which already fully
  supports chords + contexts end-to-end.

## Last Session: 2026-09-04 (T19-007 — global settings search)

**T19-007 done.** The per-category substring filter (`panes/generic.rs`'s old
`render_global_search`, which re-rendered matched fields inline in the main
content area) is replaced with a real global search: one query box, a
pre-built index over every field + custom pane, fuzzy scoring, and a jump
that navigates to the field's normal on-screen location with a highlight
pulse — matching the task's Zed-inspired design (`settings_ui.rs`'s
`StringMatchCandidate` index + `fuzzy::match_strings`).

- **`crates/settings-ui/src/search.rs`** (new file) — pure, view-free module:
  `SearchEntry`/`SearchIndex` (index built once from `all_fields()` + `pages()`
  — every `AnyField` gets one entry, `haystack = title + description +
  json_path`; every `AreaKind::Custom` page/sub-page gets one entry with a
  hand-curated keyword list, `pane_keywords()` — e.g. "keymap shortcut
  keybinding tastenkürzel hotkey" for Shortcuts, satisfying the task's own
  example). `search(index, query, limit)` scores every entry with the shared
  `labonair_command_palette::{match_score, SearchMode::Fuzzy}` (already used
  by the command palette / `@`-file picker — re-exported at the crate root,
  `fuzzy` itself is a private module), sorts descending, then groups hits by
  category (category order = the order areas first appear in the
  score-sorted list, so the best overall match's category sorts first),
  capped at 50. `SearchTarget::{Field(usize), Pane { area_index,
  subpage_index }}` is `Copy`, indexing back into `SettingsView::all_fields`/
  `AREAS`. 5 unit tests: `cursor` finds Terminal + Editor fields, an exact
  `json_path` finds its field, a keyword-only query (`tastenkürzel`) finds
  the Shortcuts pane, empty query yields nothing, and the `limit` cap holds.
- **`crates/settings-ui/src/pages.rs`** — new `section_label_for_field(area_key,
  local_key) -> Option<(sub_page_slug, section_label)>`, scanning
  `TERMINAL_MAIN`/`TERMINAL_ADVANCED`/`EDITOR_MAIN`/`EDITOR_DISPLAY`/
  `AI_GROUPS`/`PERSONALIZATION_GROUPS`/`groups_for()` — lets a search jump
  resolve which sub-page to open and which disclosure section to
  un-collapse before scrolling. `None` means the field falls through to the
  area's trailing "Other" section. 1 new test.
- **`crates/settings-ui/src/view.rs`** — `SettingsView` gained `search_index`
  (built once in `new()`, never rebuilt per keystroke per the task's own
  Warnung), `search_results: Vec<SearchRow>` + `search_selected` (recomputed
  every render via `refresh_search_results()` — cheap, ~200-entry index),
  `highlight: Option<&'static str>` + `highlight_token: u64` (pulse state,
  cleared by a `cx.spawn` + `cx.background_executor().timer(1s)` guarded by
  the token so a stale timer from an earlier jump can't clear a later one —
  same pattern as `NotificationCenter::insert`'s auto-dismiss), and
  `pending_scroll: Option<&'static str>` (consumed once by the render pass
  that lands on the target page). `activate_search_hit(target, cx)`
  navigates `active_area`/`active_subpage`, un-collapses the owning section
  via `section_label_for_field`, sets `pending_scroll` + `set_highlight`,
  and clears the query. `on_key` gained `up`/`down` (wrap `search_selected`
  through `search_results.len()`) and `enter` (activates the selected hit);
  `escape` now clears an active query first and only closes the window once
  the query is already empty (task step 5). The sidebar renders
  `render_search_results` (category headers + title/subtitle rows, click or
  Enter navigates) instead of the category list whenever the query is
  non-empty — replacing the old behavior where the category list stayed put
  and only the main content swapped.
- **`crates/settings-ui/src/panes/generic.rs`** — deleted the old
  `render_global_search` (T19-004's inline substring-filter renderer);
  `render_body` no longer special-cases a non-empty query — the main content
  area always renders the active category/sub-page normally, so a search
  jump lands the field in its real spot instead of a duplicate inline list.
  `render_generated_body` and `render_field_groups` (the AI/Personalization
  custom-pane field grid renderer) both now consume `pending_scroll`
  (tracking the row index of the matching field as rows are built, then
  `content_scroll.scroll_to_item(row)` once) and `render_field` paints a
  `c.accent.opacity(0.25)` background when `self.highlight == Some(json_path)`.
- **Known limitation carried over, not introduced by this task:** GPUI's
  `ScrollHandle::scroll_to_item(ix)` indexes the *immediate* children of the
  `track_scroll`-tracked element (confirmed by reading
  `zed-refrence/zed/crates/gpui/src/elements/div.rs`'s `child_bounds`
  population — it's filled from `request_layout.child_layout_ids`, the
  tracked div's own direct children). The settings window's tracked
  `#settings-scroll` div has exactly **one** child (the `max-w` content
  wrapper), so `scroll_to_item(row)` for `row > 0` is a no-op today — this
  was already true of the pre-existing jump-bar feature
  (`render_jump_bar`/`content_scroll.scroll_to_item(row)`, T19-004) before
  this task touched anything. Not fixed here (a real fix needs restructuring
  the scroll container so `rows` are its direct children, which is a larger,
  separate change); the search jump still fully navigates + un-collapses +
  highlights the field, only the auto-scroll-into-view is inert beyond the
  first row. Worth a dedicated follow-up if scroll-to-field precision
  matters before T19-008.
- Gates green: `cargo fmt --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace` (30 tests in `labonair-settings-ui`, full workspace suite
  unaffected). No new crate dependency (`labonair-command-palette` was
  already a `settings-ui` dep) — `scripts/check_crate_deps.py` re-run clean
  (24 crates, 98 edges, acyclic).

**Current state:** branch `master`, commit after this session's own commit
(see `git log`). Working tree also has a long-standing unrelated uncommitted
`zed-refrence/zed` submodule-pointer change (pre-existing, not touched).

**Next:** T19-008 (keymap as a file with contexts) —
`tasks/phase-18-settings-core/T19-008-keymap-file-with-contexts.md`.

**Blockers:** none. The `scroll_to_item` limitation above is a pre-existing
gap worth flagging to whoever picks up scroll-precision work, but does not
block T19-008.

## Last Session: 2026-09-04 (T19-006 — JSON-Schema generation + settings.json validation)

**T19-006 done.** `SettingsContent` now drives a real generated JSON Schema
used for both file-write convenience and on-load validation of both text-
backed settings layers (`User` + `Project`), matching Zed's
`SettingsJsonSchemaParams`/`json_schema` design at a scope this repo needs.
New/changed:

- **`crates/settings/src/schema.rs`** (new file) — `json_schema()` ==
  `serde_json::to_value(schemars::schema_for!(SettingsContent))` (schemars
  0.8 was already derived on `SettingsContent` since T19-001, so this is a
  pure wrapper, no drift risk). `validate(instance, raw_text) ->
  (Vec<SettingsValidationError>, Vec<SettingsValidationError>)` runs the
  `jsonschema` crate (0.53, added with `default-features = false` — its
  defaults pull in `reqwest`/TLS for remote `$ref` resolution, which this
  offline desktop app never needs) against the schema for type/enum errors
  (`json_path` from `ValidationError::instance_path()`, JSON-Pointer ->
  dotted path via `pointer_to_json_path`), plus a hand-rolled recursive
  `unknown_key_warnings` walker (jsonschema's own `additionalProperties`
  keyword isn't useful here — schemars 0.8 doesn't emit `additionalProperties:
  false` for plain structs, so every struct node is `properties`-shaped and
  "extra key" has to be detected by walking `properties`/`$ref`/`allOf`
  manually; `HashMap<String,T>`-shaped fields are recognized by
  `additionalProperties` being a sub-schema with no `properties` sibling and
  their *keys* are never flagged, only recursed into by value — those keys
  are user data like host ids/panel names, not settings fields).
  `description_for_path(json_path)` walks the same `$ref`/`allOf` resolution
  to pull a field's doc-comment description (or its nearest ancestor's) for
  the editor-hover helper below. 8 unit tests: schema completeness (every
  `SettingsContent` serde key + the `CursorStyle` enum's 3 variants),
  `SettingsContent::defaults()` round-trips with zero errors/warnings
  (the Warnungen's required round-trip check), one test per error class
  (wrong type, invalid enum, unknown key incl. a legacy top-level key like
  `preferences`), a map-shaped field never warning on its own keys, and both
  `description_for_path` cases.
- **`crates/settings-json/src/settings_json.rs`** — two new read-only
  helpers reusing the crate's existing `tree-sitter-json` parse (kept
  read-only, separate from the surgical-write path): `find_value_range(text,
  key_path) -> Option<Range<usize>>` (byte range of an existing key's value,
  used to derive line/col for a schema error) and `json_path_at_offset(text,
  byte_offset) -> Option<Vec<String>>` (walks *down* from a byte offset to
  the dotted key path containing it — the editor-hover helper's reverse
  lookup), plus `line_col_at(text, offset) -> (usize, usize)` (1-based,
  UTF-8-byte columns). 5 new tests.
- **`crates/settings/src/store.rs`** — `SettingsStore` gained
  `schema_errors`/`schema_warnings` (from `reload_user_layer`) and
  `project_schema_errors`/`project_schema_warnings` (from
  `reload_project_layer`, both new getters) — both computed via
  `schema::validate` right after each layer's existing JSON/JSONC parse and
  per-area `FieldError` parse, using the just-parsed `serde_json::Value` +
  raw text (so `find_value_range`'s line lookup can resolve). Cleared
  whenever a reload short-circuits on unparsable JSON/JSONC (same "stay at
  last-good-value" contract `parse_errors`/`user_json_error` already had).
  New `settings_schema_path()` (`~/.config/labonair/settings.schema.json`,
  next to `labonair-settings.json`) + `pub(crate) write_schema_file()`
  (best-effort, logged not fatal), called once from `labonair_settings::init`
  after the fs-watch spawn — satisfies the task's "schema file is written at
  startup" criterion without adding a second entry point callers have to
  remember to call.
- **`crates/settings-ui/src/view.rs`** — a second banner (`schema_banner`,
  next to the existing T19-005 JSON-syntax-error banner) lists every
  `schema_errors`/`project_schema_errors` entry in red (each formatted via
  `SettingsValidationError`'s new `Display` impl: `"{json_path}: {message}
  (line {line})"`) and every `schema_warnings`/`project_schema_warnings`
  entry in yellow, prefixed `"warning: "`. Both layers' lists are `.chain`ed
  so one banner covers whichever layer(s) are active.
- **`crates/workspace/src/views/editor.rs`** — the settings.json schema-
  hover helper (Anweisung #5; Autocomplete explicitly deferred — see the
  task file's Notizen for the reasoning): new `EditorView::hover: Option<
  (Point<Pixels>, SharedString)>` field, `is_settings_json()` (path equals
  `labonair_settings::user_settings_path()`, or is `.labonair/settings.json`
  under any project root), `byte_offset_for(Position)` (char-based Position
  -> byte offset, matching this editor's existing char-column convention),
  `update_hover(mouse_pos, cx)` (wired to a new `.on_mouse_move` listener on
  the text-area div — computes the doc `Position` at the mouse via the
  pre-existing `position_at`, then `json_path_at_offset` +
  `description_for_path`), and `render_hover_tooltip` (an absolute
  window-relative floating card glued to the last mouse position — not
  GPUI's trigger-based `.tooltip()` widget, since the hover target here is a
  specific character position inside one big text-row div rather than one
  fixed element). New `labonair-settings-json` dep on `labonair-workspace`
  (leaf, no cycle).
- **`scripts/check_crate_deps.py`** — fixed a pre-existing gap from T19-005
  (never caught because nothing runs this script as part of `cargo test`):
  `labonair-settings`'s `ALLOWED` entry was missing `labonair-settings-json`
  even though T19-005 already added that dependency. Added it, added a
  `labonair-settings-json` leaf entry, and added
  `labonair-workspace -> labonair-settings-json` for this task's editor-hover
  dep. `cargo metadata --no-deps --format-version 1 | python3
  scripts/check_crate_deps.py` now reports clean (24 crates, 98 edges,
  acyclic) — this script is not wired into `cargo test`/CI here, so it's
  worth an occasional manual run when touching cross-crate deps in the
  settings track.
- 15 new tests total (8 `schema.rs`, 5 `settings_json.rs`, 1
  `store.rs::project_layer_schema_error_is_reported`, 1 pre-existing test
  file's `schema_covers_every_area_and_an_enum` fixed to compare against a
  real serialized `SettingsContent` instance's keys rather than
  `AreaMeta::target_module` — `file_manager` (Rust field name) vs.
  `fileManager` (its `#[serde(rename)]`) don't match, so iterating
  `AREAS` directly for this check was wrong from the start). 4 gates green:
  `cargo fmt --check`, `cargo check --workspace --all-targets`, `cargo
  clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
  (workspace-wide, zero failures).

Branch: `master`, working tree has this session's changes staged for commit
(not yet committed as of this handshake write — see the commit right after
this entry lands, `feat(settings): T19-006 json schema generation`).

**Next: T19-007** (Globale Settings-Suche) —
`tasks/phase-18-settings-core/T19-007-global-settings-search.md`. No known
blockers.

## Last Session: 2026-09-04 (T19-005 — raw `settings.json` editor, comment-preserving)

**T19-005 done.** `settings.json` is now a first-class, comment-preserving
edit path alongside the generated Settings UI, matching Zed's
`settings_json`/`update_settings_file` design. New/changed:

- **`crates/settings-json/src/settings_json.rs`** (new crate,
  `labonair-settings-json`) — near-verbatim port of
  `zed-refrence/zed/crates/settings_json/src/settings_json.rs`:
  `update_value_in_json_text` (recursive old/new `serde_json::Value` diff,
  only rewrites the leaf(s) that actually changed), `replace_value_in_json_text`
  (finds a key path via a real `tree-sitter-json` syntax tree so `//`/`/* */`
  comments and trailing commas survive), array-index (`#N`) support,
  `infer_json_indent_size`. Trimmed vs. upstream: no `editing` cargo feature
  (this crate exists only for editing), `parse_json_with_comments` dropped
  (this repo already has a JSONC-tolerant parse path via `jsonc-parser` +
  `labonair_settings_content::parse`), `zed_util::RangeExt` inlined as a
  private helper so this crate has zero Zed-`util` dependency. Ported all of
  upstream's `object_replace`/`object_replace_array`/`array_replace`/
  `array_append`/`infer_indent` tests too (kept a representative subset in
  the port, the full upstream suite is >1900 lines and was already proven
  correct by Zed's own CI). Note: edition 2021 (this workspace, unlike Zed's
  2024) has no let-chains — three `if let … && …` spots from the Zed source
  had to be rewritten as nested `if let`.
- **`crates/settings/src/store.rs`** — `SettingsStore::update_user_settings`
  (the task's named entry point) replaces the old "dumb" whole-layer
  `persist_user_layer`: diffs the in-memory pre-mutation `SettingsContent`
  against the post-closure one, calls `labonair_settings_json::
  update_value_in_json_text` once over the whole tree (key_path starts
  empty — the recursion itself walks only areas whose leaves actually
  differ, so it correctly no-ops on everything unchanged and never touches
  top-level file keys that aren't part of `SettingsContent` at all, e.g. the
  legacy `preferences`/`dockLayout` blobs — those never enter the diff
  because the loop only ever iterates `old_object`/`new_object`'s own keys).
  Atomic `.tmp`+`rename` write, `.bak` on first write per process
  (`bak_written_this_session` field). `set_user_content`/`update_user` are
  now thin wrappers over `update_user_settings` — no other call site needed
  to change (`crates/settings-ui/src/view.rs::set_field_value` and
  `crates/settings-ui/src/store.rs`'s `mirror_into_settings_store` both
  already went through `update_user`, so T19-004's `SettingField.write` path
  got the surgical writer for free). New `user_json_error: Option<String>`
  field, set/cleared by `reload_user_layer` when
  `jsonc_parser::parse_to_serde_value` fails outright (computed as `Line N:
  <message>` from the error's byte-range offset) — `update_user_settings`
  refuses to write (`Err`, no file touch, no state mutation) while this is
  set, satisfying the "never blindly overwrite a file mid-edit" warning.
  Also added `pub fn user_settings_path()` / `pub fn
  ensure_user_settings_file()` (writes `crates/settings/assets/settings/
  initial_user_settings.json`, a new commented scaffold asset, only if the
  file doesn't exist yet — pattern-matched off the existing
  `ensure_project_settings_file`/`initial_project_settings.json` from
  T19-003).
- **`crates/settings-ui/src/view.rs`** — `set_field_value` now surfaces a
  blocked write via the existing `notify_error` toast instead of silently
  discarding it; `SettingsView::render` shows a persistent red banner up top
  ("labonair-settings.json has a syntax error (…) — fix it before changing
  settings here") whenever `SettingsStore::user_json_error()` is `Some`.
- **Command "Open Settings (JSON)"**: `Workspace::
  open_or_create_user_settings_json` (`crates/workspace/src/workspace.rs`,
  right after the pre-existing `open_or_create_project_settings` it mirrors)
  calls `ensure_user_settings_file` then `self.open_file(...)` to land it as
  a normal editor tab — external-change detection is free (`crates/editor`'s
  `Document::disk_mtime`/`EditorView::check_external` already exist and run
  on every tab-focus, per T19-003's same reuse). New
  `CommandId::OpenSettingsJson` (`crates/command-palette/src/palette.rs`,
  next to `OpenProjectSettings`), registered in
  `crates/shell/src/commands.rs::register_builtin_commands`. The pre-existing
  "Open settings.json" link in the Settings window's header still only does
  `cx.reveal_path` (Finder reveal) — left untouched (unrelated surgical
  scope; the new command is the in-app-editor path the task asked for).
- 12 new tests in `crates/settings/src/store.rs` +
  `crates/settings-json/src/settings_json.rs::tests` covering: comment/
  formatting preservation on a surgical write, missing-key insertion, a
  full GUI-write → file → fresh-store `reload_user_layer` round-trip, the
  no-op case (unchanged closure never touches the file), and the
  broken-JSON-blocks-writes + banner + last-good-`merged` case end to end.
  4 gates green: `cargo fmt --check`, `cargo check --workspace
  --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (all green, no new failures anywhere in the
  workspace).
- **Known, deliberately out of scope** (task's Notizen mark these
  optional/"Ggf."): no cross-process file lock for concurrent-window write
  interleaving (the atomic rename + fs-watch already prevents corruption,
  just not a rare lost-update race across two windows); per-area
  `parse_errors` (a single area having the wrong *shape* inside otherwise-
  valid JSON) does not block writes to other areas — only a fully unparsable
  file does, matching the task's specific "kaputtes JSON" wording.

Branch: `master`, working tree has this session's changes staged for commit
(not yet committed as of this handshake write — see the commit right after
this entry lands, `feat(settings): T19-005 raw json settings editor`).

**Next: T19-006** (JSON-Schema-Generierung) —
`tasks/phase-18-settings-core/T19-006-json-schema-generation.md`. No known
blockers.

## Last Session: 2026-09-04 (T19-004 — generated settings UI: `SettingField` registry + disclosure/scroll-spy/sub-page/custom-top-level navigation)

**T19-004 done — the P0-3 core Phase-18 task.** Replaced the old
hand-maintained `crates/settings-ui/src/fields.rs` (`FIELDS: &[FieldDef]`,
131 entries) + `sections.rs` (`SECTION_GROUPS`) with a UI generated from
`labonair_settings_content::SettingsContent` (T19-001) via the layered
`labonair_settings::SettingsStore` (T19-002/003), per
`docs/settings-guidelines.md`. New files:

- `crates/settings-ui/src/schema.rs` — `AnyField { json_path, control:
  FieldControl, meta: SettingsFieldMetadata, get: fn(&SettingsContent) ->
  Option<Value>, set: fn(&mut SettingsContent, Value) -> bool }`, `Copy`
  (every member is). `FieldControl` is the renderer registry: `Switch | Int |
  Float | Select(&[(token,label)]) | FontFamily | Text | Json` (the last is
  the rule-3 "anything else" raw-JSON fallback, used for every non-scalar
  field — `Vec`, `BTreeMap`, nested structs — so literally every
  `SettingsContent` leaf type has *some* renderer, satisfying reachability
  mechanically). One `field!` macro generates `get`/`set` for every entry via
  `serde_json::to_value`/`from_value` on the field's own real type — a new
  `bool` setting is one macro line, no widget code, and can never drift from
  the struct the way the old string-keyed table could. `all_fields()` — ~130
  macro-line entries covering all 12 `SettingsContent` areas (general,
  appearance, terminal, editor, file_manager, connections, workspace, ai,
  mcp, personalization, hosts, keymap).
- `crates/settings-ui/src/pages.rs` — `SettingsPage { area: &'static AreaMeta,
  body: PageBody, sub_pages: Vec<SubPage> }`, `PageBody::{Generated(Vec<
  SettingsPageItem>), Custom}`. `pages()` builds one page per `AREAS` entry
  (T19-001); curated `(section, &[local_key])` `Group` tables (ported almost
  verbatim from the old `SECTION_GROUPS`) for General/Appearance/Terminal/
  Editor/FileManager/Connections/Workspace; Terminal and Editor each get a
  `SubPage` (`terminal/advanced`, `editor/display`) splitting off their
  bigger groups (task Notizen: at least Terminal/Editor/AI need one). AI
  (`AreaKind::Custom`) folds its own `AI_GROUPS` field grid into its Custom
  body via the shared `render_field_groups` helper, then a `SubPageLink` row
  to a `providers` Custom sub-page (existing `render_providers`/
  `render_agents_section`/`render_directives_section`/`render_ai_editor`,
  relocated unchanged). Personalization similarly folds `PERSONALIZATION_
  GROUPS` (7 status-bar-button bools) above its existing bespoke layout
  editor. `leftover_fields(area, fields)` computes the trailing "Other"
  section for anything not in a curated group (and not exempted — see
  below), so a field can never silently vanish just because `pages.rs`
  forgot to place it. `DEDICATED_PANE_EXEMPTIONS: &[&str]` — a small,
  individually-commented allowlist of `json_path`s covered by pre-existing
  dedicated pane code that has real side effects a generic write wouldn't
  (MCP's 5 fields — `render_agent_bridge` also calls the live backend
  `mcp_set_port`/etc, not just a `SettingsContent` write), Hosts' 7 fields
  (explicitly deferred to T19-010 per this task's own Notizen — Hosts is a
  placeholder `render_hosts_placeholder` body for now), Personalization's 2
  `BTreeMap` fields (owned by the drag/drop layout editor), and
  `keymap.baseKeymap` (rendered directly by `render_shortcuts` as a
  "reset to preset" control). Three `pages::tests` prove every field is
  either placed, leftover-covered, or exempted — never silently unreachable
  — and that exemptions never *also* double-count in the leftover fallback.
- `crates/settings-ui/src/view.rs`/`panes/generic.rs` — `SettingsView` gained
  `active_area: usize` + `active_subpage: Option<usize>` (replacing the old
  flat `active_cat`), `all_fields: Vec<AnyField>` + `pages: Vec<SettingsPage>`
  (built once in `new()`), `collapsed_sections: HashSet<(area_idx, subpage_
  slug, section_label)>` (disclosure state, default all-open per rule 1),
  `content_scroll: ScrollHandle`. Real scroll-spy (not a fake click-only
  jump list): each generated page's rows are flat `ScrollHandle`-tracked
  children, `render_jump_bar` uses `ScrollHandle::top_item()` to find which
  section is topmost (highlighted) and `scroll_to_item(row_ix)` on click —
  confirmed real GPUI 0.2.2 API (`gpui-0.2.2/src/elements/div.rs`'s `impl
  ScrollHandle`, already used elsewhere in this repo at
  `crates/panel-ai/src/panel_ai.rs`). `render_field(&AnyField, …)` dispatches
  on `FieldControl` (reused the existing Switch/stepper/dropdown/text-box
  widgets verbatim — same visual language, just rewired to `AnyField::get`/
  `set` instead of the old `Preferences`-JSON-key path) and additionally
  renders an origin badge (`SettingsView::field_origin` → `SettingsStore::
  source_of`) + a "reset to default" link when non-default
  (`SettingsView::reset_field` — writes the field's `SettingsContent::
  defaults()` value back into the User layer; documented simplification
  since `SettingsStore` has no per-field unset, only whole-layer replace).
  `navigate_to_slug("terminal/advanced")` resolves `AREAS[].slug` + optional
  `SubPage::slug` for deep links (`SettingsTarget` global is now `Option<
  &'static str>` slug, not the deleted `SettingsTab` enum — the one external
  call site, `crates/shell/src/commands.rs`'s `OpenAiSettings` command,
  updated to `open_settings_window(Some("ai"), cx)`).
- **The `PreferencesStore`/`GlobalPreferences` bridge stays** (by design —
  `labonair-settings`'s own doc comment already called this out as
  intentional, not transitional) since terminal/editor/workspace/command-
  palette still read `GlobalPreferences`, not `SettingsStore` directly, and
  migrating every one of those call sites is explicitly out of this task's
  scope. Made it bidirectional in `store.rs`: `PreferencesStore::set_value`
  (the *old* per-key writer, still used unchanged by the Themes/Shortcuts/
  AI/Personalization panes) now also calls `mirror_into_settings_store`,
  writing the same value into `SettingsStore`'s `User` layer if a matching
  `AnyField` exists — keeps both trees consistent regardless of which path a
  given pane writes through. `SettingsView::set_field_value` (the *new*
  generated-grid writer) writes `SettingsStore` then calls `PreferencesStore
  ::reload_from_disk`, which derives `Preferences` from `Preferences::from(
  store.merged())` (the T19-001 `content_bridge` — already existed,
  `labonair-backend`) rather than re-reading the legacy flat JSON keys
  (those are never written by the new grid at all) — preserving exactly 3
  fields (`keybinds`, `bar_item_placements`, `bar_layout_migrated`) from the
  live in-memory value instead of the bridge's hardcoded defaults, since
  those 3 have no `SettingsContent` counterpart and would otherwise be
  silently clobbered on every unrelated generated-field write. `SettingsView
  ::new` also does `cx.observe_global::<SettingsStore>(|_, cx| cx.notify())`
  so an out-of-window write (e.g. project `.labonair/settings.json` editing,
  T19-003) repaints origin badges live, not just this window's own writes.
- `scripts/check_crate_deps.py` — added `labonair-settings`/`labonair-
  settings-content` to `labonair-settings-ui`'s allowed deps (was anticipated
  in a comment there since Phase-15). `bash scripts/check-crate-deps.sh`
  still green, acyclic.
- Deleted as genuinely orphaned by this change (not just "old"):
  `render_appearance` + its `bg_tile` helper (`panes/themes.rs`/`view.rs`) —
  `appearance` is `AreaKind::Generated` per `AREAS`, so its fields (incl.
  `background*`) now render through the generic grid; the old bespoke
  color-scheme-pill/background-gallery widgets had no equivalent slot in the
  new model and are a documented, Non-Goals-sanctioned visual simplification
  (the *settings* are unchanged and fully functional, just via a generic
  Text/Int/Select row instead of a custom picker). Also removed `apply.rs`'s
  `trim_ext` (only caller was the deleted background-file list).

**Deliberately out of scope / deferred** (per the task's own Notizen or
genuine architectural limits hit while implementing):
- Full migration of terminal/editor/workspace/command-palette off
  `GlobalPreferences` onto `Settings::get(cx)` directly — the bridge above
  keeps them working unchanged; that migration is a separate, much larger
  future task (referenced in `labonair-settings`'s own doc comment).
- A live `SettingsStore`-integrated GPUI test for `set_field_value` (write →
  `SettingsStore` → `pick` round-trip through the real global) — skipped
  because `labonair-settings`'s only public store constructor is `init(cx)`,
  which always targets the real `~/.config/labonair` path (no public
  test-mode constructor exists in that crate to point at an isolated temp
  dir from outside it). Covered instead by three separate, narrower test
  layers: `schema::tests::get_set_round_trips_on_the_default_tree` (pure
  `SettingsContent`-level, no GPUI global needed) + `labonair-settings`'s own
  `store::tests::*` (proves the layered-merge/persist mechanics generically,
  already 23 tests) + `pages::tests::*` (proves every field reaches some
  page). A future task adding a `labonair_settings::init_for_tests(cx, path)`
  hook would close this gap cleanly.
- Old per-field conditional visibility (`field_visible` — e.g. hide
  `sessionScrollbackLines` unless `sessionRestore` is on) was not
  reimplemented; every generated field is always shown now (a UX
  regression, not a correctness one — not required by any T19-004 AC).
- Not verified with `cargo run` — headless VPS, no GUI window (same
  standing caveat as several prior settings/titlebar tasks).

All gates green: `cargo fmt --check`, `cargo check --workspace
--all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` (0 failures across all 23 workspace crates;
`labonair-settings-ui` alone: 24 tests, all new/rewritten ones green;
`labonair-settings-content`: 14; `labonair-settings`: 23 unchanged),
`scripts/check-crate-deps.sh` (still acyclic, 96 edges, two new ones added
and pre-anticipated).

**State:** branch `master`. Next task: **T19-005** (raw `settings.json`
editable, comment-preserving — depends on T19-002 + this task, both done).

## Last Session: 2026-09-04 (T19-003 — project/folder settings layer, `.labonair/settings.json`)

**T19-003 done.** Builds directly on T19-002's `SettingsStore`
(`crates/settings`). New module `crates/settings/src/project.rs`:
- `PROJECT_SETTINGS_WHITELIST` — `(area, &[leaf keys])` allowlist, only
  `general` (startup tab/count/session-restore/restore-window-state),
  `workspace` (dock/sidebar/command-palette/bookmarks layout knobs), `editor`
  (format-on-save/tab-size/trim/insert-final-newline/word-wrap/…). Every
  other top-level area (`hosts`, `ai`, `mcp`, `connections`, `keymap`,
  `appearance`, `file_manager`, `personalization`) is dropped **entirely**
  from a project file — deliberately narrower than the task's Kontext
  examples ("Default-SSH-Host reference", "AI-directives file reference"):
  no safe scalar field for either exists yet in `SettingsContent`, so
  opening `hosts`/`ai`'s *existing* fields (credential-adjacent / network
  endpoints) would be a real hole. Adding such a field is a future, narrower
  whitelist entry, not something this task should improvise.
- `filter_and_parse(raw) -> (SettingsContent, Vec<String> rejected)` —
  whitelist-filters the raw JSON/JSONC first, then reuses
  `labonair_settings_content::parse`'s per-area fault tolerance on what's
  left. Never fails on garbage input.
- `ensure_project_settings_file(root)` — creates `<root>/.labonair/
  settings.json` from a commented scaffold
  (`crates/settings/assets/settings/initial_project_settings.json`,
  fully-commented so it parses to `{}` until edited) if missing; never
  overwrites, never touches git.

`SettingsStore` (`store.rs`) additions: `current_project: Option<(PathBuf,
WorktreeId)>` + `next_worktree_id` (v1 = one root at a time, fresh id per
switch, per the task's own Notizen), `project_watch_generation: u64` (bumped
on every real root change — the fs-watch poll task compares its captured
generation against the live one each tick and self-terminates on mismatch;
this is how "old watch unregisters, new one registers" works without a
cancel handle surviving the `cx.spawn` boundary), `project_rejected: Vec
<String>` (deduped-logged rejections from the last load).
`set_active_project_root(root)` (no-op if the canonicalized root is
unchanged), `reload_project_layer`, `rewatch_project` (force-reload +
fresh watch for the "just created `.labonair/`" case, where the root itself
didn't change so `set_active_project_root` alone would stay a no-op), and
`source_of(json_path: &str) -> SettingsLayer` (dot-path JSON walk over
every layer's `serde_json::to_value`, highest-precedence non-null wins —
generic, works for `User`/`Project`/any future layer, not project-specific).
`watch.rs` gained `spawn_project`/`keep_alive_and_poll_project` mirroring
the existing user-file watch but generation-guarded.

Because `labonair-settings` must stay a leaf crate
(`scripts/check_crate_deps.py`'s `ALLOWED["labonair-settings"]` = only
`labonair-settings-content` + `labonair-settings-macros` — no
`labonair-workspace`), the "which folder is the project root" question is
answered by the crate-root wrappers `labonair_settings::
set_active_project_root(cx, root)` / `refresh_project_watch(cx)` in
`settings.rs`, called **from** `labonair-workspace`:
`Workspace::sync_project_settings_root` (`crates/workspace/src/
workspace.rs`, new — diffs `active_cwd(cx)` against a new
`last_project_settings_root` field, called once per `render`, cheap no-op
when unchanged) and `Workspace::open_or_create_project_settings` (new
command: `ensure_project_settings_file` + `refresh_project_watch` + opens
the file via the existing `open_file` editor-tab path — there was no prior
"open a file as an in-app editor tab from a settings context" precedent;
`labonair-settings-ui` only ever does `cx.reveal_path` (Finder), which
doesn't fit "öffnet sie im Editor"). Wired as `CommandId::
OpenProjectSettings` in `crates/command-palette/src/palette.rs` +
registered in `crates/shell/src/commands.rs` (palette-only, no keybinding,
same as most other Application-section commands).

**Not verified with `cargo run`** — headless VPS, no GUI window (same
caveat as several prior titlebar/overlay tasks, see `docs/architecture.md`
§8.13/§8.14). Settings-UI display of `source_of`'s origin badge is
explicitly T19-004's job per the task text itself ("hier mindestens die API
bereitstellen") — not done here.

All gates green: `cargo fmt --check`, `cargo check --workspace
--all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` (0 failures across all 23 workspace crates,
`labonair-settings` alone: 23 tests, all new ones — 5 in `project.rs`, 7 new
in `store.rs`), `scripts/check-crate-deps.sh` (still acyclic, no new edges).

**State:** branch `master`. Next task: **T19-004** (generated settings UI —
field-renderer-from-Rust-type, per `docs/settings-guidelines.md`). Verified
its dependencies (T19-002, T16-007, T19-000) are all `✅ Done` by checking
each task file's own `## Abhängigkeiten` + `## Status` directly, not
assumed.

## Previous Session: 2026-09-04 (T19-002 — `SettingsStore` layered merge + `Settings` trait)

**T19-002 done.** New crate `crates/settings` (`labonair-settings`), workspace
member. Its only workspace deps are `labonair-settings-content` +
`labonair-settings-macros` (enforced by `scripts/check-crate-deps.sh`); no UI
crate, no `labonair-backend`.

- `store.rs`: `SettingsLayer` enum (`Default < User < Os < Profile <
  Project(WorktreeId) < Language(String)`) — **declaration order is the merge
  order** because `#[derive(Ord)]` on an enum compares by variant index
  first, so a `BTreeMap<SettingsLayer, SettingsContent>`'s key iteration
  order *is* the normative merge order for free (no separate `rank()` fn
  needed). `SettingsStore` is a GPUI `Global`; every mutator only reaches its
  fields through `cx.global_mut::<SettingsStore>()`, which GPUI's
  `Effect::NotifyGlobalObservers` already turns into an automatic
  `cx.observe_global::<SettingsStore>` notification on `recompute` — no
  bespoke observer list was needed.
  `Os`/`Profile`/`Project`/`Language` are structurally present but always
  empty this task (no `platform_overrides`/`profiles` wrapper exists yet in
  `SettingsContent`; `Project` is T19-003's job).
- `reload_user_layer`: reads `~/.config/labonair/labonair-settings.json`.
  Genuinely-corrupt JSON (fails even `jsonc_parser::parse_to_serde_value`)
  keeps the **last-good** `User` layer rather than resetting to empty — a
  distinct, stronger guarantee than `labonair_settings_content::parse`'s own
  per-area fallback (which still applies for a file that parses as JSON but
  has one broken area — that area alone defaults, the rest of the tree is
  unaffected).
- `persist_user_layer` ("dumb" write, T19-005 makes it surgical): re-reads
  the file as a generic JSON object, overwrites only the 12
  `SettingsContent` top-level keys, preserves every other key
  (`preferences`, `dockLayout`, …) untouched, `.bak`-copies the existing file
  first, atomic tmp+rename.
- `trait Settings` (`settings_trait.rs`): `from_settings`, `register(cx)`,
  `get(cx) -> &Self` (panics if never registered — mirrors Zed), plus a
  **new, non-Zed** `try_get(cx) -> Option<&Self>` for render paths that might
  run before `labonair_settings::init` (headless test harnesses).
- `#[derive(RegisterSetting)]` added to `labonair-settings-macros` (alongside
  the existing `MergeFrom` derive) — emits `inventory::submit! {
  RegisteredSetting { register: |cx| <Self as Settings>::register(cx) } }`.
  Only usable from within `labonair-settings` itself: the generated code
  addresses `::labonair_settings::{inventory,gpui,Settings,RegisteredSetting}`
  absolutely, resolvable via `extern crate self as labonair_settings;` in
  `settings.rs` (crate root re-exports `gpui`/`inventory` for exactly this).
- 6 concrete `Settings` structs (`concrete.rs`), each a newtype over its
  `SettingsContent` area merged onto that area's own `defaults()` so every
  field is guaranteed `Some`: `ThemeSettings` (wraps `appearance` —
  named for the `themes` custom-category `target_module`, not
  `AppearanceSettings`), `TerminalSettings`, `EditorSettings`, `AiSettings`,
  `WorkspaceSettings`, `PersonalizationSettings`.
- Real consumers (the task's explicit minimum): `ThemeSettings::try_get(cx)
  .reduce_motion()` in `crates/workspace/src/workspace.rs`'s `render_tab`
  (was `GlobalPreferences`); `TerminalSettings::try_get(cx)` for
  `terminal_opacity`/`copy_on_select`/right-click-pastes in
  `crates/workspace/src/views/terminal.rs`. `GlobalPreferences`/
  `PreferencesStore` are otherwise **untouched** — full migration off them is
  explicitly out of scope per the task's `## Notizen`; `try_get` (not `get`)
  was used at both sites specifically so nothing panics if a future headless
  test never calls `labonair_settings::init`.
- `watch.rs`: live fs-watch, same `notify` + `notify_debouncer_mini` pattern
  as `labonair_backend::modules::fs::watcher`. The debouncer callback (a
  background thread) only flips an `AtomicBool` — never touches GPUI
  directly, per the task's `## Warnungen`; a `cx.spawn`ed foreground `Task`
  polls that flag every 150 ms and calls `reload_user_layer` via
  `cx.update`. Watches the settings file's **parent directory**, not the
  file itself (an atomic rename-based save, which is exactly how
  `persist_user_layer` writes, replaces the inode a direct file-watch would
  go stale against) and matches by `file_name()`, not full path (macOS
  `/var/folders/...` vs `/private/var/folders/...` — see Bug memory).
- `init(cx)` wired into `crates/app/src/main.rs`, right after
  `labonair_shell::init_fonts` and before `gpui_component::init` — loads
  layers, calls `register_all(cx)`, starts the watch.
- 26 tests total in `labonair-settings` (13 unit/registry/watch — one is a
  real-filesystem `notify` round-trip, no GPUI clock involved on purpose —
  see Bug memory for why) + `#[gpui::test]`s proving observer notification
  and full registration.

### Gate results
`cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
`cargo clippy --workspace --all-targets -- -D warnings` ✅ · `cargo test
--workspace` ✅ (no failures anywhere) · `scripts/check-crate-deps.sh` ✅ (23
workspace crates, 94 internal edges, acyclic).

### Next task
**T19-003** (project/folder settings,
`tasks/phase-18-settings-core/T19-003-project-folder-settings.md`) — its own
`## Abhängigkeiten` lists only T19-002, now done. (T19-004 also only needs
T19-002 + already-done tasks, but T19-003 is the lower-numbered one and
nothing blocks it from going first.)

---

## Session: 2026-09-04 (T19-001 — `labonair-settings-content` tree + `MergeFrom`)

**T19-001 done.** New crates `crates/settings-content` (`labonair-settings-content`)
and `crates/settings-macros` (`labonair-settings-macros`), added as workspace
members. Both are leaves — no GPUI/UI/`labonair-backend` deps.

- `labonair-settings-macros`: `#[proc_macro_derive(MergeFrom)]` — field-wise
  `self.field.merge_from(&other.field)`, emits `impl crate::MergeFrom for …`
  (only ever used from within `labonair-settings-content` itself).
- `labonair-settings-content::merge_from`: trimmed port of Zed's `MergeFrom`
  trait (`Option<T>` recurses/replaces, `BTreeMap<K,V>` merges key-wise,
  everything else incl. `Vec<T>` overwrites).
- 12 area modules (`general`, `appearance`, `terminal`, `editor`,
  `file_manager`, `connections`, `hosts`, `workspace`, `ai`, `mcp`,
  `personalization`, `keymap`), each `Option<T>`-leafed, camelCase serde,
  `#[derive(... schemars::JsonSchema, MergeFrom)]`, own `defaults()`.
  `hosts::HostsContent` is the new **top-level** area (`entries:
  Vec<HostEntry>` — fresh non-secret model: `HostAuthMethod`/`HostEntry`
  (`credential_ref: Option<String>`, never a password/key)/`HostTunnel`;
  `T19-010` reconciles this with the SQLite `hosts::db::Host` row).
- `areas::AREAS`: the category registry (`AreaMeta{key,title,slug,kind,
  target_module}`), `Generated` = {general, appearance, terminal, editor,
  file_manager, connections, workspace}, `Custom` = {themes, hosts,
  shortcuts, ai, mcp, personalization} per `docs/settings-guidelines.md`
  rule 4.
- `fallible::parse`: JSONC-tolerant (via `jsonc-parser`), fault-tolerant at
  **area granularity** (documented deviation from Zed's leaf-level
  `FallibleOption` — see the task file's `## Notizen`).
- `assets/settings/default.json`: commented JSONC, `SettingsContent::defaults()
  == parse(DEFAULT_JSON)` enforced by a test.
- `impl From<&SettingsContent> for Preferences` lives in **`labonair-backend`**
  (`crates/backend/src/modules/settings/content_bridge.rs`), not in
  settings-content (orphan-rule/dependency-direction constraint) — so
  `labonair-backend` gained a new, documented one-way edge to
  `labonair-settings-content`. Recorded as a deviation in
  `docs/architecture.md` §8.15 (updates the old "`labonair-backend` → leaf"
  Ist-Graph line from §9).
- 14 tests in `labonair-settings-content`, 3 in the new
  `content_bridge` module (`labonair-backend`) — all new, all green.

### Gate results
`cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
`cargo clippy --workspace --all-targets -- -D warnings` ✅ · `cargo test
--workspace` ✅ (no failures anywhere in the workspace).

### Next task
**T19-002** (`SettingsStore` + layered merge + `Settings` trait,
`tasks/phase-18-settings-core/T19-002-settings-store-layered-merge.md`) — its
own `## Abhängigkeiten` lists only T19-001, now done.

---

## Session: 2026-09-04 (T19-000 — Settings design contract)

**T19-000 done.** Phase 17 (T18-001..T18-007) was already complete; this is
the first task of Phase 18 — Settings-System Zed-Style. Pure documentation
task, no code changes.

- New `docs/settings-guidelines.md`: the 9-point settings design contract
  (one navigation model / every setting is a typed `SettingsContent` field /
  field UI generated from the Rust type / custom panes as a sanctioned
  first-class path that still uses standard chrome / origin+reset on every
  field / global search / deep-link slugs / copy rules / no window sprawl),
  plus a Non-Goals section (pixels/widgets are `T19-004`'s job, not this
  doc's) and a Deviation process section (must cite the rule, record the
  rationale in `docs/architecture.md`, scoped to that page only — not a
  blanket exemption).
- `CLAUDE.md`: added Critical Rule 9 ("Settings-Design-Kontrakt einhalten"),
  pointing at the new doc. Rules 1–8 left untouched.
- `docs/architecture.md` §8.3 and `tasks/ROADMAP.md` (Phase-18 table listing
  T19-000 first, Erfolgskriterium 25's `docs/settings-guidelines.md`
  half-sentence) already referenced this contract from earlier planning —
  verified both match what was written, no further edits needed there.
- This is now **normative**: any settings page/field added from `T19-001`
  onward is reviewed against `docs/settings-guidelines.md`; deviations must
  be justified and recorded per its Deviation process, not silently made.

### Gate results
`cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
`cargo clippy --workspace --all-targets -- -D warnings` ✅ · `cargo test
--workspace` ✅. No code touched — `git diff --stat` for the commit shows
only `.md` files (`CLAUDE.md`, `docs/settings-guidelines.md`,
`tasks/phase-18-settings-core/T19-000-settings-design-contract.md`).

### Next task
**T19-001** (`labonair-settings-content` — typisierter Baum + `MergeFrom`,
`hosts` als eigener Top-Level-Bereich), file:
`tasks/phase-18-settings-core/T19-001-settings-content-tree.md`. Its
dependencies (T16-001, T16-007, T19-000) are all Done. Note: T19-002
(`SettingsStore` + layer merge) depends on T19-001, not directly on T19-000 —
so T19-001 is next in strict roadmap order, not T19-002.

### Note on repo state at session start
An unrelated submodule pointer drift in `zed-refrence/zed` (1-line diff, not
part of this task) was present before this session and left untouched —
not committed here.

## Last Session: 2026-09-04 (T18-007 — Philosophy verankert + Personalization settings page)

**T18-007 done.** Phase 17 is now complete (T18-001..T18-007 all Done).

### Part A — Philosophy is now normative
- `tasks/ROADMAP.md` "## Vision": added the four numbered principles from
  `bericht-architektur-rework-roadmap.md` §1 (simple fixed structure /
  personalization is first-class / modularity in code = modularity in
  product / measurable performance) inline, alongside the pre-existing
  "parity is the minimum, not the goal" sentence.
- `CLAUDE.md` (repo root): new "## Philosophie (ab Architektur-Rework)"
  section under the title block (leitsatz + link to `docs/architecture.md`);
  Critical Rules got an added **Rule 8** ("Layout-Vertrag einhalten") —
  rules 1–7 left untouched per the task's instruction.

### Part B — "Personalization" settings page
New settings category (`crates/settings-ui/src/panes/personalization.rs`,
`SettingsTab::Personalization`, deep-link slug `personalization`, inserted
into `CATEGORIES` right after "Appearance & Layout"). Two sections:
- **Statusbar layout editor** — three columns (Left / Right / Hidden) of
  chips (icon + title) for every placeable `StatusItem` except the fixed
  `panel-toggles` cluster. Per chip: ←/→ move buttons + hide/show icon
  button. "Reset to default" clears the whole `statusBarItemPlacements`
  blob. **Every write goes through the exact same
  `Workspace::set_status_bar_placement`** the in-app right-click menu
  (`crates/workspace/src/status_bar.rs`) already called (T18-005) — no
  second write path. Added `Workspace::reset_status_bar_placements` (clears
  local overrides + calls new `settings_clear_status_bar_placements` +
  bumps `StatusBarLayoutTick`) for the reset button.
- **Panel visibility** — one switch per registered panel
  (`Workspace::panel_registry()`). New persisted blob
  `panelToggleVisibility: { panelName: bool }` (backend functions
  `panel_toggle_visibility_load[_from]` / `settings_set_panel_toggle_visibility`
  / `set_panel_toggle_visibility_in`, mirroring the existing
  `statusBarItemPlacements` read-merge-write pattern in
  `crates/backend/src/modules/settings/mod.rs`, new `PanelToggleVisibilityLock`
  on `backend::App`). New shared write path **`Workspace::set_panel_toggle_visible`**
  — both this pane's switches and `PanelTogglesStatusItem`'s "Hide from
  toggle bar" right-click action (`crates/shell/src/status_items.rs`) call
  it; the item's previously session-only `hidden: HashSet` is now loaded
  from the persisted blob at construction and reloaded whenever
  `StatusBarLayoutTick` bumps (same global as T18-005, now also covers
  panel-toggle visibility — no new global introduced). Hiding a panel's
  toggle does **not** close the panel or touch its dock position; it stays
  reachable from the command palette (`Workspace::select_panel` is
  independent of the toggle-visibility blob).

### Plumbing to make this possible
- `labonair-settings-ui` gained a direct `labonair-panel` dependency (to
  name `StatusSide`/`PanelIcon`/`DockPosition` — it already reached these
  transitively via `labonair-workspace`, but Rust requires a direct dep to
  name a type). Updated `scripts/check_crate_deps.py`'s allow-list +
  `docs/architecture.md` §8.6 (new paragraph, explicitly *not* a
  resurrection of the T18-005-deleted `BarLoc` machinery).
- `SettingsView` now holds `Entity<labonair_workspace::Workspace>`
  (`SettingsView::new`'s 6th param); `SettingsDeps`/`set_settings_deps`
  (`crates/settings-ui/src/window.rs`) carry it too, published from
  `crates/shell/src/bootstrap.rs` right after `set_settings_deps(...)`'s
  existing call site (the `workspace` entity already existed earlier in
  `bootstrap()`).

### Gate results
`cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
`cargo clippy --workspace --all-targets -- -D warnings` ✅ · `cargo test
--workspace` ✅ (all crates green, no new test failures) ·
`scripts/check-crate-deps.sh` ✅ (20 crates, 88 edges, acyclic — one new
edge, `settings-ui → panel`, allow-listed).

### Non-obvious lesson (see also `bugs_and_fixes.md`)
A local closure that itself takes a generic `f: impl Fn(...)` parameter
does not compile (`E0562: impl Trait is not allowed in closure
parameters`) — only `fn` items can have `impl Trait` params. Had to hoist
the two little per-chip action-button builders out of
`render_statusbar_chip` into free `fn`s (`statusbar_glyph_btn` /
`statusbar_icon_btn`) instead of local closures capturing `id`/`c`.

### Current state
Branch `master`, working tree clean after this session's commit. Phase 17
(T18-001..T18-007) fully done.

### Next
**T19-000** (`tasks/phase-18-settings-core/T19-000-settings-design-contract.md`)
— first task of Phase 18 (Settings-System Zed-Style). No known blockers.

## Last Session: 2026-09-04 (T18-006 — Migrator `barItemPlacements` → `statusBarItemPlacements`)

**T18-006 done.** One-time, idempotent migrator for the legacy
`barItemPlacements` blob into T18-005's `statusBarItemPlacements` schema, so
users don't lose their titlebar/statusbar item customisations across the
update.

### What Was Done (T18-006)
- **`crates/backend/src/modules/settings/migrations.rs`** (new) —
  `migrate_bar_item_placements(dir: &Path) -> Result<MigrationOutcome, String>`.
  `MigrationOutcome::{AlreadyMigrated, NothingToMigrate, Migrated { migrated,
  discarded }}`. No-op if `statusBarItemPlacements` already exists or if
  `barItemPlacements` is absent. Otherwise: `.bak`s the whole settings file,
  transforms every recognised old id into the new schema, renames the old
  key to `barItemPlacements_legacy` (kept, not deleted — safety net), writes.
  4 unit tests (transform+remap, drop-obsolete, idempotency, empty case).
- **Called from `crates/shell/src/bootstrap.rs`**, first line of `bootstrap()`
  — before `register_builtin_status_items` (which calls
  `w.reload_status_bar_placements()` right after registering every item, so
  the migration must land before that first read). Logs the outcome via
  `tracing`.
- **`docs/architecture.md` §8.6** rewritten (was describing T17-003's
  transitional state as still current — stale since T18-005 already deleted
  `BarLoc`); now documents the T18-005+T18-006 end state including the id
  mapping table. §4's "titlebar scope of bar items" bullet cross-references
  it.

### Important discovery (would have been a silent data-loss bug)
The task file's own id list (`jump-hosts`, `agent-access`, `cwd-breadcrumb`,
etc. — implying old ids equal new ids) is **wrong**. Checked the real
`~/.config/labonair/labonair-settings.json` (dev machine) plus
`StatusItem::id()` impls in `crates/shell/src/status_items.rs`:
- **Old ids are camelCase** (`agentAccess`, `jumpHosts`, `cwdBreadcrumb`,
  `previewUrl`, `cursorPosition`, `ai`, `aiMini`, `aiPanel`, `explorerPanel`,
  `snippetsPanel`, `sourceControlPanel`, `tabsPanel`, plus same-name
  `bookmarks`/`notifications`/`transfers`/`updater`) — the pre-T18
  `BarItemId` enum's serde rename.
- **New ids are kebab-case** and only 9 are placeable + 1 fixed
  (`panel-toggles`): `agent-access`, `jump-hosts`, `cwd` (not
  `cwd-breadcrumb`!), `preview-url`, `cursor-position`, `bookmarks`,
  `notifications`, `transfers`, `updater`.
- A "keep known ids unchanged" migrator (what the task prose implied) would
  have silently dropped `agentAccess`/`jumpHosts`/`cwdBreadcrumb`/
  `previewUrl`/`cursorPosition` — 5 of 9 real placements — on every real
  user's machine. Implemented an explicit `ID_MAP: &[(&str, &str)]` instead,
  verified end-to-end against the real dev settings file (see below), not
  just unit tests with made-up fixture ids.
- If a future task adds/renames a `StatusItem::id()`, this migrator's
  `ID_MAP` does **not** need updating (it's a one-time historical migration
  from the pre-T18 schema) — but flag it if another schema migration is ever
  needed, the "verify against the real running id, not the task doc's
  prose" lesson generalizes.

### Manual verification (beyond the unit tests)
Ran the actual `cargo run` against the real dev
`~/.config/labonair/labonair-settings.json` (backed up first to
`/tmp/labonair-settings.pre-migration-backup.json`, plus the migrator's own
`.bak`): 16 old entries → 9 migrated / 7 discarded (`ai`, `aiMini`, `aiPanel`,
`explorerPanel`, `snippetsPanel`, `sourceControlPanel`, `tabsPanel` — matches
the id-mapping table exactly), verified only the 2 intended top-level keys
changed (everything else in the settings file byte-identical), no panic. Ran
a second time — `AlreadyMigrated`, file untouched (idempotency holds on real
data, not just the temp-dir unit tests).

### Gate Results (T18-006)
- `cargo fmt` ✅ · `cargo check --workspace --all-targets` ✅ · `cargo clippy
  --workspace --all-targets -- -D warnings` ✅ · `cargo test --workspace` ✅
  (`labonair-backend` 226 tests incl. the 4 new migration tests) ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, 87 edges, acyclic, no new
  edge — `shell → backend` already existed) · manual `cargo run` walkthrough
  against real data as described above (this session's task explicitly
  called for it, unlike prior GUI-only tasks that had to defer this to the
  user).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T18-007** — Philosophie verankern + Personalisierungs-Settings-Seite
  (`tasks/phase-17-layout/T18-007-philosophy-and-personalization-page.md`),
  deps: T18-005 (done), T18-003 (done).

---

## Previous Session: 2026-09-04 (T18-005 — Statusbar item personalization: right-click move/hide)

**T18-005 done.** The user-requested core feature: right-click any statusbar
item (except the fixed-left panel toggles) → "Move left" / "Move right" /
"Hide", persisted and synced across windows.

### What Was Done (T18-005)
- **`crates/panel/src/status.rs`** — new `StatusPlacement { side, hidden }`;
  `StatusItemRegistry` gained an `overrides: HashMap<String, StatusPlacement>`
  table + `set_overrides`/`set_override`/`override_for`; `resolve_side`/
  `is_hidden` now consult it (falling back to `default_side`/visible). 5 new
  unit tests.
- **`crates/backend/src/modules/settings/mod.rs`** — `statusBarItemPlacements`
  blob functions mirroring the existing `barItemPlacements` ones 1:1:
  `StatusBarPlacementLock`, `status_bar_item_placements_load[_from]`,
  `settings_set_status_bar_placement`/`set_status_bar_placement_in` (atomic
  read-merge-write + `.tmp`+rename). New `status_bar_lock` field on
  `backend::App`. Round-trip/merge test added.
- **`crates/workspace/src/bar_items.rs` → `status_placements.rs`** (renamed,
  content fully replaced) — the old titlebar-and-statusbar-spanning
  `BarItemId`/`BarLoc`/`Placements` enum system (a closed 15-variant list from
  before T17) is gone; it had **zero live consumers** (titlebar/statusbar have
  rendered from their own string-keyed registries since T17-003/T18-001).
  Replacement is a thin pure module: `overrides_from_blob` (JSON → `HashMap<String,
  StatusPlacement>`), `placement_patch` (partial JSON patch), and the new
  `StatusBarLayoutTick` global (cross-window reactivity tick).
- **`crates/workspace/src/workspace.rs`** — `Workspace::reload_status_bar_placements`
  (re-reads the blob, applies to the registry) and
  `Workspace::set_status_bar_placement(id, side, hidden, cx)` (applies the
  override locally + `cx.notify()` immediately, spawns the backend write, and
  only *after that write completes* bumps `StatusBarLayoutTick` via
  `cx.spawn` — bumping eagerly would race the write and this window's own
  tick-observer would reload the pre-write blob and clobber the override just
  set).
- **`crates/workspace/src/status_bar.rs`** — every cluster item except
  `"panel-toggles"` is now wrapped in a `.relative()` div with an
  `on_mouse_down(Right)` → `labonair_ui_kit::context_menu` (Move left / Move
  right / Hide, each disabled when already in that state; Hide omitted unless
  `hideable()`). `cluster()` now resolves side/hidden per item through the
  registry every render (was: static `default_side` captured once at build).
  `StatusBar::new` adds `cx.observe_global::<StatusBarLayoutTick>` → reloads +
  re-renders (the "two windows stay in sync" criterion).
- **`crates/shell/src/status_items.rs`** — registration now calls
  `w.reload_status_bar_placements()` right after registering every item (so
  overrides apply from the first paint); new `status_item_label(id)` helper
  (human titles) for the palette page below.
- **Command palette escape hatch** (criterion "hidden items reachable"): new
  `Page::StatusBarHidden` + `CommandId::ShowStatusBarItem` ("Statusbar: Show
  Hidden Item…") + `PaletteData.status_bar_hidden` + `PaletteEvent::ShowStatusBarItem`,
  wired end-to-end in `crates/shell/src/actions.rs` (`build_palette_data` /
  `handle_palette_event`) — mirrors the existing AI-sessions/git-branches
  sub-page pattern exactly.
- **`crates/settings-ui`** — the old "Titlebar & Status Bar Items" layout
  editor (`panes/themes.rs` `render_layout_editor`/`layout_seg`,
  `view.rs` `move_bar_item`/`reset_bar_layout`/`persist_bar_item`/`placements`
  field) is **deleted** — it was the only consumer of the old `BarLoc` system,
  had no titlebar-scope equivalent in the new model, and duplicating it as a
  "statusbar-only" editor now would just be redone by T18-007's real
  personalization page. `crates/shell/src/bootstrap.rs`'s
  `observe_global::<bar_items::BarLayoutTick>` (a bare repaint trigger, no
  actual data flow) and the `crates/shell/src/shell.rs` `bar_items` re-export
  shim are removed too — nothing else referenced them.

### Deviations (documented here per the project's convention, not in the task file)
- Point 7 ("Migrator … hier nur sicherstellen, dass das neue Schema sauber
  ohne Altdaten funktioniert") is satisfied trivially: the old
  `barItemPlacements` blob was never consumed by anything that affected
  rendering, so there was no real user data to worry about clobbering. The
  actual migrator ticket (T18-006) still stands as its own task if a
  best-effort import of any `barItemPlacements`-shaped side hints is wanted.
- Point 5 ("Personalisierungs-Seite T18-007 und/oder Command-Palette") — only
  the command-palette page was built this session (page + event wiring); the
  settings personalization page itself is still T18-007's job.

### Gate Results (T18-005)
- `cargo fmt` ✅ (ran, no diff after) · `cargo check --workspace --all-targets` ✅
  · `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `cargo test --workspace` ✅ (no failures; `labonair-panel` 10 tests,
  `labonair-workspace` 92 tests — both include the new override/blob tests) ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, 87 edges, acyclic, no new edge)
  · `cargo build` + a 5s `cargo run` smoke-launch succeeded with no panic (log
  clean past the CoreText font warnings that predate this change) — this
  session had no way to interact with the GUI, so the actual right-click →
  menu → move/hide → persist → cross-window-sync behavior described in the
  task's `cargo run` walkthrough (point 8) is **not** manually verified; flagged
  for the user's visual pass, same caveat as T18-001/002/003/004.

### State
- Branch `master`, committing now. No blockers.

### Next
- **T18-006** — Migrator `barItemPlacements` → `statusBarItemPlacements`
  (`tasks/phase-17-layout/T18-006-bar-item-placements-migrator.md`) — now that
  the old blob-parsing code (`BarItemId`/`BarLoc`) is gone from
  `labonair-workspace`, this task's framing needs a quick re-read before
  starting: it may reduce to "read the legacy `barItemPlacements` JSON keys
  directly with `serde_json` and best-effort map any recognizable ids into
  `statusBarItemPlacements`" rather than reusing removed types.

---

## Last Session: 2026-09-04 (T18-004 — Statusbar right: info dropdowns)

**T18-004 done.** Polish pass over the right-side statusbar cluster (`crates/shell/src/status_items.rs`) — default order, a shared popover primitive, dividers between logical groups, and a couple of real behavior fixes that fell out of reading the acceptance criteria closely.

### What Was Done (T18-004)
- **`crates/panel/src/status.rs`** — new `StatusItem::group()` (default 0) + `StatusItemRegistration::group` + `StatusItemHandle::group`, a second sort key alongside `order`.
- **`crates/workspace/src/status_bar.rs`** — `StatusBar::cluster` draws a 1px divider between consecutive items whose `group()` differs, never within a group.
- **`crates/shell/src/status_items.rs`** — reordered every right-side item to the task's default order (doc comment on `register_builtin_status_items` explains it): `cwd(10)/cursor(11)/preview(12)` group 0 → `transfers(20)/agent(30)/updater(40)/jump-hosts(50)/bookmarks(60)` group 1 → `notifications(100)` group 2, pinned rightmost and now **always visible** (only its unread badge disappears at 0 — previously the whole bell hid at 0, which contradicted the task's "immer sichtbar"). `Notifications` and `AgentAccess` dropdowns migrated off a bespoke `.absolute()` div (no outside-click/Esc close at all before) onto the new `labonair_ui_kit::popover` primitive. `Transfers` item now hides unless a transfer is queued/running (new `TransfersView::active_count()` + `Workspace::transfers_entity()`, observed directly — observing `Workspace` alone never fired, since `apply_transfer_bus_event` only notifies the `TransfersView` entity). `Updater` click now calls new `UpdaterView::open_dialog()` (reopens the existing dialog for an already-known update instead of silently re-running the network check `run_check` would have started for an `Available` state, since `is_busy()` doesn't cover `Available`).
- **`crates/ui-kit/src/popover.rs`** (new) — `popover(anchor, width, theme, dismiss, content)`: `deferred` + `anchored().snap_to_window()` (same mechanics as `settings-ui`'s `render_dropdown`) + a transparent outside-click backdrop. `context_menu` (flat `MenuItem` lists — CWD's segment/subdir menus, the panel-toggle dock menu) was deliberately left alone: different content shape, ~10 existing call sites, and the task file itself says "bis T20-001" for a real shared primitive.

### Deviations (documented in the task file)
- Transfers' "dropdown with progress" is satisfied by the pre-existing `TransfersView` fixed bottom-right panel (already has per-job progress/step log/cancel) rather than an anchored-under-the-item popover — migrating that whole subsystem was out of scope for a polish task.
- Updater's icon doesn't visually distinguish Available/Downloading/Ready (single icon+dot) — click behavior and visibility are correct; per-substate iconography wasn't touched.

### Gate Results (T18-004)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ · `cargo clippy --workspace --all-targets -- -D warnings` ✅ · `cargo test --workspace` ✅ (all existing tests still green; no new unit tests — the change is UI wiring + two small conditionals on already-tested state) · `scripts/check-crate-deps.sh` ✅ (20 crates, 87 edges, acyclic, no new edge) — no display available this session, so `cargo run` visual check was **not** done (same caveat as T18-001/002/003).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T18-005** — Statusbar-Item-Personalisierung (`tasks/phase-17-layout/T18-005-statusbar-item-personalization.md`).

---

## Last Session: 2026-09-04 (T18-003 — Statusbar left: panel controls)

**T18-003 done.** `PanelTogglesStatusItem` (`crates/shell/src/status_items.rs`)
already existed from T17-003 with the basic per-panel toggle (icon button,
click → `Workspace::select_panel`, active-panel highlight); this task added
the remaining pieces from the task file.

### What Was Done (T18-003)
- **`crates/shell/src/status_items.rs`** — `PanelTogglesStatusItem` gained:
  - a right-click (`MouseButton::Right`) context menu per toggle (`dock_menu`
    field), built with the shared `labonair_ui_kit::{context_menu, MenuItem}`
    primitive: "Dock left/right/bottom" (current position omitted) →
    `Workspace::move_panel` + `persist_docks`; "Hide from toggle bar" → a
    session-only `hidden: HashSet<SharedString>` field that filters the
    toggle list (no persistence yet — that is T18-007 "panel visibility",
    per the task's own note).
  - a real GPUI tooltip (`.tooltip(...)` + `labonair_ui_kit::Tooltip`) on
    each toggle: panel title, plus the rebindable keybind
    (`labonair_command_palette::effective_keys`) for the two panels that
    have a dedicated `ShortcutId` today (`explorer` → `SidebarToggle`,
    `ai` → `AiToggle`); the other three panels (source-control, git-graph,
    snippets) show the title alone, matching the task's "keybind if set"
    wording — they have no `ShortcutId` of their own.
  - keybind overrides read from `labonair_workspace::prefs::GlobalPreferences`
    (a `cx` global, same pattern `Workspace::primary_dock` already uses).
- The old 44 px activity rail / `render_panel_toggle` / `render_ai_toggle` /
  sidebar switch strip were already gone (confirmed by grep — no hits
  anywhere in `crates/`), so nothing to remove there.
- **Deviation (documented in the task file):** "compact mode (icons only) at
  narrow width" — the toggle strip was already icon-only unconditionally (no
  label text ever existed on these buttons since T17-003), so it is always in
  the "compact" state rather than switching at a width threshold. No
  window-width observation code was added; flagged as a call for the user's
  visual pass, per the task's own "Design-Punkt für Nutzer-Sichtprüfung" note.

### Gate Results (T18-003)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `cargo test --workspace` ✅ (no new tests — the change is pure UI wiring on
  already-tested `Workspace::move_panel`/`persist_docks`/`dock_for_panel`) ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, 87 edges, acyclic, no new edge)
  — ran on a normal macOS dev machine but no display, so `cargo run` visual
  check was **not** done this session (same caveat as T18-001/T18-002).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T18-004** — Statusbar rechts: Info-Dropdowns
  (`tasks/phase-17-layout/T18-004-statusbar-right-info-dropdowns.md`).

---

## Last Session: 2026-09-04 (T18-002 — `Cmd+F` search overlay)

**T18-002 done.** The titlebar's provisional inline search (T18-001) and the
editor's own in-buffer find bar are both gone — one workspace-level overlay
now drives search for every tab kind. Scope was widened beyond the task text
on explicit user request: full real scrollback search for terminals (not
"first visible match"), and the overlay fully replaces the editor's find bar
(not kept alongside it).

### What Was Done (T18-002)
- **`crates/terminal/src/engine.rs`** — `TerminalEmulator` gained literal
  scrollback search on top of `alacritty_terminal::term::search::{RegexSearch,
  RegexIter}`: `search_set`/`search_step`/`search_clear`/`search_count`, plus
  a `to_regex_literal` escaper with an explicit `(?-i)`/`(?i)` prefix (needed
  to override alacritty's built-in smart-case, which otherwise silently
  ignores an explicit case-sensitive request for all-lowercase queries — see
  `docs/architecture.md` §8.14). The active match is mirrored into
  `term.selection` (free rendering + `scroll_to_point`); the rest are exposed
  as a new `RenderableScreen::search` span list. 6 new engine tests.
- **`crates/terminal/src/session.rs`** — 3 new `SessionAccess` trait methods,
  implemented for both `TerminalSession` and `RemoteSession` (SSH search works
  identically to local).
- **`crates/workspace/src/views/terminal.rs`** — `search`(old, first-visible-
  match) replaced by `search_set`/`search_step`/`search_end`; renders the new
  search spans in a dim highlight color between the cell runs and the
  selection layer.
- **`crates/workspace/src/views/editor.rs`** — the whole old `FindBar` UI
  (`render_find_bar`, `FindFocus`, Tab-to-replace, Replace/Replace-All,
  `find_key`, the editor's own `Cmd+F`) deleted; replaced by a minimal
  `EditorSearch` (query + matches + active, no replace) driven externally via
  `search_set`/`search_step`/`search_close`/`search_seed`. `Document::
  replace_all` itself is untouched (still backs vim `:s`).
- **`crates/workspace/src/workspace.rs`** — new `SearchTarget` enum +
  `active_search_target`/`search_seed`/`search_set`/`search_step`/`search_end`
  route by tab kind (editor / terminal / unavailable); replaced the old
  `search_active`/`find_in_active_editor`.
- **`crates/workspace/src/search_overlay.rs`** (new) — `SearchOverlay`, a
  **bare** `ModalView` (no scrim, no `occlude()` — content stays scrollable
  while it holds keyboard focus, see §8.14 for the "no new OverlayLayer"
  decision), real `InputState` field, case-toggle, Prev/Next, Esc-close,
  process-lifetime last-query prefill (no select-all — API not reachable, see
  §8.14).
- **`crates/shell/src/titlebar.rs`** — provisional `render_search` +
  `search_*` fields/methods removed entirely.
- **`crates/shell/src/actions.rs`** / **`commands.rs`** — `toggle_search_overlay`
  helper; `CommandId::Find` now just calls it.
- **`docs/architecture.md` §8.14** (new) — full deviation record.

### Gate Results (T18-002)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `cargo test --workspace` ✅ (all crates, incl. 6 new terminal-engine search
  tests + adjusted `views::editor::tests::find_navigates_matches`) ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, 87 edges, no new edge, acyclic)
  — this session ran on a normal macOS dev machine, so `cargo test` actually
  linked and ran (unlike prior headless-VPS sessions).

### Deviations
- Editor find bar fully removed rather than kept alongside the overlay (user
  decision).
- Terminal search is full scrollback + count + next/prev rather than
  "first visible match" (user decision).
- No select-all on the pre-filled query (API not reachable in gpui-component
  0.5.1).
- `cargo run` visual checks not done (see §8.14 — same caveat as T18-001, now
  because this session had no display, not because the machine was headless).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T18-003** — Statusbar links: Panel-Steuerung
  (`tasks/phase-17-layout/T18-003-statusbar-left-panel-controls.md`).

---

## Last Session: 2026-09-04 (T18-001 — Titlebar redesign · **Phase 17 START**)

**T18-001 done — first task of Phase 17.** The titlebar is now tab strip +
`＋` new-tab menu (left) + one `⋯` icon button (right); the empty surface got
its final look + file-drop.

### What Was Done (T18-001)
- **`crates/shell/src/titlebar.rs`** — full redesign. Removed `render_app_menu`
  / the `⋯` app-menu branch. New `render_account_menu`: single
  `IconName::Ellipsis` button → hand-rolled dropdown (`absolute` under the
  button) with `Settings…` (→ `open_settings_window(None)`) + a `Profile`
  placeholder (→ "coming soon" toast via `labonair_notifications::notification_center`,
  no new crate edge). Titlebar root is now `.window_control_area(Drag)` +
  `should_move` latch (`on_mouse_down`→`on_mouse_move`→`window.start_window_move()`
  →`on_mouse_up`) and `on_click` `click_count == 2` → `window.titlebar_double_click()`
  (Zed `platform_title_bar` mechanism). `TRAFFIC_LIGHT_INSET` `#[cfg]`-split
  78 px macOS / 8 px else. `render_search` kept but **rendered as a floating
  `absolute` overlay** (provisional until T18-002); `open_search` still drives
  the `⌘F` fallback from `commands.rs`. `render` returns `AnyElement` (the
  hidden-header early-return needs the same type as the `Stateful<Div>` body).
- **`crates/workspace/src/workspace.rs`** — `render_empty_surface` rewrite:
  centred "Labonair" wordmark over shortcut-hint rows (`⌘T` New Terminal, `⌘E`
  Editor, `⌘K` Commands, `⌘,` Settings, `⌘⇧N` Hosts) with bordered key chips,
  muted-foreground. `on_drop::<ExternalPaths>` → one `open_file(path, false, …)`
  per dropped file; double-click → `new_terminal_tab` (kept). Added
  `ExternalPaths` to the `gpui` import.
  The `＋` new-tab menu + `NewTabDropdownItems` port
  (`render_tab_bar` / `render_new_tab_menu`) already existed from the T17-009
  groundwork — retained unchanged (whole `＋` opens the menu; `⌘T` quick action;
  SSH/SFTP recent hosts via `recent_hosts`, no `hosts-ui` edge).
- **`crates/shell/src/app_shell.rs`** — doc-comment only (Titlebar description).
- **`docs/architecture.md` §8.13** (new) — full deviation record.

### Deviations (T18-001)
- Task's removal targets (`render_header`/`render_search`/`render_agent_badge`)
  were already in `titlebar.rs` after T17-006, not `app_shell.rs`;
  `render_agent_badge` no longer existed.
- Right button uses `IconName::Ellipsis` (no `Settings2`/`CircleUser` glyph).
- Right dropdown hand-rolled, not `context_menu` (its full-screen overlay can't
  render from the 40 px titlebar container).
- No separate `▾` split-button; `render_search` kept as provisional overlay.
- `cargo run` visual checks (traffic-light inset, live drag, zoom, dropdown
  look, file-drop) not possible on this headless VPS.

### Gate Results (T18-001)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, **87** internal edges — no new
  edge, acyclic).
- `cargo test --workspace` **not run** — test binaries cannot link on this
  headless VPS (missing X11 dev libs); `cargo check/clippy --all-targets`
  compiled all `#[cfg(test)]` code (project-accepted substitute).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T18-002** — Suche als Overlay (`tasks/phase-17-layout/T18-002-search-overlay.md`):
  move the provisional titlebar search into a dedicated overlay, then delete
  `render_search` + the `search_*` fields from `Titlebar`.

---

## Last Session: 2026-09-04 (T17-009 — Tabs optional / empty workspace · **Phase 16 DONE**)

**T17-009 done — this is the last task of Phase 16.** The workspace may now hold
**zero** tabs and renders an empty surface; the un-closable `Home` landing tab
is gone.

### What Was Done (T17-009)
- **`crates/workspace/src/tabs.rs`** — `TabKind::Home` → `TabKind::Hosts`
  (normal, closable). `close()` lost the `len() <= 1` guard + `Home`
  special-case; `close_others` / `close_by_kind` may reach 0; new
  `TabStore::close_all`; `activate_fallback` resets `active_id = 0` when
  emptied. Tests reworked (`closing_the_last_tab_empties_the_store`,
  `close_all_clears_every_tab`).
- **`crates/workspace/src/workspace.rs`** — startup rewrite in `new`: snapshot
  replayed verbatim (0 restored → stay empty); no snapshot → `startup_tab` pref
  (`Terminal` → one local terminal, `Empty` → nothing). No auto tab ever.
  `render_content` `None` arm → new `render_empty_surface` (centred hint +
  double-click → `new_terminal_tab`). `TabKind::Hosts` render arm =
  `self.host_manager`. `open_host_manager` opens/focuses a `TabKind::Hosts`
  tab. `session_snapshot` no longer emits `Home`; `Hosts` not persisted.
  `restore_session`: `RestoreAction::Home` → silent drop. `close_all_tabs`,
  context-menu gate, `palette_tab_kind`, `closable` all de-`Home`d.
- **`crates/workspace/src/session.rs`** — `TabSnapshot::Home` /
  `RestoreAction::Home` kept **deserialise-only** (removing would fail whole
  `from_str`). New tests: `zero_tab_snapshot_round_trips_on_disk`,
  `legacy_home_snapshot_tab_still_deserialises`.
- **`crates/backend/src/modules/settings/preferences.rs`** — `StartupTab`:
  `HostManager` → `Empty` (`#[default]`), `#[serde(alias = "host-manager")]`
  migrates. New test `legacy_host_manager_startup_tab_migrates_to_empty`;
  `enums_serialize_…` updated.
- **`crates/settings-ui/src/fields.rs`** — startup-tab select
  `["terminal", "empty"]`.
- **`docs/architecture.md §8.12`** (new) — full deviation record, incl. the
  test-harness deviation (no `Workspace` harness in the crate + test binaries
  don't link here → coverage via `TabStore` / `session` / `preferences` unit
  tests instead of a zero-tab render/sweep test).

### Gate Results (T17-009)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, **87** internal edges — no new
  edge, acyclic).
- `cargo test --workspace` **not run** — test binaries cannot link on this
  headless VPS (missing X11 dev libs); `cargo check/clippy --all-targets`
  compiled all `#[cfg(test)]` code (project-accepted substitute).

### State
- Branch `master`, committing now. No blockers. **Phase 16 complete.**

### Next
- **T18-001** — Titlebar-Redesign: Tabs + `＋▾` new-tab menu + right icon
  button; empty-surface visuals + file-drop (first task of **Phase 17 —
  Neues Layout & Statusbar-Personalisierung**).

---

## Last Session: 2026-09-04 (T17-008 — `AppEvent` bus decision · **Phase 16**)

**Decision: Variant A — keep the bus, connect it.** The bus already had two real
UI consumers (`workspace.rs` SSH/transfer path, `panel-snippets` run-log) plus
≥4 concrete follow-ups, so the "≥3 consumers → keep" bar is cleared. Rationale:
`docs/adr/0002-app-event-bus.md` + `docs/architecture.md §8.11`.

### What Was Done (T17-008)
- **`crates/workspace/src/backend_event_bridge.rs`** (new, ~70 Z.) — entity
  `BackendEventBridge`: one `cx.spawn` **foreground** loop over
  `backend.events.subscribe()` (`tokio::sync::broadcast::recv` is
  runtime-agnostic), decodes each `RawEvent` → `TransferBusEvent` / `AppEvent`
  and pushes straight into `Workspace` via `entity.update`. `Lagged` → warn +
  resync; `Closed` / workspace-dropped → stop. No `tokio::spawn` + `mpsc` +
  poll-drain.
- **`crates/workspace/src/workspace.rs`** — `pub mod backend_event_bridge;`;
  removed the `ev_tx/ev_rx` + `tev_tx/tev_rx` `mpsc` plumbing + the
  `tokio.spawn` forwarder; `Workspace` field `ssh_events` / `transfer_events`
  gone, replaced by `_backend_event_bridge: Entity<BackendEventBridge>` built in
  `new` from `cx.entity().downgrade()`. The 40 ms `ssh_poll` loop now only calls
  `refresh_active_tunnels`. New `pub(crate) fn apply_transfer_bus_event`;
  `handle_ssh_event` is now `pub(crate)`.
- **`crates/app/src/main.rs`** — `spawn_event_logger` + its call + the
  `AppEvent` / `RecvError` imports are `#[cfg(debug_assertions)]`.
- **`docs/adr/0002-app-event-bus.md`** (new) + **`docs/architecture.md §8.11`**
  (new) — decision, inventory, reference consumer, follow-up tickets.

### Follow-ups noted (not in this task)
`fs:dir-changed` → explorer auto-refresh; git-status → scm auto-refresh; move
`panel-snippets` run-log onto the same foreground pattern; route
`menu:activated` through `AppShell::dispatch_command`.

### Gate Results (T17-008)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, **87** internal edges — no new
  edge, acyclic).
- `cargo test --workspace` **not run** — test binaries cannot link on this
  headless VPS (missing X11 dev libs); `cargo check/clippy --all-targets`
  compiled all `#[cfg(test)]` code (project-accepted substitute). `cargo run`
  E2E not possible (no display) — the transfer-progress push is event-driven by
  construction (foreground `recv().await`, no poll).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T17-009** — Tabs optional: Empty-Workspace-State + `TabKind::Home` /
  Host-Manager-Tab weg (Thema 1).

---

## Last Session: 2026-09-04 (T17-007 — `CommandRegistry` · **Phase 16**)

The ~50-entry `.on_action(cx.listener(Self::act_*))` chain on the shell root
and the parallel `run_palette_command` match are gone. Every command is one
`r.register(...)` line in `crate::commands::register_builtin_commands`; the
menu bar, key bindings and command palette dispatch the same
`CommandId` through `AppShell::dispatch_command` → `registry.run_for(id)`.

### What Was Done (T17-007)
- **`crates/shell/src/commands.rs`** (new, ~330 Z.) — `CommandFn =
  Rc<dyn Fn(&mut AppShell, &mut Window, &mut Context<AppShell>)>`, `struct
  Command { id, contexts, run }`, `struct CommandRegistry` (`register` /
  `run_for` / `iter` / `visible_in`), `register_builtin_commands()` (the
  **single** definition site — every former `act_*` body as a closure),
  `attach_action_handlers(root, can_split, has_split, cx)` = the
  Action→`CommandId` bridge (one `.on_action` per `menu::` action;
  context-gated `SplitRight/Down` + `ClosePane` via `if can_split` /
  `if has_split`). `impl AppShell { dispatch_command }`. 2 unit tests
  (id-uniqueness + context rules; unregistered ids → `None`).
- **`crates/shell/src/app_shell.rs`** — `command_registry: CommandRegistry`
  field + `from_parts` param; `render` now: **3** genuine window
  `.on_action`s (`ToggleFullScreen` / `Minimize` / `ZoomWindow`) + one
  `crate::commands::attach_action_handlers(root, …)` call. The 40-line
  `.on_action` cascade + the `.when(can_split)` / `.when(has_split)` guards
  are deleted.
- **`crates/shell/src/actions.rs`** — gutted: all `act_*` handlers +
  `select_tab_action!` macro + `run_palette_command` + its `dispatch!` macro
  removed. Kept: the 3 window actions, the `AppShell` helper methods the
  command closures call (now `pub(crate)`: `toggle_sidebar`, `move_panel`,
  `primary_dock`, `toggle_zen_mode`, `toggle_zen_pref`,
  `toggle_command_palette`, `show_command_palette`, `toggle_bookmarks`), the
  modal-layer mirrors, `handle_palette_event` (`Run(id)` arm →
  `self.dispatch_command`), `handle_bookmark_event`, and a **slimmed**
  `build_palette_data` (7 fields, panel-sourced choices only).
- **`crates/shell/src/{shell,bootstrap}.rs`** — `pub mod commands;`;
  `bootstrap` calls `register_builtin_commands()` and threads it into
  `from_parts`.
- **`crates/command-palette/src/palette.rs`** — `CommandId` gained 20
  menu/keyboard-only ids (`OpenCommandPalette`, `NewPreviewTab`, `Save`,
  `NewSshTab/SftpTab/SshConnection/QuickSsh`, `ClearChat`, `FocusNextPane`,
  `SelectTab1..9`, `DebugCyclePanelDock/DockZoom`). `PaletteData` slimmed
  12 → 7 fields (dropped `tabs`, `color_mode`, `editor_theme`, `font_size`,
  `toggles`). `PalettePrefs` gained `color_mode` / `editor_theme` /
  `terminal_font_size` / `toggle_state` / `keybind_overrides`; `rows()` +
  the Recently-Used block read those + render shortcut hints via
  `effective_keys` (override-aware).
- **`crates/command-palette/src/keybind.rs`** — new `keystroke_tokens` +
  `effective_keys` (override-aware display tokens); re-exported.
- **`crates/settings-ui/src/store.rs`** — the 5 new `PalettePrefs` methods
  impl'd on `PreferencesStore` (verbatim field reads).

### Deviations (accepted — task file ACs + `docs/architecture.md` §8.10 + commit)
1. `CommandRegistry` **type** lives in `labonair-shell`, not
   `labonair-command-palette` / a new crate: `CommandFn` needs `&mut AppShell`
   (the §8.4/§8.9 panel entities), only nameable there. Palette + keymap share
   the registry via the common `CommandId` enum. **No new crate edge.**
2. `build_palette_data` is **slimmed, not deleted** — pref/theme scalars +
   keybind overrides moved to `PalettePrefs` reads; panel-/settings-sourced
   choice lists still fed via `set_data` (a `labonair-panel-* →
   labonair-command-palette` back-edge / `labonair-settings-ui` dep would be a
   cycle).
3. Placeholder ids (`ZoomIn/Out/Reset`, `OpenShortcuts`, `FormatDocument`,
   palette sub-page navigators) stay **unregistered**; `dispatch_command`
   no-ops for them — byte-for-byte the pre-T17-007 no-op dispatch behaviour.

### Gate Results (T17-007)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, **87** internal edges — no new
  edge, acyclic).
- `cargo test --workspace` **not run** — test binaries cannot link on this
  headless VPS (missing `-lxcb` / `-lxkbcommon*`); `cargo check/clippy
  --all-targets` compiled all `#[cfg(test)]` code incl. the new
  `commands.rs` tests (project-accepted substitute). `cargo run` E2E not
  possible (no display).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T17-008** — `AppEvent`-Bus entscheiden (nutzen oder streichen).

---

## Last Session: 2026-09-04 (T17-006 — `AppShell` → reine Komposition · **Phase 16**)

The "payday" task of Phase 16. `crates/shell/src/app_shell.rs` went from
**2 072 → 272 lines**; no `drain_pending_*`, no per-frame `sync_live_bridge`,
no per-frame `build_palette_data`.

### What Was Done (T17-006)
- **`crates/shell/src/bootstrap.rs`** (new, ~490 Z.) — the entire former
  `AppShell::new` body: builds every child entity, runs the ordered startup
  sequence (MCP-prefs hydrate → session snapshot → theme pref → keybinds →
  settings deps → updater check), wires the reactive edges, returns
  `AppShell::from_parts(…)`. Also `register_builtin_panels` /
  `migrate_dock_layout` / `refresh_live_snapshot` moved here. The ~15
  `cx.observe(&x, |_,_,cx| cx.notify())` collapsed to the handful that still do
  work (`background`, `prefs`, `workspace`, two CWD-feed closures, live-snapshot
  refresh, `BarLayoutTick`).
- **`crates/shell/src/actions.rs`** (new, ~860 Z.) — every `act_*` handler +
  `build_palette_data` + `run_palette_command` + `handle_palette_event` /
  `handle_bookmark_event` as an `impl AppShell` block + the `select_tab_action!`
  macro. `.on_action(...)` registration list stays in `render`; only bodies
  moved. `build_palette_data` now runs at palette-open (`show_command_palette`),
  not per frame. T17-007 replaces this module with a `CommandRegistry`.
- **`crates/shell/src/titlebar.rs`** (new, ~275 Z.) — `Titlebar` entity: tab
  strip + transient inline search + `⋯` app-menu, ported verbatim from
  `render_header` / `render_app_menu` / `render_search`. Owns its own
  `zen_mode_show_header` reactivity (empty element when hidden). Redesign =
  T18-001.
- **`crates/shell/src/modals.rs`** (new, ~148 Z.) — the three `ModalView`
  wrapper newtypes + `ShellPalette` alias, moved out of `app_shell.rs`
  unchanged.
- **`crates/workspace/src/workspace.rs`** — `Workspace::render_dock` +
  `DockResize` drag payload + `set_dock_size` + `move_panel_persist` +
  `apply_live_command` added; `Workspace::render` now composes
  `[left dock | (pane group + bottom dock) | right dock]` + the drag-to-resize
  handler itself (Zed model). Shell just does `.child(workspace.clone())`.
  Reuses the existing `DragGhost`. **No new crate edge.**
- **`crates/shell/src/app_shell.rs`** — now only: `struct AppShell` (13 fields;
  8 concrete entities grouped in `ShellPanels`), `new` (one `cx.observe(theme)`
  + delegate to `bootstrap`), `from_parts`, `workspace()`/`preferences()`,
  `maybe_persist_geometry` + `bounds_differ`, `Focusable`, `Render` (pure
  composition + the `.on_action` list + the two modal-sync calls +
  `maybe_persist_geometry`).
- **`crates/shell/src/shell.rs`** — declares `actions` / `bootstrap` / `modals`
  / `titlebar`; dropped the now-dead `live_bridge` re-export shim.
- **`drain_pending_ai` / `sync_live_bridge` deleted.** AI "run in terminal" →
  `cx.subscribe_in(&ai_chat, window, …)` in `bootstrap`. LiveBridge snapshot →
  `cx.observe` on workspace + explorer (`refresh_live_snapshot`); command queue
  → `cx.spawn` + `background_executor().timer(120 ms)` loop
  (`AppShell::_live_drain`) → `Workspace::apply_live_command`.
- **`docs/architecture.md` §8.9** (new) — records the split + the accepted
  deviations (13 fields not 8; §8.4 wins; `background.layer(App)` overlay is an
  extra render child; `AppShell::new` has one observe, functional ones in
  `bootstrap`).

### Deviations (accepted, recorded in task file + §8.9 + this entry)
- `struct AppShell` keeps **13 fields**, not ≤ 8. The 8 concrete panel/feature
  entities cannot move to `Workspace` — `labonair-panel-{explorer,scm,snippets,
  ai} → labonair-workspace` already exists, so the reverse edge is a cycle; and
  `PreferencesStore` / `CommandPalette<PreferencesStore,…>` / `UpdaterView` are
  not nameable from `labonair-workspace`. `docs/architecture.md` §8.4 (which the
  project calls authoritative) explicitly says `AppShell` keeps the concrete
  handles. Grouped in `ShellPanels` for readability.
- `render` also carries the pre-existing full-window `background.layer(App)`
  wallpaper overlay (must cover titlebar+statusbar too, so it can't sink into
  `Workspace::render`).

### Gate Results (T17-006)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, **87** internal edges — no new
  edge, acyclic).
- `cargo test --workspace` **not run** — test binaries cannot link on this
  headless VPS (missing `-lxcb` / `-lxkbcommon*`); `cargo check/clippy
  --all-targets` compiled all `#[cfg(test)]` code (project-accepted
  substitute). `cargo run` E2E visual check **not possible** (no display).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T17-007** — `CommandRegistry` (replaces `crate::actions` / the `.on_action`
  cascade with a data-driven registry).

---

## Last Session: 2026-09-04 (T17-005 — `ModalLayer` + `ToastLayer` · **Phase 16**)

Replaced the six hand-wired overlay children in `AppShell::render` with **two**
reusable layers (Zed pattern), and deleted the `pending_*` / `drain_pending_*`
buffering for palette + bookmark picks.

### What Was Done (T17-005)
- **`crates/workspace/src/modal_layer.rs`** (new) — `trait ModalView:
  ManagedView` (`on_dismiss` hook + `render_bare()`); `struct ModalLayer`
  (Entity, `#[derive(Default)]`): one active modal, `toggle_modal<V,B>` /
  `open_modal<V,B>` / `hide_modal` / `active_modal::<V>()` /
  `has_active_modal` / `set_dismiss_on_focus_lost`. `show_modal` wires a
  `DismissEvent` subscription (`subscribe_in`) + `on_focus_out`, focuses the
  modal via `cx.defer_in`. `Render`: bare → view as-is; non-bare →
  `modal_scrim()` backdrop (`occlude()` + `on_mouse_down`→`hide_modal`) +
  centered `occlude()`d panel with `track_focus`. Ported/trimmed from
  `zed-refrence/.../modal_layer.rs`.
- **`crates/workspace/src/toast_layer.rs`** (new) — `ToastLayer<Th: UiTheme>`
  observes `NotificationCenter`, renders `notifications::render_overlay`. The
  stacking + `background_executor().timer()` auto-dismiss already lived in the
  center.
- **`crates/workspace/src/workspace.rs`** — `pub mod modal_layer; pub mod
  toast_layer;`.
- **`crates/command-palette/src/palette.rs`** — `impl
  EventEmitter<DismissEvent> for CommandPalette`; `close()` emits `DismissEvent`
  (guarded by `was_open`).
- **`crates/panel-explorer/src/bookmarks.rs`** — `impl
  EventEmitter<DismissEvent>`; `close()` emits it; scrim gained
  `on_mouse_down`→`close` (overlay-click-to-close) + card `.occlude()`.
- **`crates/shell/src/updater.rs`** — `pub fn dialog_visible(&self) -> bool`.
- **`crates/shell/src/app_shell.rs`** — shell-local `ModalView` wrapper
  newtypes `CommandPaletteModal` / `BookmarksModal` / `UpdaterModal` (cycle +
  orphan-rule: the trait can't be impl'd on the view crates). Fields
  `pending_commands` / `pending_bookmarks` + `drain_pending_commands` /
  `drain_pending_bookmarks` **removed**; new `modal_layer:
  Entity<ModalLayer>` + `toast_layer: Entity<ToastLayer<ThemeStore>>` fields;
  removed now-dead `notifications` field. `cx.subscribe(&command_palette/…)` →
  `cx.subscribe_in(…, window, …)` calling `handle_palette_event` /
  `handle_bookmark_event` (single-event, ex-`drain` bodies). New helpers
  `toggle_command_palette` / `show_command_palette(page)` (explicit
  `open_modal`/`hide_modal`), `toggle_bookmarks` + `sync_bookmarks_modal`,
  `sync_updater_modal` (driven mirrors, called from `render`). `render` ends
  `.child(self.modal_layer.clone()).child(self.toast_layer.clone())` — exactly
  two overlay children.
- **`docs/architecture.md` §8.8** — new; records the three accepted
  deviations (render_bare kept for the 3 legacy modals; wrapper newtypes;
  driven-vs-explicit).

### Gate Results (T17-005)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, 87 edges, acyclic — **no new
  edge**).
- `cargo test --workspace` **not run** — test binaries cannot link on this
  headless VPS (missing `-lxcb` / `-lxkbcommon*`); `cargo check/clippy
  --all-targets` compiled all `#[cfg(test)]` code (project-accepted
  substitute). `cargo run` visual check of Cmd+Shift+P / bookmarks / updater
  dialog / stacked toasts / click-outside **not possible** (no display).

### State
- Branch `master`, committing now. No blockers.

### Next
- **T17-006** — `AppShell` → reine Komposition (God-Object auflösen).

---

## Last Session: 2026-09-04 (T17-004 — `PaneGroup` n-ary split tree + persistence · **Phase 16**)

Replaced the flat binary pane tree with a Zed-style n-ary `Member` tree that
has an **optional root**, arbitrary nesting, `flexes` (sum-1.0 invariant), and
full session persistence with backward compat.

### What Was Done (T17-004)
- **`crates/workspace/src/pane_group.rs`** — rewritten. `enum SplitDirection
  {Up,Down,Left,Right}` (`axis()`, `increasing()`, `along(SplitAxis)`);
  `enum Member { Pane(PaneId), Axis(PaneAxis) }`; `struct PaneAxis { id:
  PaneId, axis: SplitAxis, members: Vec<Member>, flexes: Vec<f32> }` (even
  redistribution on insert/remove, Zed-style; `with_flexes` validates &
  renormalises a persisted vector); `struct PaneGroup { root: Option<Member> }`
  with `split(new_axis_id, target, new_pane, dir)` (handles `root==None` →
  new root; falls back to first pane if target vanished), `remove(pane) ->
  RemoveOutcome{Removed{neighbor:Option<PaneId>}|NotFound}` (collapses
  single-child axes, empties root), `set_boundary(axis_id, boundary, frac)`
  (redistributes **only** the two adjacent flexes — whole-axis sum preserved
  exactly), `reset_axis`, `panes`/`first_pane`/`find_pane`/`is_empty`.
  `WorkspaceLayout` kept (per-tab) but now `{ group: PaneGroup, active:
  Option<PaneId> }`; `split`/`close`(→`CloseOutcome` unchanged)/`set_active`/
  `set_boundary`/`reset_axis`/`leaves`/`len`/`is_empty`. 6 new unit tests.
  `PaneNode` deleted.
- **`crates/workspace/src/pane.rs`** — re-exports updated to `Member`,
  `PaneAxis`, `PaneGroup`, `RemoveOutcome`, `SplitDirection`, `WorkspaceLayout`.
- **`crates/workspace/src/session.rs`** — new `SerializedPaneGroup` (recursive
  serde enum: `Pane` / `Axis` + read-only legacy `Split` variant migrated to
  `Axis`) and `SerializedLayout { root: Option<SerializedPaneGroup>, active:
  Option<PaneId> }` (both `#[serde(default, skip_serializing_if)]`).
  `WorkspaceTabSnapshot.layout` type `WorkspaceLayout` → `SerializedLayout`
  **same field name + nesting** ⇒ old `session.json` still loads, **no
  `SNAPSHOT_VERSION` bump**. `remap_layout` now takes `&SerializedLayout`,
  rebuilds `Member` tree with fresh pane+axis ids preserving flexes & active.
  `plan_workspace` uses `leaf_count()`. 3 tests added/adjusted.
- **`crates/workspace/src/workspace.rs`** — `split`/`split_active` take
  `SplitDirection`; `resize_split`→`resize_axis(axis_id,boundary,frac)`,
  `reset_split`→`reset_axis(axis_id)`; `render_pane_node`→`render_member`
  (recursive over `Member`, resize handle between every adjacent member pair,
  `id("axis-handle-{axis_id}-{boundary}")`); `PaneResize { axis_id, boundary
  }`; `layout.active` now `Option` (fixed `focus_next_pane`,
  `close_active_pane`, `active_pane_view`, `sync_meta`, `render_content`
  Terminal arm — `root==None` still shows the old placeholder, real empty
  surface is T17-009/T18-001); snapshot uses
  `SerializedLayout::from_layout(&layout)`.
- **`crates/shell/src/app_shell.rs`** — `use crate::pane::SplitDirection`;
  the two split actions pass `SplitDirection::Right` / `::Down`.
- **`docs/architecture.md`** — new **§8.7** (n-ary `PaneGroup`, splits =
  central area only / docks single-panel, direction API, persistence
  superset / no version bump).

### Gate Results (T17-004)
- `cargo fmt --check` ✅ · `cargo check --workspace --all-targets` ✅ ·
  `cargo clippy --workspace --all-targets -- -D warnings` ✅ ·
  `scripts/check-crate-deps.sh` ✅ (20 crates, 87 edges, acyclic — no new edge).
- `cargo test --workspace` **not run** — test binaries cannot link on this
  headless VPS (missing `-lxcb` / `-lxkbcommon*`); disk ~91% full.
  `cargo check/clippy --all-targets` compiled all `#[cfg(test)]` code — the
  project-accepted substitute. `cargo run` visual 2×2 / nested check **not
  possible** (no display).

### State
- Branch `master`, committed (see `git log`). No blockers.

### Next
- **T17-005** — next earliest unstarted task (see `tasks/ROADMAP.md` /
  `tasks/phase-*`).

---

## Last Session: 2026-09-04 (T17-003 — `StatusItem` trait & `StatusItemRegistry` · **Phase 16**)

Kills the shell's `render_bar_item`-`match` cascade. Every status-bar element
is now a self-describing `StatusItem` view; `labonair-shell` registers the
concrete ones once and a `StatusBar` component in `labonair-workspace` renders
purely from the registry.

### What Was Done (T17-003)
- **`crates/panel/src/status.rs`** — `StatusItem` trait finalised: added
  `order() -> i32`, `hideable() -> bool`, `on_active_tab_changed(&mut self,
  cx)` (all defaulted); `StatusItemHandle` gained `order` / `hideable` /
  `on_active_tab_changed`; `StatusItemRegistration` gained `order: i32`;
  `StatusItemRegistry::resolve_side(id) -> StatusSide` (default-side for now,
  TODO points at T18-005's placement-blob override). +1 unit test.
- **`crates/workspace/src/status_bar.rs`** — NEW. `StatusBar` `Render` view:
  lazily builds every registered item once (constructors hand back
  pre-built-entity clones, mirroring `init_docks`), groups by side, sorts by
  `order`, renders left/right clusters. Owns the status-bar chrome
  (`h(32)` / `status_bar()` bg / `border_t`).
- **`crates/workspace/src/workspace.rs`** — `status_item_registry:
  StatusItemRegistry` + `dock_persist_hook: Option<DockPersistHook>` +
  `last_dock_save` fields; `status_item_registry()/_mut()`,
  `set_dock_persist_hook()`, `primary_dock()`, `dock_for_panel()`,
  `panel_is_active()`, `select_panel()`, `open_panel()`, `persist_docks()`
  moved here off `AppShell` (persistence goes through the shell-supplied
  callback because this crate can't reach `PreferencesStore`).
- **`crates/shell/src/status_items.rs`** — NEW. Concrete items ported 1:1 from
  the old `render_*_item` bodies: `PanelTogglesStatusItem` (aggregate — one
  toggle per `PanelRegistry` entry, AI folded in), `NotificationsStatusItem`
  (badge + dropdown), `CwdStatusItem` (own entity: `expanded` / `crumb_menu` /
  `subdir_menu` + async subdir listing + segment/subdir context menus),
  `CursorPositionStatusItem`, `PreviewUrlStatusItem`, `UpdaterStatusItem`,
  `TransfersStatusItem`, `AgentAccessStatusItem` (badge + popover),
  `JumpHostsStatusItem`, `BookmarksStatusItem`. Single
  `register_builtin_status_items(workspace, theme, notifications, updater,
  agent_access, bookmarks, cx)`. Dropdown anchors flipped `top`→`bottom`
  (status bar is at the window's bottom edge now).
- **`crates/shell/src/app_shell.rs`** — deleted ~930 lines: `render_bar_item`,
  `render_simple_bar_button`, `render_updater_item`,
  `render_notifications_item`, `render_panel_toggle`, `render_ai_toggle`,
  `render_bar_menu`, `build_bar_bucket`, `render_agent_badge`,
  `render_cwd_breadcrumb` + `render_crumb_segment` + `render_crumb_menu` +
  `render_subdir_menu` + `open_crumb_menu` + `open_subdir_menu` + `home_dir`,
  `render_statusbar`, `persist_placement`, `move_bar_item`, `panel_for_item`,
  `item_for_panel`, `primary_side`, `persist_docks`, and the fields
  `agent_access` / `agent_badge_open` / `placements` / `bar_menu` /
  `notif_open` / `breadcrumb_expanded` / `crumb_menu` / `subdir_menu` /
  `last_dock_save` / `backend` / `tokio`. New `status_bar:
  Entity<StatusBar>` field. `render_header` no longer builds bar buckets.
  `toggle_sidebar` / `move_panel` / `set_dock_size` /
  `act_debug_toggle_dock_zoom` / the dock-tab click now call
  `Workspace::persist_docks`. `BarLayoutTick` `observe_global` kept as a plain
  `cx.notify()` (T18-005 repurposes it).
- **`crates/shell/src/shell.rs`** — `pub mod status_items;`.

### Deviation (recorded — see `docs/architecture.md §8.6`, task file AC1)
`BarLoc` was **not** removed (AC1 asked for it). `BarItemId`/`BarLoc`/
`Placements`/`BAR_ITEM_ORDER`/`BarLayoutTick` in
`labonair-workspace::bar_items` stay: `labonair-settings-ui` (`view.rs`,
`panes/themes.rs` bar-item layout editor) still consumes them, and that editor
+ the `barItemPlacements → statusBarItemPlacements` migrator are explicitly
T18-005 / T18-006. Notes added to both those task files.

### Gates
- `cargo fmt --check` ✓
- `cargo check --workspace --all-targets` ✓ (only the pre-existing
  `proc-macro-error2` future-incompat dep warning)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓ (added a
  `DockPersistHook` type alias for `type_complexity`)
- `scripts/check-crate-deps.sh` ✓ (20 crates, 87 edges, acyclic — no new
  edges; `shell`/`workspace`→`panel` and `shell`→`workspace` already existed)
- `cargo test` cannot **link** on this headless VPS (`rust-lld: unable to
  find library -lxcb / -lxkbcommon / -lxkbcommon-x11`). `check --all-targets`
  + `clippy --all-targets` type-check the `#[cfg(test)]` code — accepted
  substitute; disk ~91 % full so `cargo test --workspace` not run.

### What's Next
**T17-004** — `PaneGroup` (recursive split tree + persistence, optional root).

---

## Prior Session: 2026-09-04 (T17-002 — `Dock` model (Left / Right / Bottom) · **Phase 16**)

Replaces the shell's ad-hoc dual-slot sidebar (`left_slot`/`right_slot`,
one panel each) with a real three-dock model owned by the `Workspace`.

### What Was Done (T17-002)
- **`crates/workspace/src/dock.rs`** — NEW. `struct Dock { position, panels:
  Vec<AnyPanelHandle>, active, open, size, zoomed }` — reduced port of Zed's
  `dock.rs`. Methods: `add_panel` / `remove_panel` (with active-index fixup) /
  `activate_panel` / `toggle_open` / `toggle_panel` (open+activate, or close if
  already active) / `set_zoomed` / `set_size` (clamped to per-position
  `min_size`/`max_size`, plus the active panel's own `min_size` floor) /
  `apply_order` / `apply_scalars` / `to_data`. `RESIZE_HANDLE_SIZE = px(6.)`,
  per-position `default_size`/`min_size`/`max_size`, `position_slug` /
  `position_from_slug` (the serde bridge `labonair-panel` deliberately omits).
  `DockData` serde struct `{ position, open, size, zoomed, active_panel,
  panel_order }` (Zed's `panel_positions` map dropped — a moved panel is just
  the one that appears in another dock's `panel_order`). 5 unit tests.
- **`crates/workspace/src/workspace.rs`** — `left_dock` / `right_dock` /
  `bottom_dock: Dock` fields (empty at construction). `dock()` / `dock_mut()` /
  `docks()` / `dock_of_panel()` accessors. `init_docks(layout_json, window,
  cx)` builds every registered panel once and drops it into the dock its
  persisted `panel_order` names, else its registry `default_position`, then
  applies per-dock order + scalars; first run (no layout) opens the left dock.
  `move_panel(name, to, cx) -> bool` removes from the old dock, validates via
  `PanelHandle::position_is_valid`, adds+activates+opens the new dock.
- **`crates/shell/src/app_shell.rs`** — deleted `left_slot`/`right_slot`/
  `slot`/`slot_mut`/`side_for_panel`/`select_panel_on_side`/`persist_sidebar`/
  `set_slot_width`/`render_sidebar` and the `resolve_persisted_panel` helper.
  New: `primary_dock`, `dock_for_panel` (live membership), `panel_is_active`,
  `select_panel`/`open_panel`/`move_panel`/`set_dock_size` (all operate on
  `workspace.dock_mut(pos)`), `persist_docks` (throttled write of the whole
  `[DockData;3]` to the new `dockLayout` pref), `migrate_dock_layout` (first-run
  port of the legacy `sidebar_*` prefs), and `render_dock(pos)` — one function
  for all three edges: header (title / multi-panel switcher / "move to next
  dock" arrow), active-panel body, resize handle on the inner edge
  (col-resize L/R, row-resize bottom), zoom = fill + no handle. Body layout:
  `row[ left_dock | col[ workspace / bottom_dock ] | right_dock ]` with the
  bottom dock nested inside the centre column (Zed's `workspace.rs` nesting),
  status bar a sibling below. `DockResize(DockPosition)` drag value; the
  `on_drag_move` math handles the bottom (vertical) edge too.
- **`crates/shell/src/menu.rs`** — `DebugCyclePanelDock` (`cmd-alt-shift-m`) and
  `DebugToggleDockZoom` (`cmd-alt-shift-z`) — temporary T17-002 debug
  affordances (no menu entry) to exercise `move_panel` / zoom until T18-003 /
  T18-007 add the real status-bar UI. `bindings_parse` asserts bumped 37→39,
  36→38.
- **`crates/shell/src/shell.rs`** — dropped `pub mod sidebar_slot;`.
- **`crates/shell/src/sidebar_slot.rs`** — DELETED (212 lines; its
  `sidebarSlotLogic` port is superseded by `Dock::toggle_panel`/`set_size`).
- **`crates/backend/src/modules/settings/preferences.rs`** — new `dock_layout:
  String` field (key `dockLayout`, default `""`). Legacy `sidebar_*` fields
  kept for the one-time migration read.

### Deviations from the task text (recorded)
- The task asked for `Dock::render` + the resize `on_drag_move` to live in
  `labonair-workspace`. `Dock` is a plain struct (not `Entity<Dock>` — no panel
  subscribes to its dock yet, per the task's own note), so rendering + the drag
  listener stay in the shell (`render_dock`), which owns the `Render` view and
  `cx.listener`. The dock *state/geometry/persistence* is fully in
  `labonair-workspace` as required.
- Debug affordance is two fixed keybindings, not a command-palette entry
  (equivalent, less invasive — palette `CommandId`s are a separate crate).
- "44px activity rail" — never existed in the shell code (only a stale doc
  comment), so nothing to remove.

### Gates
- `cargo fmt --check` ✓
- `cargo check --workspace --all-targets` ✓ (only the pre-existing
  `proc-macro-error2` future-incompat dep warning)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓ (fixed one
  `type_complexity` in `render_dock` by splitting the collected tuple)
- `scripts/check-crate-deps.sh` ✓ (20 crates, 87 edges, acyclic — no new edges)
- `cargo test` cannot run in this VPS environment: linking a test binary fails
  with `rust-lld: unable to find library -lxcb / -lxkbcommon / -lxkbcommon-x11`
  (no X11 dev libs installed on the headless box — `ldconfig -p` has neither).
  This is pre-existing and unrelated to T17-002. Both `labonair-workspace`
  and `labonair-shell` **compiled** their test modules cleanly under
  `cargo check --all-targets` + `clippy --all-targets` (which build the
  `#[cfg(test)]` code but skip the link); only the final link of the test
  executable fails, for the missing system libs. The new `dock.rs` tests and
  the adjusted `menu.rs` assert counts are therefore type-checked but not
  executed here.

### State / Next
- Branch `master`, committed on top of `792129e` (T17-001).
- Concurrent-session files left unstaged / untouched:
  `docs/adr/0001-crate-decomposition.md`, `bericht-workflow-rework.md`,
  `zed-refrence/`.
- **Next task: T17-003** (`StatusItem` trait & registry,
  `tasks/phase-16-registries/`).

## Last Session: 2026-09-04 (T17-001 — `Panel` trait & `PanelRegistry` wired · **Phase 16 started**)

First code task of Phase 16. Replaces the `enum SidebarPanel` + `render_panel_body`
`match` cascade in `labonair-shell` with a registry-driven panel model.

### What Was Done (T17-001)
- **`crates/panel/src/dock.rs`** — dropped the now-unused `PanelIcon::Hosts`
  variant (host manager is not a panel; `docs/architecture.md §8.1`). The
  `Panel` trait / `PanelHandle` erasure / `PanelRegistry` from the T16-005
  skeleton were already object-safe and complete; no trait change needed.
- **`impl labonair_panel::Panel`** added to all five panel views, each with a
  doc comment justifying `default_position` / `default_size` from today's
  behaviour + `reference-src`:
  - `ExplorerView` → `"explorer"`, Left, 260px, min 180px
  - `GitPanelView` (`panel-scm`) → `"source-control"`, Left
  - `GitGraphView` (`panel-git-graph`) → `"git-graph"`, Bottom, 320px
  - `SnippetsView` → `"snippets"`, Left
  - `AiChatView` → `"ai"`, Right, 380px
  Each panel crate gained a `labonair-panel` path dep (leaf crate: only
  `gpui` / `gpui-ext`, no cycle). `labonair-hosts-ui` has **no** `impl Panel`.
- **`crates/workspace/src/workspace.rs`** — `panel_registry: PanelRegistry`
  field + `panel_registry()` / `panel_registry_mut()` accessors (Zed hangs
  docks off the workspace; same pattern).
- **`crates/shell/src/app_shell.rs`** — deleted `enum SidebarPanel`, its
  `label`/`slug`/`from_slug`, `render_panel_body`, `render_tabs_panel`,
  `render_hosts_panel` (the compact Hosts-in-sidebar list is gone entirely).
  New `register_builtin_panels(...)` is the **only** place naming concrete
  panel types — it registers a constructor that clones the shell's already-built
  panel entity. `render_sidebar` now resolves the active panel purely through
  `workspace.panel_registry()` (no `match`). Persisted `sidebarActivePanel` /
  `sidebarRightActivePanel` are `persistent_name`s now; `resolve_persisted_panel`
  migrates the removed `hosts` / `tabs` values (and any unknown) onto the
  side's fallback and only accepts names the registry knows.
- **`crates/shell/src/sidebar_slot.rs`** — `SidebarSlot` stores the panel's
  `persistent_name` as a `SharedString` instead of the enum; `toggle` takes
  `impl Into<SharedString>`. Ported tests updated for the `SharedString` type.
- **`scripts/check_crate_deps.py`** — allow-list updated: `labonair-shell` +
  each `labonair-panel-*` may now depend on `labonair-panel` (annotated
  `T17-001`). `scripts/check-crate-deps.sh` green (20 crates, 87 edges, acyclic).

### Coordinator-forced deviations (recorded)
- Concurrent-session edits left unstaged / not part of this commit:
  `docs/adr/0001-crate-decomposition.md` (workflow-rework amendment),
  `bericht-workflow-rework.md`, `zed-refrence/`.
- `cargo test --workspace` is disk-banned on this VPS. Ran `cargo fmt --check` ✓,
  `cargo check --workspace --all-targets` ✓,
  `cargo clippy --workspace --all-targets -- -D warnings` ✓,
  `scripts/check-crate-deps.sh` ✓, `cargo test -p labonair-panel -p labonair-shell`
  ✓ (27 tests). Per-crate tests for the five panel crates + workspace were run
  separately.
- A previous in-progress session left the tree with compile errors
  (`SharedString` `From<&'static str>` lifetime + `as_deref` type mismatch in
  `sidebar_slot.rs` tests + `select_panel_on_side`); fixed here. See
  `memory/bugs_and_fixes.md`.

### State
- Branch `master`. **T17-001 done.** Next: **T17-002** (`Dock` model L/R/B) —
  replaces the shell's ad-hoc per-side `SidebarSlot` state with a real `Dock`
  hung off the `Workspace`, and wires `PanelEvent` zoom/close + the bottom dock
  (so `git-graph` finally opens at its default position).
- No blockers.

## Last Session: 2026-09-03 (T16-010 — build hygiene, crate-graph lint, perf baseline · **Phase 15 COMPLETE**)

Tenth and final code task of Phase 15. No crate source touched — this task adds
tooling, CI enforcement, docs and a baseline.

### What Was Done (T16-010)
- **`scripts/check-crate-deps.sh` + `scripts/check_crate_deps.py`** — mechanical
  guard for `docs/architecture.md` §3. Parses `cargo metadata --no-deps` for the
  workspace-internal edges, then enforces: a **per-crate allow-list** (each entry
  annotated with the §3 rule it follows), **acyclicity** (memoised DFS), and
  transitive **must-not-reach** invariants (no `panel-*` reaches `shell` by any
  path; no *direct* `panel→panel` edge; `panel` never reaches
  `workspace`/`shell`; `backend`/`ai`/`editor` reach no UI crate; `ui-kit` reach
  limits). Green for the real graph; verified it fails on an injected
  `panel-ai → panel-scm` edge, then reverted.
- **Two accepted deviations** encoded in the allow-list with `[deviation]`
  comments + written up in `docs/architecture.md` §8.5:
  `labonair-terminal → labonair-theme` (ANSI palette; terminal also renders GPUI
  cells) and `labonair-panel-ai → {command-palette, editor}` (slash-command
  model + composer buffer). Panels transitively reaching `panel-git-graph`
  *through* `workspace` is sanctioned per §8.4 — the check only forbids direct
  panel→panel and any path to `shell`.
- **CI**: new `crate-deps` job in `.github/workflows/ci.yml` (ubuntu, ~10 min,
  blocking) running `scripts/check-crate-deps.sh`. A PR with a forbidden edge
  goes red.
- **Crate-graph doc**: `scripts/gen-crate-graph.sh` + `scripts/gen_crate_graph.py`
  emit `docs/assets/crate-graph.dot` + a self-contained tiered
  `docs/assets/crate-graph.svg` (no Graphviz on this box; the script uses `dot`
  if present, else a built-in tiered renderer) and print the Markdown adjacency
  list now embedded in `docs/architecture.md` **§9 "Ist-Graph after Phase 15"**
  (20 crates, 81 edges, 8 tiers).
- **`docs/perf-baseline.md`** — environment (Xeon E5-2695 v4, 4 vCPU, rustc
  1.98.0, 2026-09-03) + measurements: cold `cargo check --workspace
  --all-targets` **327 s**, warm **2 s**, clippy-after-check **23 s**,
  **incremental `cargo check -p labonair-shell` after a 1-line edit in
  `app_shell.rs` ≈ 3 s** (stabilised; a 158 s first-run outlier is explained).
  The ~3 s incremental is the key number for T21-002 — the split means a
  1-line `app_shell.rs` edit no longer recompiles the old `ui` monolith.

### Coordinator-forced deviations (recorded)
- `cargo test --workspace` is **disk-banned** on this VPS. Gate actually run:
  `cargo fmt --check` ✓, `cargo check --workspace --all-targets` ✓,
  `cargo clippy --workspace --all-targets -- -D warnings` ✓,
  `scripts/check-crate-deps.sh` ✓. No crate source was modified, so there is no
  touched crate to run `cargo test -p` against.
- Baseline omits cold/warm `cargo build --release` and `cargo test --workspace`
  timings (disk budget) — deferred to T21-002 on CI hardware. Noted in
  `docs/perf-baseline.md`.

### State
- Branch `master`. **Phase 15 (T16-001 … T16-010) is COMPLETE** — the crate
  graph stands: 20 crates, acyclic, CI-enforced.
- Untracked concurrent-session edits left alone: `docs/adr/0001-crate-decomposition.md`
  (modified by another session — NOT staged), `bericht-*.md`, `zed-refrence/`.

### Next Task
**T17-001** — `tasks/phase-16-registries/T17-001-panel-trait-and-registry.md`
(`Panel` trait + `PanelRegistry` wiring). Its dep (T16-008 panel crates) is Done.

### Blockers
None.

---

## Session: 2026-09-03 (T16-009 — `labonair-shell` extracted, `crates/ui` deleted)

Ninth code task of the architecture rework. The `crates/ui` monolith is now
**gone**. Pure mechanical move — `app_shell.rs` is unchanged (its diet is
T17-006).

### What Was Done (T16-009)
- **New crate `labonair-shell`** — `crates/shell/`, explicit
  `[lib] name = "labonair_shell" path = "src/shell.rs"`, workspace member in
  place of `crates/ui` (after `crates/panel-ai`, before `crates/terminal`).
- **`git mv` into `crates/shell/src/`** (history preserved): `app_shell.rs`
  (byte-for-byte), `menu.rs`, `window_state.rs`, `assets.rs` (+ the whole
  `assets/` dir — `include_bytes!("../assets/icons/…")` is crate-relative),
  `updater.rs`, `cwd_breadcrumb.rs`, `sidebar_slot.rs`. `tests/` (only
  `no_pictograph_icons.rs`, `CARGO_MANIFEST_DIR`-relative) moved too.
- **`crates/ui` deleted** (`git rm -r crates/ui`). Everything else in it was
  already a T16-006/007/008 re-export shim; only `crates/app` depended on
  `labonair-ui`.
- **Theme-store home decision (task instr. 3): `labonair-theme`, no move.**
  T16-006 already moved the runtime `ThemeStore` closure to
  `labonair_theme::store`; `background`/`syntax_theme`/`markdown`/`tabs` already
  in `labonair-workspace`. Task instructions 3–4 were superseded by T16-006 —
  T16-009 only confirms current homes. Recorded in `docs/architecture.md`
  (new "T16-009 outcome note").
- **Coupling via `pub(crate) mod` shims in `crates/shell/src/shell.rs`**
  (T16-006/008 pattern): `crate::{background,bar_items,live_bridge,pane,session}`
  → `labonair_workspace::…`, `crate::theme` → `labonair_theme::store`,
  `crate::workspace` → `labonair_workspace::Workspace`. Zero edits to
  `app_shell.rs` / `updater.rs`.
- **`crates/app/src/main.rs`: import paths only, no logic line changed.**
  `labonair_ui::` → `labonair_shell::` (sed). `labonair-shell` re-exports the
  init hooks it doesn't own — `labonair_theme::{init_fonts, init_theme}`,
  `labonair_notifications::init as init_notifications`,
  `labonair_workspace::background::init as init_background` — so `main.rs` keeps
  one import root.
- **`transfers.rs` / `views/hosts.rs` stay in `labonair-workspace`.** `Workspace`
  still owns `Entity<TransfersView>` / `Entity<HostManagerView>`; moving them to
  the shell is a logic refactor, deferred to T17 (out of T16-009 scope). Noted
  in the architecture outcome note.
- **`labonair-shell` deps:** workspace, settings-ui, all 5 `panel-*`,
  command-palette, notifications, theme, ui-kit, gpui-ext, terminal, backend
  (+ gpui, gpui-component, serde, serde_json, tracing, dirs, tokio).

### Verification (T16-009)
- `cargo fmt --check` — ✅
- `cargo check --workspace --all-targets` — ✅ (clean; only pre-existing
  `proc-macro-error2` third-party future-incompat warning)
- `cargo clippy --workspace --all-targets -- -D warnings` — ✅
- `cargo test` — **scoped per touched crate** (`cargo test --workspace` is
  banned in this env — link-storms the disk; accepted coordinator-forced
  deviation): `cargo test -p labonair-shell` ✅ (26 lib + 1 integration:
  cwd_breadcrumb/menu/sidebar_slot/updater/app_shell/assets/window_state +
  `no_pictograph_icons`), `cargo test -p labonair` ✅ (3 smoke).
- `cargo tree --workspace | grep 'labonair-ui '` → **0 matches**.
  `cargo tree -p labonair` — direct deps: shell, terminal, editor, backend, ai,
  theme (matches `docs/architecture.md §3` rule 3).
- **`cargo run` end-to-end visual check: not possible — headless VPS, no
  display.** Build + app smoke tests stand in (same constraint as every prior
  crate-split task).

### State / Next
- Branch `master`, committed (see trailer). Working tree still carries the
  parallel planning session's untracked `bericht-*.md` / `zed-refrence/` /
  `tasks/phase-1*/` and modified `tasks/ROADMAP.md` / `docs/adr/0001` — left
  untouched, not staged.
- Disk: ~7.6 G free after this task (parallel session also building). `cargo
  clean` was run at task start per coordinator instructions.
- **Next task:** T16-010 — build-hygiene + baseline.

---

## Last Session: 2026-09-03 (T16-008 — split six panel crates out of `crates/ui`)

Eighth code task of the architecture rework. Pure mechanical crate split — no
logic refactor; `impl Panel` for the five panels is deferred to T17-001.

### What Was Done (T16-008)
Six new crates, all with explicit `[lib] name/path`, all workspace members
placed **before** `crates/ui`:

| Crate | `[lib] path` | Moved source | Direct deps beyond gpui |
|---|---|---|---|
| `labonair-panel-explorer` | `src/panel_explorer.rs` | `ui/explorer.rs` (root) + `ui/bookmarks.rs` (`bookmarks` submod) | workspace, backend, theme, ui-kit, notifications, tracing, notify, notify-debouncer-mini |
| `labonair-panel-scm` | `src/panel_scm.rs` | `ui/git.rs` | tokio, backend, theme, ui-kit, notifications |
| `labonair-panel-git-graph` | `src/panel_git_graph.rs` | `workspace/src/views/git_graph.rs` | tokio, backend, theme, ui-kit, notifications |
| `labonair-panel-snippets` | `src/panel_snippets.rs` | `ui/snippets.rs` | tokio, uuid, serde_json, backend, theme, ui-kit, notifications, **workspace** |
| `labonair-panel-ai` | `src/panel_ai.rs` | `ui/ai_chat.rs` (root) + `ui/ai_composer.rs` (`ai_composer` submod) | tokio, uuid, serde_json, ai, backend, editor, command-palette, theme, ui-kit, **workspace** |
| `labonair-hosts-ui` | `src/hosts_ui.rs` | `workspace/src/views/{hosts,ssh_connection}.rs` | tokio, uuid, serde_json, backend, theme, ui-kit, notifications |

- **Coupling broken via `pub(crate) mod` shims** inside each moved lib-root file
  (T16-006 precedent): `theme` → `labonair_theme::store`, `workspace` →
  `labonair_workspace::Workspace`, `preview` →
  `labonair_workspace::views::preview`, `markdown` /`syntax_theme` →
  `labonair_workspace::{markdown,syntax_theme}`. `bookmarks.rs`'s one non-shim
  edit: `use crate::explorer::ExplorerView` → `use crate::ExplorerView`.
- **No contract traits needed.** `hosts.rs` already emits `HostManagerEvent`
  (Connect/OpenSftp) — no direct `Workspace`/tab calls — so the "callback
  parameter" fallback in the task Warnung was unnecessary; `labonair-hosts-ui`
  has **zero** `labonair-workspace`/`labonair-panel` deps (rule 9 satisfied,
  `cargo tree -p labonair-hosts-ui` confirmed).
- **`AgentAccessStore` stays in `labonair_workspace::agent_access`** (shared with
  `Workspace`, T11-006); `labonair-panel-ai` re-exports it
  (`pub use labonair_workspace::agent_access::{AgentAccessEntry, AgentAccessStore}`).
- **New directed edges (all acyclic):** `labonair-workspace` →
  `labonair-panel-git-graph` + `labonair-hosts-ui` (it owns the tab-view
  entities: `git_graph`, `host_manager`, `ssh_connection` status store);
  `labonair-panel-{explorer,snippets,ai}` → `labonair-workspace`. Neither
  `panel-git-graph` nor `hosts-ui` depends back on `workspace`, so no cycle.
  `cargo tree` shows **no direct** `panel-* → panel-*`, `panel-* → labonair-ui`,
  or `panel-* → labonair-shell` edge. (A transitive
  `panel-explorer → workspace → panel-git-graph` path exists and is inherent /
  allowed — documented in `docs/architecture.md §8.4`.)
- **Bookmarks decision:** embedded in `labonair-panel-explorer` as a `bookmarks`
  submodule (default per task). Still an `EventEmitter` overlay view — `AppShell`
  keeps `self.bookmarks: Entity<BookmarksView>` and renders it as an overlay,
  semantics unchanged. Recorded in `docs/architecture.md §8.4`.
- **`crates/ui`:** removed the 10 `pub mod`s (explorer/git/git_graph/hosts/
  ssh_connection/snippets/ai_chat/ai_composer/agent_access/bookmarks); deleted
  the four remaining shim files (`git_graph.rs`, `hosts.rs`, `ssh_connection.rs`,
  `agent_access.rs`); repointed `lib.rs` `pub use` + `app_shell.rs` `use` at the
  new crates. Dropped now-orphaned deps: `labonair-ai`, `labonair-editor`,
  `uuid`, `notify`, `notify-debouncer-mini`.
- **`crates/workspace`:** `views.rs` lost `pub mod git_graph/hosts/
  ssh_connection`; `workspace.rs` imports `GitGraphView` from
  `labonair_panel_git_graph`, and `HostManagerView`/`HostManagerEvent`/
  `HostStatus`/`ActiveTunnelRow` + the `ssh_connection` types from
  `labonair_hosts_ui`.
- **`docs/architecture.md`:** added §8.4 (bookmarks decision + the new acyclic
  edges), updated the per-crate source-file table rows for `labonair-workspace`
  (bookmarks removed) and `labonair-panel-explorer` (+ bookmarks).

### Verification (T16-008)
- `cargo fmt --check` — ✅
- `cargo check --workspace --all-targets` — ✅
- `cargo clippy --workspace --all-targets -- -D warnings` — ✅ (two doc-comment
  notes converted from `//!` to `//` to dodge `clippy::doc_lazy_continuation`
  after a list item, in `panel_snippets.rs` / `panel_explorer.rs`).
- `cargo test` — **scoped per touched crate, all green**: `labonair-hosts-ui`,
  `-panel-scm` (16), `-panel-git-graph` (10), `-panel-explorer` (9),
  `-panel-snippets` (15), `-panel-ai` (19), `labonair-workspace` (84),
  `labonair-ui` (26+1). Full `cargo test --workspace` could **not** be run to
  completion: the host filesystem is exhausted (≈20 MB free on `/`; every
  `--workspace` test build writes multi-GB fresh artifacts and `ld` dies with
  SIGBUS/ENOSPC while linking the `labonair` **bin** — untouched by this task).
  Per the coordinator's disk-exhaustion fallback, scoped `-p <crate>` runs for
  every touched crate stand in; `check`/`clippy` pass workspace-wide.
- `cargo tree` forbidden-edge audit — ✅ (see edges note above).

### State / Next
- Branch `master`, committed (see commit trailer). Working tree also carries an
  unrelated in-flight `docs/adr/0001-crate-decomposition.md` edit + untracked
  `bericht-*.md` / `zed-refrence/` from the parallel planning session — left
  untouched.
- **Blocker for the next session:** host disk is full (`/` at 100%, ~20 MB
  free). A `cargo` full rebuild / `cargo test --workspace` will fail on linking
  until space is reclaimed (`target/debug/deps` is ~24 GB; `cargo clean` or a
  selective prune needed, or free space outside the repo).
- **Next task:** T16-009 — `labonair-shell` + `labonair-app` slim.

---

## Last Session: 2026-09-03 (T16-007 — extract `labonair-settings-ui` + palette Hosts page)

Seventh code task of the architecture rework. Pure mechanical crate split of
`crates/ui/src/settings.rs` (5 998 lines) + the bundled "Zusatz" palette
host-access rework.

### What Was Done (T16-007)
- **`crates/settings-ui/`** (`labonair-settings-ui`) created — `[lib] name =
  "labonair_settings_ui"`, `path = "src/settings_ui.rs"`. Member added **before**
  `crates/ui`. Deps: `gpui`, `gpui-component`, `serde`/`serde_json`, `tokio`,
  `tracing`, `uuid`, `labonair-theme`, `-ui-kit`, `-gpui-ext`, `-notifications`,
  `-command-palette`, `-workspace`, `-backend`, `-ai`. **No `labonair-ui` /
  `labonair-shell`** (`cargo tree -p labonair-settings-ui` confirmed —
  `labonair-workspace` + `labonair-ai` are beyond the task's literal dep list
  but required, same as T16-006's `labonair-ai`: `SettingsView` holds
  `Entity<BackgroundStore>` (workspace) + `labonair_ai::InstanceStore`).
- **`settings.rs` sliced into 13 modules** (`git rm`'d, re-created — the file was
  too intertwined for `git mv` to help). Only `mod`/`use`/`pub use`/visibility
  lines changed; every logic line is a verbatim line-range slice of the
  original:
  * `store.rs` (144) — `PreferencesStore`, `impl PalettePrefs`, `pub use
    labonair_workspace::prefs::GlobalPreferences`.
  * `window.rs` (~210) — `open_settings_window`, `settings_bounds`,
    `SettingsDeps`/`SettingsWindowRef`/`SettingsTarget` (now `pub(crate)` so
    `view.rs` can name them), `set_settings_deps`, **plus** the new
    `KeybindApplyHook` global + `set_keybind_apply_hook` + `apply_keybinds`
    (see coupling note).
  * `fields.rs` (1149) — `SettingsTab`(+`category_index` now `pub(crate)`),
    `FieldKind`, `FieldDef`, category consts, `CATEGORIES`, `FIELDS`, `d()`.
  * `sections.rs` (257) — `FieldGroup`, `SECTION_GROUPS`.
  * `apply.rs` (251) — `KbCapture`/`capture_keybind`/`overwrite_keybind`,
    `font_overrides_from` (pub(crate)), `apply_prefs_to_theme` / `theme_choices`
    / `preview_app_theme` / `activate_app_theme` (`pub` — cross-crate consumers
    in `app_shell.rs`), `scan_themes`/`read`/`save`/`delete_theme_in`/`slugify`/
    `trim_ext`/`char_of` (pub(crate)).
  * `view.rs` (~1030) — `SettingsView` struct (all fields `pub(crate)`), helper
    structs (`EditState`/`ThemeEntry`/`RemoteTheme`/`AiEditor`/`SelectMenu`/
    `KbConflict`, fields `pub(crate)`), `Palette`, `mock_community_themes`,
    lifecycle + generic-edit + `on_key` methods, `impl Focusable` + `impl
    Render`, the small render helpers (`slider_track`/`section_label`/`bg_tile`/
    `step_btn`) and `bridge_switch_row`/`bridge_int_row`. Declares a
    `pub use` re-export prelude (gpui/serde/backend imports) + `pub(crate) use
    crate::{apply,fields,sections,store,window}::*` so the pane modules only
    need `use crate::view::*`.
  * `panes/{generic,shortcuts,ai,themes}.rs` — four more `impl SettingsView`
    blocks (Rust allows an inherent impl split across files of one crate). All
    inherent methods bumped `fn` → `pub(crate) fn` (mechanical, not logic) so
    they resolve cross-module; the 5 originally-`pub` lifecycle methods stay
    `pub`. Each pane file < 1150 lines.
  * `tests.rs` — the `#[cfg(test)]` module (inner mod renamed `tests`→`cases`
    to dodge clippy `module_inception`); `use super::*` → explicit
    `use crate::{apply,fields,store}::…`.
  * `settings_ui.rs` — lib root: `mod` decls + `pub use` of the same symbols
    `crates/ui/src/lib.rs` used to export from `settings`.
- **`bar_items` relocated** `crates/ui/src/bar_items.rs` →
  `labonair_workspace::bar_items` (`git mv`) + a `pub use
  labonair_workspace::bar_items::*;` shim in `crates/ui` (T16-006 pattern).
  Needed because `labonair-settings-ui`'s Layout pane uses `BarItemId` /
  `Placements` / `placement_patch` and must not depend on `crates/ui`.
  `pub mod bar_items;` added to `crates/workspace/src/workspace.rs`. `app_shell`
  keeps `use crate::bar_items::…` unchanged (resolves through the shim).
- **`crates/ui`:** `pub mod settings;` removed, `settings.rs` deleted;
  `pub use settings::{…}` → `pub use labonair_settings_ui::{…}`;
  `crate::settings::X` → `labonair_settings_ui::X` in `app_shell.rs` (menu.rs
  had no `crate::settings::` refs — the task's note was stale). `crates/ui`
  gained a `labonair-settings-ui` path dep. `crates/app/src/main.rs` untouched
  (no settings init calls).
- **Keybind-apply coupling** (`view.rs` called `crate::menu::apply_keybinds`):
  broke it with the `set_settings_deps` pattern — a `fn(&mut App, &KeybindMap)`
  hook stored in a GPUI global (`KeybindApplyHook` in `window.rs`), installed by
  `AppShell::new` via `labonair_settings_ui::set_keybind_apply_hook(
  crate::menu::apply_keybinds, cx)`. `view.rs` now calls
  `crate::window::apply_keybinds(cx, &kb)` which is a no-op when no hook is set
  (tests/headless).
- **Palette "Zusatz" (`labonair-command-palette`, additive):**
  * `PaletteRow` gained `secondary: Option<SecondaryAction>` (`{ label:
    &'static str, key: RowKey }`). `Shift+Enter` on a row with a secondary runs
    it (`run_secondary` → shared `dispatch(RowKey)` extracted from
    `run_selected`). Footer hint bar shows `⇧↵ <label>` for the selected row.
  * `Page::HostsSsh` + `Page::HostsSftp` → single **`Page::Hosts`**
    (placeholder "Search hosts…", label "Hosts"). New `host_rows()` helper:
    primary = `ConnectHost{sftp:false}`, secondary "Open SFTP" =
    `ConnectHost{sftp:true}`, subtitle = the host `PaletteChoice.subtitle`.
  * `PaletteData.recent_hosts: Vec<PaletteChoice>` — quick-connect rows
    appended to `Page::Root` (host pre-sorts by `last_connected_at`, cap 5;
    `app_shell` fills it from `Workspace::recent_hosts(cx, 5)`).
  * `ConnectSsh` / `OpenSftp` commands both `sub_page = Page::Hosts`.
  * `CommandPalette::open_to_page(Page, …)` added; `Cmd+Shift+N`
    (`act_new_ssh_connection`) now opens the palette to `Page::Hosts` instead of
    `Workspace::open_host_manager`.
  * `OpenHostManager` command unchanged (T19-010 will re-point it). No
    `command-palette → hosts-ui` edge (`cargo tree` confirmed).
  * `subtitle = user@addr:port` from item 7 is **not** wired — `Workspace::
    recent_hosts`/`known_hosts` only expose `(id, name)`; subtitle stays `None`
    for now. Note for T19-010.

### Verification (T16-007) — all green
- `cargo fmt --check` — exit 0 (whole workspace).
- `cargo check --workspace --all-targets` — exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0 (only the
  pre-existing `proc-macro-error2` future-incompat note).
- `cargo test --workspace` — 0 failed. `labonair-settings-ui` = **75 passed**
  (the moved field/category/keybind/theme-file tests); `labonair-command-palette`
  = 63 passed (incl. the updated `Page::Hosts` list tests).
- `cargo tree -p labonair-settings-ui` — no `labonair-ui` / `labonair-shell`.
  `cargo tree -p labonair-command-palette` — no `labonair-hosts-ui`.
- ⚠️ Disk hit 100 % during the `cargo test --workspace` test-binary link storm.
  Recovered by `rm -rf target/debug/deps/*.tmp*` + deleting stale test/bin
  executables in `target/debug/deps` (NOT `.rlib`/`.rmeta`), then re-ran
  serialized (`cargo test -j 1`). **Do not** `rm -rf target/debug/build` — it
  forces a full from-scratch rebuild.

### What's Next
- **T16-008** `tasks/phase-15-crate-split/T16-008-split-panel-crates.md` —
  ausgliedern der Panel-Crates (+ `labonair-hosts-ui`).

### Blockers
- None. Disk very tight (~4-5 G free after the test run; `cargo clean` may be
  needed before T16-008).

---

## Last Session: 2026-09-03 (T16-006 — extract `labonair-workspace`)

Sixth code task of the architecture rework. **Largest single move of the phase.**
Expanded scope beyond a "pure move" because `Workspace` is coupled (struct
fields built in `Workspace::new`) to `ThemeStore`, `GlobalPreferences`,
`AgentAccessStore`, `BackgroundStore`, `TabStore`/`TabKind` and owns
`Entity<HostManagerView>` + `Entity<TransfersView>`. Followed the coordinator's
resolution: move the **full structural closure** into `labonair-workspace`, plus
relocate `ThemeStore` into `labonair-theme`.

### What Was Done (T16-006)
- **`crates/workspace/`** (`labonair-workspace`) created — `[lib] name =
  "labonair_workspace"`, `path = "src/workspace.rs"` (lib root). Member added
  before `crates/ui`. Deps: `gpui`, `image`, `serde`/`serde_json`, `tokio`,
  `uuid`, `chrono`, `tracing`, `dirs`, `labonair-theme`, `-ui-kit`, `-gpui-ext`,
  `-notifications`, `-command-palette`, `-panel`, `-terminal`, `-editor`,
  `-backend`, `-ai`. **No `labonair-ui` / `labonair-shell`** (`cargo tree -p
  labonair-workspace` confirmed — only `labonair-ui-kit` shows, which is
  allowed). `labonair-ai` added beyond the task's literal dep list because
  `live_bridge.rs`'s `WorkspaceLiveBridge` impls `labonair_ai::LiveBridge`.
- **`git mv` into `crates/workspace/src/`** (history preserved):
  `workspace.rs` (lib root), `pane.rs`, `session.rs`, `live_bridge.rs`,
  `tabs.rs`, `agent_access.rs`, `background.rs`, `bell.rs`, `markdown.rs`,
  `syntax_theme.rs`, `transfers.rs`; and into `src/views/`: `terminal.rs`,
  `editor.rs`, `sftp.rs`, `preview.rs`, `ssh_connection.rs`, `git_graph.rs`,
  `diff.rs`, `hosts.rs`.
- **New modules in `labonair-workspace`:**
  * `src/pane_group.rs` — the recursive split-tree (`PaneNode`,
    `WorkspaceLayout` + all tree tests) split out of `pane.rs`; `pane.rs` keeps
    the leaf primitives (`PaneId`, `SplitAxis`, `CloseOutcome`, `MIN_RATIO`) and
    re-exports `PaneNode`/`WorkspaceLayout` so `crate::pane::…` paths keep
    resolving. Minimal split for T17-004; no full refactor.
  * `src/prefs.rs` — `GlobalPreferences(pub Preferences)` newtype + `impl
    Global`, moved out of `crates/ui/src/settings.rs`. `settings.rs` now does
    `pub use labonair_workspace::prefs::GlobalPreferences;`.
  * `src/drag.rs` — `DraggedPaths` + `shell_quote` + `quote_paths`, extracted
    from `crates/ui/src/explorer.rs` (needed by `views::terminal` for
    drag-into-terminal). `explorer.rs` re-imports them from here.
  * `AskAboutSelection` gpui action defined in the lib root (extracted from
    `crates/ui/src/menu.rs`'s `actions!` block) so `views::terminal` can
    dispatch it without a cycle; `menu.rs` re-exports it.
  * `pub mod theme { pub use labonair_theme::store::*; }` shim so every
    `crate::theme::…` path in the moved code resolves unchanged.
- **`hosts` + `transfers` are acknowledged TEMPORARY residents** of
  `labonair-workspace` — `Workspace` owns those view entities today. Long term
  `views/hosts.rs` → `labonair-panel-hosts` (T16-008), `transfers.rs` →
  `labonair-shell`. `TODO(T16-008)` / `TODO(shell)` comments added at each
  module head; `docs/architecture.md` records the temporary placement (plus a
  full "T16-006 outcome note").
- **ThemeStore relocation:** `crates/ui/src/theme.rs` → `labonair_theme::store`
  (`ThemeStore`, `ThemeMode`, `FontOverrides`, `GlobalTheme`, `init`,
  `init_fonts`, `theme_store`, `active_theme`, `modal_scrim`, `SCROLLBAR_SIZE`,
  `menu_metrics`). Re-exported from `labonair_theme`'s lib root. The
  `impl labonair_ui_kit::UiTheme for ThemeStore` moved into `crates/ui-kit`
  (`impl UiTheme for labonair_theme::ThemeStore`) — orphan rule: `crates/theme`
  must not depend on `crates/ui-kit`. `crates/theme` gained a `[dev-dependencies]
  gpui { test-support }` for the moved `ThemeStore` tests. `crates/ui/src/
  theme.rs` shrank to `pub use labonair_theme::store::*;`.
- **`crates/ui` keeps thin `pub use` shims** for every moved module
  (`agent_access`, `background`, `live_bridge`, `markdown`, `syntax_theme`,
  `tabs`, `transfers`, `session`, `pane`, `workspace`, `views/*`), so
  `app_shell.rs` / `menu.rs` / `snippets.rs` / `bookmarks.rs` / `explorer.rs`
  needed no import-path churn. `bell` module removed from `crates/ui` (no
  remaining consumer there). One real cross-crate visibility fix:
  `Workspace::render_tab_bar` was `pub(crate)` (called only from `app_shell`) →
  now `pub`.
- `Workspace::new` argument list/order unchanged.

### Verification (T16-006) — all green
- `cargo fmt --check` — clean (after `cargo fmt` fixed import ordering in the
  rewritten `workspace.rs` use-block).
- `cargo check --workspace --all-targets` — exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0 (only the
  pre-existing `proc-macro-error2` future-incompat *note*, not a lint).
- `cargo test --workspace` — exit 0, 0 failed (moved tests run in the new crate:
  `pane_group`, `drag`, `background`, `views::terminal`, `views::hosts`,
  session/live_bridge, etc.).
- `cargo tree -p labonair-workspace` — no `labonair-ui` / `labonair-shell` edge.
- Disk fell to ~8.9G free during `cargo test` link; still completed. Consider
  `rm -rf target/debug/incremental` before the next heavy task.

### What's Next
- **T16-007** `tasks/phase-15-crate-split/T16-007-extract-settings-ui-crate.md`
  — extract `labonair-settings-ui`.

### Blockers
- None. Disk tight (~9G free).

---

## Planning Update: 2026-09-03 (Workflow-Rework — Themen 1–3 in Phasen 15–18 eingearbeitet)

**Nur Task-/Doku-Änderungen, kein Code.** Drei Workflow-Reworks in den laufenden
Architektur-Rework integriert. T16-006 lief währenddessen → alles konzeptionell
≤ T16-006 ging in **T16-007**.

1. **Tabs optional** — alle Tabs schließbar → leere Workspace-Fläche
   (Doppelklick → lokales Terminal); `TabKind::Home` entfällt.
2. **Host-Zugang ohne Host-Manager-Tab/-Panel** — Verbinden über
   Command-Palette-`Page::Hosts` (`Enter` = SSH, `Shift+Enter` = SFTP) +
   `＋▾`-Titlebar-Menü; Verwalten über neue Top-Level-Kategorie **Settings ›
   Hosts**. `labonair-panel-hosts` entfällt aus dem Ziel-Graph →
   `labonair-hosts-ui` (kein Panel, kein `impl Panel`). `PanelIcon::Hosts`
   (aus T16-005) wird in T17-001 mit entfernt.
3. **Settings-Redesign** — Zed-Settings-System wie geplant, **plus**
   Design-Kontrakt (`docs/settings-guidelines.md`, T19-000), Kategorie→Abschnitt-
   Disclosure-Navigation, erstklassiger Pfad für Custom-Top-Level-Kategorien.

**Geändert:** `docs/architecture.md` (neuer §8 „Deviations"), `tasks/ROADMAP.md`
(Tabellen 15–18 + Workflow-Rework-Abschnitt + Erfolgskriterium 27), `T16-007`,
`T16-008`, `T17-001`, `T17-004`, `T17-006`, `T18-001`, `T18-003`, `T19-001`,
`T19-004`, `T19-009`.
**Neu:** `T17-009` (Tabs optional / Empty-Workspace), `T19-000`
(Settings-Design-Kontrakt), `T19-010` (Settings › Hosts),
`bericht-workflow-rework.md` (Begründung).

Nächster Task unverändert: **T16-006** (läuft) → dann **T16-007** (jetzt inkl.
Palette-`Page::Hosts`-Zusatz).

---

## Last Session: 2026-09-03 (T16-005 — labonair-panel contracts crate)

Fifth code task of the architecture rework. **New leaf contracts crate, no
existing code migrated, no behaviour change.**

### What Was Done (T16-005)
- **`crates/panel/`** (`labonair-panel`) created — lib root `src/panel.rs`
  (`[lib] name = "labonair_panel"`, `path = "src/panel.rs"`). Two modules:
  * `src/dock.rs` — `DockPosition {Left,Right,Bottom}` (+ `ALL`, `next()`),
    `PanelIcon` (small closed enum: `Explorer|SourceControl|GitGraph|Hosts|
    Snippets|Ai` — deliberately NOT importing `labonair_ui_kit::IconName`, so
    the crate stays a near-leaf; shell maps variants to real icons),
    `PanelEvent {Activate,Close,ZoomIn,ZoomOut}`, `trait Panel: Focusable +
    Render + Sized` (`persistent_name`, `title`, `icon`, `position`,
    `position_is_valid`, `set_position`, `default_size`, `min_size`),
    `trait PanelHandle: Send + Sync` + blanket `impl<T: Panel + 'static> for
    Entity<T>` + `type AnyPanelHandle = Arc<dyn PanelHandle>` (Zed's
    handle-wrapper pattern for object safety), `PanelConstructor` alias,
    `PanelRegistration`, `PanelRegistry` (`register`/`iter`/`for_position`/
    `get`/`len`/`is_empty`, `impl Global`).
  * `src/status.rs` — `StatusSide {Left,Right}`, `StatusItemHide` (port of
    Zed `HideStatusItem`; carries `Arc<dyn Fn(&mut App)>` — no serde/settings
    dep yet, T18-005 swaps the body), `trait StatusItem: Render` (`id`,
    `default_side`, `render_status` [named to avoid `Render::render`
    collision], `hide`), `trait StatusItemHandle` + blanket impl +
    `AnyStatusItemHandle`, `StatusItemConstructor`, `StatusItemRegistration`,
    `StatusItemRegistry` (same method surface, `impl Global`).
  * Every trait/enum has a doc comment citing the Zed source file and listing
    the deliberate omissions vs. the Zed original.
- **Deps** (`cargo tree -p labonair-panel`): `gpui`, `labonair-gpui-ext` only.
  `cargo tree` shows **no** edge to `labonair-workspace` / `labonair-shell` /
  any `labonair-panel-*` (none exist yet) — architecture §3 rule 1 satisfied.
  `docs/architecture.md` already listed this crate; no doc change needed (the
  `PanelIcon` decision matches the "define a light enum here" option it
  offered).
- **No existing code migrated.** `SidebarPanel` / `BarItemId` in
  `crates/ui/src/app_shell.rs` are untouched — T17-001/003 will rebuild on
  these contracts. `Tabs` is intentionally absent from `PanelIcon` (it becomes
  titlebar chrome, not a panel).
- **Workspace `Cargo.toml`**: `crates/panel` added as a member before
  `crates/ui`. No crate depends on it yet.
- **Tests**: 7 unit tests in the new crate (registry register/replace/filter/
  lookup for both registries, `DockPosition::next` wrap). Stub constructors use
  `Arc::new(|_, _| unreachable!())` — never invoked by the bookkeeping methods,
  so no `gpui::App` needed.

### Verification (T16-005)
- `cargo fmt --check` (workspace) — clean.
- `cargo check --workspace --all-targets` — exit 0 (6m full rebuild).
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0
  (pre-existing `proc-macro-error2` future-incompat note only).
- `RUSTDOCFLAGS="-D warnings" cargo doc -p labonair-panel --no-deps` — clean.
- `cargo test -p labonair-panel` — 7 passed, 0 failed.
- `cargo test --workspace` was started but **killed to protect disk** (free
  space fell 18G→8G during the test-binary codegen). Scoped per-crate test is
  the sanctioned fallback here (see session prompt): the only touched crate
  with tests is the new leaf `labonair-panel`; no other crate depends on it or
  changed, and workspace check + clippy (both `--all-targets`) pass.
  Reclaimed space afterwards with `rm -rf target/debug/incremental`.

### What's Next
- **T16-006** `tasks/phase-15-crate-split/T16-006-extract-workspace-crate.md`
  — extract `labonair-workspace`.

### Blockers
- None. Disk is tight (~11G free after cleanup); prefer scoped `cargo test -p`
  over `--workspace` for the next tasks.

---

## Prior Session: 2026-09-03 (T16-004 — extract labonair-command-palette)

Fourth code task of the architecture rework. **Move + proper decoupling
(Option 1), zero behaviour change.**

### What Was Done (T16-004)
- **`crates/command-palette/`** (`labonair-command-palette`) created — lib root
  `src/command_palette.rs` (`[lib] name = "labonair_command_palette"`,
  `path = "src/command_palette.rs"`). Split into three modules:
  * `src/fuzzy.rs` — `SearchMode` + `match_score`. `from_pref`/`to_pref`
    (which named `labonair_backend::…::PaletteSearchMode`) dropped; replaced by
    `SearchMode::from_label` / `label`. The `PaletteSearchMode → SearchMode`
    conversion now lives in the `crates/ui` `PalettePrefs` impl.
  * `src/keybind.rs` — `ShortcutId`, `ShortcutGroup`, `Shortcut`, `SHORTCUTS`,
    `shortcuts`/`shortcut`/`shortcut_keys`, `RESERVED_ACCELERATORS`, `Conflict`,
    `normalize` (priv), `find_conflict`, `shortcut_slug`/`shortcut_from_slug`,
    `KeybindMap`, `effective_binding`, `resolve_conflict` + their 9 tests.
  * `src/palette.rs` — `CommandContext`, `CommandId`, `toggle_pref_key`, `Page`,
    `Command`/`COMMANDS`, `commands`/`command`/`available`/`search_mode`/
    `search`/`command_for_shortcut`/`context_of`, `PaletteEvent`,
    `PaletteChoice`, `PaletteData`, `recent` mod, `RowKey`/`PaletteRow`, the
    generic `CommandPalette<P, W, Th>` view + its 11 tests.
- **Decoupling (the non-trivial bit).** The view no longer names any
  `crates/ui` type. It is generic over three contracts:
  * `Th: labonair_ui_kit::UiTheme` — added defaulted accessors `foreground`,
    `card`, `muted`, `primary`, `status_success`, `selected_fill` (verbatim
    `theme().core.*` / `theme().status.success` reads, matching `ThemeStore`'s
    inherent methods 1:1). `modal_scrim()` inlined in `palette.rs` as
    `gpui::black().opacity(0.30)`.
  * `P: PalettePrefs` (new trait in `palette.rs`) — the 6 `command_palette_*`
    getters the view reads + `set_command_palette_search_mode`. `impl` for
    `PreferencesStore` in `crates/ui/src/settings.rs` (verbatim field reads;
    setter is the old `set_value("commandPaletteSearchMode", …)` call).
  * `W: PaletteWorkspace` (new trait) — `palette_active_context` +
    `palette_tab_rows` (owned `PaletteTabRow { id, label, kind_title, is_ssh }`).
    `impl` for `Workspace` in `crates/ui/src/workspace.rs`.
  * `context_of` now takes the palette-owned `PaletteTabKind` (not `crates/ui`'s
    `TabKind`). `workspace.rs` gained a private `palette_tab_kind(TabKind)` map.
- **Theme enums moved.** `ThemePreference` + `EditorThemeId` (with `slug`,
  `from_slug`, `ALL`) moved from `crates/ui/src/theme.rs` to a new
  `crates/theme/src/prefs.rs`, re-exported from `labonair_theme` **and** from
  `crate::theme` (`pub use labonair_theme::{EditorThemeId, ThemePreference};`)
  so every existing `crate::theme::` / `labonair_ui::` path is unchanged.
  `ThemeMode` stayed in `crates/ui` (not needed below).
- **Deps** (`cargo tree -p labonair-command-palette`): `gpui`, `serde_json`,
  `labonair-theme`, `labonair-ui-kit`, `labonair-gpui-ext`, `labonair-backend`.
  **No** `labonair-ui`. `labonair-backend` is a UI-free leaf — needed for
  `recent`'s `config_dir()` + `serde_json` (kept the recents-file behaviour 1:1
  rather than routing it through another trait). Acyclic graph preserved.
- **`crates/ui`**: `pub mod command_palette;` removed, old
  `crates/ui/src/command_palette.rs` `git rm`'d. `pub use command_palette::{…}`
  → `pub use labonair_command_palette::{…}` (re-export list unchanged, so
  `labonair_ui::CommandPalette` etc. still resolve). Import paths updated in
  `ai_composer.rs`, `menu.rs`, `settings.rs`, `app_shell.rs`, `workspace.rs`.
  `app_shell` field is now
  `Entity<CommandPalette<PreferencesStore, Workspace, ThemeStore>>`; the
  `CommandPalette::new(theme, workspace, prefs, cx)` call site is unchanged
  (types infer from the field).
- **Workspace `Cargo.toml`**: `crates/command-palette` added as a member
  (before `crates/ui`) + direct `path` dep in `crates/ui/Cargo.toml`.

### Verification (T16-004)
- `cargo fmt --check` — clean.
- `cargo check --workspace --all-targets` — exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0
  (pre-existing `proc-macro-error2` future-incompat note only).
- `cargo test --workspace` — see commit; palette/keybind/fuzzy tests run in the
  new crate (20 tests), `labonair-ui` + others unaffected.
- `cargo run` GUI check not performed (headless VPS). Change is import paths +
  a generic-over-contracts view over identical token/pref/tab reads; covered by
  the moved unit tests + full workspace compile.

### What's Next
- **T16-005** `tasks/phase-15-crate-split/T16-005-panel-contracts-crate.md`
  — `labonair-panel` contracts crate.

### Blockers
- None.

---

## Prior Session: 2026-09-03 (T16-003 — extract labonair-notifications)

Third code task of the architecture rework. **Pure move + generic-over-theme,
zero behaviour change.**

### What Was Done (T16-003)
- **`crates/notifications/`** (`labonair-notifications`) created — lib root
  `src/notifications.rs` (explicit `[lib] name = "labonair_notifications"`,
  `path = "src/notifications.rs"`). `crates/ui/src/notifications.rs` moved 1:1
  via `git mv`. File is 685 lines but not split (task said splitting is
  optional; kept as one file to minimise churn).
- **Public API unchanged**: `NotificationCenter`, `Notification`,
  `NotificationAction`, `Severity`, `GlobalNotificationCenter`,
  `notification_center`, `notify_err`, `init` (re-exported as
  `init_notifications`), `render_overlay`.
- **Theme decoupling** (the one non-trivial bit): the old file used
  `crate::theme::ThemeStore` (a `crates/ui` type) directly. Replaced with the
  `labonair_ui_kit::UiTheme` trait — same pattern T16-002 used for `button()`.
  * `Severity::color` → `fn color(self, theme: &impl UiTheme)`, reads
    `theme.theme().status.{info,success,warning,error}`.
  * `render_overlay` → `pub fn render_overlay<Th: UiTheme + 'static>(center,
    theme: &Entity<Th>, cx)`. Reads `theme.theme().core.{card,foreground}` +
    `theme.muted_foreground()` / `theme.border()`. Token values identical to
    the old `ThemeStore` accessors (they were 1:1 field reads). Call site in
    `app_shell.rs` (`render_overlay(&self.notifications, &self.theme, cx)`)
    unchanged — `ThemeStore: UiTheme` already holds, `Th` infers.
- **Deps**: `gpui`, `labonair-theme`, `labonair-ui-kit`, `labonair-gpui-ext`.
  No dep on `labonair-ui` (verified via `cargo tree -p labonair-notifications`).
- **`crates/ui`**: `pub mod notifications;` removed; `pub use notifications::{…}`
  → `pub use labonair_notifications::{…}` (re-export kept so
  `labonair_ui::init_notifications` etc. and `crates/app/src/main.rs` are
  untouched). All `crate::notifications::` → `labonair_notifications::` across
  13 files (sed + `cargo fmt` to re-group `use` lines). `app_shell.rs` import
  became `use labonair_notifications::{self as notifications, NotificationCenter};`
  to keep the `notifications::render_overlay` call site verbatim.
  `crates/ui/Cargo.toml` gains the `labonair-notifications` path dep.
- **Workspace `Cargo.toml`**: `members` gains `crates/notifications` (before
  `crates/ui`). Direct `path = "../notifications"` dep, matching the repo's
  existing convention (the task text mentioned a `[workspace.dependencies]`
  entry, but T16-001/T16-002 established that local crates use direct path deps
  — followed prior art for consistency).

### Verification (T16-003)
- `cargo fmt --check` — clean (exit 0).
- `cargo check --workspace --all-targets` — exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `cargo test -p labonair-notifications` — 8 passed (all the moved tests).
  `cargo test -p labonair-ui` — 246 + 1 passed (nothing broke).
- `cargo tree -p labonair-notifications` — deps are only gpui-ext / theme /
  ui-kit; **no** `labonair-ui`.
- `cargo run` GUI check (startup toast + error toast position/dismiss/styling)
  not performed — headless VPS, no display. The change is behaviour-neutral
  (import paths + a generic theme param over identical token reads), covered by
  the 8 passing tests and a full workspace compile.
- Pre-existing unrelated `proc-macro-error2 v2.0.1` future-incompat note — not a
  `-D warnings` failure.

### What's Next
- **T16-004** `tasks/phase-15-crate-split/T16-004-extract-command-palette-crate.md`
  — extract `crates/ui/src/command_palette.rs` into `labonair-command-palette`.
  Dependencies (T16-002, T16-003) now satisfied.

### Blockers
- None.

---

## Prior Session: 2026-09-03 (T16-002 — labonair-gpui-ext + labonair-ui-kit skeleton)

First code task of the architecture rework. **Pure move + re-export, zero
behaviour change.** Rust toolchain is now installed on the VPS
(`source "$HOME/.cargo/env"` if `cargo` is not on PATH).

### What Was Done (T16-002)
- **`crates/gpui-ext/`** (`labonair-gpui-ext`) created — lib root
  `src/gpui_ext.rs` (explicit `[lib] name/path`). One module: `pub mod prelude`
  re-exporting `gpui::prelude::*` plus the ~30 concrete `gpui` types that recur
  across the port's `use gpui::{…}` lines (derived from a `grep`, not guessed).
  Leaf crate, dep = `gpui` only.
- **`crates/ui-kit/`** (`labonair-ui-kit`) created — lib root `src/ui_kit.rs`
  (explicit `[lib] name/path`). `button.rs`, `context_menu.rs`, `icon.rs`,
  `text_field.rs` moved 1:1 from `crates/ui/src/components/` via `git mv`.
  `ui_kit.rs` re-exports the **exact** old symbol set from `components/mod.rs`
  (`button`, `ButtonSize`, `ButtonVariant`, `DISABLED_OPACITY`, `context_menu`,
  `MenuClick`, `MenuItem`, `file_icon`, `folder_icon`, `IconName`, `field_input`,
  `text_field`, `InputEvent`, `InputState`, + `Badge`, `Switch`, `Tooltip` from
  `gpui-component`) plus the new `UiTheme`. Deps = gpui, gpui-component,
  labonair-theme, labonair-gpui-ext.
- **`UiTheme` trait** (`crates/ui-kit/src/theme.rs`) — thin token accessor so
  ui-kit does not depend on `crates/ui`'s runtime `ThemeStore` (a GPUI entity,
  not pure-token → cannot move to `labonair-theme` yet; the Zed `ui`/`theme`
  split, `docs/architecture.md` §2.1). One required method
  `theme(&self) -> &labonair_theme::Theme`; `radius()` / `muted_foreground()` /
  `border()` are defaulted 1:1 derivations. `button()` / `context_menu()` /
  `colors()` now take `&impl UiTheme`. `crates/ui/src/theme.rs` gains
  `impl labonair_ui_kit::UiTheme for ThemeStore`. Call sites (`self.theme.read(cx)`)
  unchanged — they only satisfy the bound.
- **`IconName::ALL`** promoted from `#[cfg(test)] const` to `pub const` so the
  icon→asset cross-check test can run from `crates/ui` (the SVG bundle stays in
  `crates/ui/src/assets.rs`, which is `labonair-app`-bound per the arch table).
  `every_icon_variant_has_an_asset` now lives in `assets.rs`; `ui-kit`'s
  `icon.rs` keeps a self-contained path-format/uniqueness test.
- **`crates/ui`**: `mod components` removed; all `crate::components::` usages in
  17 files re-pointed to `labonair_ui_kit::` (mechanical sed + `cargo fmt` to
  re-group the new external `use` lines). `crates/ui/Cargo.toml` gains
  `labonair-ui-kit` + `labonair-gpui-ext` path deps.
- **Workspace `Cargo.toml`**: `members` gains `crates/gpui-ext` + `crates/ui-kit`
  (before `crates/ui`). Local-crate deps follow the repo's existing direct
  `path = "../x"` convention (repo does not use `[workspace.dependencies]` for
  local crates).

### Verification
- `cargo fmt --check` — clean (exit 0).
- `cargo check --workspace --all-targets` — exit 0 (compiles all test/bench/
  example code too, so every test still compiles).
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `cargo test -p labonair-gpui-ext -p labonair-ui-kit -p labonair-ui --lib` —
  exit 0: `labonair-ui-kit` 5 passed (button ×2, context_menu ×1, icon ×2),
  `labonair-ui` 254 passed (incl. the relocated `every_icon_variant_has_an_asset`),
  `labonair-gpui-ext` 0 tests. These are exactly the crates T16-002 touches.
- Full `cargo test --workspace` could **not** complete: the VPS root FS is at
  ~100 % (110 G/118 G used, external pressure) and cargo ran out of space
  linking the per-crate test binaries for the *unmodified* crates
  (`labonair-terminal`/`-ai`/`-backend`) — `No space left on device (os error 28)`
  and `-lxcb` link errors before X11 dev libs were apt-installed. Not a
  T16-002 regression. Freed space by `rm -rf target/debug/incremental` +
  `cargo clean`; per-crate testing of the changed crates then passed.
- Pre-existing, unrelated `cargo` note about `proc-macro-error2 v2.0.1`
  (transitive dep of `gpui-component`) — does not fail `-D warnings`.

### What's Next
- **T16-003** `tasks/phase-15-crate-split/T16-003-extract-notifications-crate.md`
  — extract `crates/ui/src/notifications.rs` into `labonair-notifications`.
  Dependency (T16-002) now satisfied.

### Blockers
- None.

---

## Prior Session: 2026-09-03 (T16-001 — architecture-rework ADR + target crate graph)

Kicked off the architecture rework (roadmap phases 15–21). **Documentation only;
no `crates/` change.**

### What Was Done (T16-001)
- **`docs/architecture.md`** created — the authoritative target for every rework
  task (T16-002 … T22-001). Seven sections: (1) philosophy + the four
  principles; (2) target crate graph (7 → ~22 crates) with a per-crate purpose
  sentence and a table mapping every current `crates/ui/src/*.rs` to its target
  crate; (3) eight binding, CI-checkable dependency rules (`labonair-panel` has
  no workspace-track dep; panel crates never depend on each other / `shell` /
  `workspace`; `shell` is the only crate that knows concrete panels;
  backend/ai/terminal/editor have no UI dep; `ui-kit` deps limited to
  gpui/gpui-component/theme/gpui-ext; graph acyclic); (4) layout contract with
  the ASCII diagram + explicit removals (header inline search, `⋯` app-menu
  button, 44px activity rail, titlebar bar-item scope, `drain_pending_*`);
  (5) Zed pattern catalog — table, each row a concrete `zed-refrence/zed/crates/`
  path; (6) settings-layer overview default→user→OS→project→language;
  (7) naming convention `labonair-<name>` / `crates/<name>/` / explicit
  `[lib] path = crates/<name>/src/<name>.rs`.
- **`docs/adr/0001-crate-decomposition.md`** created (new `docs/adr/` dir) —
  standard ADR format: Context (monolith numbers: ui ~48k lines, settings.rs
  5 957, workspace.rs 4 076, app_shell.rs 2 983; god-object, frame buffers,
  latent Panel↔Workspace cycle, parallel `FIELDS` table, slow builds),
  Decision (~22 crates + trait registries), Alternatives (status quo / extract
  only settings / feature folders — all rejected, with reasons), Consequences
  (more `Cargo.toml`, compiler-enforced APIs, faster incremental builds,
  one-time migration cost).
- **`CLAUDE.md`** — the "## Architecture" section now points to both new docs as
  the authoritative target architecture.
- **`tasks/ROADMAP.md`** — "## Vision" gained a philosophy paragraph (parity is
  now the *minimum*, not the goal). The phases 15–21 section + rework success
  criteria 22–26 (present as an uncommitted change) are committed with this task.
- This commit also stages the previously-untracked task dirs
  `tasks/phase-15-crate-split/` … `tasks/phase-21-gpui-decision/`, plus the two
  new planning reports `bericht-architektur-rework-roadmap.md` and
  `vergleichsbericht-zed-vs-rust.md`. `zed-refrence/` left untracked (local Zed
  checkout for API spelunking).

### Verification
- Gates **not run**: this environment has no Rust toolchain (`cargo`/`rustc`
  absent, no `~/.cargo`). T16-001 touches zero files under `crates/`
  (`docs/*.md`, `CLAUDE.md`, `tasks/*.md` only), so `cargo fmt --check` /
  `check` / `clippy -D warnings` / `test` are unchanged-green by construction.
  Recorded in `memory/bugs_and_fixes.md`.
- All six non-gate acceptance criteria met (7 sections present; dep rules
  explicit + testable; removals named; ADR in standard format; ROADMAP lists
  phases 15–21 with every task id; CLAUDE.md references `docs/architecture.md`).

### What's Next
- **T16-002** `tasks/phase-15-crate-split/T16-002-gpui-ext-and-ui-kit-skeleton.md`
  — extract `crates/ui/src/components/*` into `labonair-ui-kit`, create the
  `labonair-gpui-ext` prelude crate, re-export, re-point all call sites.
  Deps (T16-001) satisfied. **Needs a Rust toolchain to verify.**

### Blockers
- No Rust toolchain in the current environment — future code tasks must install
  `rustup` before they can run the gates.

---

## Prior Session: 2026-09-03 (settings-window audit — docs correction)

Investigated the settings system vs. the reference Tauri `open_settings_window`.
**Finding: the port is already at parity with what GPUI 0.2.2 allows.** Since
the Block C `open_settings_window` work, settings render in their own
`cx.open_window` OS window (860 px × 80 %-display clamp `[580, 900]`, min
720×480, transparent titlebar, `SettingsTab` deep-links, live re-target via the
`SettingsTarget` global). The in-`AppShell` overlay is gone (legacy `render`
branch kept for tests only). T16-011 already fixed the model (46→165 fields,
default parity, `editorVimMode`→`vimMode` serde key).

Verified against docs.rs: **`gpui` 0.2.2 `WindowOptions`** has *no* always-on-top
/ window-level, *no* max-size, *no* parent-window field. So the reference
`always_on_top(true)`, `max_inner_size(1400,900)` and `parent(main)` lifecycle
tie are **unportable** on this GPUI — same class as the missing WebView preview.
There is also no per-window hide (close destroys; state is in shared entities so
reopen is lossless).

- **`b3a5ccd` docs(settings): correct stale "modal overlay" comments.** The
  module doc comment in `crates/ui/src/settings.rs` and the `AppShell` comment
  at `app_shell.rs:300` still described the old modal overlay. Rewrote both to
  describe the real separate-window impl + document the GPUI limitations above.
  Comment-only; no gate run (no Rust toolchain in this session's environment —
  needs `cargo fmt --check` / `check` / `clippy -D warnings` / `test` on a dev
  box, but the change cannot affect compilation).

Untracked `zed-refrence/` (local Zed source checkout for API spelunking) — not
committed, not gitignored; leave as-is unless the user wants it ignored.

---

## Prior Session: 2026-09-03 (Final-3% round — LiveBridge + churn + icons + ModelPicker)

Four commits after `860e9dc`, all four gates green each time:

- **`463a361` feat(ai): real LiveBridge wiring.** `AiChatStore` was on
  `NoLiveBridge` → agent terminal tools inert + relative paths resolved
  against the process cwd. New `crate::live_bridge::WorkspaceLiveBridge`:
  `Send + Sync` handle with a snapshot cell (cwd / workspace_root / terminal
  buffer / ssh tab id / has_terminal) + a command queue.
  `AppShell::sync_live_bridge` (each render) writes the snapshot from
  workspace+explorer and drains queued writes into
  `Workspace::run_in_active_terminal` / `inject_into_active_terminal`.
  `Workspace::active_terminal_lines`, `TerminalView::recent_lines`. Tests for
  snapshot read-through + write gating / queue drain.
- **`946023f` refactor(ui): unify buttons + breadcrumb menus.** hosts.rs
  `btn()` now delegates to `crate::components::button` (Xs; Default/Outline)
  — all ~39 call sites unchanged bar a trailing `cx`. `render_crumb_menu` +
  `render_subdir_menu` rebuilt on the shared `components::context_menu`
  primitive. No behaviour change.
- **`d33fcfe` feat(ui): broaden file-icon coverage.** 8 new Lucide glyphs
  (archive, brackets, file-terminal, film, key-round, music, table, type) +
  reworked `file_icon`: ~90 extensions over ~18 distinct icons (JS/TS →
  braces, other scripts → brackets, shell → file-terminal, spreadsheets →
  table, video → film, audio → music, fonts → type, keys → key-round).
- **`20f77ee` feat(ai): ModelPicker — search + All/Favorites/Recent + provider
  rail.** New backend `model_prefs` module (`ModelPrefs { favorites, recent }`,
  toggle/push_recent + JSON persist, 3 tests). `AiChatView` ModelPicker
  rebuilt: 380px panel, lazy `InputState` search, tab row, provider rail,
  per-row star toggle, capability line. `select_model` records recency.

**Voice / whisper** — deliberately left a stub (needs an external whisper
backend + a user decision); the TODO in `render_composer` is precise.

---

## Prior Session: 2026-09-03 (Final-5% round — AI composer power-user + palette symbols + small items)

Five commits after `1ed4481`, all four gates green each time
(`cargo fmt --all`, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace`).

- **`d248ae6` feat(ai): composer slash-commands + `@`-file picker.** New pure
  `crate::ai_composer` (port of `slashCommands.ts`): `SLASH_COMMANDS`
  (`/init`, `/plan`), `parse_slash`/`SlashOutcome`, `wrap_with_command_marker`,
  `detect_popup` (per-keystroke `/` and `@` detection), `filter_slash` /
  `filter_files` (palette `Fuzzy` `match_score`), `apply_file_mention`. Wired
  into `AiChatView`: subscribe `InputEvent::Change` → `refresh_popup`; inline
  popover above the composer, click + Enter completion; `@`-files via
  `labonair_backend` `fs_search` over the workspace cwd. `plan_mode` flag
  added to `AiChatStore`. `send()` intercepts `/init` `/plan`.
- **`f9fd757` feat(ai): command-snippet Run + AI/Shell toggle.** Assistant
  shell code blocks (≤8 lines) get a "Run" button → `AiChatEvent::RunInTerminal`
  → `AppShell` drains → `Workspace::run_in_active_terminal`. AI/Shell toggle
  in the composer footer (Shell mode runs the text in the terminal, "Run"
  button). New `EventEmitter<AiChatEvent>` + `pending_ai` drain.
- **`d0c3cf1` feat(ai): plan mode — PlanModeStrip + PlanDiffReview.**
  `AiChatStore.plan_queue` + `plan_reject`/`plan_discard_all`/`plan_apply_all`;
  `dispatch_tool_calls` diverts `write_file`/`edit`/`multi_edit`/
  `create_directory` into the queue while plan mode is active (proposed
  content via `plan_edit_from_call`, mirroring each tool's replacement logic);
  `PLAN_MODE_PROMPT` system block on plan turns. PlanModeStrip above the
  composer; PlanDiffReview full-panel overlay (per-file rows, +/- stats,
  expandable line-level diff via `labonair_editor` `Diff::compute`, Apply
  all / Discard all).
- **`64e44c4` feat(editor): palette Go to Symbol / outline.** New
  `labonair_editor::symbols::document_symbols(lang, text)` — raw TreeSitter
  parse + the grammar's bundled `TAGS_QUERY` (rust/python/js/ts/go/c/cpp/java)
  → `Vec<DocumentSymbol { name, kind, line }>`. `EditorView::document_symbols`
  + `goto_line`; `Workspace::active_editor_symbols` + `active_editor_goto_line`.
  `AppShell` fills `PaletteData.symbols`; Outline rows → `RowKey::GoToLine` →
  `PaletteEvent::GoToLine` → caret jump + scroll.
- **`fd445c9` feat(ui): previewUrl + tabsLocation + git Open Diff (Split).**
  `terminal::detect_preview_url` scans recent output for a loopback dev-server
  URL; `Workspace::active_preview_url`; statusbar `previewUrl` item renders it
  as a click-to-open chip (was permanent `None`). `tabsLocation`: titlebar tab
  strip hidden when `== "sidebar"`; `BarItemId::TabsPanel` toggle +
  `SidebarPanel::Tabs` wired into `panel_for_item`/`item_for_panel`
  (`render_tabs_panel` already existed). Git panel: "Open Diff (Split)"
  context-menu item + Split/Unified toggle in the diff header;
  `split_hunk_rows` renders side-by-side old/new columns.

### Verified
All four gates green after every commit. Test count grew from 696 → ~706
(`ai_composer` 6, `ai_chat` +3 shell/plan, `editor::symbols` 3,
`terminal::detect_preview_url` 1).

---

## Prior Session: 2026-09-03 (Block F polish D — statusbar + shared-menu cleanups)

### What Was Done (commit after `e93b72b`)

- **`render_bar_menu`** (bar-item context menu) migrated to the shared
  `components::context_menu` primitive — Side (Left/Right, `checked` radios),
  Location (Titlebar/Status Bar), Hide (`EyeOff` icon), with section labels.
- **Tab-bar empty-area menu** — right-click anywhere on the tab strip now
  opens the new-tab dropdown (`workspace.rs render_tab_bar`).
- **"Ask AI about Selection" wired end-to-end**: `EditorView::selected_text`
  + `TerminalView::selection_text` (both filter empty) → new
  `Workspace::active_selection() -> Option<(&'static str, String)>` →
  `AppShell::act_ask_about_selection` (registered as an `on_action` for
  `menu::AskAboutSelection`; the `AiAskSelection` shortcut + the palette
  `AskSelection` command already dispatch it) → `AiChatView::attach_selection`
  + reveal the AI dock. New "Ask AI about Selection" item in the terminal
  context menu (disabled without a selection).

**Statusbar items already done** (verified, not re-touched): `cursorPosition`
(`Ln x, Col y` from `workspace.active_editor_cursor`) and the 11px text size
are already in `render_bar_item` / `render_statusbar`. `previewUrl` stays a
self-hiding placeholder — dev-server-URL detection from terminal output is
not ported.

**Not migrated / deferred (documented):**
- `hosts.rs` `☑/☐` glyphs — none exist (Block E's host-manager rewrite
  already uses real toggle buttons via `self.btn`).
- host-manager `btn` → `components::button` — pure churn on a working shim;
  intentionally left (as the Block E handoff noted).
- breadcrumb `crumb_menu` / `subdir_menu` — already feature-complete
  hand-rolled menus (copy abs/rel path, cd here, cd new tab, move to bar);
  migrating to the primitive is churn without behaviour change.
- "Open Diff (Split)" — the git panel only has an inline unified diff; a
  split/editor-tab diff surface doesn't exist.
- `tabsLocation` gating for the Tabs sidebar panel.
- Palette `outline` / Go-to-Symbol — needs a TreeSitter document-symbol pass
  in `editor.rs` (`labonair-editor` has the grammars; no symbol query yet).

### Verified
`cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (695 tests).

---

## Session: 2026-09-03 (Block F polish A — AI-panel composer + strips)

### What Was Done (commit after `1f2f21a`)

**AI panel composer — real `text_field`**
- `AiChatView.composer_input: Option<Entity<InputState>>` created lazily on
  the first `render` via `ensure_composer(window, cx)` (window is available
  in `render`; solves the window-threading problem without touching `new` or
  the headless test helper). `composer_seed: String` is the pre-render / test
  fallback. `composer_text(cx)` reads from whichever is live.
- Multi-line `InputState` (`auto_grow(2,8)`). `window.subscribe` on the input:
  `PressEnter { secondary:false }` → strip the just-inserted `\n` + `send`;
  `secondary:true` (⌘↵) → `enqueue`. `render_composer` renders
  `field_input(input).appearance(false)`.
- `send`/`enqueue`/`clear_composer` take `Option<&mut Window>` (subscribe has
  a window, the test doesn't). Removed `on_composer_key` / `composer_focused`.
- Test `composer_clears_on_send_and_attachments_manage` updated
  (`composer_seed` + `send(None, cx)`).

**⌘↵ enqueue + QueueStrip**
- `AiChatStore.prompt_queue: Vec<String>` + `enqueue_prompt` (sends
  immediately if idle, else queues), `dequeue_prompt`, `queued_prompts`.
  The `spawn_stream` completion handler drains one queued prompt when the run
  is done and no tool approvals pend. QueueStrip rendered above the composer
  (each row = `↳` + text + `x`).

**TodoStrip** — `AiChatStore::active_todos()` reads the `TodoStore` for the
active session; rendered above the composer as `TODO n/total` + a checklist
(✓ / ▸ / ○).

**Connect banner** — `AiChatStore::needs_connection()` (`resolve_target`
fails) → red "No model connected — add a key in Settings → AI" strip.

**Voice** — visible inert "voice (soon)" stub in the composer footer with a
`TODO(T16-019)` comment.

**Inline agent / directive editors** — `settings.rs` `AiEditor` keydown-buffer
modal (3 fields: Name/Description/Instructions or Handle/Name/Content; Tab
cycles, Enter saves, Shift+Enter newline in the last field, Esc cancels).
"Edit" button per custom agent + every directive; persists via the backend
stores.

**NOT done in polish A** (genuinely large, precise pointers below):
- Slash-commands (`/init`, `/plan`, …) with an autocomplete popover — needs a
  command registry + a popover anchored to the composer + prefix parsing on
  every keystroke (the real `InputState` doesn't surface per-keystroke text
  to the view without another subscription).
- `#`-directive inline expansion — parse `#handle` tokens in the outgoing
  body against `directives::load()` and splice `content` in (do this in
  `AiChatView::send` before `compose_message`).
- `@`-file picker — fuzzy over the live-bridge cwd; same popover machinery as
  slash-commands.
- PlanModeStrip / PlanDiffReview — the store has no plan-mode concept
  (`reference-src/src/modules/ai/**` `usePlanMode`); needs a store flag +
  a diff-review surface.
- ContextPillsRow — `split_context_blocks` already exists (`ai_chat.rs`
  tests); render the extracted chips as a row under a user message.
- `CommandSnippet` rendering — detect fenced ```bash blocks in assistant
  messages and render a run button (reference `components/ai-elements/
  CommandSnippet.tsx`).
- AI⇄Shell toggle — needs an `AiChatView` → workspace event to write the
  composer text to the active terminal (ai_chat has no workspace handle).

### Verified
`cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (695 tests).

### Next
Polish B — command-palette visuals (640px / 40px rows / accent bar / recents
/ Kbd chips / footer / section headings / empty state).

---

## Session: 2026-09-02 (Block F Commit 5 — palette fill-ins + cross-report cleanups)

### What Was Done (commit after `74cb4ea`)

- **Command-palette sub-pages completed**:
  - `ai-sessions` → `PaletteData.ai_sessions` from
    `AiChatView::session_choices`; `RowKey::SwitchAiSession` →
    `PaletteEvent::SwitchAiSession` → `AiChatView::switch_to_session` +
    reveal the AI dock.
  - `git-branches` → `PaletteData.git_branches` from
    `GitPanelView::branch_choices` (`(name, is_current, is_remote)`);
    `RowKey::SwitchBranch` → `PaletteEvent::SwitchBranch` →
    `GitPanelView::checkout`.
- **`ShortcutsOpen` binding reconciled** `cmd-k` → `cmd-shift-/` (`⌘?`),
  matching `reference-src/src/modules/shortcuts/shortcuts.ts` (`shortcuts.open`
  = `["⌘","?"]`). Removed the stale "cmd-k is deliberate" comment.

**Palette `outline` (Go to Symbol) page** — still empty: the editor
(`editor.rs`) has no symbol-outline extraction yet; wiring it needs a
TreeSitter document-symbol pass. Documented, deferred.

### Verified
`cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (695 tests).

---

## CONSOLIDATED — remaining deferred items (after the Final-5% round, 2026-09-03)

**AI composer power-user affordances — ALL DONE this round:**
- ~~Slash-commands (`/init` `/plan`) + popover~~ — DONE `d248ae6`.
- ~~`#`-directive inline expansion~~ — DONE (`expand_directive_tokens`).
- ~~`@`-file picker~~ — DONE `d248ae6` (`fs_search` over the workspace cwd;
  falls back to the process cwd because no real `LiveBridge` is wired — see
  below).
- ~~PlanModeStrip / PlanDiffReview~~ — DONE `d0c3cf1`.
- ~~ContextPillsRow~~ — satisfied by the existing removable `attachments`
  row on the composer (chips render + removing one strips it from the send).
- ~~`CommandSnippet` Run button~~ — DONE `f9fd757`.
- ~~AI⇄Shell toggle~~ — DONE `f9fd757` (`AiChatEvent::RunInTerminal`).
- ~~Palette `outline` / Go-to-Symbol~~ — DONE `64e44c4`.
- ~~"Open Diff (Split)"~~ — DONE `fd445c9`.
- ~~`tabsLocation` gating~~ — DONE `fd445c9`.
- ~~`previewUrl` statusbar detection~~ — DONE `fd445c9`.

**Done in the Final-3% round:** real `LiveBridge` wiring (`463a361`),
host-manager `btn` + breadcrumb menu unification (`946023f`), file-icon
coverage (`d33fcfe`), ModelPicker search/tabs/provider-rail (`20f77ee`).

**Still open (each with a concrete reason):**
- **Voice / whisper** — inert stub with a concrete TODO in `render_composer`
  (`crates/ui/src/ai_chat.rs`): needs a mic-capture path + a local whisper
  transcription backend + a user decision on which backend. No Rust crate
  wired.
- **File icons — colour** — the port's icons are monochrome (tinted by the
  theme); the reference's Catppuccin set is per-type coloured. ~18 distinct
  shapes now, but not coloured. Vendoring a coloured iconify set is a
  separate, large asset task.
- **Tabs-panel fallback** — if `tabsLocation` flips away from `"sidebar"`
  while the Tabs panel is active, the panel body still shows Tabs until the
  user switches panels (the toggle disappears correctly). Minor edge.
- **LiveBridge terminal-context cost** — `sync_live_bridge` re-reads the last
  200 terminal lines every render; fine in practice but could be throttled /
  made lazy if it ever shows up in a profile.
- Command-palette hover-to-select for theme preview (keyboard nav only).

**Overall parity vs the reference:** ~98–99%. Every core workflow, all
Block-F subsystems, the AI-composer power-user affordances, plan mode, palette
Go-to-Symbol, previewUrl / tabsLocation / split-diff, the real LiveBridge, and
the full ModelPicker all work. The residual is voice input (needs a backend +
a decision) and coloured file icons (asset task).

---

## Session: 2026-09-02 (Block F Commit 4 — T16-019 AI panel + agents/directives backend)

### What Was Done (commit after `55f703f`)

**Agents / Directives backend (deferred from Block C — DONE)**
- New `crates/backend/src/modules/agents/` — `Agent` model, 5 built-in agents
  (`builtin:coder|architect|reviewer|security|designer`), `load`/`save` to
  `labonair-agents.json`, pure `upsert`/`remove`, `new_agent_id`. 4 tests.
- New `crates/backend/src/modules/directives/` — `Directive` model (`#handle`
  + content), `load`/`save` to `labonair-directives.json`, `normalize_handle`
  (port of `normalizeHandle`), pure `upsert`/`remove`. 2 tests.

**AI panel (`ai_chat.rs`) — partial decomposition**
- **AgentSwitcher**: new header dropdown (`agent_menu`) listing all agents;
  `AiChatView` loads agents on `new`, `set_agent` persists + pushes the
  agent's instructions into `AiChatStore::agent_instructions`. `spawn_stream`
  prepends a `ChatMessage::system(instructions)` to every turn's history
  (unless one is already present).
- **ModelPicker is now a dropdown** (`model_menu` + `render_model_menu` over
  `MODELS`) — replaced the click-to-cycle `cycle_model`.
- **Expandable `ToolCallChip`**: `expanded_tools` set; collapsed row = tool
  icon (`tool_icon`) + name + one-line `tool_summary` (path/command/query
  from the args JSON) + status + chevron; click expands to full args +
  result (4000-char cap, was a flat 600-char truncation).
- **Settings → AI**: the "Agents & Directives" placeholder is replaced with
  real **Agents** (list, Set active, Delete custom, New Agent, open config
  folder) and **Directives** (list, Delete, New Directive) sections backed
  by the new stores.
- Tests: `model_picker_sets_ref` (replaces `cycle_model_changes_ref`).

**NOT done in T16-019** (documented for a follow-up — none block use):
- Real `text_field` composer — the swap is invasive (window threading
  through `new`/`send`/subscribe + the no-window test helper `make_view`);
  the keydown-buffer composer still works. This is the one unmet "Verify"
  bullet.
- slash-commands (`/init`, `/plan`, …), `#`-directive inline expansion in
  the composer, `@` file picker, QueueStrip, TodoStrip (a `TodoStore`
  exists in `labonair_ai` but is unrendered), PlanModeStrip / PlanDiffReview,
  ContextPillsRow, AI⇄Shell toggle, ⌘↵ enqueue, connect banner,
  `CommandSnippet` rendering, voice/whisper stub, inline agent/directive
  editors (currently edit the JSON files directly).

### Verified
`cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (695 tests).

### Next
Block F Commit 5 — palette ai-sessions/git-branches/outline pages; icon
audit; `cmd-k` vs reference; remaining `vergleichsbericht-*` "not done" items.

---

## Session: 2026-09-02 (Block F Commit 3 — T16-018 theme marketplace rest)

### What Was Done (commit after `66c718a`)

The backend already had `theme_fetch_index` / `theme_download` / `theme_create`
/ `theme_delete` / `themes_get_all` (`crates/backend/src/modules/themes/`) —
this commit is the **UI wiring**.

- **`settings.rs` Themes pane**: new **Installed / Community tabs**
  (`themes_community_tab`). Community tab lists `RemoteTheme` entries fetched
  from `COMMUNITY_INDEX_URL` (`Snenjih/labonair-themes/main/index.json`), with
  a `mock_community_themes()` fallback + error banner on fetch failure.
  Per-entry **Install** (`theme_download` → `refresh_themes`, `installing_themes`
  spinner) / **Uninstall** (reuses `delete_theme` — resets `appTheme` if
  active). New **"New Theme…"** button → keydown-buffer name prompt →
  `theme_create` → activate.
- **Palette hover-preview** (Block D leftover): `ThemeStore` gains a transient
  `preview: Option<Theme>` (overrides `theme()`, never persisted) +
  `preview_theme_file` / `cancel_preview` (cleared by `reresolve_custom`).
  `command_palette.rs`: new `PaletteEvent::PreviewAppTheme(Option<String>)`
  emitted by `sync_theme_preview` on every selection/navigation change while
  on `Page::Themes` (and a revert `None` on leave / close / any other page).
  `app_shell` drain → new `settings::preview_app_theme` (resolves the file +
  persisted variant, calls `ThemeStore::preview_theme_file` / `cancel_preview`).
  Unit test `hover_preview_overrides_then_reverts`.

**Not done in T16-018**: `themes_get_dir` "open folder" is the existing
"Open themes folder" button (fine); palette theme rows don't preview on
mouse-hover (no hover-to-select in the palette — keyboard nav only).

### Verified
`cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (689 tests).

### Next
Block F Commit 4 — T16-019 AI panel decomposition + agents/directives backend.

---

## Session: 2026-09-02 (Block F Commit 2 — T16-016 dual-dock dynamic sidebars)

### What Was Done (commit after `8d12e1f`)

**T16-016 — dual-dock dynamic sidebars**

- **New pure module `crates/ui/src/sidebar_slot.rs`** — Rust port of
  `sidebarSlotLogic.ts` + its test file: `is_collapsed`, `resolve_toggle`
  (Expand / Collapse / Switch), `resolve_resize`, and a `SidebarSlot`
  struct (`open` / `width` / `panel` / `last_open_width`) with `toggle`.
  8 unit tests (all reference cases ported).
- **`app_shell.rs` refactor**: the single `sidebar_open/sidebar_side/
  sidebar_width/active_panel` quartet → **two independent `SidebarSlot`s**
  (`left_slot`, `right_slot`) that can both be open at once (Explorer left +
  AI right, etc. — previously AI toggling closed Explorer).
  - `slot`/`slot_mut(side)`, `primary_side` (from `sidebarPosition` pref),
    `side_for_panel` (per-item `bar_item_placements` side → global → AI=Right).
  - `select_panel` / `select_panel_on_side` route through `SidebarSlot::toggle`.
  - **`open_panel`** (show, never toggle-closed) — palette `OpenSnippetsPanel`
    / `FocusSourceControl` / new-AI-session now use it instead of the toggle.
  - **`move_panel(panel, to)`** — collapses the panel in its old slot, opens
    it in the new one, and persists the bar-item side. Exposed via a `←/→`
    affordance in each sidebar's header.
  - **Debounced persistence** (`persist_sidebar`, 300ms throttle) writes all
    six `sidebar*` / `sidebarRight*` pref keys; **restore-from-prefs** in
    `AppShell::new`.
  - `SidebarResize(BarSide)` payload → the resize handler resizes the right
    slot (`set_slot_width`).
- **`SidebarPanel`**: removed vestigial `GitGraph` (it's a `TabKind` from
  Block B — the shell no longer owns a `GitGraphView`; the workspace's tab
  view is now the shared entity via new `Workspace::set_git_graph`, which
  also finally wires the CWD feed to the Git Graph tab). Added **`Tabs`** and
  **`Hosts`** panels + `render_tabs_panel` (clickable open-tab list →
  `reveal_tab`) / `render_hosts_panel` (known-host list → `open_ssh_tab`).
  `slug`/`from_slug` for persistence.

**Not done in T16-016**: the bar-item context menu's Left/Right radios still
only move the *item* placement (next toggle picks up the new dock) — they
don't relocate an already-open panel live (the header `←/→` affordance and
`move_panel` do). Tabs-in-sidebar `tabsLocation` gating + "leaving sidebar →
fall back to explorer" rule not implemented (the Tabs panel is always
available as a dock panel).

### Verified
`cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (688 tests).

### Next
Block F Commit 3 — T16-018 theme marketplace (fetch index / download /
install / uninstall / theme_create + Community tab + palette hover-preview).

---

## Session: 2026-09-02 (Block F Commit 1 — T16-017 shared ContextMenu + all view menus)

### What Was Done (commit after `c366f7c`)

**T16-017 — shared `ContextMenu` primitive + migrate/add every view menu**

- **New `crates/ui/src/components/context_menu.rs`**: `MenuItem` builder
  (`new`/`separator`/`label`/`submenu`, `.icon`/`.destructive`/`.disabled`/
  `.checked`/`.on_click`) + `context_menu(anchor, &ThemeStore, dismiss, items)`
  → full-screen backdrop (left+right click dismiss) + radix-styled card
  (`popover` bg, `border`, `p-1`, `shadow_lg`, `min-w 160`, `rounded_sm`
  items `px-2 py-1.5 text-13`, hover `accent`, destructive = `destructive`
  colour, disabled dims + no handler, one-level hover sub-menus via
  `group_hover`). Exported `MenuClick` alias for call-site helper sigs.
  Unit test `builder_sets_item_flags`.
- Handlers run with `&mut App`; call sites capture `cx.entity()` and
  `v.update(cx, |this, cx| …)`.
- **Migrated (were hand-rolled `div` menus):** workspace tab menu +
  new-tab "+" menu (`workspace.rs`), explorer file-tree menu (`explorer.rs`
  — also fixed the fixed `top:26/left:10` anchor → real cursor position),
  SFTP file-list menu (`sftp.rs` — `Menu` now carries `pos`).
- **New menus added (reference-parity item sets):**
  - Workspace tab: **Rename** (new inline keydown-buffer editor, `rename_tab`
    field + `on_rename_key`, commits on Enter / tab-switch), Duplicate,
    Keep Tab Open, Close / Close Others / Close All / Close All ⟨kind⟩,
    Grant AI Agent Access (checked).
  - Explorer node: added Open, Reveal in Finder, Copy Relative Path (kept
    Copy/Cut/Paste). Destructive Delete.
  - SFTP list: added **Open**, transfer labels aligned ("Download to…/Upload
    to Remote…"), two-click destructive Delete preserved.
  - **Terminal pane** (`terminal.rs`, new): honours `terminalRightClickPastes`
    pref — paste-on-right-click when set, else a menu **Copy** (disabled w/o
    selection) / **Paste** / **Clear** (`^L`). Covers the SSH-terminal case
    (unified `TerminalView`). "Ask AI about Selection" deferred (needs the
    `AskAboutSelection` action wired end-to-end — see below).
  - **Snippet item** (`snippets.rs`, new): Run in Terminal / Run Silent /
    Run (Inject) / Copy Command / Edit / Duplicate / Delete (destructive) —
    `menu` field + right-click on the row.
  - **Git-graph commit** (`git_graph.rs`, new): View Changes, Checkout
    (detached), **Create Branch Here…** (new modal keydown prompt →
    `git_create_branch`), Cherry-pick, Copy Hash / Copy Short Hash.
    New `run_git_op` helper.
  - **Source-control file change** (`git.rs`, new): Stage/Unstage (per
    section), Discard Changes (destructive), Add to .gitignore, Add to
    .git/info/exclude (new `add_to_gitignore`/`add_to_exclude` methods),
    Open Diff.
  - **Host card / list item** (`hosts.rs`, new — one menu for list+grid):
    Connect SSH, Open SFTP, Edit, Duplicate, Export to SSH Config (new
    `export_host` → clipboard), Delete (destructive).
  - **Host group chip** (`hosts.rs`, new): **Rename Group** (new modal
    keydown prompt → `groups_update`), Delete Group (destructive).

**Still not done in T16-017** (documented, low value / cross-cut):
- Tab-bar empty-area menu (#3), CWD-breadcrumb menu (#15), bar-item /
  Appearance-preview menu (#16/#17 — ties into T16-016 dual-dock; do there).
- "Ask AI about Selection" terminal item + the `menu::AskAboutSelection`
  action are still unwired end-to-end (`attach_selection` only called by
  tests). Belongs in Commit 5.
- "Open Diff (Split)" — the git panel has only an inline unified diff; no
  split/editor-tab diff exists.

### Verified
`cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (680 tests).

### Next
Block F Commit 2 — T16-016 dual-dock dynamic sidebars.

---

## Session: 2026-09-02 (Block F — partial: SFTP connecting screen, theme variants, palette fill-ins)

### What Was Done (branch `master`, commit after `4976de9`)

Block F is large (four T16-016..019 subsystems + cross-block cleanups). This
session delivered the tractable, verifiable slices; the big architectural
pieces are itemised under "Block F — still outstanding" below.

**SFTP connecting screen (T16-015 follow-up — DONE)**
- `open_sftp` (`workspace.rs`) now calls `ssh_connection.begin(.., ConnectionKind::Sftp, None)`
  so all SSH bus events for the SFTP session id (`ssh_connect_log`,
  known-hosts, auth, passphrase, session-established) flow into the shared
  `ConnectionStatusStore` and drive the same loading UI terminal tabs use.
- New `SftpEvent::ConnResult { session_id, error }` emitted by
  `SftpView::connect`; `Workspace::on_sftp_event` maps it to
  `set_state(Connected)` / `set_error` (only if never connected).
- `render_body` for `TabKind::Sftp` shows `render_ssh_loading` while the
  entry `is_blocking()`. `render_ssh_loading` resolves `tab_id` from
  `sftp_sessions` too; the error-screen **Retry** button re-runs
  `SftpView::reconnect` for SFTP sessions (vs `retry_ssh` for terminals).
- Tab close removes the SFTP session from `ssh_connection`.

**Theme variant selection (T16-018 core — DONE)**
- `crates/theme/import.rs`: `ThemeFile::resolve_variant(dark, Option<&str>)`
  now honours a named variant key; new `variant_choices(dark)` +
  `Theme::from_theme_file_variant(file, dark, key)`. Unit test
  `named_variant_selection_picks_the_requested_scheme` (Catppuccin latte/
  frappe/macchiato/mocha).
- `ThemeStore`: `custom_variant: Option<String>` field, `set_custom_variant`,
  `custom_theme_file()`, `custom_variant_key()`, `import_theme_file_variant`.
  `reresolve_custom` uses the selected variant.
- `settings.rs`: `render_variant_picker` segmented control in the Themes pane
  (shown when the active imported theme has >1 variant for the current mode);
  `set_theme_variant` persists `themeVariantOverrides[id][mode] = key` and
  applies live. `apply_stored_variant` re-applies on activation.
- **App-theme now persists**: `activate_theme` / `import_theme_from` /
  `delete_theme` write the `appTheme` pref; `apply_prefs_to_theme` restores
  the JSON theme **and** its stored variant on startup, and clears the custom
  theme when `appTheme == "default"`. (Block C had left `appTheme` unwired.)

**Command-palette fill-ins (partial — DONE: themes + snippets)**
- `PaletteData.app_themes` populated from `settings::theme_choices()` with an
  `active` flag; `Page::Themes` rows → `RowKey::SetAppTheme` →
  `PaletteEvent::SetAppTheme` → `settings::activate_app_theme`.
- `PaletteData.snippets` populated from `SnippetsView::snippet_choices()`;
  `Page::Snippets` rows → `RowKey::RunSnippet` → `PaletteEvent::RunSnippet` →
  `SnippetsView::run_by_id` (default exec mode, full variable/host-picker
  flow reused).
- ai-sessions / git-branches / outline palette pages: still empty (need
  view-internal accessors on `AiChatView` / `git.rs` / editor outline).

### Verified
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` — all green (679 tests, 0 failures).

### Block F — still outstanding (attempt in the next session)

**T16-016 — Dual-dock dynamic sidebars** — NOT STARTED. Needs: refactor
`AppShell` sidebar into a reusable `SidebarSlot` × 2 (primary/secondary),
derive left/right from the existing `sidebar_position` pref, persist
`open`/`active_panel`/`width` per slot (pref keys `sidebar*` /
`sidebarRight*` + `barItemPlacements` all already exist in `Preferences`),
port `sidebarSlotLogic.ts` as a pure unit-tested module, add
`move_panel(from,to)` + a rail context-menu affordance, split
`open_panel` (show) from `toggle_panel`, add `Tabs` + `Hosts` panels, drop
`SidebarPanel::{GitGraph, Ai}` (GitGraph is a `TabKind`, AI is its own dock).
Ref: `reference-src/src/modules/statusbar/lib/{useSidebar,sidebarSlotLogic}.ts`,
`app/components/SidebarContent.tsx`.

**T16-017 — Context menus** — NOT STARTED. Build ONE shared `ContextMenu`
primitive in `crates/ui/src/components/` (cursor-anchored, backdrop dismiss,
separators, destructive styling, disabled items, submenus, radio/checkbox).
Migrate the 3 ad-hoc menus (`workspace.rs` tabs, `explorer.rs` tree,
`sftp.rs` list) onto it, then add the 11 missing menus from
`vergleichsbericht-subagent-3.md` §5 gap table (terminal pane, SSH terminal,
source-control file change, git-graph commit, host card, host list item,
host group, snippet item, appearance/bar-item preview, CWD breadcrumb,
tab-bar empty area) + fill the item gaps in the existing ones.

**T16-018 — Theme marketplace** — variant selection DONE (above); STILL
MISSING: `theme_fetch_index` / `theme_download` backend (reqwest) +
`MOCK_COMMUNITY_THEMES` + `Snenjih/labonair-themes` index URL; install/
uninstall/`theme_create` ("New Theme"); a "Community" tab in the Settings
Themes pane; palette theme **hover live-preview** + revert-on-cancel
(`on_preview`/`on_leave` — the palette row model has no preview hook yet).
Ref: `reference-src/src-tauri/src/modules/themes/**`,
`reference-src/src/settings/sections/ThemeMarketplace.tsx`.

**T16-019 — AI panel decomposition** — NOT STARTED. Break `ai_chat.rs`
(1.7k lines) into the reference sub-components: real ModelPicker dropdown
(not click-cycle), AgentSwitcher, slash-commands, `#` directives, `@` file
picker, QueueStrip, TodoStrip, PlanModeStrip/PlanDiffReview, attach button,
ContextPillsRow, AI⇄Shell toggle, ⌘↵ enqueue, connect banner,
`CommandSnippet` rendering, expandable `ToolCallChip`, real `text_field`
composer. Plus the deferred `crates/backend/src/modules/{agents,directives}`
stores + Settings **Agents** / **Directives** sections + voice/whisper stub.
Ref: `reference-src/src/modules/ai/**`, `components/ai-elements/**`.

**Cross-block deferred (from all 4 reports) — still open:**
- Icon system: no real icon crate for file/folder icons — `explorer.rs` /
  `sftp.rs` still use emoji; Catppuccin file-icon resolver not ported
  (report 3 §2). `components::icon` (Lucide) exists but explorer/sftp/tabs
  not migrated.
- Command-palette view metrics still ~520px / 26px rows (report 3 §1c.1),
  no footer search-mode toggle, no fuzzy/startsWith, no recents UI, no
  palette preferences pane, no icons/subtitles/rightLabel chips in rows.
- `ShortcutsOpen` bound to `cmd-k` vs reference `cmd-?` (report 3 §1b) —
  unreconciled.
- `hosts.rs` checkbox glyphs `☑/☐` + `⚠` severity glyphs not real controls.
- Host manager `btn` helper not migrated to `components::button`.
- Host list auto-refresh on window focus (only 30s poll).



### What Was Done (branch `master`, commit after `c58652e`)

**T16-015 — SSH connecting state machine + screen (new)**
- `crates/ui/src/ssh_connection.rs` (new): `ConnectionStatusStore` GPUI entity
  (port of `connectionStatusStore.ts`). `ConnectionState` = `Idle |
  QuickConnectPassword | Connecting | WaitingTrust | WaitingAuth |
  WaitingPassphrase | Connected | Error`. `ConnectionEntry` carries state,
  error, `prompt_message`/`is_2fa`, trust fingerprint/mismatch,
  `jump_host_name`, `kind` (Terminal/Sftp), 4-stage progress (`stage` +
  `stage_done`) and the live log `Vec<String>`. `detect_stage()` maps
  `log_step!` lines from `ssh::client` → `(stage_idx, done)`. Full unit tests
  (stage-advance monotonic + whole state machine).
- `crates/backend/src/events.rs`: new typed `AppEvent::SshConnectLog {
  session_id, message }` mapped to the existing `"ssh_connect_log"` bus event
  (backend already emitted it; it was just never typed).
- `crates/ui/src/workspace.rs`:
  - `ssh_connection: Entity<ConnectionStatusStore>` field; `connect_host`
    calls `begin(...)` and no longer writes `"Connecting…"` into the PTY feed.
  - `handle_ssh_event` feeds every SSH event into the store
    (`SshConnectLog` → `push_log`, `KnownHostsWarning` → `set_trust`,
    `AuthRequired` → `set_auth_prompt`, `PassphraseRequired` →
    `set_passphrase`, `SessionEstablished` → `Connected`, `ConnectionLost` →
    `set_error` only if not yet connected). `spawn_ssh_connect`'s failure path
    now `set_error`s instead of red ANSI in the terminal. `submit_prompt` /
    `cancel_prompt` / `retry_ssh` drive `resume` / `set_error`.
  - New `render_ssh_loading(entry, cx)` full-pane view: host label + jump-host
    badge, 4-stage progress row (TCP → Handshake → Auth → Shell/SFTP), a
    state card (trust / auth / passphrase / error with Retry + Edit Host +
    Close, or a plain connecting line with Cancel), and the live connection
    log panel. Rendered in the `TabKind::Workspace` tab body whenever the
    tab's session is in a blocking state; the old modal `render_ssh_prompt`
    + its overlay are removed. `SshPrompt` slimmed to just `ssh_id` (+ buffer)
    since the store now holds the display data. `pending_tab_close` queue
    added for the error screen's Close button.
  - Known gap: the **SFTP** connect path (`open_sftp`) does not yet populate
    the store, so SFTP tabs don't show the loading screen (the store + stage
    labels already support `ConnectionKind::Sftp`).

**T16-014 — Host Manager master/detail rebuild** (`crates/ui/src/hosts.rs`)
- Layout: centred modal → **persistent 340px left side panel + detail pane**.
  Side panel: search box w/ `user@host[:port]` quick-connect parse + suggestion
  card, actions toolbar (New Host / New Group / Sort cycle / Grid-List toggle /
  Credentials / Import / Export), group filter chips (with per-group delete +
  drop-target), active-tunnels panel, and the host list (list or grid).
- Host list items: host icon, pin marker, ping/reachability dot (30s TCP-probe
  worker `_ping_task` + `refresh_ping`), live connection status label, click =
  select into detail, **drag-and-drop** to reorder (`hosts_reorder`) or drop
  onto a group chip to re-group (`move_host_to_group`). `DraggedHost` payload +
  `HostDragGhost`.
- Detail pane = `render_detail` + `render_detail_tab`: header (icon picker
  toggle, eyebrow, save-status indicator, Connect / SFTP / **Test Connection**
  (`ssh_test_connection`) / Duplicate / Delete, close) + **4 tabs
  General / SSH / SFTP / Tunnels**.
  - General: name/address/port/username, auth (Password/Key/**Credential**/None),
    key path / password, credential + group + jump-host cyclers, pin-to-top,
    notes.
  - SSH: start directory, sudo password, keep-alive interval/tries, **startup
    snippet + Execute/Inject mode** (snippet list loaded from
    `snippets::db::snippets_get_all`), Block AI Agent Access.
  - SFTP: SFTP start directory. Tunnels: existing tunnels editor.
  - **Icon picker** row (curated `HOST_ICONS` set of `IconName`s).
- **Debounced autosave** (1s, `schedule_autosave` + `edit_gen`) for existing
  hosts with a `SaveStatusIcon`-style indicator; new host keeps an explicit
  "Add Host" button. `reload_list_only` refreshes rows without disturbing the
  open form.
- Backend `startup_snippet_id`/`startup_snippet_mode`/`icon` now wired through
  `submit_form`. Stray "Tags" field already removed in the prior commit.
- New tests: `quick_connect_target` parse, `visible_hosts` filter+sort,
  icon/snippet/tab mapping.

### Verified
`cargo fmt --all --check`, `cargo check`/`clippy --workspace --all-targets
-D warnings`, `cargo test --workspace` — all green (678 tests, 0 failures).

### Deferred / next
- SFTP loading screen wiring (store supports it; `open_sftp` needs the
  `begin`/event calls).
- `components::button` migration of the host manager's `btn` helper (kept the
  existing pill-radius shim for churn-safety).
- Auto-refresh on window focus for the host list (30s poll is in place).
- Next: Block F (Sidebars / CtxMenus / Theme / AI).

---

## Session: 2026-09-02 (Block E — Host Manager & SSH, first pass)

### What Was Done (branch `master`, commit after `3f7beab`)

- **Command palette host wiring (fully done)**:
  - `command_palette.rs`: new `PaletteEvent::ConnectHost { host_id, sftp }` +
    `RowKey::ConnectHost`. `choice_rows` gained a `key_for: fn(&PaletteChoice)
    -> RowKey` param; `HostsSsh`/`HostsSftp` pages now produce actionable rows
    (`sftp` false/true), all other choice pages pass `|_| RowKey::Noop`
    (unchanged behavior). `run_selected` emits the new event.
  - `app_shell.rs`: `build_palette_data` now populates `PaletteData.hosts`
    from `workspace.known_hosts(cx)`. `drain_pending_commands` handles
    `ConnectHost` → `workspace.open_ssh_tab` / `open_sftp_tab`.
  - Net effect: `Connect SSH…` / `Open SFTP…` palette sub-pages list real
    hosts and open the tab on Enter/click.

- **Host form correctness fixes (T16-014, partial — still the single modal, NOT
  yet master/detail)** in `crates/ui/src/hosts.rs`:
  - Auth taxonomy fixed: `AuthMethod::Agent` → `AuthMethod::Credential`,
    backend string `"agent"` → `"credential"` (which `ssh::client` /
    `config_parser` actually special-case — the old `"agent"` string was a
    dead value). `from_str` still accepts legacy `"agent"`.
  - Credential picker is now gated on `auth == Credential` (was always shown).
  - Removed the stray "Tags (comma separated)" field (not in the reference
    form); `submit_form` now sends `None` for tags. `tags` column untouched.
  - Added + persisted 6 previously-missing fields (backend model + columns
    already existed): `sudo_password` (keychain, "(set)" placeholder, Password
    auth only), `default_path_sftp`, `keep_alive_interval`, `keep_alive_tries`,
    `notes`, `pin_to_top` (toggle). Wired through both `hosts_create` and
    `hosts_update` arms of `submit_form`.
  - New test `host_form_prefills_and_serializes_the_block_e_fields`.

### Verified
`cargo fmt --all --check`, `cargo check/clippy --workspace --all-targets
-D warnings`, `cargo test --workspace` — all green (16 suites, 0 failures).

### NOT done — remaining Block E (hand off to next session)
- **T16-014 master/detail rebuild**: left ~340px host list pane + persistent
  detail pane (not modal); 4-tab form (General/SSH/SFTP/Tunnels); icon picker
  (`icon` + `startup_snippet_id`/`mode` fields still unwired); autosave +
  save-status indicator; **Test Connection** button; search box + quick-connect
  (`user@host:port` parse); grid/list toggle; sort control; group filter chips;
  per-host ping/reachability; drag-and-drop reorder + into groups.
- **T16-015 (entirely untouched)**: per-session `ConnectionStatus` store;
  full-pane `SshLoadingScreen` (5-state machine + 4-stage progress + live
  connection-log); structured error screen w/ retry; wire russh flow in
  `crates/backend/src/modules/ssh` to emit status transitions; replace
  `workspace.rs::render_ssh_prompt` + the `"Connecting…"` PTY write.
- Next task id: continue Block E (T16-014 master/detail + T16-015), then Block F.

---

## Session: 2026-09-02 (Block D — Command Palette rebuild, T16-013)

### What Was Done (commit after `16495bd`, branch `master`)

- **`crates/ui/src/command_palette.rs`** — full view rebuild + data-layer growth:
  - **Layout parity**: 640px card, 40px min rows, left accent bar on the active
    row (`border_l_2` + `primary`), 28px icon chip (`IconName` on every row),
    title + subtitle, right-aligned `<Kbd>` chips + `rightLabel` (ON/OFF/active,
    success-tinted), section headings, "Recently Used" group, footer bar
    (search-mode toggle + result count + `↑↓/↵/⌫/Esc` hints), breadcrumb with
    clickable segments when inside a sub-page, "No results found." empty state.
  - **Fuzzy search**: new `SearchMode {Contains,StartsWith,Fuzzy}` + `match_score`
    (ranked — consecutive/prefix bonuses, gap penalty). Reads
    `commandPaletteSearchMode`; footer chip cycles + persists it. `search_mode()`
    ranks; old `search()` kept as a Contains shim for existing tests.
  - **Sub-pages**: `Page` enum 2 → 12 (`Root` + 11 named: Zoom, Tabs, ColorMode,
    EditorTheme, Themes, HostsSsh, HostsSftp, Snippets, AiSessions, Outline,
    GitBranches). Multi-level page stack (`Vec<Page>`), push/pop, Esc clears
    query → pops → closes, Backspace-on-empty pops.
  - **Dynamic sources**: `PaletteData` snapshot (`AppShell::build_palette_data`,
    synced each render via `CommandPalette::set_data`). Wired now: open tabs,
    color mode, editor themes, terminal font size, ~9 settings toggles (live
    `rightLabel`). Hosts / snippets / AI sessions / git branches / editor outline
    pages render a clean empty state — `PaletteData` has the fields ready for
    Blocks E/F to populate.
  - **Static registry** grew ~33 → ~50 commands (icons on all; nav rows for
    Adjust Font Size, Connect SSH, Open SFTP, Change App/Color/Editor Theme,
    Switch AI Session, Run Snippet, Git Switch Branch, Go to Symbol; toggle
    commands for word-wrap / line-numbers / format-on-save / cursor-blink /
    pane header+footer / vim mode; Manage AI Keys & Models).
  - **Palette prefs honored**: position (top/high/center), opacity (card alpha),
    show-recent, history-size (recent list cap), close-on-overlay-click.
  - **Recently Used** persisted to `command-palette-recent.json` in the config
    dir (`recent` submodule; debug-formatted `CommandId` slugs).
  - New `PaletteEvent::{SetColorMode,SetEditorTheme}`; `command_palette.rs` tests
    +8 (fuzzy modes, ranking, mode cycle, page metadata, toggle-key map,
    editor-theme label, distinct page labels).
- **`crates/ui/src/app_shell.rs`** — `CommandPalette::new` takes `Entity<PreferencesStore>`;
  `build_palette_data` + `set_data` sync in `render`; `drain_pending_commands`
  handles the 2 new events (persist pref + `apply_prefs_to_theme`);
  `run_palette_command` gains toggle arms (reusing `toggle_zen_pref`) + AI-settings
  dispatch + exhaustive no-op arms for the sub-page navigator ids.

### State / Verified
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo test --workspace` — all green.

### Not Done / Deferred (for Blocks E/F)
- **Live theme hover-preview** + populated **Themes** page — needs the multi-theme
  list / preview API that the theme audit (subagent-3 §Theme system) assigns to
  Block F.
- **Hosts / Snippets / AI-sessions / Git-branches / Outline** sub-page *data* —
  those stores aren't reachable from `AppShell` yet (Blocks E/F). Pages + the
  `PaletteData` fields + `PaletteChoice` shape are in place; wiring is a
  fill-in-the-vec once the entities exist.
- **Backdrop blur** (`commandPaletteBlur`) and **entrance animation**
  (`commandPaletteAnimation`) — GPUI has no `backdrop-filter` and no trivial
  fade/zoom entrance; approximated by the modal scrim. Revisit if GPUI gains it.
- **ConnectSsh / OpenSftp / RunSnippet / CheckoutBranch / GoToSymbol** execution
  paths (events not added yet — nav rows currently dead-end at the empty page).

### Next: Block E (Hosts / SSH)

---

## Prev Session: 2026-09-02 (Block C cont. 2/2 — dropdowns, special sections)

### What Was Done (commit after `47f6127`)

- **T16-010 finished.**
  - Real floating **`Select` dropdown** (`deferred` + `anchored().snap_to_window()`
    + occluding backdrop) replaces click-to-cycle — `SettingsView::dropdown:
    Option<SelectMenu>` (`crates/ui/src/settings.rs`).
  - **`FieldKind::Float`** (min/max/step in hundredths, `bump_float`) — rows
    `appLineHeight`, `editorLineHeight`, `terminalLineHeight`,
    `terminalLetterSpacing`.
  - **`FieldKind::FontFamily`** — a dropdown built from a scanned system-font
    list (`labonair_backend::modules::fonts::fonts_list_system`, loaded async
    into `SettingsView::system_fonts` on open) with a `(default)` sentinel that
    clears the pref; `appFontFamily` / `terminalFontFamily` / `editorFontFamily`
    now use it.
  - **Slider look**: bounded `Int`/`Float` steppers get a read-only filled
    `slider_track(fraction)` beneath them.
  - **Conditional rows** (`field_visible`): `sessionScrollbackLines` &c. only
    when `sessionRestore`, `terminalCursorBlinkInterval` only when blink,
    `editorAutoSaveDelay` only when auto-save ≠ off, ssh/explorer reconnect
    detail rows, bookmark action rows, autocomplete provider/model rows.
  - Nav = the 10 reference top-level entries with nested sub-section headers
    (`SECTION_GROUPS`); **Themes** top-level; **AI Agent Bridge** rendered
    inside **Connections**; **Command Palette + Source Control + Bookmarks**
    folded into **Workspace**; grouped global search; **About hero** in General.

- **T16-012.**
  - **Bar-item layout editor** (`render_layout_editor`) in the Appearance pane —
    every `BAR_ITEM_ORDER` item gets Titlebar/Status · L/R · Hidden controls +
    "Reset layout", built on `crates/ui/src/bar_items.rs`. Persists via the
    backend blob and bumps the new `bar_items::BarLayoutTick` global; `AppShell`
    `observe_global`s it and re-reads `Placements` live.
  - **Theme grid** (`render_themes`) — `ThemeCard`-style grid over the scanned
    themes (built-in + `~/.config/labonair/themes/*.json`) with Activate /
    Delete / Import / Export / Open-folder.
  - **AI Providers section** (`render_providers`) — functional list over
    `labonair_ai::InstanceStore` (add via a provider chip row, remove), API keys
    stored in the OS keychain via `labonair_ai::secret_store::{set,clear,get}_instance_key`
    (synthetic `provkey:<id>` editing key routed in `commit_edit`), never in
    prefs JSON. Shows the active model ref.
  - Background-image management already lived in `render_appearance` (grid +
    import + delete + opacity/blur/tint) — kept.

- **Side-effect propagation** in `set_pref`: documented that the
  `GlobalPreferences` republish (in `PreferencesStore::set_value`) is the port's
  generic `applySettingChange` (terminal/editor/workspace observe it); added
  explicit hooks for `defaultModelId` → `InstanceStore::set_active_model_ref`
  and `reduceMotion`/`appCornerRadius`/`appLineHeight` → theme re-sync.

### Known gaps / follow-ups

- **Agents / Directives editor** — not built: there is *no* backend agents/
  directives store in the port yet (reference `agentsStore`/`directivesStore`
  have no Rust counterpart). The AI pane shows a note. Needs new backend
  modules first.
- **Slider** is a stepper + visual track, not a drag handle (no cheap way to
  map a drag to a value without capturing element bounds).
- **FontPicker** is a plain dropdown, not the reference's searchable combo.
- Bar-item live refresh re-reads the whole blob on every `BarLayoutTick`; fine
  at edit frequency.

### Next: Block D (Command Palette).

---

## Prior Session: 2026-09-02 (Block C cont. 1/2 — separate window + section tree) → commit 47f6127

## Prior Session: 2026-09-02 (Block C — Settings: full preference model)

### What Was Done (commit after `634706f`)

- **T16-011 — preference model expansion + compat/default fixes (DONE).**
  `crates/backend/src/modules/settings/preferences.rs`: `Preferences` grew from
  46 fields to ~165, matching `reference-src/.../store.ts::DEFAULT_PREFERENCES`
  key-by-key (camelCase serde names; `lmstudioBaseURL`/`mlxBaseURL`/
  `openaiCompatibleBaseURL`/`ollamaBaseURL` carry explicit `#[serde(rename)]`
  for the capital `URL`). Added groups: Appearance & Layout (appTheme,
  themeVariantOverrides, background*, appCornerRadius, tabsLocation,
  sidebarTabInfoLine, sidebarGroup*, barItemPlacements, barLayoutMigrated,
  badgesAlwaysVisible, titlebarsIconsPosition), Status Bar toggles (7),
  Sidebar/HM state (sidebar*, hm*), Terminal (terminalDefaultPath,
  newTabInheritsCwd, confirmCloseTerminalTab, terminalFontWeight,
  terminalLetterSpacing, terminalLineHeight, terminalCursorBlinkInterval,
  terminalRightClickPastes, terminalWordSeparator, terminalScrollSensitivity,
  terminalFastScrollModifier, terminalShowPaneHeader/Footer, terminalUseWebGL,
  terminalComposer*/terminalBlocks*), Editor (editorLineHeight, editorAutoSave
  + delay, trim/insertFinalNewline, bracketMatching, showCursorPosition,
  showSelectionStats, showOutline, indentationGuides, autocompleteDebounceMs,
  maxFileSizeMb), File Manager (sftpShowUpFolder, explorerShowHiddenByDefault,
  sftpColumn*, sftpRemoteEditShowTransfers, sftpMaxRemoteFileSizeMb,
  sftpDefaultConflictResolution, sftpChunkSizeKb, sftpOnFolderFileError),
  Connections (hostPingInterval, ssh*, explorer*), Command Palette (blur,
  opacity, position, animation, historySize, closeOnOverlayClick), Bookmarks
  (7 keys), AI (temperature, autoOpenMiniOnSend, notifyOnHeadlessCommand,
  shellMax*, defaultModelId, customInstructions, autocomplete*, provider
  base-URL/model-id pairs), MCP mirror (mcpBridge*/mcpNotifyOnActivity).
  - **BUG FIX**: `vimMode` was serialized as `editorVimMode` — now
    `#[serde(rename = "vimMode")]`; the settings UI `FIELDS` key updated to match.
  - **Default fixes** (reference parity): sessionRestore→false,
    defaultStartupTab→host-manager (enum `#[default]` moved), notifyOnErrors→false,
    sessionScrollbackLines→1000, scrollbackMaxSizeMb→10, scrollbackRetentionDays→0,
    terminalFontSize→14, terminalCursorStyle→bar, terminalScrollback→5000,
    editorTabSize→2, editorTheme→"atomone", sftpMaxConcurrentTransfers→2,
    gitStatusPollIntervalMs→5000, aiMaxAgentSteps→24, aiTerminalContextLines→300,
    appFontFamily→`"Inter Variable", sans-serif`, terminal/editor font family→
    full `"JetBrains Mono", SFMono-Regular, Menlo, monospace` stack.
  - **PORT-ONLY fields kept + documented in doc-comments**: `terminal_opacity`,
    `editor_relative_line_numbers`, `editor_theme:"auto"` (Vim/GPUI extensions;
    `editor_relative_line_numbers` still feeds `editor_prefs()`).
  - New tests: `reference_settings_blob_roundtrips` (30+ reference keys),
    updated `enums_serialize_to_reference_token_strings` (asserts `vimMode`,
    no `editorVimMode`), updated default assertions. `cargo test --workspace` green.
  - Design decision: `bar_item_placements` / `theme_variant_overrides` kept as
    `BTreeMap<String, serde_json::Value>` for lossless roundtrip (typed access
    stays in `bar_items.rs`); string-valued selects modelled as `String` not new
    enums (matches existing `editor_theme`; keeps `set_value` validation lossless).
  - Editor dotted reference keys (`editor.fontFamily`, `editor.formatOnSave`,
    `editor.indentWithTabs`, `editor.showCursorPosition`, …) are stored as flat
    camelCase (`editorFontFamily`, …) — the port already nests everything under a
    `"preferences"` object so the flat reference file never roundtrips 1:1 anyway;
    only the explicitly-flagged `vimMode` rename was applied.

- **T16-010 (PARTIAL).** `crates/ui/src/settings.rs`: added `Connections` and
  `Bookmarks` top-level categories; `FIELDS` grew by ~80 rows covering the new
  model keys (Switch / Int-stepper / cycle-Select / Text — the existing modal's
  control set). `render_appearance` gained a "Layout" sub-section
  (tabsLocation, appCornerRadius, sidebarGroup*, badgesAlwaysVisible, zen flags).

### NOT Done (remaining Block C — needs its own session)

- **T16-009 — Settings as its own OS window.** Still a modal overlay in
  `AppShell`. Needs `cx.open_window` (pattern in `crates/app/src/main.rs`),
  `SettingsWindow` root wrapped in `gpui_component::Root`, shared entities,
  close=hide, reopen-focuses, `SettingsTab` deep-link enum + menu "Settings → AI".
- **T16-010 remainder** — real `Select` dropdown (still click-to-cycle), Slider,
  FontPicker, Float NumInput, conditional rows, About hero, nested Section→Group
  tree matching `reference-src/src/settings/sections/*`, move AI Agent Bridge
  under Connections, merge Command Palette + Source Control under "Workspace",
  add a "Themes" top-level entry, `SETTING_DEFINITIONS`-style registry + grouped
  global search.
- **T16-012 — special sections** — Theme grid (ThemeCard/ThemeThumbnail),
  background image mgmt polish, bar-item layout editor (reuse `bar_items.rs`),
  Providers/Agents/Directives (need new backend modules + keychain wiring).
- **Side-effect propagation** (`set_pref`) for the new keys (reference
  `applySettingChange`) — currently only `theme`/`keybinds`/typography propagate.

### Next: finish T16-009 → T16-010 → T16-012, then Block D (Command Palette).

---

## Prior Session: 2026-09-02 (Block B cont. — unibar model, bar-item ctx menu, breadcrumb)

### What Was Done (commit after `b463874`)

- **T16-005 — unibar model (DONE).**
  - `crates/backend/src/modules/settings/mod.rs`: added `bar_item_placements_load[_from]`
    (reads the `barItemPlacements` blob) + `set_bar_item_placement_in(dir,…)` as the
    testable sync core of the existing async `settings_set_bar_item_placement`.
    New backend test `bar_item_placement_round_trips_and_merges`.
  - New `crates/ui/src/bar_items.rs` — pure model: `BarItemId` (15 variants, serde
    strings 1:1 with `barItems.ts`), `BarCategory`, `BarLoc {Titlebar,Statusbar}`,
    `BarSide {Left,Right}`, `BarItemPlacement {bar,side,hidden}`, `BAR_ITEM_ORDER`,
    `default_placement` (= `DEFAULT_BAR_ITEM_PLACEMENTS`), `Placements`
    (`from_blob` merge / `visible_items_for` / `panel_dock_side`), `placement_patch`,
    `divider_indices` (the `withDividers` rule). 5 unit tests.
  - `AppShell`: holds `placements: bar_items::Placements` (loaded from the backend
    blob at construction), `backend`/`tokio` handles for persistence.
    `build_bar_bucket(bar, side)` + `render_bar_item(id, compact)` are shared by
    **both** `render_header` and `render_statusbar` — all hardcoded children removed.
    Items implemented: updater (Download, shown only when an update is
    pending/ready), notifications (Bell + count + popover with "Clear all"),
    jumpHosts (Server → host manager), agentAccess (folds in the existing badge),
    transfers (→ `reveal_transfers`), bookmarks (→ popover), explorer/snippets/
    sourceControl panel toggles (drive `select_panel_on_side` honouring the item's
    `side`), cwdBreadcrumb, cursorPosition (`Ln x, Col y` from the editor), aiMini /
    aiPanel (toggle the AI dock). `tabsPanel` renders nothing (titlebar tabs
    location, matching the reference `renderBarItem` null case); `previewUrl`
    renders nothing (dev-server URL detection from terminal output is not ported).
    Placement changes persist via `settings_set_bar_item_placement` (fire-and-forget).

- **T16-006 — bar-item context menu + interactive breadcrumb (DONE).**
  - `render_bar_menu` — the shared right-click menu on every non-breadcrumb bar
    cluster: Left / Right, Titlebar / Status Bar, Hide (port of `BarItemContextMenu`).
  - New `crates/ui/src/cwd_breadcrumb.rs` — pure `segments_from_cwd` / `relative_path`
    / `dirname` / `basename` / `resolve_provider` ported from `pathUtils.ts` +
    `CwdBreadcrumb.tsx`; 7 unit tests incl. the `CwdBreadcrumb.test.ts` provider
    cases.
  - `AppShell::render_cwd_breadcrumb` — real component: `~`/home collapse, click a
    parent segment → `Workspace::send_cd` (POSIX-quoted `cd`), current segment →
    subdirectory dropdown (`render_subdir_menu`, listed in the background via
    `tree::read_dir_page`; remote SSH listing deferred), `…` overflow-collapse
    (>4 parents) with an expand toggle, file-mode (dir segments navigate, filename
    is a non-clickable leaf), "no directory" state. Per-segment right-click
    (`render_crumb_menu`): Copy absolute / relative path, Open in current / new
    terminal, then the bar-item Move to Titlebar / Status Bar / Hide.

- **T16-004 / T16-007 leftovers.**
  - The invented 44px activity rail is **gone** — panel switching now runs through
    the statusbar panel-toggle bar items. Added a **right dock**: `sidebar_side`
    follows the toggled item's `side`, and `render()` places the sidebar before or
    after the workspace accordingly (resize handle + drag-delta flip with it).
  - `⋯` header button now opens a dropdown (Settings / Keyboard Shortcuts / Themes…).
  - `+` new-tab dropdown gained **Git Graph** (now a real `TabKind::GitGraph` tab —
    `Workspace` lazily owns a `GitGraphView`; `CommandId::OpenGitGraph` opens the
    tab too) and flattened **SSH · <host>** / **SFTP · <host>** recent-host entries
    (`Workspace::recent_hosts`, sorted by `last_connected_at`) + "All hosts…".
  - Tab context menu: added **Keep Tab Open** for peek editor tabs.

### NOT Done / deferred
- `previewUrl` bar item + dev-server URL detection from terminal output.
- Remote (SSH) directory listing in the breadcrumb subdir dropdown (local only).
- Inline tab **Rename** (needs a new tab-strip edit-state; `set_custom_title` exists).
- `aiMini` `AgentStatusPill` (status text) — the item is a plain toggle for now.
- True nested submenus in the `+` dropdown (flattened instead — the hand-rolled
  overlay can't nest; a real menu primitive would be needed).
- `badgesAlwaysVisible` preference (badges always self-hide when empty).

### State
- Branch `master`. `cargo fmt --check`, `cargo check/clippy --workspace
  --all-targets -D warnings`, `cargo test --workspace` all green
  (UI 221 tests, backend 210).

### Risks for later blocks
- `SidebarPanel::{GitGraph, Ai}` variants + `AppShell.git_graph` are now partly
  vestigial (Git Graph is a tab; AI is the `aiPanel` dock). A later cleanup can
  drop `SidebarPanel::GitGraph`.
- Bar-item popovers (notifications, agent badge, ⋯) are hand-rolled absolute
  overlays, not `gpui_component` popups — if Block A's Root overlay layers get
  composed, migrate these.
- The breadcrumb subdir dropdown and crumb/bar menus are AppShell-root overlays
  anchored in window coords (correct there, unlike the Workspace-rooted tab menus).

## Last Session: 2026-09-02 (Block B — window chrome & unibar, partial)

### What Was Done
Feature-parity audit **Block B — Window Chrome & Unibar** (`vergleichsbericht-subagent-1.md`).

- **T16-004 — single transparent titlebar (DONE).**
  - `crates/app/src/main.rs`: `TitlebarOptions` → `appears_transparent: true`,
    `title: None` (macOS `hiddenTitle` equivalent), `traffic_light_position:
    Some(point(px(19.0), px(13.0)))` so the lights vertically-centre in the 40px
    header. Added `point` import.
  - `crates/ui/src/app_shell.rs` `render_header`: deleted the hardcoded
    `"Labonair"` text node and the stray hamburger sidebar-toggle. The header is
    now the ONE bar; it hosts the tab strip (`flex_1`) + the traffic-light left
    inset. The dead `⋯` app-menu button is still there (no dropdown yet — its
    4-item menu is a T16-005 follow-up).
  - `render_sidebar`: removed the 44px activity rail's `border_r_1` (the
    "phantom vertical bar") and gave the rail a `sidebar_bg`; replaced the 6px
    solid resize `handle` with a 1px border line inside a transparent grab zone
    (reference `ResizableHandle` shape). **The rail itself is kept** for now —
    it is still the only panel switcher until the T16-005 panel-toggle bar
    items land; removing it without a replacement would break panel switching.
  - `crates/ui/src/workspace.rs`: `render_tab_bar` is now `pub(crate)`, `h-7`,
    no bottom border / bg, and is rendered **inside** `AppShell::render_header`
    (`self.workspace.update(|w,cx| w.render_tab_bar(cx))`). Removed the standalone
    36px bordered strip from `Workspace::render` (the "third location").
  - Tab-menu / new-tab-menu anchors are captured in window coords but drawn in
    the `Workspace` overlay (which starts below the 40px header), so both now
    subtract `TITLEBAR_OFFSET = 40.0`.

- **T16-007 — tabs (PARTIAL).**
  - `+` button now opens a dropdown (`render_new_tab_menu`, new `new_tab_menu:
    Option<Point>` state on `Workspace`): Terminal ⌘T / Editor ⌘E / Preview ⌘P /
    ─ / Open Host Manager. **Not done:** Git Graph entry (no tab opener exists —
    it's still a sidebar panel), SSH▶/SFTP▶ recent-host submenus (hand-rolled
    overlay can't nest; needs the host list + a submenu primitive).
  - Tab context menu: added **Duplicate** (sets active + `duplicate_active_tab`),
    **Close All** (`close_all_tabs` — new, closes every non-Home tab), and the
    per-kind plural label ("Close All Terminals" etc. via new
    `TabKind::plural_label`, ported from `pluralLabelFor`). **Not done:** inline
    Rename (needs a new tab-strip edit state), Keep Tab Open, the
    workspace-vs-non-workspace menu split, icons/separators,
    `host.block_agent_access` disabled state.

- **T16-008 — native menu + handler gaps (DONE).**
  - `menu.rs`: App menu — dropped the extra separators + the `Services`
    os_submenu (removed `SystemMenuType` import); kept `Check for Updates…` as a
    documented deliberate T15-005 addition, now placed cleanly. File menu —
    removed the `Save` item (reference has none; the `cmd-s` binding stays).
    Window menu — removed `Command Palette…` (the shortcut stays).
  - Wired previously-dead `on_action` handlers on `AppShell`:
    `menu::ToggleAiPanel` (Cmd+I now works → `select_panel(Ai)`), `NewAiSession`
    (→ `AiChatView::new_session` + reveal panel), `ClearChat` (→
    `AiChatView::clear_active_chat` — new: delete active session + new, mirrors
    `menu:clear_chat`), `OpenHostManager` / `NewSshTab` / `NewSftpTab` /
    `NewSshConnection` / `NewQuickSsh` (all → `Workspace::open_host_manager`,
    new: focus/open the Home dashboard, matching every reference
    `useMenuBridge` handler which is just `openHomeTab()`).
  - New tests: `menu` binding counts unchanged (still 37/36); added
    `tabs::plural_labels_match_reference`.

### NOT Done (remaining Block B)
- **T16-005 — unibar model.** `BarItemId` enum / `BarItemPlacement` /
  `build_bar_bucket` / `render_bar_item` / persistence via the existing (still
  dead) `settings_set_bar_item_placement` backend fn — **not started.** Header
  and statusbar still hardcode their children. This is the biggest remaining
  piece; `vergleichsbericht-subagent-1.md` §2 is the spec.
- **T16-006 — bar-item context menu + interactive CwdBreadcrumb.** Not started;
  breadcrumb is still non-interactive split-on-`/` text (`app_shell.rs`).
- **T16-004 leftovers:** the activity rail still exists (kept for function); the
  `⋯` header button still has no dropdown; no right dock / `side` honouring.
- **T16-007 leftovers:** SSH/SFTP submenus, inline rename, menu split.

### State
- Branch `master`, committed on top of `f9a10c3` (Block A).
- `cargo fmt --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace` all green.

### Risks for later blocks
- **C (Settings):** T16-005 persistence must use
  `settings_set_bar_item_placement`; add the `BarItemId`↔serde map there.
- **F (Sidebars/CtxMenus):** the kept activity rail + `SidebarPanel::{GitGraph,
  Ai}` variants should be removed when the bar-item panel toggles + dual dock
  land; Git Graph should become a `TabKind::GitGraph` tab (also unblocks the
  `+` dropdown's "Git Graph" entry).
- Tab/new-tab overlay menus use a fixed `TITLEBAR_OFFSET = 40.0` fudge because
  they're anchored from window coords into the `Workspace` overlay — if the
  header height or layout changes, revisit.

## Last Session: 2026-09-02 (Block A — shared component + icon layer)

### What Was Done
Feature-parity audit **Block A — Fundament (P0)** (`vergleichsbericht-subagent-3.md` / `-4.md`).

- **T16-001 — gpui-component dependency + shared primitives.**
  - Added `gpui-component = "0.5.1"` to `[workspace.dependencies]` (it targets
    `gpui ^0.2.2`, matching the repo). Wired `gpui_component::init(cx)` and an
    asset source in `crates/app/src/main.rs`; the window root is now wrapped in
    **`gpui_component::Root`** (required so its `Input`/popover/dialog layers
    resolve — `Root::read` panics otherwise). On macOS server-side decorations
    `Root`'s `window_border()` is a no-op, so no layout shift expected.
  - New `crates/ui/src/components/` module:
    - `button.rs` — `button(id, &ThemeStore, ButtonVariant, ButtonSize)` builder.
      Six variants × eight sizes, pill radius (`radius.xl4` == 13px ==
      reference `rounded-4xl`), transparent border, per-variant hover — all 1:1
      from `reference-src/src/components/ui/button.tsx`. Unit-tested.
    - `icon.rs` — `IconName` enum (macro-generated path table) + `.svg(color)`
      renderer + `file_icon`/`folder_icon` resolvers (reduced `iconResolver.ts`).
    - `text_field.rs` — re-exports gpui-component `InputState` / `Input` (as the
      `TextField` primitive) + `text_field` / `field_input` helpers.
    - re-exports `Badge` / `Switch` / `Tooltip` for later blocks.
  - `crates/ui/src/assets.rs` — `Assets` (`gpui::AssetSource`) embedding ~40
    Lucide SVGs bundled under `crates/ui/assets/icons/` (ISC licensed).
  - `hosts.rs::btn` restyled to the reference pill radius / transparent-border
    treatment (full swap of the 32 call sites onto `components::button` is
    incremental — Block B, when the host-manager panel is rebuilt).
- **T16-002 — real text input (canary).** `explorer.rs` inline create/rename
  row now uses a real `InputState`/`Input` (caret, mouse selection, clipboard
  paste, IME, undo) instead of the focus-div that pushed chars via
  `on_key_down`. Enter → commit via `InputEvent::PressEnter`, Esc → cancel.
  `edit_focus` / the char-pushing `on_edit_key` body removed. Other call sites
  (AI composer, host form, SSH prompts, search boxes) left for later blocks —
  the widget is ready.
- **T16-003 — icon system + emoji purge.** Replaced every emoji / geometric
  pseudo-icon in `app_shell`, `tabs`, `explorer`, `ai_chat`, `sftp`, `snippets`,
  `git`, `hosts`, `notifications`, `transfers`, `workspace` with `IconName`
  SVGs (`TabKind::indicator()` and `SidebarPanel::glyph()` now return
  `IconName`; host checkboxes → `Square`/`SquareCheck`; severity/warn glyphs →
  `TriangleAlert`/`CircleCheck`/`CircleX`/`Info`; jump-route `⤳` → `→`).
  New CI guard `crates/ui/tests/no_pictograph_icons.rs` fails if any emoji or
  listed pseudo-icon glyph reappears in non-comment / non-test code.
  Deliberately kept (audit-sanctioned): status dots `○ ◐ ●`, disclosure carets
  `▸ ▾`, arrows, `·`, key-combo symbols `⌘ ⌃ ⌥` in help strings.

### Current State
- Branch `master`. `cargo fmt --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace` — all green. (Pre-existing unrelated uncommitted `CLAUDE.md` edit
  left untouched.)

### What's Next
- **Block B** of the audit. First prerequisite: compose gpui-component's
  notification / dialog / sheet render layers into the tree (Root currently
  renders only the child view) before using Select/Dropdown/Dialog/Tooltip/
  ContextMenu. Then the dynamic command palette, dynamic sidebars, master/detail
  Host Manager, CWD breadcrumb, SSH loading screen, AI composer rebuild.

### Blockers / notes
- The explorer TextField canary + `Root` wrapper are compile+clippy+test clean
  but **not runtime-verified** (no GUI here) — worth an eyeball in the manual
  `cargo run` round: (a) explorer rename/new-file field types correctly,
  (b) no global layout shift from the `Root` wrapper.

---

## Prev Session: 2026-09-02 (T13-005 — Remaining shortcut handlers)

### What Was Done
- **T13-005 ✅ Done.** Wired the non-menu shortcuts the T15-006 audit flagged,
  1:1 with `reference-src/src/modules/shortcuts/lib/useShortcutHandlers.ts`.
  - **Preferences (`crates/backend/src/modules/settings/preferences.rs`):** new
    `zen_mode_show_header` / `zen_mode_show_statusbar` bool fields (serde
    `zenModeShowHeader` / `zenModeShowStatusbar`, default `true`) + roundtrip
    test.
  - **`crates/ui/src/menu.rs`:** `actions!` gains `SelectTab1..9`,
    `FocusNextPane`, `ToggleZenMode` (no menu entries — reference has none);
    11 new `rebind!` calls. Binding-count tests 26→37 / 25→36.
  - **`crates/ui/src/command_palette.rs`:** `CommandId::{ToggleZenModeHeader,
    ToggleZenModeStatusbar, ToggleZenMode}` + 3 `COMMANDS` rows in a new
    "Settings" section ("Toggle: Show Header Bar / Show Status Bar / Zen Mode");
    `ToggleZenMode` carries `shortcut: Some(ViewZenMode)` so
    `command_for_shortcut` resolves. Tab-index / `pane.focusNext` stay
    `None` (parity — dispatched via menu action, not the palette).
  - **`crates/ui/src/workspace.rs`:** `select_tab_by_index(idx)` (port of
    `selectByIndex`), `focus_next_pane` (port of `pane.focusNext` —
    `leaves()` → next index cyclic, no-op on single pane / non-workspace).
  - **`crates/ui/src/app_shell.rs`:** `select_tab_action!` macro → 9
    `act_select_tab_N` handlers, `act_focus_next_pane`, `act_toggle_zen_mode`
    + `toggle_zen_mode` / `toggle_zen_pref` helpers (persist via
    `PreferencesStore::set_value`). All 11 registered as `on_action`; palette
    arms for the 3 zen commands. `render` now hides header / statusbar per
    the zen prefs (`.children(Option<AnyElement>)`).
- **T15-006 checklist** updated (command-palette / shortcuts rows + follow-up
  table mark T12-003 & T13-005 done).
- Verify: `cargo fmt --all --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — all green. backend 208→209 tests.

### Current State
- Branch `master`, committed. Pre-existing uncommitted `CLAUDE.md` edit is
  **not ours** — left untouched, excluded from the commit.

### What's Next
- All roadmap tasks (`tasks/phase-*`) are ✅ Done. Only remaining item is the
  **manual `cargo run` acceptance round** (T15-006 template) by the user.
  `tasks/phase-14-testing/T15-001-feinschliff-catalog.md` is a living design
  doc without a Status header, not a blocking task.

### Blockers / notes for next session
- Zen prefs are exposed only via the command palette + `Cmd+Shift+Z` (matches
  reference — no settings-panel row).

---

## Prev Session: 2026-09-02 (T12-003 — Path bookmarks)

### What Was Done
- **T12-003 ✅ Done.** Ported `reference-src/src/modules/bookmarks/`.
  - **Model (`crates/backend/src/modules/bookmarks/mod.rs`, new):** `PathBookmark`
    `{ id, path, label?, host_id? }`, `bookmark_key` (`local` for `None`),
    `is_bookmark_orphaned` (host gone → flag, keep), `compute_add_bookmark`
    (dedupe per `(host_id, path)`; `None` = no-op; label-update instead of a
    second entry), `compute_remove_by_path` / `_by_id`, `is_bookmarked`,
    `find_bookmark`, `BookmarkContext` (Local/Host/Sftp/None) +
    `filter_for_context` → host-grouped `BookmarkSection`s, and JSON persistence
    (`load`/`save` → `<config_dir>/labonair/bookmarks.json`, corrupt file = empty).
    +15 unit tests. Registered in `modules/mod.rs`.
  - **UI (`crates/ui/src/bookmarks.rs`, new):** `BookmarksView` overlay popover
    (same "always a child, toggle `open`" pattern as `CommandPalette`), grouped
    list + per-row remove (`×`) + orphan tag + "+ Add current folder" header
    action. Emits `BookmarkEvent::{OpenLocal, OpenRemote}`.
  - **Wiring:** `command_palette.rs` — `CommandId::OpenPathBookmarks` + COMMANDS
    row with `shortcut: Some(BookmarksOpen)` (so `command_for_shortcut` resolves;
    +assert). `menu.rs` — `OpenPathBookmarks` action + `rebind!` for
    `ShortcutId::BookmarksOpen` (Cmd+Shift+O), bindings-count tests 25→26 / 24→25.
    `app_shell.rs` — `bookmarks` field, observe+subscribe, `pending_bookmarks`
    drained in `render` (`drain_pending_bookmarks`): OpenLocal → explorer
    `set_root_str` + focus Explorer panel; OpenRemote → `workspace.open_sftp_tab`.
    `act_open_path_bookmarks` on_action + `run_palette_command` arm.
  - **Explorer:** `ExplorerView::root()` getter + "Bookmark This Folder" context
    menu item (`bookmark_folder`, local-only, saves via the model).
  - **Workspace:** `known_hosts` / `active_host_id` / `open_sftp_tab` accessors.
- Verify: `cargo fmt --all --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
  — all green. backend 193 → **208**; ui 202 (unchanged count).

### Current State
- Branch `master`, committed. Pre-existing uncommitted `CLAUDE.md` edit is
  **not ours** — left untouched, excluded from the commit.

### What's Next
- Remaining follow-ups: **T13-005** (remaining shortcut handlers). Plus the
  manual `cargo run` acceptance round (T15-006).

### Blockers / notes for next session
- Remote bookmark jump opens/focuses the host's SFTP tab but does not yet
  navigate that tab to the bookmarked path (SFTP view has no external
  "goto path" API) — acceptable for now, note for a later polish pass.
- Explorer is always local in the current port, so the context-menu
  "Bookmark This Folder" only ever creates local bookmarks; remote bookmarks
  are added via the popover's "Add current folder" while an SSH/SFTP tab is active.

---

## Prev Session: 2026-09-02 (T06-005 — Editor soft-wrap + audible terminal bell)

### What Was Done
- **T06-005 ✅ Done.** Closed the two parity gaps logged by the T15-006 audit.
  - **Editor soft-wrap (`crates/ui/src/editor.rs`):**
    - New `Wrap { cols }` geometry helper (`cols == 0` = off; char-grid wrap,
      no word-boundary breaking, like CodeMirror `EditorView.lineWrapping` for
      code). `EditorView::wrap_cols()` derives the column count from
      `editor_word_wrap` + measured content width / char advance.
    - Renderer: builds a `layout: Vec<(line, top_px, visual_row_count)>` for the
      visible logical lines with cumulative Y offsets instead of a fixed
      `line * line_h` grid. Text rows get `.w(content_w)` (GPUI native wrap) and
      a per-visual-segment selection highlight; gutter number renders once per
      logical line at its first visual row; caret + current-line band use the
      segment offset.
    - Navigation: `wrap_vertical` (Up/Down move ±`cols` within a wrapped line,
      then cross to the previous/next logical line) and `wrap_horizontal`
      (Home/End = start/end of the current visual segment), wired into `on_key`
      with a fallback to logical `Motion` when wrap is off. `position_at` (mouse
      hit-testing) walks visual rows when wrapped.
    - +3 tests (`wrap_rows_math`, `soft_wrap_navigation_crosses_visual_rows`,
      `soft_wrap_disabled_falls_back_to_logical`).
  - **Audible terminal bell (`crates/ui/src/bell.rs`, new + `terminal.rs`):**
    - `crate::bell::{should_ring, ring}` — port of `rendererPool.ts::playBell`.
      `ring` gates on `terminal_bell`, applies a 120 ms debounce, and on macOS
      plays the system alert sound via a detached `afplay`
      (`/System/Library/Sounds/Tink.aiff`) — dependency-free; the web app's
      800 Hz `AudioContext` tone has no GPUI equivalent. Linux `play()` is a
      no-op (deferred, per task notes).
    - `TerminalView` poll loop now scans drained events for `TerminalEvent::Bell`
      and calls `bell::ring` with the live `GlobalPreferences`.
    - +2 tests (gate follows preference; `ring` is a no-op when disabled).
- Verify: `cargo fmt --all --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — all green. ui **197 → 202**; all other crates
  unchanged (backend 193, terminal 67, editor 60, theme 25, ai 75, app-smoke 3).

### Current State
- Branch `master`, committed. Pre-existing uncommitted `CLAUDE.md` edit is
  **not ours** — left untouched, excluded from the commit.

### What's Next
- Remaining follow-ups: **T12-003** (path bookmarks), **T13-005** (remaining
  shortcut handlers). Plus the manual `cargo run` acceptance round (T15-006).

### Blockers / notes for next session
- Soft-wrap text uses GPUI's native word-wrap while the caret/selection math is
  a fixed char grid — near wrap points on prose with long words the visual
  break can drift a column from the caret column. Fine for code; revisit with
  manual segmentation if it looks off in the live app.
- Bell uses `afplay` (always present on macOS); no bundled WAV asset.

---

## Prev Session: 2026-09-02 (T15-006 — Feature-parity acceptance / FINAL roadmap task)

### What Was Done
- **T15-006 ✅ Done.** Full module inventory of the pure-Rust port against
  `reference-src/` (every `src/modules/` + `src-tauri/src/modules/` folder + the
  ~150 `generate_handler!` commands). Result checklist + deviation list written
  into `tasks/phase-14-testing/T15-006-feature-parity-acceptance.md`.
  - **Findings:** the port is functionally complete for every backend module
    (ssh/sftp/git/fs/pty/hosts/credentials/snippets/secrets/themes/backgrounds/
    scrollback/settings/shell/terminal_exec/mcp/fonts/errors/menu_sync/dock_menu/
    updater) and every frontend module — **except** the `preview/` tab, which
    was a placeholder (`other => placeholder(...)` in `workspace.rs`, menu action
    `NewPreviewTab` unhandled).
  - **Gap fixed this session — native Preview pane (`crates/ui/src/preview.rs`,
    new, +3 tests, ui 194→197):** the documented WebView replacement, now real.
    - Images (`png/jpg/jpeg/gif/webp/bmp/ico`) → native `img()` (validate +
      re-encode to PNG via the `image` crate, same as `background.rs`).
    - Markdown/text (`md/markdown/txt/text`) → native render via the existing
      `crate::markdown` parser (own compact block renderer in `preview.rs`).
    - HTML/PDF/SVG/`http(s)` URLs → address bar + **"Open in system browser"**
      button (`/usr/bin/open` macOS, `xdg-open` Linux).
    - Wired: `menu::NewPreviewTab` handler `act_new_preview_tab` +
      `.on_action` (`app_shell.rs`); `Workspace::new_preview_tab` /
      `open_preview` + `previews: HashMap<u64, Entity<PreviewView>>` + render
      arm + `retire_tab` cleanup (`workspace.rs`); Explorer context-menu
      **"Open in Preview"** for previewable files (`explorer.rs`,
      `open_in_preview`); session restore now `RestoreAction::Preview { url }`
      instead of Skip (`session.rs`, test updated).
  - **Version bumped:** `crates/app/Cargo.toml` `0.1.0` → **`1.0.0`** (first
    feature-complete release; single source for packaging/updater/smoke-test).
  - **Remaining gaps → follow-up task files created (all non-core):**
    - `tasks/phase-11-snippets-palette/T12-003-path-bookmarks.md` — `bookmarks/`
      module (local/remote dir bookmarks, `Cmd+Shift+O`) not ported.
    - `tasks/phase-12-settings/T13-005-remaining-shortcut-handlers.md` —
      `tab.selectTab1..9`, `pane.focusNext`, `view.zenMode` (+ zen prefs) have
      no dispatch (deferred & signed off in T13-004).
    - `tasks/phase-05-editor/T06-005-soft-wrap-and-terminal-bell.md` — editor
      `editor_word_wrap` has no renderer effect; `terminal_bell` is a stored
      pref with no audible beep.
    - ROADMAP.md updated with the 3 new task rows.
- Verify: `cargo fmt --all --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — all green. ui **194→197**, all others unchanged
  (backend 193, terminal 67, editor 60, theme 25, ai 75, app-smoke 3).

### Current State
- Branch `master`, committed. Pre-existing uncommitted `CLAUDE.md` edit is
  **not ours** — left untouched, excluded from the commit.

### What's Next
- **Roadmap complete.** Remaining work = the 3 follow-up tasks above
  (T12-003, T13-005, T06-005) + the manual `cargo run` acceptance round by the
  user (T15-006 warning: only sign off after real side-by-side testing).
- Bake in the real `UPDATE_PUBLIC_KEY` minisign pubkey before first signed
  release (unchanged blocker from T15-005).

### Blockers / notes for next session
- Preview markdown renderer has no inline styling (bold/italic/link render as
  plain text) and no syntax highlighting in code blocks — deliberately compact.
- `docs/performance.md` "Recorded runs" table still empty.

---

## Prev Session: 2026-09-02 (T15-005 — Auto-updater, macOS)

### What Was Done
- **T15-005 ✅ Done.** Native reimplementation of the Tauri updater flow
  (port of `reference-src/src/modules/updater/`), on top of the T15-004
  manifest module.
  - **Decision:** custom minimal updater, **not Sparkle** — the app already
    publishes a Tauri-shaped `latest.json` with minisign signatures, so reusing
    that avoids a Swift shim + a 2nd signing system (Zed does the same).
    Documented in `docs/RELEASE.md`.
  - **`crates/backend/src/modules/updater/install.rs`** (new, +7 tests,
    backend 186→193):
    - `fetch_manifest(endpoint)` — reqwest GET + `UpdateManifest::parse`.
    - `download_update(url, on_progress)` — streamed via
      `reqwest::Response::chunk()` (no `futures` dep), progress callback per
      chunk.
    - `verify_update(artifact, sig_b64, pubkey_b64)` — `minisign-verify` 0.2
      (Ed25519, pre-hashed). `sig_b64` = base64 of the whole `.minisig` file
      (Tauri shape). **Empty key or empty signature → hard error** (safe
      default; `UPDATE_PUBLIC_KEY` is an empty placeholder until a real key is
      baked in).
    - `apply_macos_update(archive, bundle)` — unpack `<name>.app.tar.gz`
      (flate2+tar), move running `.app` aside, rename new into place, **roll
      back on failure**, clean staging/backup.
    - `current_app_bundle()` (walks `current_exe()` ancestors for `*.app`),
      `relaunch(bundle)` (`open` + `exit(0)`).
    - `should_auto_check()` / `record_check_now()` — `CHECK_INTERVAL` = 6 h,
      timestamp in `~/.config/labonair/updater-last-check`.
    - dev-dep `minisign = "0.7"` (signing) for the positive/negative sig tests.
  - **`crates/ui/src/updater.rs`** (new, +6 tests, ui 188→194) — `UpdaterView`
    GPUI entity+dialog. `UpdaterStatus` mirrors the reference union
    (Idle/Checking/UpToDate/Available/Downloading/Ready/Error). `run_check
    (manual)` (quiet auto vs. reporting manual), `install()` (tokio task →
    `mpsc` progress channel → `cx.spawn` recv loop → Ready → 700 ms →
    `relaunch`). Dialog markup + button labels ("Later", "Install & restart",
    "Installing…", "Close") 1:1 with `UpdaterDialog.tsx`; progress bar =
    `div().w(relative(frac))`. Errors → notification toasts.
  - **`crates/ui/src/app_shell.rs`** — owns `updater: Entity<UpdaterView>`,
    quiet startup check when `checkForUpdates` pref on, `act_check_for_updates`
    handler (menu `CheckForUpdates` + new command-palette entry), renders the
    dialog as a root child.
  - **`crates/ui/src/menu.rs`** — dropped the `CheckForUpdates` toast stub
    (now AppShell-handled, like `OpenSettings`).
  - **`crates/ui/src/command_palette.rs`** — new `CommandId::CheckForUpdates`
    ("Check for Updates…", Application section).
  - **`scripts/package-macos.sh`** — now also emits
    `Labonair_<version>_<arch>.app.tar.gz` + `latest.json` (signature inline,
    base64 of `.minisig` when `LABONAIR_UPDATER_KEY` + `minisign` present, else
    empty). **`.github/workflows/release.yml`** — `brew install minisign`,
    write `LABONAIR_UPDATER_PRIVATE_KEY` secret to a temp key file, upload
    `*.dmg` + `*.app.tar.gz` + `latest.json`.
  - **`docs/RELEASE.md`**, **`CHANGELOG.md`** updated (decision, signing
    setup, artifacts, limitations).
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` — all green. backend **186→193**,
  ui **188→194**; theme 25, ai 75, terminal 67, editor 60, app smoke 3
  unchanged.

### Current State
- Branch `master`, committed. Pre-existing uncommitted `CLAUDE.md` edit is
  **not ours** — left untouched, excluded from the commit.

### What's Next
- **T15-006** — feature-parity acceptance (final gate; bump version in
  `crates/app/Cargo.toml` there).

### Blockers / notes for next session
- **`UPDATE_PUBLIC_KEY` is empty** — bake in the real minisign pubkey +
  add `LABONAIR_UPDATER_PRIVATE_KEY` / `LABONAIR_UPDATER_KEY_PASSWORD` CI
  secrets before the first signed release. Until then the in-app updater
  refuses every update by design.
- End-to-end updater test (real signed test-version → download → apply →
  relaunch) **not runnable here** (no Developer ID, no published release).
  Logic is unit-tested; do a manual run on a real release.
- `latest.json` from a per-arch CI run is single-arch — merge the
  `darwin-aarch64` + `darwin-x86_64` blocks before upload (noted in
  `docs/RELEASE.md`).
- `docs/performance.md` "Recorded runs" table still empty.

---

## Prev Session: 2026-09-02 (T15-004 — Packaging & release + license audit)

### What Was Done
- **T15-004 ✅ Done.** Release/distribution foundation for the GPUI binary
  (no `tauri bundle` equivalent — hand-rolled).
  - **`crates/backend/src/modules/updater/mod.rs`** (new, +6 tests, backend
    180→186) — Tauri-compatible `latest.json` decision layer:
    `UpdateManifest { version, notes, pub_date, platforms }` /
    `UpdatePlatform { url, signature }`, dependency-free `SemVer` (ignores
    `-pre`/`+build`, optional leading `v`), `UpdateManifest::available_for
    (current, target)` / `available()` → `Option<AvailableUpdate>` only on a
    strictly-newer version *with* an artifact for this platform.
    `UPDATE_TARGET` const (`darwin-aarch64|darwin-x86_64|linux-x86_64|
    linux-aarch64`), `DEFAULT_UPDATE_ENDPOINT` (this fork's GH releases
    `latest.json`), `CURRENT_VERSION = env!("CARGO_PKG_VERSION")`. Re-exported
    from `labonair_backend`. Download/verify/apply + "update available" dialog
    are explicitly **T15-005** — this is only the format + version check.
  - **`packaging/macos/`** — `Info.plist` (template with `__VERSION__` /
    `__BUILD__`, id `com.labonair.app`, developer-tools category, doc types),
    `Labonair.entitlements` (copied from reference — keychain group, sandbox
    off, network client), `AppIcon.icns` + `icon.png` (copied from
    `reference-src/src-tauri/icons/`).
  - **`scripts/package-macos.sh`** — `cargo build --release -p labonair` →
    assembles `target/release/bundle/macos/Labonair.app/Contents/{MacOS,
    Resources}`, version from `crates/app/Cargo.toml` (the single source),
    `CFBundleVersion` = `git rev-list --count HEAD`, `plutil -lint`. Opt-in
    `--dmg` (hdiutil), opt-in codesign (`LABONAIR_SIGN_IDENTITY`, hardened
    runtime + entitlements) and notarization (`LABONAIR_NOTARY_PROFILE`,
    `notarytool --wait` + `stapler`). Never blocks when unset. bash 3.2-safe
    array expansion.
  - **`scripts/smoke-test.sh`** + **`crates/app/tests/smoke.rs`** (+3 tests) —
    build bundle → structural checks (binary executable, `Info.plist` lints,
    version substituted, identifier, icon, PkgInfo, optional signature) →
    `cargo test -p labonair --test smoke` (backend SQLite init in a temp dir,
    real `/bin/sh` PTY round-trip via `TerminalSession`, update-manifest check
    against the bundled version). `LABONAIR_SMOKE_LAUNCH=1` also `open`s the
    app 5s then quits it (needs a window server).
  - **`.github/workflows/release.yml`** (new) — on `v*` tag: run smoke-test +
    `package-macos.sh --dmg` on `macos-latest`, attach the dmg to the GH
    release. Signing secrets optional.
  - **`docs/RELEASE.md`** — version source, build/sign/notarize procedure,
    universal-binary recipe, Linux perspective (AppImage/Flatpak later,
    `scripts/package-<os>.sh` switch), auto-update foundation, artifact table,
    known limitations vs. the original (no WebView preview, macOS/Linux only,
    update check-only until T15-005, no packaged Linux release).
  - **`docs/LICENSES.md`** — full `Cargo.lock` license sweep (~1000 crate
    versions). **Result: clear.** No GPL/AGPL/LGPL-only dependency. **GPUI
    0.2.2 + all `gpui_*` crates are `Apache-2.0`** (the historical GPL concern
    is moot). `self_cell` (`Apache-2.0 OR GPL-2.0-only`) and `r-efi` take the
    permissive branch. MPL-2.0: `option-ext` (file-level, compliant as-is),
    `dwrote` (Windows-only, not compiled), `cbindgen` (build tool, not
    distributed). Fonts = SIL OFL 1.1. Ship a `cargo-about`-generated
    `THIRD-PARTY-LICENSES.txt` per release (not committed).
  - **`CHANGELOG.md`** (new, repo root) — Keep-a-Changelog, `[Unreleased]`
    documents this task + known limitations.
- Version left at **0.1.0** (port not yet feature-complete — T15-006 is the
  acceptance gate). Bump in `crates/app/Cargo.toml` at first real release.
- Verify: `cargo fmt --all --check`, `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — all green. backend **180→186**, app smoke
  **0→3**; theme 25, ui 188, ai 75, terminal 67, editor 60 unchanged.
  `scripts/smoke-test.sh` run locally end-to-end (release build + bundle +
  structural checks + smoke tests).

### Current State
- Branch `master`, committed. Pre-existing uncommitted `CLAUDE.md` edit is
  **not ours** — left untouched, excluded from the commit.

### What's Next
- **T15-005** — Auto-updater (download + minisign verify + apply + "update
  available" dialog, on top of `labonair_backend::updater`).
- Then T15-006 — feature-parity acceptance (final gate; bump version there).

### Blockers / notes for next session
- `docs/performance.md` "Recorded runs" table still empty — fill from a
  `cargo run --release` on Apple Silicon.
- `latest.json` must be generated + uploaded alongside release artifacts
  (format documented in `docs/RELEASE.md` / the `updater` module).
- No signing certs available in this environment — signing/notarization is
  written + documented but never executed here.

---

## Prev Session: 2026-09-02 (T15-003 — Cross-platform & performance + deferred pixel items D1–D6)

### What Was Done
- **T15-003 ✅ Done.** Measure-first per the task warning: the GPUI-native
  architecture already removes the WebView/IPC overhead and a static review
  found the known hot paths already guarded, so this pass = document the
  yard-stick + close the deferred pixel catalog, not speculative tuning.
  - **`docs/performance.md`** (new) — baseline measurement method + recorded-
    runs table (pending a release machine), target envelope (cold start
    <400ms, idle RSS <150MB, …), a pre-release manual regression checklist,
    an inventory of every hot-path guard already in the code
    (startup async workers, lazy grammars, alacritty render diff, explorer
    `generation`/page-cap, git-graph `uniform_list`, git poll `refreshing` +
    `target_gen` + no-op-on-no-root, incremental AI streaming), the
    follow-ups deliberately deferred (Explorer/SFTP true windowing; pausing
    git poll off-screen), and macOS/Linux cross-platform notes.
  - **Deferred visual items D1–D6 from the T15-001 catalog — all closed** by
    extracting canonical reference values into tested constants/helpers:
    - **D1** `crates/ui/src/theme.rs` — `ThemeStore::hover_fill()` (= `accent`)
      + `selected_fill()` (= `muted`), matching the reference
      `focus:bg-accent` / `data-selected:bg-muted`. Command-palette rows moved
      off the ad-hoc `accent` selection + `border` hover to `selected_fill()`.
    - **D2** — `scrollbar_thumb()` / `scrollbar_thumb_hover()` = foreground
      @ 0.22 → 0.34 alpha; `theme::SCROLLBAR_SIZE = 10.0`
      (`.themed-scrollbar`). Not yet wired to a widget (no scrollbar
      component adopted) — helper + constant ready.
    - **D3** `crates/ui/src/terminal.rs` — cursor proportions vs xterm: beam
      1px (was 2), underline 1px (was 2), new `CursorShape::HollowBlock` arm
      = 1px outline (xterm `cursorInactiveStyle: "outline"`); focused block
      keeps the 0.55 translucent fill.
    - **D4** `crates/theme/src/tokens.rs` — `CubicBezier::eval(t)`
      (Newton-Raphson + bisection Bézier solve) + `TAB_IN_FROM_SCALE = 0.86`
      const. `crates/ui/src/workspace.rs::render_tab` now wraps the tab in a
      GPUI `with_animation` opacity fade over `--dur-base` with `--ease-premium`.
      GPUI 0.2.2 `Div` has **no scale transform**, so the `scale(0.86)→1` part
      of `@keyframes labonair-tab-in` is opacity-only. Reduce-motion pref
      clamps the duration to 10µs (mirrors the reference `0.01ms` rule).
    - **D5** `crates/ui/src/theme.rs` — `theme::menu_metrics` module
      (`CONTAINER_PAD 6` / `COMMAND_CONTAINER_PAD 4` / `ITEM_PAD_X 12` /
      `ITEM_PAD_Y 8` / `ITEM_GAP 10` / `POPOVER_PAD 16`) from the reference
      `dropdown-menu` / `command` / `popover` classes. Command-palette row
      `px` aligned to `ITEM_PAD_X`.
    - **D6** — `community_theme_partial_import_round_trips_visually` test in
      `theme.rs`: partial community theme applies only its tokens, rest stay
      on default, survives export → re-import with no RGB drift.
  - **`tasks/phase-14-testing/T15-001-feinschliff-catalog.md`** — deferred
    section rewritten to "resolved in T15-003" with per-item status.
- Verify: `cargo fmt --all --check`, `cargo check --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — all green. theme **23 → 25**
  (`cubic_bezier_eval_endpoints_and_monotonic`,
  `tab_in_from_scale_matches_reference_keyframe`), ui **185 → 188**
  (`polish_metrics_match_reference_css`, `polish_fills_derive_from_the_active_theme`,
  `community_theme_partial_import_round_trips_visually`), all others unchanged
  (backend 180, ai 75, terminal 67, editor 60, app 0).

### Current State
- Branch `master`. Commit this session (see `git log`). Not pushed (push when asked).
- Pre-existing **uncommitted `CLAUDE.md` edit is NOT ours** — left untouched, excluded from the commit.

### What's Next
- **T15-004** — Packaging & release (incl. GPUI/ztracing GPL license audit).
- Then T15-005 (auto-updater), T15-006 (feature-parity acceptance).

### Blockers / notes for next session
- `docs/performance.md` "Recorded runs" table is empty — fill it from a
  `cargo run --release` on an Apple Silicon machine before the T15-004 release.
- D2 scrollbar helper is defined but unused until a visible-scrollbar widget
  exists; wire it then (Explorer/SFTP/Settings/Snippets panels).
- D4 tab animation is opacity-only; if GPUI gains a `Div` transform, add the
  `scale(0.86)` using `TAB_IN_FROM_SCALE`.

---

## Prev Session: 2026-09-02 (T15-002 — Error handling & robustness, app-wide)

### What Was Done
- **T15-002 ✅ Done.** Formalised the app-wide error catalog on top of the
  existing `LabonairError` (T01-002) + notification system (T04-004). The
  codebase was already disciplined (Critical Rule 6 — near-zero risky
  `unwrap()`/`expect()` in non-test code; a grep for `parse().unwrap()` /
  `read_to_string().unwrap()` / `env::var().unwrap()` etc. across all crates
  turned up **only test code**), so this task was about consolidation, not a
  panic hunt.
  - **`crates/backend/src/modules/errors.rs`** — rewrote:
    - **7 new `LabonairError` variants** (kept the original 5 +
      all serde `code` tags): `NotConnected`, `NotFound`, `PermissionDenied`,
      `InvalidInput`, `Timeout`, `Conflict`. (`NotConnected` = "no live session"
      vs `NetworkError` = "wire failed".)
    - **`ErrorCategory`** enum (Ssh/Sftp/Fs/Git/Ai/Terminal/Settings/Network/
      Other, kebab-case serde) + `LabonairError::category()`.
    - **`LabonairError::user_message()`** — friendly sentence per variant:
      "what went wrong" + "what to do next", carries the raw detail but never
      the serde code tag / stack trace.
    - **`RecoveryHint`** enum (Reconnect/Retry/Resend/Diagnose/GoBack/FixInput/
      CheckSettings) + `LabonairError::recovery()` mapping (NetworkError /
      NotConnected → Reconnect, Timeout → Retry, InvalidInput → FixInput, …).
    - **`LabonairError::classify(msg)`** — one shared heuristic string
      classifier that replaces the 2 near-identical `classify_ssh_error` /
      inline sftp matchers. Order: auth → host-key → timeout → network →
      not-connected → permission → not-found → conflict → Internal. Always
      preserves the original string as the variant detail.
    - **Smarter `From` impls:** `std::io::Error` now maps by `ErrorKind`
      (NotFound/PermissionDenied/TimedOut/AlreadyExists/connection-family →
      the matching variant, else `IoError`); `rusqlite::Error::QueryReturnedNoRows`
      → `NotFound`; `russh` / `russh-sftp` errors route through `classify`.
    - **+13 tests** (backend 167 → 180): classify buckets + detail
      preservation, per-variant category stability, friendly-message content,
      recovery-hint mapping, every `ErrorKind` branch.
  - **`crates/backend/src/modules/ssh/client.rs`** — `classify_ssh_error` now
    delegates to `LabonairError::classify` (behaviour is a superset of the old
    inline matcher).
  - **`crates/backend/src/modules/sftp/connection.rs`** — the inline
    `result.map_err(|s| …)` block replaced with `.map_err(LabonairError::classify)`.
  - **`crates/backend/src/lib.rs`** — re-exports `ErrorCategory`, `RecoveryHint`.
- Verify: `cargo fmt --all --check`, `cargo check --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — all green. backend **167 → 180**, all other
  crates unchanged (ai 75, terminal 67, editor 60, theme 23, ui 185, app 0).
- **Known gaps / for later (deliberately not done — this task is "polish", not
  a rewrite):**
  - `crates/backend/src/modules/sftp/net_error.rs::is_network_error` was left
    as-is. Its semantics intentionally differ from `classify` (it treats
    "no sftp session" / timeouts as *network* so the transfer worker drops the
    session + shows a reconnect affordance). Aligning it would need re-checking
    its ~15 tests; not worth the churn.
  - `git::classify_apply_error` and `ai::AiError` keep their own domain-specific
    catalogs (already user-friendly, already tested). They are peers of
    `LabonairError`, not folded into it.
  - The new `user_message()` / `recovery()` / `category()` are available for the
    UI to consume; wiring every existing toast call-site to route through them
    is a broader UI pass — the notification infra (`notify_err`,
    `Notification::error().action(…)`) and SSH reconnect affordance already
    exist and already produce readable messages.
- **Next task:** T15-003 — Cross-platform & performance optimization
  (`tasks/phase-14-testing/T15-003-cross-platform-performance.md`).

## Prev Session: 2026-09-02 (T15-001 — Visual parity verification / design polish)

### What Was Done
- **T15-001 ✅ Done.** Static visual-parity audit of the GPUI port against the
  frozen `reference-src/` design spec + first round of polish fixes.
  - **New `tasks/phase-14-testing/T15-001-feinschliff-catalog.md`** — living
    checklist. Findings: theme tokens (colors, radii, shadows, animation,
    typography) are already a verified 1:1 port of `globals.css` (`theme` crate
    tests confirm). Editor syntax palettes + git-graph lane colors are
    intentionally not `globals.css` tokens (upstream schemes / categorical
    ramp). Six genuinely visual items (hover/focus tint amounts, scrollbar
    metrics, terminal cell/cursor proportions, tab-entrance curve, popover
    density, imported-theme round-trip) are logged as **D1–D6** for the
    live side-by-side pass in T15-003 — they can't be resolved by static audit.
  - **Fix C1 — modal scrim consolidation.** The port had **6 divergent**
    hardcoded overlay backdrops (`black@0.4`, `rgba(0x00000099)` ×5,
    `rgba(0x000000aa)`, `rgba(0x00000080)`, `hsla(…0.5)`). Reference
    `dialog.tsx` / `alert-dialog.tsx` / `sheet.tsx` all use one `bg-black/30`.
    New **`crate::theme::modal_scrim()`** (`black @ 0.30`, theme-independent by
    design — documented) now used by all 9 overlay sites: `command_palette`,
    `snippets`, `settings`, `explorer`, `transfers`, `sftp`, `hosts` (×4),
    `workspace` (prompt + close-confirm). The `TabDragPreview` pill in
    `workspace.rs:204` keeps its own fill — it's a drag chip, not a scrim.
  - **Regression guards (task step 7):**
    - `crates/theme/src/tokens.rs` — `body_text_meets_wcag_aa_contrast`: WCAG
      relative-luminance helpers + assertions that fg/bg and terminal fg/bg
      clear 4.5:1 and muted-fg clears 3:1, in **both** variants. A token edit
      that breaks legibility now fails `cargo test`.
    - `crates/ui/src/theme.rs` — `modal_scrim_matches_reference_dialog_overlay`
      pins the shared constant to black @ 0.30.
- Verify: `cargo fmt --all --check`, `cargo check --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — all green. theme **22 → 23**, ui **184 → 185**.
- **Known gaps / for later:** T15-001 is explicitly iterative. The static audit
  + token verification + scrim fix are done; the pixel-level side-by-side
  comparison (catalog items D1–D6) is deferred to **T15-003** per the task's own
  "Weiterführende Tasks" pointer. No reference *screenshots* were captured (the
  original app isn't runnable in this environment) — audit was against the
  `reference-src/` source (Tailwind classes + `globals.css`), which is the
  repo's designated authority.
- **Next task:** T15-002 — Fehlerbehandlung & Robustheit (app-weit)
  (`tasks/phase-14-testing/T15-002-error-handling-robustness.md`).

## Prev Session: 2026-09-02 (T14-002 — Scrollback persistence)

### What Was Done
- **T14-002 ✅ Done.** Persist each restorable local terminal pane's scrollback
  on quit and replay it into the re-spawned shell on the next launch, plus
  orphan/retention cleanup. Port of reference `src/modules/session/scrollback.ts`
  + `src-tauri/src/modules/scrollback/`.
  - **`crates/backend/.../modules/scrollback/mod.rs`** — the module already
    existed (only `truncate_scrollback` was live). Converted `scrollback_save`
    / `scrollback_load` / `scrollback_cleanup` from unused `async` to sync
    small-file IO (called from the same startup/shutdown paths as
    `session.json`), added `scrollback_delete`, and split each into a
    dir-parametrized `*_in` core so they're unit-testable against a temp dir.
    Storage: gzip `<data_dir>/scrollback/<pane-uuid>.ansi.gz`, atomic
    tmp+rename, front-truncation with visible overflow notice, absolute
    `HARD_MAX_UNCOMPRESSED_BYTES` ceiling. +6 tests.
  - **`crates/terminal/src/engine.rs`** — `TerminalEmulator::serialize_scrollback(max_lines)`:
    history + visible grid as plain text (`\r\n`-joined, trailing blanks
    trimmed, leading blanks dropped). Returns `""` on the alternate screen
    (don't persist a TUI's transient buffer). +2 tests.
  - **`crates/terminal/src/session.rs`** — new `SessionOptions::replay_scrollback`;
    `TerminalSession::spawn` feeds it into the emulator *before* starting the
    reader thread, so restored history lands above fresh shell output with no
    race. +1 test.
  - **`crates/terminal/src/registry.rs`** — `SessionHandle::serialize_scrollback`
    (Local only; `None` for remote/alt-screen/empty).
  - **`crates/backend/.../settings/preferences.rs`** — new `session_scrollback_lines`
    (5000), `scrollback_max_size_mb` (5), `scrollback_retention_days` (14).
  - **`crates/ui/src/settings.rs`** — 3 new Terminal FIELDS rows (generic path).
  - **`crates/ui/src/session.rs`** — `PaneSessionSnapshot.scrollback_id: Option<String>`
    (`#[serde(default)]`, no version bump — old snapshots restore w/o
    scrollback); `RestoreAction::LocalWorkspace` carries `scrollback_ids`. +1 test.
  - **`crates/ui/src/workspace.rs`** — `PaneEntry.scrollback_id`; `spawn_session`
    gained `replay_scrollback_id` + returns the id (loads persisted scrollback
    when restoring); `snapshot_workspace_tab` serializes + `scrollback_save`s
    each local pane on every capture (30s timer + quit); `restore_local_workspace`
    threads the ids through; `retire_pane` → `scrollback_delete`; new
    `cleanup_scrollback` runs once at end of `Workspace::new`.
  - **`crates/ui/src/app_shell.rs`** — quit hook wipes all scrollback via
    `scrollback_cleanup(&[], None)` when `session_restore` is off.
- Verify: `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (backend 161→167, terminal 64→67, ui 183→184),
  `cargo fmt --all --check` — all green.
- **Known gaps / for later:**
  - Scrollback is persisted as plain text (no colors) — matches the task note
    ("Zellen ohne überflüssige Ansi"); the reference keeps SGR via xterm's
    SerializeAddon. Revisit if colored replay is wanted.
  - No dormant-ring / background-tab buffering (reference `flushDormantScrollback`)
    — our sessions never pause, so the emulator always holds current history;
    the 30s capture covers force-quit.
  - Manual in-place shell restart (`SessionHandle::restart`) reuses the stored
    `SessionOptions`, so it would re-replay the original scrollback. Minor;
    not wired to persistence. Noted in bugs_and_fixes.
  - Alt-screen quit: scrollback capture is skipped entirely while a TUI holds
    the alt screen (returns `""`, existing file left as-is). No primary-grid
    extraction (alacritty doesn't expose the inactive grid cleanly).

## Previous Session: 2026-09-02 (T14-001 — Session persistence: tabs/layout)

### What Was Done
- **T14-001 ✅ Done.** Restore the previous tabs + split-pane layout on
  restart. Port of reference `src/modules/session/` (`types.ts` / `capture.ts`
  / `restore.ts` / `store.ts`).
  - **`crates/ui/src/session.rs`** (new) — pure model + decision layer, no
    GPUI. `SessionSnapshot { version, saved_at, active_tab_index, tabs }`;
    `TabSnapshot` enum (`Home` / `Workspace` / `Editor` / `Preview` / `Sftp`;
    transient AiDiff/Git* kinds are not persisted, like the reference skipping
    `ai-diff`). `WorkspaceTabSnapshot` carries `custom_title`, the serde-able
    `WorkspaceLayout` (structure + ratios + active leaf — already `Serialize`
    from T04-002) and a per-leaf `Vec<PaneSessionSnapshot>` (`Local`/`Ssh`,
    cwd, host_id) in `layout.leaves()` order. `plan_restore(snapshot,
    host_exists, file_exists, alloc_pane) -> Vec<RestoreAction>` is the
    testable decision fn: missing file / deleted host → `RestoreAction::Skip`;
    single-pane SSH tab → `SshWorkspace` (lazy reconnect); multi-pane →
    `LocalWorkspace` with a fresh-id-remapped layout (`remap_layout`).
    Persistence to `<data_dir>/labonair/session.json` (`load_snapshot` /
    `save_snapshot` / `clear_snapshot`, version-checked, stale file deleted).
    +7 unit tests.
  - **`crates/backend/.../settings/preferences.rs`** — new
    `session_restore: bool` (default **true**). +1 assertion.
  - **`crates/ui/src/settings.rs`** — new General FIELDS row `sessionRestore`
    (Switch). Works through the existing generic `set_value` path.
  - **`crates/ui/src/workspace.rs`** — `Workspace::new` gained an
    `Option<SessionSnapshot>` arg: if present it calls the new
    `restore_session()` (plan + execute: `open_workspace` / `connect_host` /
    `open_file` / `open_sftp`, re-spawn one PTY per leaf via
    `restore_local_workspace`, then re-activate the snapshot's active tab),
    else falls back to the old Home + terminal bootstrap. New
    `session_snapshot(&self, cx)` capture. New `_session_save` task writes a
    snapshot every 30s (`SESSION_SAVE_INTERVAL`) when `session_restore` is on
    (covers force-quit).
  - **`crates/ui/src/hosts.rs`** — new `HostManagerView::host_ids()`.
  - **`crates/ui/src/app_shell.rs`** — loads the snapshot up-front
    (`preferences_load().session_restore.then(load_snapshot)`) and passes it to
    `Workspace::new`; the `on_window_should_close` hook now also captures
    (`session_snapshot` → `save_snapshot`) or `clear_snapshot()`s when the pref
    is off.
- Verify: `cargo check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace` (ui 176→183, backend 161 unchanged),
  `cargo fmt --all --check` — all green.
- **Known gaps / for later:**
  - Preview and Git* tabs are captured-skipped (Preview) / not captured
    (Git*) — those tab kinds have no real workspace implementation yet
    (placeholders), so persisting them would be speculative. Revisit when the
    web-preview replacement / git-graph tabs land.
  - Multi-pane workspace tabs that contained an SSH pane are restored as
    all-local terminals (matches the reference's `kind:"local"` fallback for
    panes it can't reconnect). Single-pane SSH tabs reconnect via
    `connect_host` (already non-blocking, shows "Connecting…").
  - No unsaved-editor "ask before quit" dialog on the capture path yet
    (task ⚠); untitled editors are simply not persisted. Dirty *named*
    editors are re-opened from disk (unsaved buffer content is not stored).
  - `restore_session` runs inside `Workspace::new`; `connect_host` /
    `open_sftp` there spawn async tasks — fine, non-blocking.
- **Next task:** Phase 13 is complete. Next is **Phase 14 — Testing & Polish**,
  starting with **T15-001** — Visual parity (`tasks/phase-14-testing/`).

### (previous) T13-004 — Shortcut configuration

### What Was Done
- **T13-004 ✅ Done.** Persistent, user-editable keyboard-shortcut bindings
  with conflict detection, immediate effect and native-menu sync.
  - **`crates/backend/.../settings/preferences.rs`** — new
    `keybinds: BTreeMap<String,String>` field (slug → keystroke; `""` =
    disabled; absent = built-in default; empty map = fresh install on
    defaults). +1 test (`keybinds_default_empty_and_roundtrip`).
  - **`crates/ui/src/command_palette.rs`** — data layer:
    `shortcut_slug` / `shortcut_from_slug` (ids 1:1 with the reference
    `shortcuts.ts` string literals), `pub type KeybindMap`,
    `effective_binding(id, overrides)` (`None` = disabled),
    `resolve_conflict(binding, exclude, overrides)` — override-aware port of
    `useKeybindsStore` conflict check (still blocks `RESERVED_ACCELERATORS`).
    +4 tests.
  - **`crates/ui/src/menu.rs`** — `bindings()` → `bindings(&KeybindMap)`;
    the 18 rebindable shortcuts resolve through `effective_binding` (macro
    `rebind!`), 7 fixed/OS-reserved accelerators (Save, NewSshConnection,
    ToggleFullScreen, OpenSettings `cmd-,`, Minimize, Quit, HideApp) stay
    hardcoded. New `pub fn apply_keybinds(cx, &KeybindMap)` =
    `clear_key_bindings()` + `bind_keys(bindings(kb))`. `menu::init` **no
    longer binds keys** (AppShell owns application, and runs first). +1 test.
  - **`crates/ui/src/app_shell.rs`** — startup applies
    `menu::apply_keybinds(cx, &prefs.keybinds)` right after the theme/font
    prefs push.
  - **`crates/ui/src/settings.rs`** — new **"Keyboard Shortcuts"** category
    (custom pane like Appearance): every `SHORTCUTS` row with its effective
    binding, click-to-record (`recording: Option<ShortcutId>`,
    `on_key`→`record_key` captures `Keystroke::unparse()`, requires a
    non-shift modifier, Esc cancels), per-row **Reset** (shown when
    overridden), **Reset all**, and a local search filter (label + slug).
    Conflict → inline banner with **Overwrite** (gives the combo to the new
    shortcut and unbinds the previous owner — no silent double-binding) /
    **Cancel**. Reserved-accelerator capture → error toast, refused. Pure
    helpers `capture_keybind` / `overwrite_keybind` + `KbCapture` enum for
    testability. `set_pref("keybinds", …)` calls `menu::apply_keybinds` so
    changes are live with no restart; GPUI re-derives the menu accelerators
    from the same keymap. +5 tests (incl. gpui `keybinds_persist_and_reset`).
- Verify: `cargo check --workspace`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace`, `cargo fmt --all --check` — all
  green. backend **160 → 161**, ui **166 → 176** (command_palette +4,
  settings +5, menu +1), ai 75 unchanged.
- **Known gaps / for later:** the non-menu shortcuts (`tab.selectTab1..9`,
  `pane.focusNext`, `view.zenMode`, `bookmarks.open`) are listed and
  rebindable + persisted, but have no runtime dispatch yet (they had none
  before this task either) — they light up when their feature/phase wires a
  menu action. Live key-capture shows the final chord only (no incremental
  modifier preview). No cross-window keybind-changed event bus (single
  window; `apply_keybinds` is called directly).
- **Next task:** first unstarted task after phase-12 — check `tasks/ROADMAP.md`
  (Phase 13: Session-Persistence & Scrollback).

### (previous) T13-003 — Terminal & Editor settings

### What Was Done
- **T13-003 ✅ Done.** Terminal / Editor preference fields wired so changes
  take effect live and persist.
  - **`crates/backend/.../settings/preferences.rs`** — new fields
    `terminal_opacity` (100), `editor_relative_line_numbers`, `editor_vim_mode`,
    `editor_theme` ("auto"); `Preferences::editor_prefs()` projects the editor
    fields onto the existing `settings::editor::EditorPrefs` (keeps the
    persisted `hlsearch`/`incsearch`/`smartcase` Vim search opts). +2 tests.
  - **`crates/ui/src/settings.rs`** — new `GlobalPreferences(Preferences)` gpui
    `Global`, republished by `PreferencesStore::set_value` (and a startup
    `publish_global`). New FIELDS rows: `terminalOpacity`,
    `editorRelativeLineNumbers`, `editorVimMode`, `editorTheme` (Select of the
    10 `EditorThemeId` slugs). `apply_prefs_to_theme()` / `font_overrides_from()`
    push font + editor-syntax settings into `ThemeStore` at startup and on every
    `set_pref`. +2 tests.
  - **`crates/ui/src/theme.rs`** — `ThemeStore` gained `FontOverrides`
    (app / editor / terminal family + size, empty/0 = keep theme value),
    applied on top of the built-in *and* imported themes. New `custom_base`
    holds the pristine imported theme so `set_font_overrides()` can rebuild the
    overridden `custom` without baking values in destructively
    (`rebuild_custom()` / `reresolve_custom()`). New `ui_font_size()`. +1 test.
  - **`crates/ui/src/editor.rs`** — `EditorView` observes `GlobalPreferences`;
    `apply_prefs()` reconciles the Vim layer (on/off/opts) *without touching the
    document buffer*; `indent_unit()` makes a Tab press honour
    `editor_tab_size` / `editor_indent_with_tabs`; the gutter line-number
    visibility now follows `editor_line_numbers` / `editor_relative_line_numbers`
    (Vim `:set number` still overrides while Vim mode is on). +1 test.
  - **`crates/terminal`** — new `EmulatorConfig` + `TerminalEmulator::new_with`;
    `SessionOptions` gained `scrollback` / `cursor_shape` / `cursor_blink`,
    threaded into the alacritty `Config` (`scrolling_history` +
    `default_cursor_style`). Applies to **new** sessions only. +1 test.
  - **`crates/ui/src/workspace.rs`** — `spawn_session` reads
    `GlobalPreferences` for the shell program, scrollback depth and cursor
    shape/blink of each new local terminal.
  - **`crates/ui/src/terminal.rs`** — copy-on-select is now gated by
    `terminal_copy_on_select` (was unconditional); `terminal_opacity < 100`
    dims the root background and drops the fill on cells that keep the default
    background so the app wallpaper shows through.
  - **`crates/ui/src/app_shell.rs`** — root element gets
    `.font(ui_font()).text_size(ui_font_size())` so the UI font family + size
    settings apply app-wide.
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` — all green. backend **158 → 160**,
  ui **163 → 166**, terminal **63 → 64**, ai 75 unchanged.
- **Known gaps / for later:** editor soft-wrap is not implemented — the Phase 5
  editor is a fixed-line-height absolute grid; `editor_word_wrap` is stored +
  shown in settings but has no renderer effect yet. Audible `terminal_bell` is
  a stored pref only (no sound hooked up). Scrollback / cursor-style changes
  apply to newly-spawned sessions, not already-running PTYs.
- **Next task:** T13-004 — Shortcut configuration.

### (previous) T13-002 — Appearance & theme settings

### What Was Done
- **T13-002 ✅ Done.** Appearance pane in the settings modal, wiring the
  deferred T02-003 theme import/export into the UI, plus background-image and
  UI-font controls.
  - **`crates/ui/src/settings.rs`** — `SettingsView` now takes the
    `Entity<BackgroundStore>` and renders `"Appearance"` as a custom pane
    (like `AGENT_BRIDGE`):
    - **Color scheme** — System / Light / Dark buttons → `set_pref("theme")`
      (already flows to `ThemeStore::set_preference` + persists).
    - **Themes** — list of built-in "Labonair" + user themes scanned from
      `config_dir()/themes/*.json`; per-row Activate / Delete; `Import theme…`
      (native picker → copy into themes dir → `ThemeStore::import_theme_file`)
      and `Export active theme…` (`active_theme_file(name).to_json()` →
      `prompt_for_new_path`). `active_theme_id` tracks the selection; `default`
      clears the custom override. Warnings from import surface as a toast.
    - **Background image** — None tile + per-image tiles (select / delete) +
      `+ Add` (`BackgroundStore::prompt_and_import`); when an image is active,
      steppers for wallpaper opacity / blur / tint opacity drive
      `BackgroundStore` (its own persistence in `labonair-settings.json`).
    - **Typography** — `appFontFamily` (new pref, Text), `appFontSize` (Int),
      `reduceMotion` (Switch) rendered via the generic `render_field`.
    - Theme dir ops extracted to dir-parameterised free fns (`scan_themes`,
      `read_theme_file_in`, `save_theme_file_in`, `delete_theme_in`) so tests
      run against a tempdir, never the real `~/.config`. 6 new tests
      (slugify, trim_ext, scan skips junk / never shadows built-in `default`,
      save→read→delete roundtrip + built-in delete guard, appFontFamily
      persistence).
  - **`crates/backend/src/modules/settings/preferences.rs`** — added
    `app_font_family: String` (default `""` = system) in the Appearance group.
  - **`crates/ui/src/app_shell.rs`** — passes `background.clone()` into
    `SettingsView::new`.
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` — all green (ui lib 168 tests).
- **Known gaps / for later:** background *color* tint picker omitted (GPUI has
  no colour input; tint opacity still adjustable, colour stays default black).
  `appFontFamily` / `appFontSize` persist but are not yet pushed into the
  live `ThemeStore` typography (runtime UI-font re-application belongs to
  T13-003 / font work). Theme *marketplace* (remote index browse/download)
  from `reference-src` `ThemeMarketplace.tsx` is not part of this task.
- **Next task:** T13-003 — Terminal & Editor settings.

### (previous) T13-001 — Settings structure & preferences

### What Was Done
- **T13-001 ✅ Done.** Central preferences model + store + settings window,
  and the AI Agent Bridge (MCP) settings pane that T11-006 / T12-001 deferred
  here.
  - **`crates/backend/src/modules/settings/preferences.rs`** (new, 6 tests) —
    typed `Preferences` struct (~33 fields, grouped General / Appearance /
    Terminal / Editor / File Manager / Command Palette / Source Control / AI),
    typed enums `ThemePref` / `StartupTab` / `CursorStyle` / `PaletteSearchMode`
    (serde tokens match the reference `store.ts`). Stored as a `preferences`
    object in the shared `labonair-settings.json` (same file editor/mcp/
    bar-items use). `#[serde(default)]` per field → missing fields fall back
    field-by-field. **Corruption defence:** a settings file that isn't a valid
    JSON object is renamed to `labonair-settings.json.bak` and defaults load
    (never crash / clobber). Public `preferences_load` / `preferences_save`
    + `preferences_load_from` / `preferences_save_to` (explicit dir, for tests).
    `settings/mod.rs` gained `pub mod preferences;`.
  - **`crates/ui/src/settings.rs`** (new, 5 tests):
    - `PreferencesStore` GPUI entity — holds `Preferences`, generic
      key-addressed `value(key)` / `set_value(key, json)` (serialize model →
      swap one key → deserialize back to validate; wrong-typed value is
      rejected + logged, not stored). Persists + `cx.notify()` only when the
      value parsed AND changed. `with_dir()` ctor keeps tests off the real
      settings file.
    - Table-driven field defs: `FieldKind` (Switch / Int{min,max,step} /
      Select(&[token]) / Text), static `FIELDS` (33 rows) + `CATEGORIES`.
      Select options ARE the serialized token strings (test-pinned).
    - `SettingsView` — modal overlay (command-palette pattern; GPUI has no
      child-window story wired). Category rail + search box (flat filter across
      all categories) + scrollable field list. Switch = toggle, Int = `[−] n
      [+]`, Select = click-cycles, Text = click-to-edit hand-rolled key buffer
      (Enter commits / Esc cancels). Single `FocusHandle`, one `on_key_down`
      state machine (Search vs EditField).
    - **AI Agent Bridge pane** (`AGENT_BRIDGE` category, custom render): enable
      switch, port, max-timeout, auto-revoke, notify-on-activity, "Regenerate
      token" button, and the `claude mcp add … --header "Authorization: Bearer
      …"` setup command + Copy (shown only when enabled + token present). Reads
      `McpPrefs` / `mcp_get_status`; writes persist via `mcp_prefs_save` **and**
      spawn the matching `mcp_set_*` on tokio (mirrors reference
      `ConnectionsSection.tsx` `AgentBridgeSection`).
  - **`crates/ui/src/app_shell.rs`** — owns `prefs: Entity<PreferencesStore>`
    + `settings: Entity<SettingsView>`. At startup the persisted `theme` pref
    is pushed into `ThemeStore::set_preference`; `SettingsView::set_pref`
    re-pushes it on every change (the one value modules can't observe
    generically yet). New `act_open_settings` handler bound to `menu::
    OpenSettings` (so `cmd-,`, the menu item, and the `OpenSettings`
    command-palette entry all toggle the modal). `pub fn preferences()`
    accessor. Modal rendered as a root child.
  - **`crates/ui/src/menu.rs`** — removed the `OpenSettings` "arrives in a
    later phase" toast stub (now handled by `AppShell`).
  - `lib.rs` — `pub mod settings` + re-exports (`PreferencesStore`,
    `SettingsView`, `FieldDef`, `FieldKind`, `FIELDS`, `SETTINGS_CATEGORIES`).
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` — all green. backend **153 → 158**
  (+5), ui **152 → 157** (+5), ai 75 unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left untouched / uncommitted.
- **Next: T13-002 — Appearance & Theme settings**
  (`tasks/phase-12-settings/T13-002-appearance-theme-settings.md`). Then
  T13-003 (wires Terminal/Editor to actually consume `PreferencesStore` — only
  `theme` is wired so far) and T13-004 (shortcut config).

### Notes / Quirks (T13-001)
- Only `theme` is wired into a live module. Terminal/editor still read their
  own config (`settings::editor` key, terminal defaults). The roadmap splits
  that into T13-002 / T13-003 — `PreferencesStore` is the store they'll read
  from; `AppShell` already holds it and `preferences()` exposes it.
- `Render::render` must return `impl IntoElement` whose concrete type is stable
  across `return` branches — mixing `Div` (early `return div()`) with
  `Stateful<Div>` fails; use `.into_any_element()` on **both** branches (a bare
  `-> gpui::AnyElement` return type trips `refining_impl_trait`).
- `on_click` / `overflow_y_scroll` need a `.id(...)` (stateful element) first —
  same rule as elsewhere.
- Generic pref get/set via `serde_json::to_value` / `from_value` round-trip is
  the cheap way to avoid a giant per-field match; `Select` options must be the
  exact serde token (`"host-manager"`, `"startsWith"`, …), pinned by a test.
- `PreferencesStore` tests must use `with_dir(temp)` — `preferences_save()`
  writes the real `~/.config/labonair/labonair-settings.json`.

---
## Prev Session: 2026-09-02 (T12-002 — Command palette & shortcut system)

### What Was Done
- **T12-002 ✅ Done.** New `crates/ui/src/command_palette.rs` (~880 lines incl.
  11 tests) — port of `reference-src/src/modules/command-palette/*` +
  `.../shortcuts/*`.
  - **Shortcut table (`shortcuts.ts` port):** `ShortcutId` (30 variants),
    `ShortcutGroup`, static `SHORTCUTS` table with cheat-sheet display tokens
    (`keys`) + GPUI keystroke string (`binding`). Helpers `shortcuts()`,
    `shortcut(id)`, `shortcut_keys(id)`.
  - **Conflict detection (`conflictDetector.ts` port):** `find_conflict(binding,
    exclude) -> Option<Conflict>` with modifier-order-insensitive `normalize()`
    (handles the `cmd--` minus-key case). `RESERVED_ACCELERATORS` = Settings
    (`cmd-,`) + New SSH Connection (`cmd-shift-n`). Note: `cmd-k` is NOT reserved
    here — it is the real rebindable `ShortcutId::ShortcutsOpen` (the reference's
    reservedAccelerators.ts / shortcuts.ts disagreed on ⌘K vs ⌘?; resolved in
    favour of a single rebindable entry to avoid a self-conflict).
  - **Command registry (`useCommandRegistry` port):** `CommandContext` (Terminal
    / Editor / Sftp / Home / SshTerminal), `CommandId` (28 variants), static
    `COMMANDS` table (id/title/section/contexts/shortcut). `available(ctx)`
    implements the reference `filterByContext` (no-context cmds always show;
    context-scoped only when active). `search(query, ctx)` title+section
    substring filter. `command_for_shortcut(id)`, `context_of(kind, is_ssh)`.
    Domains covered: Layout, Tab Actions, Terminal, Connections, Search, View,
    AI, Snippets, Source Control, Editor, Application.
  - **Palette view:** `CommandPalette` GPUI modal overlay (`Entity`,
    `EventEmitter<PaletteEvent>`). Cmd+P toggles (bound in `menu.rs`). Dimmed
    backdrop + centered card, search input line, sectioned result list, arrow
    up/down wraparound, Enter runs, Esc closes / pops sub-page. `SwitchTab`
    command pushes a follow-up page listing open tabs (argument-input level).
    Hand-rolled key buffer (same pattern as `snippets.rs` / `git.rs`).
  - **Execution:** palette emits `PaletteEvent::{Run(CommandId), SwitchToTab(id)}`
    → `AppShell` queues into `pending_commands`, drained in `render` (where
    `&mut Window` exists — same trick `Workspace` uses for window-less subs).
    `AppShell::run_palette_command` dispatches the matching GPUI menu action for
    most commands (identical path to the native menu — stub-now-wire-later, like
    menu.rs) and services SwitchTab / DuplicateTab / CloseOtherTabs /
    ClearTerminal / panel-focus (Ai, Snippets, GitGraph, SourceControl) directly.
    `FormatDocument` is a registered-but-inert stub until the editor formatter
    phase (documented in code).
  - **New `Workspace` methods:** `active_context`, `close_other_tabs`,
    `duplicate_active_tab` (terminal re-spawn / editor re-open), `clear_active_terminal`.
  - **`menu.rs`:** new `CommandPalette` action + `cmd-p` binding + "Command
    Palette…" item in the Window menu. `bindings_parse` test count 24 → 25.
  - **`lib.rs`:** `pub mod command_palette` + re-exports (`CommandPalette`,
    `CommandId`, `PaletteEvent`, `ShortcutId`, `shortcuts`, `shortcut`,
    `find_conflict`, `command_for_shortcut`).
- Verify: `cargo check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo fmt --all --check` — all green. ui
  **141 → 152** (+11), other crates unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left untouched / uncommitted.
- **Next: T12-003** (or next unstarted task in `tasks/phase-11-snippets-palette/`
  / then phase-12). Check ROADMAP.

### Notes / Quirks (T12-002)
- `gpui::prelude::FluentBuilder` must be imported for `.when()` on
  `Stateful<Div>` (a stateful/`.id()` element) — plain `Styled` is not enough.
- `gpui::Keystroke::parse` (`platform/keystroke.rs`) is the cheap way to
  validate a binding string in a test without `KeyBinding::new` (which panics).
- `window.dispatch_action(Box<dyn Action>, cx)` exists on gpui 0.2.2 `Window` —
  lets the palette reuse the exact menu-action code path.

---
## Previous Session: 2026-09-02 (T12-001 — Command-Snippets system)

### What Was Done
- **T12-001 ✅ Done.** New `crates/ui/src/snippets.rs` (~1970 lines incl. 15
  tests) — `SnippetsView` GPUI sidebar panel, port of
  `reference-src/src/modules/snippets/*`. Backend (`modules/snippets/db.rs` CRUD
  + `exec.rs` local/SSH run + `snippet_run_cancel`) already existed from the
  T01-002 bulk port; this task built everything above it.
  - **Pure helpers (ported test suites):** `extract_snippet_variables` /
    `substitute_snippet_variables` (hand-rolled `${NAME}` / `${NAME:-default}`
    scanner replacing the JS regex — no `regex` dep in `crates/ui`; full
    `SHELL_RESERVED_VAR_NAMES` list; raw textual substitution, **no
    shell-quoting**, matching the reference `snippetVariables.ts` exactly),
    `parse_tags` / `serialize_tags` (port of `snippetUtils.ts`). All 13 ported
    JS assertions reproduced + a run-log ring-buffer test (newest-first, cap 50).
  - **Panel:** grouped snippet list (collapsible groups + item count, "Other"
    for ungrouped), search toggle/filter (name/command/description), row actions
    RUN / log / ▲▼ reorder / edit / duplicate / delete, "+ Add group" inline
    field, per-group delete. Create/edit form (name, description, command,
    group chips, Local/SSH target toggle, host chips, Terminal/Silent/Inject
    mode toggle, working-dir). Hand-rolled key-buffer text fields (same pattern
    as `git.rs`), `Field` enum routed through `on_key`.
  - **Variable prompt modal:** shown before a run when the command has
    `${VAR}`s; per-var value fields (default pre-filled), Tab/Enter to advance,
    Enter on last field runs.
  - **Host picker modal:** shown for SSH snippets with `host_id = None` ("ask at
    runtime"); missing-host (`host_id` set but gone) → error toast, no run.
  - **Execution:** `inject` → `Workspace::inject_into_active_terminal`;
    `terminal` local → new `Workspace::run_snippet_local` (spawns a tab, writes
    `cmd\n` to the handle); `terminal` SSH → new
    `Workspace::run_snippet_ssh_terminal` (connects, queues the command in
    `Workspace::pending_snippet_ssh`, flushed on `SshSessionEstablished`);
    `silent` → `snippet_run_local` / `snippet_run_ssh` on `tokio.spawn`, output
    streamed into the log drawer via a bus subscription
    (`snippet_run_output` / `snippet_run_done` raw events → mpsc → 60ms cx.spawn
    poll loop). Silent SSH needs a live session for the host
    (`Workspace::ssh_session_for_host`) — else logs an error line, same as the
    reference.
  - **Log drawer:** bottom drawer, run tab list + selected-run output (stdout/
    stderr colouring, `[exit N]` / cancelled markers), Cancel (running run),
    Clear, Close.
  - Wired into `SidebarPanel::Snippets` in `app_shell.rs` (variant + ✂ glyph
    already existed); new `snippets: Entity<SnippetsView>` field.
- Verify: `cargo check --workspace`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo fmt --all --check` — all green.
  ui **126 → 141** (+15), backend 153 / ai 75 unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left untouched / uncommitted.
- **Next: T12-002 — Command-Palette**
  (`tasks/phase-11-snippets-palette/T12-002-command-palette.md`).

### Notes / Quirks (T12-001)
- The reference `substituteSnippetVariables` does **no** escaping/quoting — it
  is a raw `String.replace`. The task text asks for "defensiv escapen" but the
  ported test suite pins the raw behaviour (`echo ${NAME}` + `{NAME:"world"}` →
  `echo world`), so parity wins. Reserved names are never extracted, so never
  substituted — they pass straight through to the real shell.
- No `regex` dependency in `crates/ui`; `scan_variables` is a byte-scan
  replicating `/\$\{([A-Z_][A-Z0-9_]*)(?::-([^}]*))?\}/g` (first char
  `[A-Z_]`, tail `[A-Z0-9_]`, optional `:-default` with no `}` in the default).
- SSH "terminal" mode writes the command into the tab's PTY **after**
  `SshSessionEstablished` (queued in `Workspace::pending_snippet_ssh` keyed by
  `ssh_id`) — writing before the remote PTY channel is open silently drops it.
- Local "terminal" mode writes `cmd\n` to the fresh session handle immediately;
  the kernel PTY buffers it until the shell reads, so no artificial delay.
- `SnippetsView` owns its own bus subscription (like `Workspace`) — it is an
  `AppShell` child, not a `Workspace` child, so it can't reuse the workspace's
  forwarding channel.
- Reorder (▲▼) renumbers the whole group 0..n via `snippets_reorder`; there is
  no drag-and-drop (GPUI DnD for list rows isn't wired anywhere yet — same
  situation as the explorer's file DnD which is minimal).
- The panel is untested at the view level (GPUI views aren't unit-testable
  here); the 15 tests cover the pure variable/tag/run-log logic — the seam that
  actually carries risk.

---

## Prev Session: 2026-09-02 (T11-006 — MCP-Bridge Grants UI & Settings)

### What Was Done
- **T11-006 ✅ Done.** UI + persistence for the MCP bridge. Backend enforcement
  (grant refusal, live-revoke on `hosts_update`, `block_agent_access`
  column/migration) was already in place from the T01-002 bulk port; this task
  added everything above it.
  - `crates/backend/src/modules/settings/mcp.rs` (new, 3 tests) — `McpPrefs`
    (`bridge_enabled` / `bridge_port` / `max_command_timeout_secs` /
    `auto_revoke_minutes` / `notify_on_activity`) load/save into the shared
    `labonair-settings.json` under an `mcp` key, same pattern as
    `settings::editor`. This is the **load-bearing** config — `McpState` has no
    persistence of its own. `settings/mod.rs` gained `pub mod mcp;`.
  - `crates/ui/src/agent_access.rs` (new, 2 gpui tests) — `AgentAccessStore`
    GPUI entity, port of the reference `agentAccessStore.ts`: `BTreeMap<tab_id,
    AgentAccessEntry>` local mirror of `McpState.grants` + `bridge_enabled` /
    `notify_on_activity` flags. `set_grant()` mirrors optimistically then runs
    `mcp_set_session_grant` on `tokio.spawn`, rolling the mirror back + pushing
    an error toast if the backend rejects the grant (host-blocked). `hydrate()`
    / `set_bridge_enabled()` / `set_notify_on_activity()` / `clear_local()`.
    Exported from `lib.rs` (`AgentAccessEntry`, `AgentAccessStore`).
  - `crates/ui/src/workspace.rs`:
    - new `agent_access: Entity<AgentAccessStore>` field + `Workspace::new`
      param (call site in `app_shell.rs` updated), observed.
    - `mcp_grant_target(tab_id)` → `Option<McpGrantTarget>` (new type alias for
      the `(session_id, label, kind, host_id, local_pty_id)` 5-tuple — a bare
      5-tuple return trips `clippy::type_complexity`). SSH tabs → `SessionKind::
      Ssh` + `ssh_id` + `host_id`; local `Workspace` tabs → `SessionKind::Local`
      + `local_pty_id` (from `TabData.session_id as u32`).
    - tab context menu: "Grant AI Agent Access" toggle item (`✓` prefix when
      granted), shown only when the bridge is enabled and the tab is an SSH or
      local terminal tab.
    - `handle_ssh_event`: `McpActivity` now pushes an info toast
      (`"Agent: {action} — {label}"`) when `notify_on_activity` is on (was just
      `tracing::debug!`); new `McpGrantExpired { tab_id }` arm clears the local
      mirror (auto-revoke sweep / host-block).
    - `retire_tab` gained a `cx` param (4 call sites updated) and now revokes
      the backend grant + clears the mirror for any granted tab being closed.
    - new pub `reveal_tab(id, window, cx)` for the badge's "jump to tab".
  - `crates/ui/src/app_shell.rs`:
    - owns the shared `agent_access: Entity<AgentAccessStore>` + `agent_badge_open`.
    - startup block: `mcp_prefs_load()` → `AgentAccessStore::hydrate` + a
      `tokio.spawn` that pushes port / max-timeout / auto-revoke into `McpState`
      and, if `bridge_enabled`, calls `mcp_set_enabled(true, …)` (mirrors the
      reference `useMcpTabBridge.ts` re-sync effect).
    - `render_agent_badge` — header badge (shield glyph + count pill), hidden
      unless the bridge is on AND ≥1 grant; popover lists granted tabs with a
      jump button + a revoke `✕`. Port of `AgentAccessBadge.tsx`.
  - `crates/ui/src/hosts.rs`: `HostForm` gained `block_agent_access` (init from
    `Host`, blank default `false`), an "AI Agent Access" allow/block toggle
    button in `render_form`, and `Some(block_agent_access)` is now passed to
    `hosts_create` / `hosts_update` (was `None`).
- **Partially blocked on T13-001.** There is still no Settings *window* in the
  Rust app, so the visible "AI Agent Bridge (MCP)" settings pane (enable switch,
  `claude mcp add …` setup command + Copy, Regenerate-token button, port /
  max-timeout / auto-revoke number inputs, notify toggle) is **deferred**. All
  of its plumbing is done: `mcp_set_port` / `mcp_set_max_command_timeout_secs` /
  `mcp_set_auto_revoke_minutes` / `mcp_set_enabled` / `mcp_regenerate_token` /
  `mcp_get_status` exist; `McpPrefs` load/save exists; `AgentAccessStore`
  mirrors `bridge_enabled` / `notify_on_activity`. T13-001 just needs to build
  the window and port `ConnectionsSection.tsx`'s `AgentBridgeSection`.
- Verify: `cargo check --workspace`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo fmt --all --check` — all green.
  backend **150 → 153** (+3 `McpPrefs`), ui **124 → 126** (+2 `AgentAccessStore`),
  ai 75 unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left untouched / uncommitted.
- **Phase 10 (AI-Chat) is complete.** Next: **T12-001 — Befehl-Snippets-System**
  (`tasks/phase-11-snippets-palette/T12-001-snippets-system.md`).

### Notes / Quirks (T11-006)
- Grant enforcement is **server-side** (T11-005 backend). `AgentAccessStore` is
  a comfort mirror only — never trust it for a security decision.
- Local-tab grants use `TabData.session_id as u32` as `local_pty_id`. The
  reference `open_tab` tool is SSH-only anyway; local grants only matter for the
  manual context-menu toggle + `close_tab`.
- `AgentAccessStore::set_grant` runs the backend call on `tokio.spawn` because
  `mcp_set_session_grant` does a blocking sqlite `block_agent_access` lookup —
  it's `async` but never actually yields.
- The header badge lives in `AppShell::render_header` (our top bar), not a
  separate header component — the reference `AgentAccessBadge` sits next to the
  other bar badges; same spot here. `badgesAlwaysVisible` preference not ported
  → badge simply hides when empty.
- gpui test gotcha: `cx.new()` inside `cx.update(|cx| …)` needs
  `use gpui::AppContext;` imported.

---

## Prev Session: 2026-09-01 (T11-005 — MCP-Bridge Server)

### What Was Done
- **T11-005 ✅ Done.** The backend `mcp` module (`crates/backend/src/modules/mcp/`
  — `server.rs` rmcp/axum Streamable-HTTP server + 6 tools, `osc133.rs` vte
  parser with 6 tests, `mod.rs` `McpState`/grants/settings/auto-revoke sweeper)
  was already fully ported wholesale in T01-002 and is wired into `App`
  (`app.mcp`, sweeper spawned at startup). SSH + local PTY sessions already
  carry the `agent_tap` broadcast (T07-001 / T03-005). This task added the
  **missing GPUI coordinator** — the half of the reference `useMcpTabBridge.ts`
  that drives real tab actions:
  - `crates/backend/src/events.rs` — `AppEvent::McpOpenTabRequest.path` made
    `Option<String>` (server emits only `{request_id, host_id}` — it was
    silently failing `from_raw` before, a real bug); `McpCloseTabRequest` gained
    `session_id: Option<String>` (server emits it); new variants
    `McpServerError { message }` + `McpActivity { label, action, detail }` with
    event-name + `from_raw` mappings. 3 new tests (backend 148 → 150).
  - `crates/ui/src/workspace.rs` — `McpTabOp` queue (`pending_mcp`), drained in
    `render` (needs `&mut Window`). `handle_ssh_event` now handles the 4 new MCP
    events:
    - `McpOpenTabRequest` → `mcp_open_tab`: verify host exists → `connect_host`
      (now returns `Option<String>` = the new tab's `ssh_id`) → auto-grant via
      `mcp_set_session_grant(tab_id, ssh_id, true, …, SessionKind::Ssh, host_id)`
      → `mcp_tab_op_response(ok, session_id=ssh_id, tab_id)`. `host_id: None` →
      immediate error response.
    - `McpCloseTabRequest` → `mcp_close_tab`: find the SSH tab whose backend
      session == `session_id`, `do_close` it, respond ok/err on whether the tab
      is gone.
    - `McpServerError` → error toast via `notifications::notification_center`
      (backend already flipped `enabled=false`).
    - `McpActivity` → `tracing::debug!` for now (the preference-gated
      notification is T11-006).
  - `retire_tab` now revokes the MCP grant for any SSH tab being closed
    (grant keyed by `tab_id`, so `mcp_set_session_grant(tab_id, "", false, …)`),
    satisfying "Tab-Schließen widerruft den Grant".
- Verify: `cargo check --workspace`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo fmt --all --check` — all green.
  backend **150** (+2 MCP event tests, one of the two test fns adds 2 asserts),
  ui 124 / ai 75 unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left untouched / uncommitted.
- **Next: T11-006 — MCP-Bridge UI & Grants**
  (`tasks/phase-10-ai-chat/T11-006-mcp-bridge-ui-grants.md`) — Settings→AI Agent
  Bridge pane (enable/port/token/timeout/auto-revoke), the tab context-menu
  "Grant AI Agent Access" toggle + header badge, the `mcpNotifyOnActivity`
  preference that consumes `McpActivity`, and the local-tab grant path
  (`SessionKind::Local` + `local_pty_id`) + `McpGrantExpired` local-mirror
  clearing.

### Notes / Quirks (T11-005)
- The whole backend `mcp` module predated this task (T01-002 bulk port). Only
  the GPUI-side event coordinator + the `AppEvent` shape fixes were new work.
- **SSH grant `session_id` == the `ssh_id` UUID**, not the terminal registry's
  `u64` session id — that UUID is the key in `app.ssh` sessions, which
  `server.rs`'s `write_to_ssh_session` / `get_session_arc!` require. `SshTab`
  already stores it as `ssh_id`.
- **Local-tab MCP (`SessionKind::Local`, `local_pty_id`) is not wired** —
  `open_tab` in the reference is SSH-only anyway; `close_tab` / grants for local
  tabs need the local-PTY tab's `pty_id` plumbed from the workspace, which is
  T11-006's grant-UI work.
- `open_tab` grants **before** the SSH connection completes (matches the
  reference auto-grant: create tab → grant → respond, connection resolves
  async). `run_command` against it will just fail until the session is up.
- No new UI integration test — GPUI views aren't unit-testable here; the OSC133
  parser (6 tests) + the new `AppEvent::from_raw` MCP decode tests cover the
  seams. The rmcp server itself has no test (needs a live listener + a real
  granted SSH/local session).

---

## Prev Session: 2026-09-01 (T11-004 — Agent/Tool-System und Live-Bridge)

### What Was Done
- **T11-004 ✅ Done.** New `crates/ai/src/tools/` module — pure-Rust,
  framework-agnostic port of `reference-src/src/modules/ai/tools/*`, `agents/*`
  and `lib/{security,todos,useAiLiveBridge}.ts`. `crates/ai` now deps
  `labonair-backend` (no cycle).
  - `security.rs` — `check_readable` / `check_writable` (+ `_resolved`
    symlink-aware via `std::fs::canonicalize`), `check_shell_command`
    (blocks `rm -rf /`, `dd of=/dev/*`, `mkfs`/`fdisk`/`diskutil erase`,
    `--no-preserve-root`, + refuses any secret-file/dir reference),
    `check_destructive_command` (warning labels, non-blocking). Deny-list on
    **read AND write**; `..`/UNC/drive-letter/NTFS-ADS path normalization.
    Ported `security.test.ts` (12 tests).
  - `todos.rs` — `TodoStore` per chat session
    (`~/.config/labonair/labonair-todos.json`), `validate_todos` (≤1
    `in_progress`, non-empty titles).
  - `live_bridge.rs` — `LiveBridge` trait (lazy `cwd`/`workspace_root`/
    `terminal_context`/`active_ssh_tab_id`/`inject_into_active_pty`/
    `send_to_active_terminal`), `NoLiveBridge` default + `StaticLiveBridge`
    for tests, `terminal_context_block()` (`<terminal-context cwd=…>`),
    `resolve_path()` (relative → cwd).
  - `host.rs` — `ToolHost` trait (fs + shell). `NativeHost` → backend
    `fs::{file,grep,mutate,tree}` + `sh -c` with kill-on-timeout, off any UI
    thread. `ScratchHost` (std::fs + naive walk) for tests.
  - `registry.rs` — `Tool` trait (name / description / JSON-Schema /
    `needs_approval` / `run`), `ToolRegistry::builtin()` = **14 tools**:
    read_file, list_directory, write_file*, create_directory*, edit*,
    multi_edit*, grep, glob, run_command*, terminal_read, terminal_write*,
    suggest_command, todo_write, run_subagent (`*` = approval-gated).
    `ToolContext` carries a shared `Arc<Mutex<HashSet<String>>>` read_cache so
    the read-before-edit invariant survives the approval-gated follow-up turn.
    `ToolRegistry::read_only()` = the sub-agent subset.
  - `run.rs` — `ToolTurn`: `begin()` auto-executes read-only calls + queues
    gated ones; `resolve(id, approved)` executes (or records a clean
    rejection); `into_messages()` → `Role::Tool` result messages to re-send.
  - `subagent.rs` — `SUBAGENTS` catalog (explore / code-review / security /
    general — all read-only tools, no recursion), `SubagentRunner` trait +
    `NoopSubagentRunner`. (A model-driven runner is left as a follow-up.)
  - `sessions.rs` — `record_tool_result` (card → Done/Error + inserts a
    `Role::Tool` message after its owning assistant msg), `begin_continue`
    (fresh streaming assistant placeholder + full history for the next turn),
    `active_pending_tool_calls`.
  - `crates/ui/src/ai_chat.rs` — `AiChatStore` now:
    - passes `registry.tool_defs()` into `stream_chat` (was `Vec::new()`);
    - `spawn_stream(history)` extracted, shared by `send` + continuation;
    - `dispatch_tool_calls` after every finished turn: auto-runs read-only
      calls on `tokio::spawn_blocking` (result via `oneshot` + `cx.spawn`),
      records each into the store, and — when no gated calls remain — calls
      `begin_continue` + `spawn_stream` so **the run continues
      automatically**;
    - `resolve_tool_call(id, approved)` executes the approved tool off-thread
      then auto-continues; rejection records `"Rejected by user."` and still
      continues so the model can react. Falls back to the plain
      `SessionStore::resolve_tool_call` when no tracked pending call (keeps the
      store-driven test working).
    - `set_live_bridge(Arc<dyn LiveBridge>)` hook; default is `NoLiveBridge`.
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` — all green. **ai 45 → 75**
  (+30: security 8, todos 3, live_bridge 3, registry 8, run 5, subagent 1,
  sessions 1 [`tool_result_recorded_then_run_continues`], + host/glob covered
  via registry). ui **124** unchanged, backend **148** unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.
- **Next: T11-005 — MCP-Bridge-Server**
  (`tasks/phase-10-ai-chat/T11-005-mcp-bridge-server.md`).

### Notes / Quirks (T11-004)
- **No real workspace-backed `LiveBridge` yet.** The trait is `Send + Sync`
  with sync methods and no `cx`, so it can't read `Entity<Workspace>` at call
  time. Shipped `NoLiveBridge` + a `set_live_bridge` setter; the terminal
  tools + `terminal_context_block` are fully implemented/tested against
  `StaticLiveBridge`. Wiring: app-shell should push a `LiveSnapshot`
  (cwd + buffer tail) into an `Arc<Mutex<…>>` on every active-tab change and
  hand a bridge over that. Thin follow-up.
- **Sub-agent runner is `NoopSubagentRunner`** in the UI — the read-only
  catalog + tool dispatch + approval semantics are done and tested; an actual
  bounded model loop (`AiClient` + resolved target, ≤12 steps) is the missing
  piece, same shape as the main run.
- `run_command` is the single shell tool (reference has `bash_run` visible +
  `bash_run_headless`); it runs headless in the session cwd via
  `NativeHost::run_shell`. The visible-in-a-real-terminal variant needs the
  live bridge + `terminal_exec` wiring — later.
- With `NoLiveBridge`, `cwd()` is `None` → relative tool paths error and
  grep/glob need an explicit `root`. That's expected until the bridge lands.
- `crates/ai` deliberately gained a `labonair-backend` dep — checked: backend
  has no `ai` dep, no cycle. Build cost already paid (ui deps both).

---

## Prev Session: 2026-09-01 (T11-003 — Chat UI & streaming markdown)

### What Was Done
- **T11-003 ✅ Done.** GPUI chat panel rendering off the T11-002 `AiChatStore`.
  - `crates/ui/src/markdown.rs` (new, pure, 8 tests) — a small streaming-aware
    Markdown parser replacing the web `streamdown`. `parse_markdown` →
    `Vec<MdBlock>` (Heading/Paragraph/Code{lang,text,closed}/Bullets/Ordered/
    Quote/Rule/Table), `parse_inline` → `Vec<Inline>` (Text/Code/Bold/Italic/
    Link). Unterminated markers (open fence, dangling `**`/`` ` ``/`[`) degrade
    to literal text so a mid-stream document stays stable; a prefix-sweep test
    asserts every streamed prefix parses without panic.
  - `crates/ui/src/ai_chat.rs` (+~950 lines) — `AiChatView` GPUI entity:
    - Header: session title + dropdown (switch / delete / `+` new), model-ref
      pill (click cycles the `MODELS` catalog via `AiChatStore::set_model_ref`),
      run-status line (Thinking/Streaming/Awaiting approval/Error).
    - Message list: role-styled (user = right-aligned accent bubble, assistant =
      left block, system = compact), `overflow_y_scroll` + `ScrollHandle`.
      Auto-scroll: `stick_bottom` flag recomputed from the scroll offset on
      every wheel event (`is_at_bottom(offset_y, max_h, 48px)`); while true the
      view pins to bottom, so scrolling up detaches and returns re-attaches.
    - Assistant rendering: per-message markdown cache keyed by
      `(msg id, content.len())` — only the growing trailing message re-parses
      per token, history is untouched (the "don't re-render the whole verlauf"
      warning). Blocks → GPUI elements; inline bold/italic/code/link via
      `StyledText::with_highlights`. Fenced code blocks: `SyntaxHighlighter`
      (T06-002) highlight via `EditorPalette`, language label + Copy button
      (`cx.write_to_clipboard`).
    - Reasoning: collapsible "Thinking" block per message.
    - Tool-approval cards: pending (Streaming/AwaitingApproval) shows
      Approve/Reject; Done/Error shows the result. Wired to new
      `SessionStore::resolve_tool_call(id, approved)` (ai crate) → sets the card
      to Done/Error with a placeholder result and settles run status (real
      execution is T11-004).
    - Composer: hand-rolled multi-line key-buffer input (Enter sends,
      Shift/Alt+Enter newline), Send/Stop button, attachment chips with remove.
      `compose_message` prepends `<selection source=…>` / `<file path=…>` /
      `<image path=…>` blocks (which `derive_title` already strips). Public
      `attach_selection` / `attach_file` for a later terminal/editor
      "Ask AI about selection" wire.
  - `AiChatStore`: added `last_usage()`, `resolve_tool_call(...)`.
  - `crates/ui/src/app_shell.rs` — owns `ai_chat: Entity<AiChatView>`
    (constructs `AiChatStore::new(tokio)` + view), `render_panel_body` routes
    `SidebarPanel::Ai` → the view. `git_graph` now gets `tokio.clone()`.
  - `crates/ui/src/lib.rs` — `pub mod markdown` + re-exports `AiChatView`,
    `Attachment`, `AttachmentKind`.
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` — all green. ai **45** (+1:
  `resolve_tool_call_settles_card_and_status`), ui **124** (+13: 8 markdown +
  5 chat: compose/at-bottom/context-split/composer-clear/tool-card/model-cycle),
  backend 148 unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.
- **Next: T11-004 — Agent/Tool-System und Live-Bridge**
  (`tasks/phase-10-ai-chat/T11-004-agent-tool-system.md`) — the actual
  tool-execution loop behind the approval cards.

### Notes / Quirks (T11-003)
- Chat is a **dockable sidebar panel** (`SidebarPanel::Ai`), not a workspace
  tab. The reference `AiMiniWindow` is a floating/dockable mini-window; the
  sidebar slot was already reserved and avoids tab/pane plumbing. Moving it to
  a detachable window is a later polish.
- **No Settings→AI provider pane** (API-key entry UI). Only display + a
  model-cycle pill. `AiChatStore` still uses the OS keyring from T11-001, so
  with no key `send` fails the run and the error renders in the message list.
  The key-entry form belongs with the Phase 12 Settings UI.
- Markdown code highlighting reuses `labonair_editor::SyntaxHighlighter` — a
  fresh highlighter is built per code block per render (`update` + `line_runs`
  like `editor.rs`). Cheap for chat-sized snippets; the message-level parse
  cache keeps the hot path (streaming trailing message) from re-highlighting
  earlier blocks… actually it does re-render all blocks of the *trailing*
  message each token, but not the rest of the history.
- Attachments have no file/image picker (GPUI has none wired). `attach_file`
  reads via `std::fs` (truncated to 16k chars); images carry the path only —
  vision payloads are a later pass.
- GPUI: `overflow_y_scroll` / `on_click` require the element to be stateful
  (`.id(...)` first) — every clickable/scrollable div in the panel has an id.
  `ScrollHandle::{offset,max_offset,scroll_to_bottom}` from gpui 0.2.2
  `elements/div.rs`; `.track_scroll(&ScrollHandle)` on the list.

---

## Prev Session: 2026-09-01 (T11-002 — Chat store & session management)

### What Was Done
- **T11-002 ✅ Done.** Chat sessions + send/stream orchestration on top of the
  T11-001 `crates/ai` layer.
  - `crates/ai/src/sessions.rs` (new, pure Rust, UI-framework-agnostic) — port
    of `reference-src/src/modules/ai/store/chatStore.ts` + `lib/sessions.ts`:
    - Message model: `SessionMessage` (id/role/content/reasoning/tool_calls/
      tool_call_id/status/error/created_at), `MessageStatus`
      (Streaming|Final|Error), `SessionToolCall` + `ToolCallStatus`
      (Streaming|AwaitingApproval|Done|Error).
    - `SessionMeta` (id/title/created_at/updated_at), `RunStatus`
      (Idle|Thinking|Streaming|AwaitingApproval|Error).
    - `SessionStore` — sessions list + active id + per-session messages, backed
      by a single atomic JSON blob (`~/.config/labonair/labonair-sessions.json`,
      tmp+rename). `load` guarantees the "always ≥1 session + valid active id"
      invariant and reuses a leading untitled "New chat" across restarts.
      CRUD: `new_session`/`switch_session`/`delete_session`(falls back / spawns
      fresh)/`rename_session`. Change notification via a `revision()` counter.
    - Orchestration transitions (no I/O): `begin_send(text) -> Vec<ChatMessage>`
      (appends user msg + streaming assistant placeholder, auto-derives title
      via `derive_title` which strips `<terminal-context>/<selection>/<file>`
      blocks), `apply_event(StreamEvent)` (folds text/reasoning/tool-call deltas
      into the trailing assistant msg), `finish_run`, `stop`, `fail_run`,
      `reset_active_run` (provider/key switch — settles live state, keeps all
      session data; mirrors the reference only resetting `agentMeta`).
      Persists on send/finish/stop, never per token.
  - `crates/ui/src/ai_chat.rs` (new) — `AiChatStore` GPUI entity wrapping
    `SessionStore` + `InstanceStore` + `AiClient` + a `TokioHandle`. `send()`
    spawns the `AiClient::stream_chat` consumer on Tokio, forwards each event
    into the store via `cx.spawn` + `this.update` + `cx.notify()`, with a
    `generation` guard so stale runs drop their events. `stop`/`set_model_ref`/
    session ops all notify. `resolve_target` error (e.g. no key) → `fail_run`.
    Exported as `labonair_ui::{AiChatStore, init_ai_chat}`. **Not yet wired into
    `Workspace`** — that's T11-003 (chat UI).
  - `crates/ui/Cargo.toml`: added `labonair-ai` path dep.
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` — all green. ai **44 tests** (+10:
  session CRUD, restart persistence, send sequence, stop, tool-call awaiting,
  error event, provider-switch reset, title derivation, revision). ui **111**
  (+2: `session_ops_notify`, `send_without_key_records_error`). backend 148
  unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.
- **Next: T11-003 — Chat UI & streaming markdown**
  (`tasks/phase-10-ai-chat/T11-003-chat-ui-markdown.md`). Build the GPUI chat
  panel/view rendering off `AiChatStore`; wire it + a Settings→AI provider pane
  into `Workspace`. T11-004 is the agent/tool execution loop.

### Notes / Quirks (T11-002)
- `SessionStore` persistence is a single whole-file JSON blob, not the
  reference's per-key `LazyStore` (`sessions` / `activeId` / `messages:<id>`).
  Simpler and enough for parity; the "don't write per token" rule is honoured
  by only persisting at send/finish/stop, not by a debounce timer.
- `reset_active_run` intentionally does NOT delete messages — the reference
  `chatStore.setApiKeys` only resets `agentMeta` to idle. "Provider switch
  resets the chat" == resets run status/tokens, sessions & history stay.
- GPUI notification coalescing: multiple `cx.notify()` inside one
  `entity.update` block fire the observer once. The notify test does one op per
  `update` + `cx.run_until_parked()` between them.
- tokio mpsc `Receiver::recv().await` works fine inside a `cx.spawn` future
  (gpui executor, no tokio runtime) — mpsc needs no reactor. The
  `AiClient::stream_chat` call itself is kept on `self.tokio.spawn` since it
  calls `tokio::spawn` internally.

---

## Session: 2026-09-01 (T11-001 — AI provider integration / Multi-Provider BYOK)

### What Was Done
- **T11-001 ✅ Done.** Filled the previously-stub `crates/ai` with the pure-Rust
  replacement for the reference app's Vercel-AI-SDK layer
  (`reference-src/src/modules/ai/`). New modules:
  - `config.rs` — `ProviderId` (13 providers: openai/anthropic/google/xai/
    cerebras/groq/lmstudio/openai-compatible/deepseek/mistral/openrouter/mlx/
    ollama), 21-entry static `MODELS` catalog (ids/labels/hints/context limits/
    tags), cloud + local base URLs, `ProviderFamily` (OpenAi | Anthropic |
    Google), `needs_key`/`is_keyless`, `find_model`/`model_context_limit`/
    `model_keeps_reasoning`.
  - `message.rs` — provider-agnostic interface: `ChatMessage`/`Role`/`ToolCall`/
    `ToolDef`/`ChatConfig`/`Usage` and `StreamEvent` (`TextDelta`,
    `ReasoningDelta`, `ToolCallStart/Delta/End`, `Usage`, `Done{finish_reason}`,
    `Error(AiError)`). A well-formed stream ends with exactly one Done or Error.
  - `error.rs` — `AiError` with `from_status` (401/403→Auth, 429→RateLimit,
    400/404/422→BadRequest, 5xx→ServerError) + `from_reqwest` (timeout/network);
    pulls `error.message` out of the JSON body for the display string.
  - `sse.rs` — incremental `SseDecoder` (multi-line `data:`, `event:`, CRLF,
    comment lines, chunk-split frames, `finish()` flush).
  - `adapters.rs` — per-family `build_request` (URL + headers + JSON body) and
    stateful `StreamParser` (`OpenAiState`/`AnthropicState`/`GoogleState`)
    decoding provider SSE → `StreamEvent`s. OpenAI covers 11 of 13 providers;
    Anthropic extracts system messages + defaults `max_tokens`; Google puts the
    key in the query string and maps assistant→"model".
  - `secret_store.rs` — `SecretStore` trait + `KeyringSecretStore` (OS keyring
    via the `keyring` crate, service `labonair-ai`) + `MemorySecretStore` for
    tests. Per-instance keys under `inst-<id>`, legacy per-provider under
    `<provider>-api-key`. Keys never touch disk/logs/app-state.
  - `instances.rs` — `ProviderInstance` + `InstanceStore` persisting to
    `~/.config/labonair/labonair-ai.json` (instances + `active_model_ref` +
    recents). `parse_model_ref`/`make_model_ref` (`"model@instanceId"`),
    `resolve_instance`, `auto_name`, `rename_for_duplicates`.
  - `client.rs` — `AiClient::stream_chat(target, config, messages, tools)
    -> ChatStream` (mpsc receiver + `tokio::task` handle; `cancel()` / `Drop`
    abort the request and close the HTTP connection). `resolve_target(model_ref,
    &InstanceStore, &dyn SecretStore)` → provider/family/base_url/api_key/model,
    erroring `MissingKey` for keyless-less cloud providers.
- Deps added to `crates/ai/Cargo.toml`: `futures-util`, `bytes`, plus
  `thiserror`/`dirs`/`uuid` (all workspace versions). `Cargo.lock` updated.
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` — all
  green. New: **ai 34 tests** (adapter request shapes per family, SSE parsing
  per family incl. tool calls + usage + errors, keyring lifecycle, instance
  store persistence, target resolution, a real end-to-end test that streams
  OpenAI SSE from a throwaway `tokio::net::TcpListener` HTTP server, plus the
  connection-error + cancel paths). backend 148 / ui 109 unchanged.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.
- **Next: T11-002 — Chat store & session management**
  (`tasks/phase-10-ai-chat/T11-002-chat-store-sessions.md`). Builds the session
  history / persistence layer on top of this crate; T11-003 is the chat UI,
  T11-004 the agent/tool loop.

### Notes / Quirks (T11-001)
- `crates/ai` is **not yet wired into the UI or app** — no `AiClient` is
  constructed anywhere, no keyring writes happen. That's deliberate: T11-002
  (store) and T11-003 (chat UI + Settings→AI) own the wiring. The crate is a
  self-contained, fully-tested library for now.
- Keys use the real **OS keyring** (`keyring` crate) via `KeyringSecretStore`,
  *not* the backend `secrets.rs` file store. The task text says "via Backend
  secrets (T01-002)" but also "OS-Keyring"; the keyring keeps `crates/ai`
  free of a `labonair-backend` dependency and matches the security warning
  ("nur Keyring"). If a later task wants the file store, add a `SecretStore`
  impl that calls `backend::secrets`.
- The reference's `buildLanguageModel` delegates all wire formatting to
  `@ai-sdk/*`. Those packages are gone, so the three adapters are written from
  scratch. OpenRouter / DeepSeek / xAI / Groq / Cerebras / Mistral / LM Studio
  / MLX / Ollama / openai-compatible all share the **OpenAI** adapter (endpoint
  `<base>/chat/completions`, Bearer auth); only Anthropic and Google have
  bespoke adapters.
- Google `:streamGenerateContent` never sends a `[DONE]` marker — the client
  calls `StreamParser::finish()` on connection close to synthesize the final
  `Done`. OpenAI's `finish()` is also called if `[DONE]` is dropped.
- Tool-call arguments stream as raw JSON-string deltas (`ToolCallDelta`); the
  caller concatenates them and parses once `ToolCallEnd` fires. Anthropic's
  `input_json_delta` and Google's one-shot `functionCall.args` are both
  normalised to this shape.
- Model catalog uses the reference's (fictional, future-dated) model ids
  verbatim (`gpt-5.5`, `claude-opus-4-7`, `gemini-3.1-pro`, …) — they're the
  reference's chosen identifiers, adjust when wiring real API calls.
- `InstanceStore` writes `labonair-ai.json` (separate from the shared
  `labonair-settings.json` the reference uses via `LazyStore`) to avoid
  coupling to the backend settings module; revisit if Phase 12 wants one file.

---

## Prev Session: 2026-09-01 (T10-001 — Git-Graph rendering / commit graph)

### What Was Done
- **T10-001 ✅ Done.** New `crates/ui/src/git_graph.rs` (~1200 lines incl. 10
  tests) — `GitGraphView` GPUI entity, port of the reference
  `src/modules/git-graph/` module (`GitGraphPane`, `GitGraphCanvas`,
  `GraphRail`, `CommitDetailPanel`, `lib/graphLayout.ts`, `lib/laneColors.ts`,
  `lib/useGitGraph.ts`). No backend work — `git_get_log` (with `skip` offset
  pagination + over-fetch-by-one), `git_get_current_branch`,
  `git_get_commit_numstat`, `git_get_commit_diff`, `git_is_repo`,
  `git_get_repo_root` all already existed.
  - Pure, unit-tested: `build_graph_layout` (direct port of `buildGraphLayout`
    — stateful left-to-right lane sweep producing `LayoutCommit`s with
    top/bottom `GraphEdge`s: Straight / Merge / Branch), `initial_graph_page_size`
    (500 local / 200 remote), `parse_numstat` (binary `-`, tabs-in-path),
    `classify_ref` (tag `^v\d` / remote `origin|upstream|<slug>/` / local, with
    the `feat|fix|chore|…/` allow-list), `is_no_repo_error`, `relative_age`,
    `format_commit_date` (self-contained days→civil, no chrono), `lane_color` /
    `avatar_color` / `initials`.
  - View: toolbar (repo name + parent path, Local/Remote badge, "Ns ago" age
    that repaints on a 30s tick, refresh), virtualised commit list
    (`gpui::uniform_list`, 32px rows — only visible rows build elements),
    column headers, "Load more commits" footer (real `--skip` pagination),
    generation guard (`gen`, bumped on root/session/reload) drops stale
    responses.
  - Graph rail is painted with absolutely-positioned `div` segments (vertical
    lane lines + horizontal L-connectors for merge/branch + a coloured node
    dot, ringed when selected), clamped to `MAX_VISIBLE_LANES = 12`. Not a
    `<canvas>` — GPUI 0.2.2's `canvas()` paint API is undocumented and div
    segments match the rest of the codebase (explorer/sftp/git all use plain
    `div` + `overflow_y_scroll`, no virtualiser until now).
  - Ref/tag badges on rows (lane-tinted, `HEAD` marker when the ref == current
    branch), author avatar (initials, deterministic colour), relative +/-
    change counts.
  - Commit detail panel (right, 320px): avatar header, email, click-to-copy
    full hash, parent short-hashes, subject, numstat file list with +/-
    totals, Older/Newer nav between commits, "View diff" toggle that lazily
    fetches `git_get_commit_diff` and renders it with a local colored
    unified-diff line renderer (same approach as `git.rs`'s hunk preview —
    `DiffView` wants two texts, not a unified patch).
  - Wired into `SidebarPanel::GitGraph` in `app_shell.rs` (the variant + rail
    glyph already existed but rendered a "coming later" placeholder): new
    `git_graph: Entity<GitGraphView>` field, constructed next to `git_panel`
    (so `GitPanelView::new` now takes `backend.clone()`/`tokio.clone()`), fed
    the active terminal cwd via the same `observe(&workspace)` +
    initial-set_root pattern as `git_panel`/`explorer`. `render_panel_body`
    routes `SidebarPanel::GitGraph` → the view.
  - `crates/ui/src/lib.rs` — `pub mod git_graph` + `pub use
    git_graph::GitGraphView`.
- Verify: `cargo check --workspace`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` (ui 99 → 109), `cargo fmt --all
  --check` — all green.

### State / Next
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.
- Phase 9 (Git-Graph) is now complete. **Next: Phase 10 — AI-Chat-System**,
  first task **T11-001** (AI-Provider-Integration, Multi-Provider BYOK),
  `tasks/phase-10-ai-chat/`.

### Notes / Quirks (T10-001)
- The `TabKind::GitGraph` / `TabKind::CommitDiff` tab variants exist in
  `tabs.rs` but are **unused** — this task renders the graph as a *sidebar
  panel* (`SidebarPanel::GitGraph`), not a workspace tab, because the panel
  slot was already reserved and it avoids the tab/workspace plumbing. The
  reference opens it as a tab; a follow-up could move it if tab semantics
  (per-repo tabs, pinned path) are wanted. "View Changes → CommitDiff tab" and
  the row context menu (checkout/cherry-pick/create-branch-here) from
  `GitGraphPane` are **not** ported — detail-panel inline diff covers the
  "see the commit's changes" criterion; the branch ops all live in the
  Source-Control panel (T09-002).
- No `sshd` in CI ⇒ no live remote test; the layout algorithm + all parsing
  seams are unit-tested (10 tests incl. feature-branch merge, octopus merge,
  lane reuse, linear history). The backend git log/numstat/diff fns have their
  own coverage from T09-00x.
- Rail edges are div segments, not bezier paths — merge/branch connectors are
  drawn as an L (vertical + horizontal + vertical), not a curve. Visually
  close enough; revisit with `gpui::canvas` + `PathBuilder` if the Zed source
  yields a confirmed path-painting API.
- `git_get_log` is always called with `all_branches = true` (matches the
  reference `git.getLog(path, limit, true, …)`), so row 0 is the newest commit
  across *all* refs, not necessarily HEAD — the `HEAD` badge is driven by
  `git_get_current_branch` string-matching a ref name, which is correct for a
  normal checkout and simply absent when detached.

---

## Prev Session: 2026-09-01 (T09-002 — Branch management & stash)

### What Was Done
- **T09-002 ✅ Done.** Branch/tag/stash UI added to `crates/ui/src/git.rs`
  (`GitPanelView`), a GPUI-native port of the reference `BranchBar` /
  `BranchDropdown` / `NewBranchDialog` / `StashPanel` components. No new
  backend work — all commands (`git_checkout_branch`, `git_create_branch`,
  `git_delete_branch`, `git_rename_branch`, `git_get_branches`,
  `git_create_tag` / `git_get_tags` / `git_delete_tag` / `git_push_tag`,
  `git_stash_push` / `list` / `pop` / `apply` / `drop`) already existed and
  the workspace-state bundle already carries `branches` / `current_branch` /
  `stash` / `tags`, so the poll loop feeds everything.
  - Pure helpers (unit-tested): `filter_branches` (case-insensitive, split by
    local/remote), `map_checkout_error` ("would be overwritten" → "stash
    first"), `is_unmerged_branch_error`, `stash_display_message` (blank →
    "WIP"), `is_stash_conflict_error` / `stash_conflict_message` (pop/apply
    conflict → "stash was kept"), `resolve_default_from_ref` (unwraps
    "HEAD detached at <hash>").
  - Branch bar: the branch label is now a toggle (`⌥ <branch> ▾`) that opens
    an inline branch picker rendered between the file list and the bar.
  - Branch picker (`render_branch_picker`): filter field, "+ New Branch"
    inline form (name / from / checkout-toggle / error), local branch rows
    (✓ current, ↑ahead/↓behind chips, ✎ rename inline, ✕ delete), collapsible
    remote section (checkout only), collapsible Tags section (+ new-tag form
    name/message/from, per-row ↑push / ✕delete). Delete uses an inline
    `confirm_bar`; an unmerged-branch delete escalates to a "force delete?"
    confirm. Rename swaps the row for an inline text field.
  - Stash panel (`render_stash_panel`): collapsible "STASHES (n)" section
    above the diff, + opens an inline message form, per-row `A`pply / `P`op /
    ✕drop (drop behind a confirm bar). Pop/apply conflicts surface the
    "stash kept" message via `notify_err`.
  - Text input: GPUI has no built-in single-line input here, so all fields
    are hand-rolled — a `Field` enum marks the one active field and the panel
    root's `on_key_down` routes keystrokes to it (`on_field_key`), same
    approach as the existing commit box. `text_field()` renders each one;
    clicking it sets `active_field` + focuses the panel root.
  - `dispatch()` helper = `run_op` variant that hands the raw `Result` to a
    callback (for inline errors / follow-up state) instead of always toasting.
- Verify: `cargo check`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo test --workspace` (backend 148, ui 99 — incl. 6 new UI helper tests
  + 2 new backend live tests `branch_tag_stash_lifecycle` /
  `checkout_with_conflicting_local_changes_is_rejected`), `cargo fmt --check`
  — all green.

### State / Next
- Branch: `master`. Next task: **first not-done task after T09-002** — check
  `tasks/ROADMAP.md` (phase 09 Git-Graph is referenced as follow-up).
- Note: pre-existing uncommitted `CLAUDE.md` edit (not ours) left untouched.

---

## Prior Session: 2026-09-01 (T09-001 — Source-Control panel: Git status & staging)

### What Was Done
- **T09-001 ✅ Done.** The backend git module
  (`crates/backend/src/modules/git/`, ~2200 lines, `git` CLI wrapper — no
  libgit2) was already fully ported + tested (146 backend tests incl. a
  live hunk-apply smoke test). This task is the GPUI Source-Control panel
  that wires it up.
  - **`crates/ui/src/git.rs`** (new, ~1160 lines incl. 12 tests) —
    `GitPanelView` GPUI entity.
    - Pure helpers ported from `source-control/lib/diffHunks.ts`:
      `parse_diff_hunks` / `build_hunk_patch` / `is_whole_file_single_hunk`
      (unified-diff → per-file hunk structs; truncated diffs return `[]`;
      CRLF content bytes preserved). Plus `validate_commit_message`,
      `status_letter`, `bucketize` (dedupes conflicted entries that the
      porcelain parser files into both staged+unstaged — mirrors
      `SourceControlPanel.tsx`).
    - Polls `git_is_repo` → `git_get_repo_root` → `git_get_workspace_state`
      (the batched bundle) every 2s (×3 for remote). Generation guard
      (`target_gen`, bumped only on genuine root/session change) +
      `refreshing` flag so a stale response can't overwrite a newer
      target's state — direct port of `useGitStatus.ts`'s
      `generationRef`/`isRefreshingRef`.
    - Renders: action bar (Refresh / Stage all ⇄ Unstage all / Discard /
      Clean), unified-diff preview for the selected file (own lightweight
      renderer — `DiffView` from T06-004 wants two texts, not a unified
      diff string; hunk staging needs the raw unified diff anyway), file
      list categorised Conflicts / Staged / Changes / Untracked
      (collapsible, status-letter badge + colour, per-row stage/unstage +
      discard, click-to-preview), branch bar (branch, ↑ahead/↓behind,
      Fetch/Pull/Push/Publish/Force + merge/rebase/cherry-pick banner with
      Continue/Abort), commit form (key-input message box, ⌘/Ctrl-Enter or
      button, non-empty + something-staged validation).
    - Hunk staging: `apply_hunk(idx, reverse)` → `parse_diff_hunks` on the
      loaded diff → `build_hunk_patch` → `git_stage_hunk`/`git_unstage_hunk`
      (`git apply --cached [--reverse]`). Falls back to whole-file
      `git_stage_file`/`git_unstage_file` for new/deleted files
      (`is_whole_file_single_hunk`).
    - Force-push uses `git_push_force_with_lease` and requires an explicit
      second click ("Force" → "Confirm force"); never automated.
    - All backend calls dispatched via `self.tokio.spawn`, results folded
      back with `cx.spawn` + `this.update`; errors → `notify_err` toast;
      every mutating op refreshes on completion.
  - **`crates/ui/src/app_shell.rs`** — owns `git_panel:
    Entity<GitPanelView>`, constructed in `new` (needs `backend` +
    `tokio`, so `Workspace::new` now takes `backend.clone()`/
    `tokio.clone()`). `render_panel_body` routes
    `SidebarPanel::SourceControl` → the panel. An `observe(&workspace)`
    forwards the active terminal cwd into `git_panel.set_root` (same
    pattern as the explorer).
  - **`crates/ui/src/lib.rs`** — `pub mod git` + `pub use
    git::GitPanelView`.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace
    --all-targets -- -D warnings`, `cargo check --workspace`, `cargo test
    --workspace` all green. ui tests 83 → 95.

### Notes / Quirks (T09-001)
- **Rust string `\` line-continuation strips the leading space of the
  next line** — that space is the diff *context-line* marker, so test
  fixtures must be built with `[...].join("\n")`, not a `"...\n\` literal.
  (Cost ~20 min: the CRLF fixtures silently lost their context markers.)
- `git_get_workspace_state` / all `git_*` fns take `(path, session_id:
  Option<String>, sftp_state: &SshState, app: App)` — call as
  `git::fn(root, sid, &backend.ssh, backend.clone())`. Local repo →
  `session_id: None`.
- `git_push_force_with_lease` takes `remote: Option<String>, branch:
  Option<String>` (not bare `String` like `git_push_set_upstream`).
- The panel currently targets **local repos only** (`session_id` field
  exists + `set_session` is wired, but nothing calls it yet — remote-repo
  Source Control needs the active SSH tab's session id plumbed from the
  workspace, a thin follow-up).
- **No new integration test** — the backend git module already has
  full coverage (status parsing, stage/unstage, hunk apply/unapply
  end-to-end against a throwaway repo, stale-patch classification). The
  UI seams (`parse_diff_hunks`, `build_hunk_patch`,
  `is_whole_file_single_hunk`, `validate_commit_message`, `status_letter`,
  `bucketize`, `short_path`) are unit-tested in `git.rs` (12 tests,
  fixtures byte-mirrored from `diffHunks.test.ts`).
- FS-watcher-driven refresh is **not** wired — the 2s poll (matching the
  reference default `gitStatusPollIntervalMs`) covers external changes.
  `app.watcher` exists for a later precision pass.
- No tree view / sort options / stash dialog / branch dropdown — those are
  T09-002 (branch + stash) and the reference's `SourceControlActionBar`
  dropdown extras.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T09-002 — Branch management & stash** (`tasks/phase-08-git-ui/
  T09-002-branch-stash.md`): branch dropdown/switch/create/delete/rename +
  stash push/list/pop/apply/drop UI. Backend fns already exist
  (`git_checkout_branch`, `git_create_branch`, `git_stash_*`, …). Hook the
  stash/branch UI into `GitPanelView`'s branch bar + action bar.

---

## Prev Session: 2026-09-01 (T08-002 — SFTP transfers upload/download/queue)

### What Was Done
- **T08-002 ✅ Done.** The backend transfer worker
  (`sftp/worker.rs`, `sftp/commands.rs`) was already fully ported + tested
  (146 backend tests, folder recursion, MD5 verify, cancel tokens, sticky
  session reconnect requeue) — this task is the GPUI queue UI + event
  wiring + drag/context-menu triggering.
  - **`crates/ui/src/transfers.rs`** (new, ~950 lines incl. 6 tests) —
    `TransfersView` GPUI entity. `TransferBusEvent::from_raw(name, payload)`
    decodes the four worker events off the broadcast bus
    (`transfer_progress` → full `TransferJob`, `transfer_step`,
    `file_conflict`, `file_error`) — the typed `AppEvent` can't carry them
    (shape mismatch). `apply(ev)` maintains `jobs: Vec<JobRow>` (newest
    first, `JobRow` adds UI-only `conflict`/`file_error` pause state),
    `steps` per job, and `sticky` overwrite/skip policy per session.
    Bottom-right pill (`N active · M total`) toggles a 380px panel: per-job
    row = direction arrow, dest filename, status pill (running/done/paused/
    failed/cancelled + `· K skipped`), collapsible step log, cancel ✕,
    theme-tinted progress bar (`gpui::relative(pct)`), `src → dest` +
    bytes/speed. Conflict modal (Overwrite / Skip / Rename… with
    `suggested_rename` seed / Overwrite all / Skip all — "…all" sets the
    session sticky + fans out to paused siblings). File-error modal (Skip /
    Skip all / Abort). Resolutions go back via
    `sftp::commands::resolve_conflict`; cancel via `cancel_transfer`.
    Emits `TransfersEvent::Completed { session_id, direction }`.
  - **`crates/backend/src/modules/sftp/mod.rs`** — `#[derive(Clone)]` on
    `TransferWorkerState` (all fields already Clone: `mpsc::Sender` +
    2×`Arc`) so the UI can move a handle into `tokio.spawn`.
  - **`crates/ui/src/sftp.rs`** — new `SftpEvent::Enqueue { session_id,
    src_path, dest_path, direction }` + `SftpDrag { from: Side, paths }`
    payload + `DragGhost`. Each row is now `.on_drag(SftpDrag…)`; each pane
    body is `.on_drop(&SftpDrag)` → `enqueue(from, paths)` when the drop
    landed on the opposite pane (upload local→remote, download remote→
    local, into that pane's current dir). Context menu gains "Upload to
    Remote" / "Download to Local" per entry. `pub fn reload_side(remote)`
    for post-transfer refresh. Folders transfer recursively (backend).
  - **`crates/ui/src/workspace.rs`** — owns `transfers: Entity<TransfersView>`
    + `transfer_events: mpsc::Receiver<TransferBusEvent>`. The existing bus
    forwarder task now also decodes `TransferBusEvent` (checked before
    `AppEvent`). The 40ms `ssh_poll` loop drains transfer events into
    `transfers.apply`. `on_sftp_event` handles `Enqueue` → spawn
    `enqueue_transfer` + `transfers.reveal()`. New `on_transfers_event`:
    `Completed` → find the `SftpView` by session id and `reload_side`
    (Upload → remote pane, Download → local pane). `render` adds
    `.child(self.transfers.clone())` (pill/panel only occupy their own
    box; modal is a full inset-0 backdrop).
  - **`crates/ui/src/lib.rs`** — `pub mod transfers` + re-exports.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace
    --all-targets -- -D warnings`, `cargo check --workspace`, `cargo test
    --workspace` all green. ui tests 77 → 83.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **Phase 8 — Git UI & Source Control** (`tasks/phase-08-*`, first task
  T09-001 or the lowest-numbered pending — check `tasks/ROADMAP.md`).
  Phase 7 (SFTP) is now complete.

### Notes / Quirks (T08-002)
- **No live SFTP transfer integration test** — same rationale as T07-00x /
  T08-001 (no `sshd` in CI). The worker's transfer/folder/conflict/cancel
  logic has 146 backend tests already; the UI seams (`percent`,
  `format_bytes`, `base_name`, `suggested_rename`, `TransferBusEvent::
  from_raw`, `status_label`) are unit-tested in `transfers.rs`.
- Transfer events are forwarded off the bus **before** `AppEvent` decoding
  because `file_conflict` is *also* an `AppEvent` variant but with a
  different payload shape — `TransferBusEvent::from_raw` must win.
- The queue panel is a fixed bottom-right pill/panel, not a Phase-12
  configurable bar-item. No `badgesAlwaysVisible` equivalent — the pill
  only shows when `jobs` is non-empty.
- Conflict "Rename" seeds an editable buffer with `suggested_rename` (e.g.
  `report_1.tar.gz`) rather than the reference's free-form field.
- Drag-drop uses the same `.on_drag`/`.on_drop` value-payload pattern as
  the explorer (`DraggedPaths`). There is no drop-target highlight yet
  (the reference dims the hovered pane) — a small follow-up.
- "Download to Local" / "Upload to Remote" always target the *opposite
  pane's current directory* (matching drag semantics). The reference's
  "Download to…/Upload here…" open a native folder picker, which GPUI
  doesn't have wired yet.
- `sftp_update_transfer_settings` (concurrency / chunk size / default
  conflict policy) is not called from the UI yet — the worker keeps its
  defaults (2 concurrent, 64 KiB, "ask"). Wire it in the Phase 12 Settings
  UI (reference: `bootstrapTransferSettingsSync`).
- Connection-loss during a transfer: the worker emits `ssh_connection_lost`
  and marks the job `Failed`; the SFTP tab's own Retry reconnects the
  session and the worker's `SessionReconnected` requeue path handles
  in-flight jobs. The UI just shows the failed job — no dedicated
  "reconnect & resume" button.

---

## Previous Session: 2026-09-01 (T08-001 — SFTP file browser)

### What Was Done
- **T08-001 ✅ Done.** Dual-pane SFTP browser as a `TabKind::Sftp` tab. The
  backend SFTP layer (`ssh/sftp.rs`, `sftp/connection.rs`,
  `sftp/worker.rs`) was already fully ported + unit-tested (146 backend
  tests) — this task is the GPUI UI + workspace wiring.
  - **`crates/ui/src/sftp.rs`** (new, ~1080 lines incl. 8 tests) — `SftpView`
    GPUI entity. Left pane = local FS (`fs::tree::list_dir_entries_sync` on
    the background executor + `fs::mutate::{create_file,create_dir,rename,
    delete}_sync`), right pane = remote FS over SFTP. Per pane: title +
    up / reload / hidden-toggle buttons + click-to-edit address bar,
    generation-guarded async loads, error banner, dirs-first sort, inline
    rename / new-file / new-folder (focus + key-buffer pattern from the
    explorer), row select + double-click (dir → navigate, file → open).
    Right-click context menu (New Folder/File, Rename, Copy Path, 2-click
    Delete, and for remote entries Permissions… / Properties… / Edit Remote
    File, plus Refresh). chmod/chown dialog (`sftp_chmod` + `sftp_chown`,
    Tab cycles octal/owner/group fields). Properties dialog (type, size,
    perms, mtime; `sftp_calculate_size` for dirs). Remote connect via
    `sftp::connection::sftp_connect` with a Connecting / Error+Retry / Ready
    state. Emits `SftpEvent::{OpenLocalFile, OpenRemoteFile}`.
  - **`crates/ui/src/workspace.rs`** — `sftp_views` / `sftp_sessions` /
    `remote_edits` maps + `pending_sftp` / `pending_open` queues (drained in
    `render`, mirroring `pending_connect`). `open_sftp(host_id)` opens/refocus
    a `Sftp` tab + `SftpView` (fresh uuid session id). `on_sftp_event`:
    `OpenLocalFile` → `open_file`; `OpenRemoteFile` → spawn
    `prepare_remote_edit` → `open_remote_edit` opens an editor tab on the
    temp copy titled `"<name> (remote)"` and records a `RemoteEdit`.
    `watch_editor` now: skips the filename-title overwrite for remote-edit
    tabs, and on a `dirty → clean` transition spawns `save_remote_edit` to
    push the temp copy back. `retire_tab` drops the SftpView + calls
    `sftp_disconnect`, and for remote-edit editor tabs spawns
    `cleanup_remote_edit_temp`. `render_content` handles `TabKind::Sftp`.
  - **`crates/ui/src/hosts.rs`** — `HostManagerEvent::OpenSftp(String)` + an
    "SFTP" button on each host row.
  - **`crates/ui/src/lib.rs`** — `pub mod sftp` + re-exports.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace
    --all-targets -- -D warnings`, `cargo check --workspace`, `cargo test
    --workspace` all green. ui tests 69 → 77.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T08-002** — SFTP transfers (upload/download/queue) (`tasks/phase-07-sftp/
  T08-002-sftp-transfers.md`). Backend transfer worker (`sftp/worker.rs`,
  `sftp/commands.rs::enqueue_transfer` etc.) already exists — this is the
  queue UI + drag-between-panes + context-menu "Download to…/Upload here…".

### Notes / Quirks (T08-001)
- **No live SFTP integration test** — same rationale as T07-00x (no `sshd` in
  CI). The pure seams are unit-tested in `sftp.rs` (`parent_path`,
  `join_path`, `sanitize_entry_name`, `perm_string_to_octal`, `sort_entries`,
  `format_epoch`, `format_bytes`, `Pane::visible`); the backend SFTP ops
  themselves have 146 backend tests.
- GPUI: context-menu / dropdown row `div()`s need an explicit `.id(...)`
  before `.on_click` — `on_click` lives on `StatefulInteractiveElement`, only
  impl'd for `Stateful<Div>`. A plain `Div` silently lacks it (the explorer's
  `item` helper adds `.id("cm-…")` at each call site for this reason).
- Remote pane starts at `/` (not the SFTP home) — there is no public
  `canonicalize` in `ssh/sftp.rs`, and `default_path_sftp` is only set for
  hosts that configured it. Address bar + double-click navigation cover it;
  wiring a home-dir resolve is a small follow-up.
- Remote-edit save-back is detected via the editor's `dirty → clean`
  transition in `watch_editor` (there is no dedicated `Saved` event). It
  fires once per save. No conflict detection if the remote file changed
  underneath the temp copy — `save_remote_edit` is a plain overwrite.
- Conflict/2-click delete is in the context menu only (no modal), matching
  the reference `SftpContextMenu`. `sftp_delete` won't remove non-empty
  remote dirs (backend limitation — unlink-then-rmdir, no recursion).
- Chmod dialog octal field accepts up to 4 digits but only the low 3 apply
  (special bits unsupported), same as the reference `PropertiesDialog`.

---

## Previous Session: 2026-09-01 (T07-003 — SSH config import/export)

### What Was Done
- **T07-003 ✅ Done.** Backend parse/import/export was already ported in
  `crates/backend/src/modules/ssh/config_parser.rs`; this session hardened it
  and built the UI.
  - **`config_parser.rs`** — parser now recognises `Match` (suppresses key
    capture until the next top-level `Host`) and `Include` (read, not
    followed). New `ImportConflict { Skip, Overwrite, Rename }` enum;
    `import_ssh_config_entries` gained a `conflict: ImportConflict` param and
    now snapshots existing host names → resolves alias collisions per policy
    (Skip = leave + still usable as a ProxyJump target, Overwrite = UPDATE the
    mapped fields in place, Rename = insert as `alias-2`/`-3`/…). Returns the
    ids created **or** overwritten. New `write_ssh_config_export(block, append)`
    — atomic temp+rename write to `~/.ssh/config`, chmod 0600 on unix, `append`
    inserts after existing content; caller gates the non-append path. +4 tests
    (representative parse incl. wildcard/Match/Include/`=`-form/ProxyJump,
    import+mapping+ProxyJump resolution, Skip/Overwrite/Rename, export
    well-formedness + Import→Export→Import round-trip). backend 142→146.
  - **`crates/ui/src/hosts.rs`** — `ImportState`/`ExportState` on
    `HostManagerView`, free fns `cycle_conflict`/`conflict_label`. Toolbar
    gains "Import SSH config" + "Export SSH config". Import modal: async
    `parse_ssh_config_cmd`, per-entry checkbox rows (alias, addr:port, user,
    auth, `via <jump>`, "already exists" marker), select/deselect all, a
    cycling "On conflict: skip/overwrite/rename" button, `N of M selected`
    footer, runs `import_ssh_config_entries` then toast + `reload`. Export
    modal: host checkbox list (all pre-selected), "Copy to clipboard"
    (`cx.write_to_clipboard`) and "Append to ~/.ssh/config"
    (`write_ssh_config_export(_, true)`). New `modal_shell` helper. +2 tests
    (conflict cycle, export preselects all hosts). ui 67→69.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
    -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` all
    green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T08-001** — SFTP-Dateibrowser (`tasks/phase-07-sftp/`). Deps T07-001,
  T04-001/2, T05-001 (all done).

### Notes / Quirks (T07-003)
- Conflict policy is **global** for the import batch (one toggle), not
  per-row — matches the task's overwrite/skip/rename wording without a
  per-entry UI. The reference dialog only *warns* on duplicates (by
  `address:port`); this port keys conflicts by **alias/name** and actually
  acts on them.
- Rename collision suffix is `-2`, `-3`, … (there is no UNIQUE constraint on
  `hosts.name`; rename is best-effort cosmetic dedup).
- Overwrite updates only the mapped fields (host_address, port, username,
  auth_method, private_key_path) — never touches group, credential, tags,
  tunnels of the existing row.
- `write_ssh_config_export` is only ever called with `append = true` from the
  UI (the task warns against silent overwrite). The `append = false` branch
  exists but is unused — wire it to an explicit "replace file" action if ever
  needed.
- No test writes to the real `~/.ssh/config`; the file writer is covered by
  the export-text generator + round-trip parse instead.
- IdentityFile → stored as a path on the host (`auth_method = "key"`,
  `private_key_path`), never imported as a credential — matches the task note
  ("kein Klartext von Key-Inhalten").

---

## Previous Session: 2026-09-01 (T07-002 — Jump hosts & tunnels)

### What Was Done
- **T07-002 ✅ Done.** Backend jump-host routing + local-forward tunnels
  already existed (`ssh/client.rs::resolve_jump_host` / `connect_via_jump`,
  `ssh/tunnels.rs`); this task was the UI + wiring.
  - **`crates/backend/src/modules/ssh/tunnels.rs`** — `TunnelConfig` now
    derives `Serialize` + `PartialEq`, `type` defaults to `"local"` on parse
    (round-trips unchanged); dropped the `#[allow(dead_code)]` on `id`/
    `tunnel_type`. `TunnelEntry` gained `configs: Vec<TunnelConfig>`. New
    `pub fn active_tunnels(&TunnelState) -> Vec<ActiveTunnel>` (sorted by
    `(host_id, local_port)`) for the UI panel. +4 tests (JSON shape round-trip,
    `active_tunnels` listing/sort, ref-count-gated shutdown, unknown-host
    no-op). **Only `type:"local"` forwarding is supported — matches the
    reference (`reference-src/.../hosts/types.ts` `type: "local"` only); the
    task's remote/dynamic mention is beyond reference parity and was not
    implemented.**
  - **`crates/ui/src/hosts.rs`** — `HostForm` gains `jump_host: Option<usize>`
    (index into `self.hosts`, never the host being edited) and
    `tunnels: Vec<TunnelDraft>` + a `scratch` fallback string. New
    `HostField::Tunnel{LocalPort,RemoteHost,RemotePort}(usize)` inline-edit
    targets. Free fns `parse_tunnels` / `serialize_tunnels` (drops incomplete
    rows, `type:"local"`). Form now renders a "Jump host (ProxyJump)" cycle
    button + a "Tunnels" section (add / edit 3 fields / remove per row).
    `submit_form` passes `Some(jump_host_id_or_"")` + `Some(tunnels_json)` to
    `hosts_create`/`hosts_update` (`""` clears the jump host). New pub
    `host_name()` / `jump_host_label()` accessors, `ActiveTunnelRow` type +
    `set_active_tunnels()` + an "Active tunnels" panel. `from_host` signature
    gained a `hosts: &[Host]` param. +3 tests.
  - **`crates/ui/src/workspace.rs`** — `ssh_tab_title(host, jump)` helper
    (SSH tab title shows `SSH · host  ⤳ bastion` when routed through a jump
    host). `connect_host` reads the jump label from the host manager for the
    title. On `SshSessionEstablished` → `start_tunnels()` (spawns
    `ssh_start_tunnels`, mirrors the reference's `session_established` hook);
    `retire_tab` SSH teardown also calls `ssh_stop_tunnels`. The 40ms SSH
    poll loop now calls `refresh_active_tunnels()` → pushes `ActiveTunnelRow`s
    into the host manager (deduped, only notifies on change). +1 test.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
    -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` all
    green. Tests: backend 138→142, ui 63→67.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T07-003** — SSH config import/export (`tasks/phase-06-ssh-ui/
  T07-003-ssh-config-import-export.md`). Dep: T07-001 (done). Backend
  `ssh/config_parser.rs` already exists.

### Notes / Quirks (T07-002)
- **No live jump/tunnel integration test** — same rationale as T07-001 (no
  `sshd` in CI). Routing/tunnel logic is unit-tested at the seams
  (resolution, ref-counting, JSON round-trip, title annotation).
- Tunnel type is **local-forward only** end to end. The `type` field is kept
  in the JSON purely for round-trip fidelity with the reference.
- No standalone "stop this one tunnel" control: tunnels are host-config and
  auto start/stop with the SSH session (backend ref-counts per host). The
  "Active tunnels" panel is read-only status, matching the reference (which
  has no tunnel-management UI at all).
- Multi-hop jump chains are still unsupported (a jump host's own
  `jump_host_id` is never followed — documented in `resolve_jump_host`). The
  form only lets you pick one bastion.
- `HostForm::field_mut` returns `&mut self.scratch` for a stale tunnel-field
  index instead of panicking (indices only ever come from `render`, but the
  guard keeps it total).

---

## Previous Session: 2026-09-01 (T07-001 — Host manager & SSH connection)

### What Was Done
- **T07-001 ✅ Done.** First UI↔backend-wired feature. Host-manager dashboard,
  SSH connect flow, SSH terminal tabs, credential manager.
  - **`crates/terminal/src/session.rs`** — new `SessionAccess` trait (render /
    cwd / metadata / mode_state / selection / scroll / ai_context) impl'd for
    both `TerminalSession` and the new **`RemoteSession`** (transport-backed
    terminal: same `TerminalEmulator`, bytes come from SSH not a local PTY).
    `RemoteSession::new(colors, dims, RemoteWriter, RemoteResizer) -> (Self,
    RemoteFeed)`. `RemoteFeed { feed(&[u8]), mark_disconnected() }` — the SSH
    reader pushes output through `feed`; DA/DSR replies go back out via the
    writer. `RemoteWriter/RemoteResizer = Arc<dyn Fn(..) + Send + Sync>`.
  - **`crates/terminal/src/registry.rs`** — `Slot.session` is now
    `Mutex<SessionBackend>` (`Local(TerminalSession) | Remote(RemoteSession)`);
    all `SessionHandle` methods dispatch. `SessionHandle::with` now hands out
    `&dyn SessionAccess`. New `TerminalRegistry::create_remote(...) ->
    (SessionId, RemoteFeed)`. `restart` errors on remote sessions. +1 test.
  - **`crates/ui/src/hosts.rs`** (new, ~780 lines) — `HostManagerView`
    (`TabKind::Home` content). Groups (ungrouped + named, collapsible,
    create/delete), host rows with live status dot + Connect / Edit /
    Duplicate / Delete, host add/edit form modal (name/address/port/user/
    auth-method[password|key|agent|none]/key-path/password/start-dir/tags +
    credential & group cycle pickers), credential manager modal (list, new
    password/key credential, ed25519 keygen via backend, delete; public key
    shown in a toast). All persistence via `labonair_backend::modules::{hosts,
    credentials}`. Inline text fields use the explorer's focus+key-buffer
    pattern. Emits `HostManagerEvent::Connect(host_id)`. +2 tests.
  - **`crates/ui/src/workspace.rs`** — owns `backend: labonair_backend::App`,
    `tokio: runtime::Handle`, `host_manager`, `ssh_tabs:
    HashMap<SessionId, SshTab>`. Landing tab is now `TabKind::Home` (then a
    terminal tab, still active on start). `connect_host()` builds writer/
    resizer closures that `tokio.spawn` `ssh_pty_write` / `ssh_pty_resize`,
    calls `registry.create_remote`, opens a Workspace tab + `TerminalView`,
    then `spawn_ssh_connect` (calls `ssh_connect`, streams `SshPtyEvent::Data`
    into the `RemoteFeed`). Backend broadcast bus → `std::sync::mpsc` →
    `cx.spawn` 40ms poll → `handle_ssh_event`: `known_hosts_warning` /
    `auth_required` / `passphrase_required` raise an `SshPrompt` modal
    (trust / password / passphrase); `session_established` → status Connected
    + clears prompt; `ssh_connection_lost` → `feed.mark_disconnected()` +
    status Failed. Trust accept → `ssh_trust_host(id, true)`; password/
    passphrase submit → re-run `ssh_connect` with the override (the backend's
    fail-then-re-prompt model). `retire_tab` disconnects the SSH session.
    `pending_connect` drained in `render` (needs `&mut Window`).
  - **`crates/ui/src/app_shell.rs`, `crates/app/src/main.rs`** — thread
    `backend` + `runtime.handle().clone()` (captured before the runtime is
    `mem::forget`-leaked) through `AppShell::new` → `Workspace::new`.
  - **`crates/backend/src/modules/ssh/client.rs`** — +3 tests (trust-host
    oneshot release, unknown session no-panic, connect-to-missing-host errors).
  - Cargo: `crates/ui` gains `tokio` + `uuid` deps.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
    -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` all
    green. Tests: backend 135→138, terminal 62→63, ui 61→63.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T07-002** — Jump-Hosts & Tunnel (`tasks/phase-06-ssh-ui/T07-002-*`). Dep:
  T07-001 (done). Backend `ssh/tunnels.rs` + jump-host resolution in
  `ssh_connect` already exist; this is largely UI (host-form jump-host picker,
  tunnel editor) + wiring.

### Notes / Quirks (T07-001)
- **No live SSH-server integration test.** Setting up `sshd` in CI is
  platform-fragile; connection-flow error handling + the trust mechanism are
  unit-tested instead. A real end-to-end connect test against a throwaway
  `sshd` is a good follow-up (task lists it under Notizen).
- The UI has **no tokio runtime on the GPUI thread** — `main.rs` leaks the
  runtime and passes a `Handle`. All backend async calls go through
  `tokio.spawn(...)` returning a `JoinHandle`, then `cx.spawn` awaits that
  handle (polling a `JoinHandle` needs no runtime context). This is the
  pattern every future UI↔backend feature should follow.
- `hosts_update` skips a field when its `Option` arg is `None`; there is no
  clean "clear the group" path (passing `Some("")` would leave an orphan FK).
  The form only *sets* a group; credential clearing works (`Some("")`).
- Auth method "agent"/"none" are stored but the backend's agent path only
  triggers under `auth_method == "key"` with a key file — deeper agent-only
  semantics are a backend concern, out of scope here.
- SSH prompt modal is focused from `render` via a `prompt_shown` transition
  guard (the event handler has no `&mut Window`).

---

## Previous Session: 2026-09-01 (T06-004 — Diff view)

### What Was Done
- **T06-004 ✅ Done.** Reusable line-diff core + GPUI diff pane.
  - **`crates/editor/src/diff.rs`** (new, ~530 lines incl. 13 tests) —
    line-based **Myers** O(ND) diff (`myers`, trace + backtrack). Public API:
    `Diff::compute(old, new)` / `compute_with_context(old, new, ctx)` →
    `Diff { lines: Vec<DiffLine>, hunks: Vec<Hunk> }`. `DiffLine { tag:
    ChangeTag (Equal/Delete/Insert), old_line/new_line: Option<usize> (1-based),
    text }`. `Hunk { old_start/old_len/new_start/new_len, lines }` +
    `header()` (`@@ -a,b +c,d @@`) + `change_counts()`. Hunks group changed
    lines whose 3-line context windows touch (`gap > 2*ctx+1` splits).
    `Diff::is_unchanged()` / `stats() -> (ins, del)`. `side_by_side(&Hunk) ->
    Vec<SideRow>` pairs delete-run/insert-run index-wise into
    `RowKind::{Context,Delete,Insert,Replace}` rows with `SideCell { line,
    text }`. Deliberately NOT the textual git-diff format (warning #2 in the
    task) — plain manipulation-friendly line list.
  - **`crates/editor/src/lib.rs`** — `pub mod diff` + re-exports.
  - **`crates/ui/src/diff.rs`** (new, ~430 lines incl. 4 gpui tests) —
    `DiffView` GPUI component. `set_content(old, new, title, cx)` recomputes;
    `DiffLayout::{Unified, Split}` toggle (`toggle_layout` / button / `s` key);
    hunk navigation `next_hunk`/`prev_hunk` (clamped, `j`/`k` / arrows /
    header ↑↓ buttons), active hunk gets a left border in `modified` colour.
    Unified: dual line-number gutter + `+`/`-` sign, row tint from theme
    `success`/`error`. Split: two columns w/ divider, `Replace` rows tinted
    `modified`. Hunk header row uses `info`. Ellipsis divider row for
    skipped context. `on_stage_hunk(Fn(usize, &mut Window, &mut App))` hook
    (type alias `StageHunkFn`) prepared for Phase 8 hunk staging — no
    staging logic here.
  - **`crates/ui/src/theme.rs`** — added `ThemeStore::status_modified()`
    accessor (`status.modified`).
  - **`crates/ui/src/lib.rs`** — `pub mod diff` + `pub use diff::{DiffLayout,
    DiffView}`.
  - Tests: editor 60 (+13), ui 61 (+4). `cargo fmt --all --check`, `cargo
    clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
    all green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-005** (or next lowest-numbered pending in `tasks/phase-05-editor/`;
  T06-001..004 all Done) — check `tasks/ROADMAP.md`.

### Notes / Quirks (T06-004)
- Diff body renders every hunk row directly (no virtualization). Hunks
  already exclude unchanged gaps so it's bounded by change size; the task's
  "consider windowed rendering for huge diffs" note is deferred — wire
  `uniform_list` if a real perf problem shows up with Git/AI diffs.
- The ellipsis divider is informational only (not click-to-expand). Full
  context is available in `Diff::lines` if an expand affordance is wanted
  later.
- `side_by_side` pairs a delete-run with the *immediately following*
  insert-run; a hunk with interleaved equal lines between them produces
  separate single-sided rows, which is correct.
- No tab/AI/Git wiring yet — `DiffView` is standalone and exported; Phase 8
  and Phase 10 call `set_content` + `on_stage_hunk`.

---

## Previous Session: 2026-09-01 (T06-003 — Vim mode)

### What Was Done
- **T06-003 ✅ Done.** Self-contained modal Vim layer over `Document` (no
  external vim crate — the ones that exist are bound to their own buffer
  types; approach documented in the module header).
  - **`crates/editor/src/vim.rs`** (new, ~950 lines incl. 20 tests) —
    `Vim` state machine: `VimMode` (Normal/Insert/Visual/VisualLine/Command),
    `VimOptions` (number/relativenumber/hlsearch/incsearch/smartcase/expandtab/
    tabstop/shiftwidth), `VimKey` input alphabet, `VimResponse { handled, save,
    quit, reload }`. `on_key(&mut Document, VimKey) -> VimResponse`.
    - Motions: h/j/k/l, w/W/b/B/e/E, ge, 0/^/$/gg/G, `{`/`}`, `%` (bracket
      match), f/F/t/T + `;`/`,`, all with counts; combine with operators
      (`d3w`, `2dd`, `dj`, `dt.`).
    - Operators d/c/y + doubled (dd/cc/yy) + D/C/Y; `cw`→`ce` quirk.
    - Edits: x/X (count), r, ~, J, s/S, i/I/a/A/o/O, p/P (charwise +
      linewise register), u, Ctrl-R.
    - Visual + visual-line: motions extend, d/x/c/y/p operate on selection
      (charwise via `doc.anchor`, linewise via a tracked anchor line).
    - Ex line: `:w :q :wq :x :e :noh`, `:s/a/b/[g]` + `:%s/…`, `:set
      [no]opt` / `:set opt=N`. `/` `?` search + `n`/`N` wired to
      `crate::search` (smartcase-aware); `hlsearch` matches exposed on
      `Vim::search_matches`.
    - Undo units: every operator goes through `range_delete` which does
      `set_caret`×2 + `backspace` — `set_caret` breaks history coalescing so
      each command is its own undo step; insert-mode typing stays one step.
  - **`crates/editor/src/lib.rs`** — `pub mod vim` + re-exports (`Vim`,
    `VimKey`, `VimMode`, `VimOptions`, `VimResponse`).
  - **`crates/backend/src/modules/settings/editor.rs`** (new) — `EditorPrefs`
    (vim_mode + the 8 vim options) persisted as the `editor` object inside the
    shared `labonair-settings.json`; `editor_prefs_load()` /
    `editor_prefs_save()` (merge-preserving, atomic tmp+rename). `pub mod
    editor;` added to `settings/mod.rs`. 2 tests.
  - **`crates/ui/src/editor.rs`** — `EditorView.vim: Option<Vim>` built in
    `new()` from `editor_prefs_load()`. `on_key`: after the Cmd/Alt shortcut
    blocks, `vim_key(ks)` translates the keystroke (arrows→hjkl outside insert,
    Ctrl-R→Redo, printable→Char) and `handle_vim()` runs it, then
    bump_syntax/ensure_visible/refresh_matches/emit + acts on save/reload/quit.
    New `EditorEvent::CloseRequested` (from `:q`/`:wq`). Bottom status line
    (`render_vim_status`) shows the mode / live `:`-`/` command line + caret
    pos. Gutter honours `number` / `relativenumber` when vim is on.
  - **`crates/ui/src/workspace.rs`** — `watch_editor` handles
    `EditorEvent::CloseRequested` → `tabs.close(tab_id)` + `retire_tab`.
  - Tests: editor 29→49, backend +2, ui 56→57. `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets -D warnings`, `cargo test
    --workspace`, `cargo check --workspace` all green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-004** — Diff view (`tasks/phase-05-editor/T06-004-diff-view.md`).

### Notes / Quirks (T06-003)
- No Phase 12 settings UI yet: vim mode is toggled only via the persisted
  `editor.vimMode` key in `labonair-settings.json` (read once at
  `EditorView::new`) and at runtime via `:set` for the session. Wire the
  toggle into the Phase 12 Editor settings section when it lands, and call
  `editor_prefs_save` there.
- `r` / `~` currently produce **two** undo steps (delete + insert) because
  `Document` has no in-place replace; acceptable but could be tightened later.
- `:q` close path skips `focus_active` (subscribe closure has no `&Window`),
  so focus isn't moved to the next tab's view after `:q`. Minor.
- hlsearch matches are collected on `Vim::search_matches` but not yet painted
  by the editor renderer — search still moves the caret to matches, which is
  the required integration. Painting them is a small follow-up.
- `Document` caret/selection fields (`cursor`, `anchor`) are `pub`, which the
  vim layer relies on for visual mode + range math.

---

## Previous Session: 2026-09-01 (T06-002 — syntax highlighting & language detection)

### What Was Done
- **T06-002 ✅ Done.** Tree-sitter syntax highlighting + language detection +
  editor colour schemes. No web tech; native `tree-sitter` / `tree-sitter-highlight`.
  - **`crates/editor/Cargo.toml`** — added `tree-sitter 0.25`, `tree-sitter-highlight
    0.25`, and 14 grammar crates: rust, json, python, javascript, typescript,
    go, c, cpp, css, html, bash, toml-ng, yaml, java.
  - **`crates/editor/src/language.rs`** — `Language` gains `Hash`; new variants
    Java/Php/Xml/Ruby/Swift/Kotlin; more filename rules (Containerfile,
    dotfiles, `.dev`), more extensions (java/php/xml/svg/rb/swift/kt/…);
    `Language::has_grammar()`. +2 test assertions blocks.
  - **`crates/editor/src/syntax.rs`** (new) — `SyntaxHighlighter`:
    - `HighlightKind` (20 coarse token classes) + `HIGHLIGHT_NAMES` capture list
      + `capture_kind()` dotted-name → kind mapping.
    - `config(Language) -> Option<&'static HighlightConfiguration>`: lazy per
      language, built on first use, `Box::leak`-ed into a `OnceLock<Mutex<HashMap>>`.
      `build_config` wires each grammar's `HIGHLIGHT(S)_QUERY` (JS uses
      `HIGHLIGHT_QUERY`; TS/C++ concat the JS/C base query; note const-name
      inconsistency across crates). Unsupported langs (plain/md/sql/php/xml/
      ruby/swift/kotlin) → `None` → no spans (default-fg fallback).
    - `update(text, revision, visible_byte_range)`: re-parses only when the
      revision changed or the cached `covered` window no longer contains the
      viewport; keeps spans for `visible ± 32 KiB`; **breaks out of the
      Tree-sitter event stream once past the window**; 2 MiB hard size guard.
    - `line_runs(line, line_start_byte) -> Vec<StyledRun>` splits one line into
      styled/plain runs for the renderer.
    - 7 `#[test]`s (rust/python/json snippet token checks, plain-text = no spans,
      line partition exactness, viewport skips offscreen tail, revision cache reuse).
  - **`crates/editor/src/lib.rs`** — `pub mod syntax` + re-exports.
  - **`crates/ui/src/theme.rs`** — `EditorThemeId` (Auto + 9 named: atomone,
    aura, copilot, github-dark/light, nord, tokyo-night, xcode-dark/light —
    mirrors `reference-src/.../editor/lib/themes.ts`) with `slug`/`from_slug`/
    `ALL`; `ThemeStore.editor_theme` field + `editor_theme()` / `set_editor_theme()`.
  - **`crates/ui/src/syntax_theme.rs`** (new) — `EditorPalette` (one `Hsla` per
    `HighlightKind`). `resolve(id, &ThemeStore)`: `Auto` derives colours from the
    app-theme tokens (primary/accent/status_*/muted_foreground/foreground) so it
    follows light/dark + imported themes; named schemes use fixed `Roles` hex
    palettes. 3 tests (auto tracks app mode, named stable + app-independent,
    every kind resolves).
  - **`crates/ui/src/editor.rs`** — `EditorView` gains `syntax: SyntaxHighlighter`
    + `syntax_rev` (bumped in `edit`/`after_edit`; `resync_syntax()` on load/
    reload sets language + invalidates). `render()`: builds `doc_text` +
    per-visible-line byte offsets, calls `syntax.update(..)` for the visible
    range, resolves `EditorPalette` from `theme.editor_theme()`, and renders
    each line via `gpui::StyledText::with_highlights(Vec<(Range, HighlightStyle)>)`
    (falls back to a plain `div` child when a line has no spans). Theme changes
    already repaint via the existing `cx.observe(&theme)`.
  - **`crates/ui/src/lib.rs`** — `pub mod syntax_theme` + re-exports
    (`EditorPalette`, `EditorThemeId`).
  - Tests: editor 21→29, ui 53→56. `cargo fmt --all --check`, `cargo clippy
    --workspace --all-targets -D warnings`, `cargo test --workspace`, `cargo
    build --bin labonair` all green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-003** — Vim mode (`tasks/phase-05-editor/T06-003-vim-mode.md`). Dep: T06-001.

### Notes / Quirks
- Grammar crates export their highlights query under **inconsistent names**:
  `HIGHLIGHTS_QUERY` (rust/json/python/ts/go/css/html/java/toml-ng/yaml) vs
  `HIGHLIGHT_QUERY` (javascript/c/cpp/bash). Check per crate.
- `tree_sitter_highlight::Language` is a **private** re-export — use
  `tree_sitter::Language` (`tree-sitter` is a direct dep). `LanguageFn` →
  `Language` via `.into()`.
- TS grammar exposes `LANGUAGE_TYPESCRIPT` / `LANGUAGE_TSX` (not `LANGUAGE`);
  its highlights query must be concatenated **after** the JS one.
- `HighlightConfiguration::new` in 0.25 takes `(Language, name: impl Into<String>,
  highlights, injections, locals)` — the `name` param was added in 0.23.
- `tree-sitter-highlight` has no byte-range restriction API; viewport bounding is
  done by breaking the event iterator once `Source.start >= window_end`.
- `StyledText::with_highlights` uses the enclosing element's text style as the
  default run style (delayed); ranges must be char-boundary + sorted + non-overlapping.
- `tree-sitter-md` / markdown grammar not wired (block/inline split API, odd
  const names) — Markdown detected but not highlighted. Same for SQL/PHP/XML/
  Ruby/Swift/Kotlin (no grammar crate added). Easy to add later — one arm in
  `build_config` + one dep.

---

## Prior Session: 2026-09-01 (T06-001 — editor foundation & file open/save)

### What Was Done
- **T06-001 ✅ Done.** Native code-editor foundation. No CodeMirror / web tech;
  a framework-free editing model in the `labonair-editor` crate plus a GPUI
  view in `labonair-ui`.
  - **`crates/editor/`** (was an empty stub) — new modules:
    - `buffer.rs` — `TextBuffer` (`Vec<String>` lines, char-indexed
      `Position`), `insert`/`delete` handle multi-line + Unicode (columns are
      char indices, not bytes), round-trips trailing newline.
    - `history.rs` — `History` undo/redo of full snapshots, coalesces
      same-kind edits within 600ms; `EditKind::Barrier` (paste/reload/replace-
      all) never coalesces; caret moves call `break_coalescing`.
    - `search.rs` — literal (non-regex) `find_all` / `next_match` (wrapping) /
      `replace_all`, case-sensitive + whole-word options, single-line matches.
    - `language.rs` — `Language` enum, extension/filename detection (status
      bar + T06-002 grammar hook prep).
    - `document.rs` — `Document` = buffer + caret + selection + history +
      saved-text baseline (`is_dirty`), `disk_mtime` + `external_change`
      flag, `Motion` enum (word/line/doc/page nav with sticky goal column),
      `insert`/`backspace`/`delete_forward`/`undo`/`redo`/`replace_all`/
      `reload`/`mark_saved`.
    - Cargo.toml: dropped the unused `gpui` dep (core is framework-free).
  - **`crates/backend/src/modules/fs/file.rs`** — added blocking sync fns for
    the editor (run on `cx.background_executor().spawn`): `load_editor_file_sync`
    (`EditorLoad::{Text{content,mtime},Binary,TooLarge}`, 10 MB limit, null-byte
    + strict-UTF-8 sniff — same rules as `fs_read_file`), `save_editor_file_sync`
    (atomic temp+rename, returns new mtime), `file_mtime_sync`. Existing async
    fns untouched.
  - **`crates/ui/src/editor.rs`** (new) — `EditorView` GPUI entity:
    viewport-based line rendering (only visible lines painted) with a
    line-number gutter, caret + selection overlays, current-line highlight;
    keyboard editing via `on_key_down` (printable chars, Enter, Tab→4 spaces,
    Backspace/Delete, arrows/Home/End/PageUp-Down, Alt+arrows = word,
    Cmd+arrows = line/doc, Shift extends selection); Cmd+S save, Cmd+Z/Cmd+Shift+Z
    undo/redo, Cmd+A select-all, Cmd+C/X/V clipboard, Cmd+F find bar.
    Find bar: query + replace fields (Tab toggles), Enter/Shift-Enter next/prev,
    Cmd+Enter replace-all, Cmd+C/Cmd+W toggle case/whole-word, Esc closes.
    External-change: `check_external` re-stats on tab activation — auto-reloads
    if clean, else shows a conflict banner (Reload / Keep mine). Emits
    `EditorEvent::{Changed,Edited}`. All file IO on the background executor.
  - **`crates/ui/src/workspace.rs`** — `editors: HashMap<u64, Entity<EditorView>>`;
    rewrote `open_file(path, peek, window, cx)` with full **peek-tab** semantics
    (single click = peek/reuse the peek tab, double-click or already-open = 
    permanent, first edit un-peeks via `EditorEvent::Edited`); `new_editor_tab`
    (Cmd+E), `save_active` (Cmd+S), `find_in_active_editor` (routes Cmd+F);
    `render_content` renders the `EditorView` for `TabKind::Editor`;
    `focus_active` + `retire_tab` + ActiveTabChanged subscription (fires
    `check_external`) handle editors.
  - **`crates/ui/src/menu.rs`** — new `Save` action, `cmd-s` binding, File ▸ Save
    menu item (binding count test 23→24).
  - **`crates/ui/src/app_shell.rs`** — `act_save`, `act_new_editor_tab` wired as
    root actions; `act_find` now tries the editor find bar first, falls back to
    the terminal/header search.
  - **`crates/ui/src/theme.rs`** — added `ThemeStore::buffer_font_size()`.
  - **`crates/ui/src/explorer.rs`** — `open_file` passes `peek = click_count < 2`.
  - Tests: editor crate 21, ui crate 53 (2 new editor-view tests). Full
    `cargo test --workspace` green; clippy `-D warnings` + `cargo fmt --check`
    clean.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-002** — syntax highlighting & language detection (Tree-sitter grammars;
  `Language` enum already in place as the hook point).

### Notes / Quirks
- gpui 0.2.2 `ScrollDelta::Lines(p)` — `p.y` is already `f32` (no `f32::from`);
  `Pixels(p)` needs `f32::from(p.y)`.
- Clippy rejects an inherent method named `from_str` (confusable with
  `FromStr::from_str`) even with `-D warnings` off-by-default lint promoted —
  used `TextBuffer::from_text`.
- `on_click` needs `gpui::StatefulInteractiveElement` in scope + an `.id(...)`
  on the div.

---

## Prior Session: 2026-09-01 (T05-002 — drag-and-drop & advanced file actions)

### What Was Done
- **T05-002 ✅ Done.** Explorer drag-and-drop + copy/cut/paste buffer + OS
  file drop, ported from `reference-src/src/modules/explorer/lib/`
  (`explorerDrag.ts`, `useInternalDrop.ts` `canDropInto`/`resolveDropTarget`,
  `useOsFileDrop.ts`, `useFileTree.movePath`) and backend
  `src-tauri/src/modules/fs/mutate.rs` (`fs_copy_into`).
  - **`crates/backend/src/modules/fs/mutate.rs`** — new blocking sync fns:
    `move_into_sync(src, dest_dir) -> Result<String,String>` (rename into a
    dir; refuses overwrite / move-onto-own-parent / move-into-self-or-
    descendant), `copy_into_sync(&[String], &str)` (extracted core of
    `fs_copy_into`, which now just wraps it in `spawn_blocking`). +2 tests
    (move roundtrip + conflict + into-self + no-op + missing-dest guards;
    copy conflict-rename `name (1).ext` + source-left-in-place + missing src).
    backend tests 131 → 133.
  - **`crates/ui/src/explorer.rs`**:
    - module-level: `pub struct DraggedPaths { paths: Vec<PathBuf> }` (drag
      payload, exported from `lib.rs`), `DragPreview` render entity (pointer
      chip), `ClipOp{Copy,Cut}` + `Clipboard{op,paths}`, `pub fn shell_quote`
      / `pub fn quote_paths` (single-quote wrap unless all-safe chars),
      `can_drop_into(src,dest)` (port of `canDropInto`).
    - `ExplorerView`: `selected: Option<PathBuf>` → `selection: Vec<PathBuf>`
      (Cmd/Shift-click toggles additive; plain click still selects+opens/
      expands). New fields `clipboard: Option<Clipboard>`, `drop_target:
      Option<PathBuf>`.
    - copy/cut/paste: `clip_set`/`clip_clear`/`is_cut`/`paste_into(dir)` +
      `action_paths(path)` (whole selection if `path` is in it, else just it).
      Paste runs `copy_into_sync` (Copy) or a loop of `move_into_sync` (Cut,
      then clears buffer) on the background executor, reloads affected dirs,
      toasts errors. Guards dest against self/descendant.
    - drag: folder + file rows get `.on_drag(DraggedPaths, |_,_,_,cx|
      cx.new(DragPreview))`. Folder rows + the root list container get
      `.on_drag_move::<DragMoveEvent<DraggedPaths>>` (sets `drop_target` for
      the accent highlight), `.on_drop::<DraggedPaths>` → `drop_move` (move,
      `can_drop_into`-filtered), `.on_drop::<ExternalPaths>` → `drop_external`
      (copy_into + success toast). List container drop = into root.
    - terminal drop: `crates/ui/src/terminal.rs` root div gets
      `.on_drop::<DraggedPaths>` → `send_input(quote_paths(&paths) + " ")`.
    - UI: clipboard banner above the list ("N items copied/cut" + Paste +
      Clear), cut rows render at `opacity(0.5)` + error color, drop-target
      folder rows render `bg(accent)`. Context menu gains Copy / Cut /
      Paste (Paste only when buffer non-empty). Explorer-level `on_key`:
      Cmd-C/X/V, Esc clears buffer then selection.
    - +3 pure tests (`can_drop_into` noop/self/descendant, `shell_quote` +
      `quote_paths`, clipboard buffer set/replace/discard). ui tests 48 → 51.
  - **`crates/ui/src/lib.rs`** — `pub use explorer::{DraggedPaths, ExplorerView}`.
- **GPUI API (gpui 0.2.2):** `ExternalPaths(pub(crate) SmallVec<[PathBuf;2]>)`
  with `.paths() -> &[PathBuf]` + blanket `Render`; `InteractiveElement::
  on_drop::<T>` pushes per-TypeId so multiple `.on_drop` with different `T`
  on one element is fine (unlike `.on_drag`, which debug-panics on a second
  call); `ClickEvent::modifiers() -> Modifiers` (`.secondary()` = Cmd on
  mac, `.shift`); `cx.new(...)` inside an `on_drag` constructor closure
  (`&mut App`) needs `use gpui::AppContext`.
- **Deviations / limits:** no name-conflict *dialog* on drop/paste — a move
  onto an existing name fails with an error toast (backend refuses overwrite),
  copy auto-renames `name (1).ext` (reference `fs_copy_into` behavior). No
  chmod/chown: `localFsProvider` in the reference has no chmod/chown (SFTP-
  only — Phase 07), and the task instructions don't mention it, so
  `ChmodChownDialog` is not ported here. `drop_target` highlight clears on
  drop/handler, not on drag-cancel-outside (stale until next notify). Range
  (shift-click) select is treated as additive-toggle, not a true range.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace` (backend 133,
  ui 51), `cargo build --bin labonair` — all green. Not visually run — user
  should `cargo run` and check: drag a file onto a folder → it moves (folder
  highlights while hovering), drag a file onto a terminal pane → its quoted
  path is typed, right-click → Copy/Cut then right-click a folder → Paste,
  Cmd-C/Cmd-X/Cmd-V, cut items dim red, banner shows + Clear works, drop a
  file from Finder onto the tree → it is copied in.

### Current State
- Branch `master`, ~24 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded.
- `reference-src/` untouched. **Phase 04: T05-001 + T05-002 done.**

### What's Next
- Next roadmap task after T05-002 (see `tasks/ROADMAP.md`) — Phase 04 is
  complete if T05-002 is its last task; otherwise the next `T05-*`. Likely
  Phase 05 (Editor) T06-001.

---

## Session: 2026-09-01 (T05-001 — file tree & explorer basics)

### What Was Done
- **T05-001 ✅ Done.** Sidebar file explorer, ported from
  `reference-src/src/modules/explorer/` (`useLocalExplorerStore` +
  `useFileTree` + `buildTreeRows` + `FileTreeNode`) and the backend
  `src-tauri/src/modules/fs/` (`tree.rs`, `mutate.rs`, `watcher.rs`).
  - **`crates/backend/src/modules/fs/tree.rs`** — added blocking, in-process
    entry point `read_dir_page(path, offset, limit, show_hidden)` (=
    `list_dir_entries_sync` + `paginate_dir_entries`, both made `pub`),
    `pub const DEFAULT_LOCAL_PAGE_LIMIT`, `#[derive(Clone, Debug, ...)]` on
    `EntryKind` / `DirEntry` / `DirReadPage`. +3 tests (dirs-first
    case-insensitive sort, hidden toggle, missing dir → Err not panic).
  - **`crates/backend/src/modules/fs/mutate.rs`** — added sync variants
    `create_file_sync` / `create_dir_sync` / `rename_sync` / `delete_sync`
    (same semantics as the async commands, no `spawn_blocking`). +2 tests
    (create/rename/delete roundtrip incl. conflict refusal, delete-missing
    → Err).
  - **`crates/ui/src/explorer.rs`** (new) — `TreeModel` (pure port of
    `useLocalExplorerStore` + `buildTreeRows`: per-dir node map
    `Loading|Loaded{entries,has_more}|Error`, `generation` stale-guard, lazy
    `needs_load` dedup, `toggle_show_hidden` cache-invalidate-but-keep-expanded,
    `rows()` flatten with pending-create / rename / loading / error / load-more
    rows) + `ExplorerView` GPUI entity wrapping it:
    - `set_root`/`set_root_str` (bumps generation, clears cache, loads root),
      `load_dir` runs `tree::read_dir_page` on `cx.background_executor().spawn`
      and drops the result if `generation` moved,
    - toolbar: new file / new folder / refresh / toggle-hidden / collapse-all,
    - rows in an `overflow_y_scroll` column (chevron + glyph + name, indent by
      depth, selection highlight, gitignored = muted), click folder →
      expand/collapse, click file → `Workspace::open_file` (Editor
      placeholder tab titled with basename, path in `TabData.path`),
    - right-click → context menu (New File/Folder, Rename, Delete, Copy Path,
      Open in Terminal) anchored top-left with a click-catch backdrop,
    - inline text field (hand-rolled like the header search: `key_char` +
      backspace/enter/escape) for create + rename → `mutate::*_sync` off-thread,
      reload parent on success, error toast (`notification_center`) on failure,
    - **embedded watcher**: `notify-debouncer-mini` (300 ms, non-recursive,
      watch-set synced to loaded dirs); debounced callback pushes affected
      parent dirs into an `Arc<Mutex<HashSet>>` drained by a 400 ms
      `cx.spawn` loop that force-reloads any still-loaded dirty dir.
    - delete confirmation modal overlay.
    - 6 pure `#[test]`s on `TreeModel` (flatten+depth+collapse, lazy-load
      dedup, set_root generation+clear, hidden-toggle invalidation,
      watch-target set, glyph map).
  - **`crates/ui/src/workspace.rs`** — added `new_terminal_tab_in(cwd, …)`
    (Explorer "Open in Terminal") and `open_file(path, …)` (Editor placeholder
    tab); `render_content` Editor arm now shows the file path.
  - **`crates/ui/src/app_shell.rs`** — `AppShell` owns
    `explorer: Entity<ExplorerView>`; `render_panel_body` returns the real
    view for `SidebarPanel::Explorer`; a `cx.observe(&workspace)` pushes the
    active terminal cwd (fallback `$HOME`) into `ExplorerView::set_root_str`.
  - **`crates/ui/Cargo.toml`** — `notify = "6"`, `notify-debouncer-mini = "0.4"`.
  - **`crates/ui/src/lib.rs`** — `pub mod explorer;` + `pub use ExplorerView`.
- **GPUI notes:** `overflow_y_scroll` is a `StatefulInteractiveElement` method
  → the scroll container needs an `.id(...)` first. `EditMode` enum gets
  `#[derive(Default)]` + `#[default]` on the unit variant. On macOS the
  `notify` `FsEventWatcher` exposes inherent `watch`/`unwatch` so the
  `notify::Watcher` trait import is unused (removed). `ClipboardItem::new_string`
  for Copy Path.
- **Deviations:** no `@tanstack/react-virtual` windowing (plain scroll column;
  500-entry page cap + lazy load bound the element count); watcher is embedded
  in the entity instead of the backend event bus; file-type icons are a small
  emoji glyph map, not the material-icon-theme port; SFTP/remote scope +
  `remoteScopeCache` LRU are Phase 07, not ported here; chmod/chown dialog and
  DnD are T05-002.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin
  labonair` — all green. Counts: **backend 131 (+5)**, **ui 48 (+6)**. Not
  visually run — user should `cargo run` and check: Explorer panel shows the
  cwd tree, folders expand/collapse, click a file opens an "Editor" tab,
  right-click → New File/Folder/Rename/Delete/Copy Path/Open in Terminal,
  create a file in Finder and watch the tree refresh, toggle-hidden button
  shows/hides dotfiles.

### Current State
- Branch `master`, ~23 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded.
- `reference-src/` untouched. **Phase 04: T05-001 done, T05-002 next.**

### What's Next
- **T05-002** (`tasks/phase-04-explorer/T05-002-drag-drop-actions.md`) —
  drag-and-drop + advanced file actions (chmod/chown, copy/cut/paste, OS drop).
  Dep: T05-001 (done).

---

## Session: 2026-09-01 (T04-005 — native macOS menus + Dock menu)

### What Was Done
- **T04-005 ✅ Done.** Native macOS menu bar + Dock context menu, ported from
  `reference-src/src-tauri/src/lib.rs` `build_menu`, `modules/dock_menu.rs`,
  `modules/menu_sync.rs`.
  - **`crates/ui/src/menu.rs`** (new):
    - `gpui::actions!(labonair, [...])` — 40 unit-struct actions covering every
      menu entry (File/Edit/View/Terminal/Connections/AI/Window/App).
    - `init(cx: &mut App)`: `cx.bind_keys(bindings())` (23 `KeyBinding`s mirroring
      the reference accelerators — drives both the menu shortcut hint *and*
      dispatch), 8 app-global `cx.on_action` handlers (Quit→`cx.quit()`,
      HideApp/HideOthers/ShowAll→`cx.hide*`, About/OpenSettings/AiSettings/
      CheckForUpdates→info toast placeholders via `notification_center`),
      `cx.set_menus(app_menus())`, `#[cfg(macos)] cx.set_dock_menu(dock_menu())`.
    - `app_menus()` — 8 submenus, structure/order/labels 1:1 with the reference
      (App menu adds Services `os_submenu` + Check for Updates; Edit uses
      `MenuItem::os_action(.., OsAction::{Undo,Redo,Cut,Copy,Paste,SelectAll})`).
      Window submenu named exactly `"Window"` so GPUI wires the live window list.
    - `dock_menu()` — New Terminal Tab / New SSH Connection… / — / Open Host Manager.
    - 3 unit tests (binding count = parse-all-succeed, menu-bar name order,
      dock entry count).
  - **`crates/ui/src/app_shell.rs`** — 12 action handler methods
    (`act_new_terminal_tab`, `act_close_tab`, `act_close_pane`, `act_split_right/
    _down`, `act_find`, `act_toggle_sidebar`, `act_toggle_fullscreen`,
    `act_minimize`, `act_zoom_window`, `act_next_tab`, `act_prev_tab`) registered
    via `.on_action(cx.listener(...))` on the root element. `SplitPaneRight/Down`
    are only registered `when(active_is_terminal)`, `ClosePane` only
    `when(active_has_split)` — so those menu items grey out automatically
    (macOS calls `validate_menu_item` → `is_action_available` against the live
    focus dispatch tree on every menu open; no `set_menus` re-sync needed).
    Removed `AppShell::on_key_down` (Cmd-B / Cmd-F) — now the `ToggleSidebar` /
    `Find` actions.
  - **`crates/ui/src/workspace.rs`** — added thin `pub` wrappers
    (`new_terminal_tab`, `split`, `close_active`, `close_pane`, `cycle`,
    `active_has_split`) so menu + shortcut share one path. Removed the
    Cmd-T/W/D/Shift-D arms from `Workspace::on_key_down` (now actions; the
    Cmd-Shift-`[`/`]` tab-cycle arms stay as an extra binding alongside the new
    `ctrl-tab` / `ctrl-shift-tab`).
  - **`crates/app/src/main.rs`** — `labonair_ui::init_menus(cx)` right after
    `open_window`, before `cx.activate(true)`.
  - **`crates/ui/src/lib.rs`** — `pub mod menu;` + `pub use menu::init as init_menus`.
- **GPUI API used (gpui 0.2.2, verified in source `src/platform/app_menu.rs` +
  `src/platform/mac/platform.rs` + `examples/set_menus.rs`):**
  `App::set_menus(Vec<Menu>)`, `App::set_dock_menu(Vec<MenuItem>)` (mac only),
  `Menu { name: SharedString, items: Vec<MenuItem> }`,
  `MenuItem::{action(name, impl Action), os_action(name, action, OsAction),
  os_submenu(name, SystemMenuType::Services), separator}`,
  `gpui::actions!(namespace, [Name, ...])` (unit structs, derive `gpui::Action`),
  `KeyBinding::new(&str, action, Option<&str>)` (**panics** on bad keystroke —
  `"cmd--"` / `"cmd-="` / `"ctrl-cmd-f"` all parse fine),
  `App::{quit, hide, hide_other_apps, unhide_other_apps}`,
  `Window::{toggle_fullscreen, minimize_window, zoom_window}` (all `&self`).
  Menu enable/disable is driven entirely by `is_action_available` (focus
  dispatch tree) — GPUI's `MenuItem::Action` has **no** `enabled` field and no
  checkmark support, so dynamic checkmarks / recent-hosts / window-list beyond
  what GPUI auto-provides are deferred to their feature phases.
- **Deviations / limits:** items for not-yet-built features (New SSH/SFTP/Preview/
  Editor Tab, Open Host Manager, New SSH Connection, New Quick SSH, New AI
  Session, Ask about Selection, Clear Chat, Toggle AI Panel, Keyboard Shortcuts,
  Zoom In/Out/Reset) have **no handler** → render disabled (per task note);
  their phase adds `.on_action` and they light up. `CloseTab` maps to the
  existing pane-aware `close_active_pane_or_tab` (closes pane if split, else
  tab); `ClosePane` (`cmd-shift-w`) always closes the pane. About / Settings /
  Check for Updates are toast placeholders (Settings = Phase 12, updater =
  T15-005). No `MenuItemRegistry`-style accelerator hot-swap (that's Phase 12
  shortcut config) — bindings are static in `menu::bindings()`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` (all green; **ui 42, +3**),
  `cargo build --bin labonair` — all green. Not visually run — user should
  `cargo run` and check: full menu bar (Labonair/File/Edit/View/Terminal/
  Connections/AI/Window), Cmd-T / Cmd-W / Cmd-D / Cmd-F / Cmd-B still work via
  the menu path, "Split Pane …" greys out on the Home tab and enables on a
  terminal tab, "Close Pane" greys out until a tab is split, right-click the
  Dock icon → New Terminal Tab works.

### Current State
- Branch `master`, ~22 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded.
- `reference-src/` untouched. **Phase 03 complete** (T04-001..005 all done).

### What's Next
- **Phase 04 — File-Explorer: T05-001** (`tasks/phase-04-explorer/T05-001-*`) —
  Dateibaum & Explorer-Grundlagen. Dep: T04-002 (done). No blockers.

---

## Session: 2026-09-01 (T04-004 — notifications / toast system)

### What Was Done
- **T04-004 ✅ Done.** App-wide notification/toast system, ported from
  `reference-src/src/modules/notifications/` (`useNotificationStore` +
  `NotificationDropdown`).
  - **`crates/ui/src/notifications.rs`** (new):
    - `NotificationCenter` GPUI entity — `items: Vec<Active>` newest-first, `next_id`,
      `notify_on_errors: bool` (default `true` until T13-001 wires the real preference).
      Ported store logic: **2s spam guard** on `title + body + severity` vs `items[0]`
      (`insert(notif, now, cx)` takes an explicit `now` and is `pub` for deterministic
      tests; `push`/`push_action_result` call it with `Instant::now()`), **100-item cap**
      (`truncate`), **error gate** — `push` drops `Severity::Error` when
      `!notify_on_errors` (ref `addNotification`), `push_action_result` bypasses it
      (ref `addActionResultNotification`). `dismiss(id)`, `clear_all()`, `len`/`is_empty`,
      `snapshots() -> Vec<ToastSnapshot>` (render view), `trigger_action(id, window, cx)`
      (fires the `FnMut` callback once, then removes the toast).
    - `Severity{Info,Success,Warning,Error}` + `default_timeout()` = info 5s / success 4s /
      warning 8s / **error None** (manual dismiss — ref never auto-cleared errors) + `glyph()`
      + `color(&ThemeStore)`.
    - Per-toast **auto-dismiss**: `insert` does `cx.spawn(async move |this, cx| { timer(d).await;
      this.update(..dismiss(id)) })` when a timeout resolves.
    - `Notification` builder (`info/success/warning/error(title, body)` + `.source()` /
      `.timeout()` / `.action()`), `NotificationAction::new(label, impl FnMut(&mut Window, &mut App))`
      (`type ActionCallback = Box<dyn FnMut(..)>`; manual `Debug`).
    - `render_overlay(&Entity<NotificationCenter>, &Entity<ThemeStore>, &mut App) -> Option<AnyElement>`
      — `.absolute().top_4().right_4()` flex-col stack of 360px cards (`bg-card`, 1px border in the
      severity color, `shadow_lg`, glyph + title[semibold] + optional source badge + muted body,
      close ✕, optional accent action button). The container only occupies its own top-right box, so
      clicks elsewhere pass through — **warning satisfied** (only the cards are interactive).
    - Global `GlobalNotificationCenter` + `init(cx)` / `notification_center(cx)`, and the
      `notify_err(title, Result<T, String>, &mut App) -> Option<T>` helper (pushes an error toast via
      the action-result path on `Err`).
    - 8 `#[gpui::test]`s: newest-first + ids, spam guard (block/allow by body/type/time),
      different-titles kept, error gate + action-result bypass, 100 cap, dismiss/clear,
      action label wiring, auto-dismiss after `advance_clock`.
  - **`crates/ui/src/theme.rs`** — new accessors `status_error()` / `status_warning()` /
    `status_info()` / `status_success()` (read `theme().status.*`).
  - **`crates/ui/src/app_shell.rs`** — `AppShell` gained a `notifications: Entity<NotificationCenter>`
    field + `new()` param, observes it, renders `render_overlay(...)` via `.children(toasts)` after the
    background layer. `#[cfg(debug_assertions)]` startup demo toast (acceptance: reachable from anywhere).
  - **`crates/app/src/main.rs`** — `labonair_ui::init_notifications(cx)` before `AppShell::new`.
  - **`crates/ui/src/lib.rs`** — `pub mod notifications;` + re-exports (`init_notifications`,
    `notification_center`, `notify_err`, `Notification`, `NotificationAction`, `NotificationCenter`,
    `Severity`, `GlobalNotificationCenter`).
- **No new deps.** GPUI APIs used were all already known (`cx.spawn` async-closure form,
  `background_executor().timer`, `on_click` needs `StatefulInteractiveElement` in scope, tuple
  `ElementId` `(&str, u64)`, `.children(Option<AnyElement>)`).
- **Deviations / notes:** the reference notifications module is a *persistent dropdown* (bell icon +
  popover list, no auto-dismiss); the task asked for a *toast* system, so this port keeps the store
  semantics 1:1 but renders stacked auto-dismissing toasts instead of the popover. No bell/dropdown
  entry point in the status bar yet (needs the bar-item placement settings from T13 + an icon set) —
  the store keeps the full history regardless, so a dropdown can be added later without touching it.
  `notify_on_errors` defaults to `true` (reference default is `false`, gated by a preference we don't
  have yet). `relativeTime` / "Copy" affordance from the dropdown not ported (toast-inappropriate).
- **Verified:** `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (all suites green; **ui 39, +8**), `cargo fmt --all --check`,
  `cargo build --bin labonair` — all green. Not visually run — user should `cargo run` and check:
  a demo info toast appears top-right on launch and auto-dismisses after ~5s; toasts stack; ✕ closes
  one; clicking through the area where a toast *isn't* still hits the UI behind it.

### Current State
- Branch `master`, ~22 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit.
- `reference-src/` untouched.

### What's Next
- **T04-005 — Native macOS menus** (`tasks/phase-03-tabs-workspace/T04-005-native-macos-menus.md`).
  Dep: T04-003 (done). No blockers.

---

## Session: 2026-09-01 (T04-003 — app-shell & window chrome)

### What Was Done
- **T04-003 ✅ Done.** Introduced `AppShell` as the app's root coordinator (ported from `reference-src/src/app/components/AppShell.tsx` + `modules/header/Header.tsx` + `modules/statusbar/StatusBar.tsx`). `cargo run` now shows the full app layout.
  - **`crates/ui/src/app_shell.rs`** (new) — `AppShell` GPUI root view. Pure composition, no feature logic:
    - Owns `theme`, `background`, `workspace: Entity<Workspace>` (creates the `TerminalRegistry` + `Workspace` here now, moved out of the old `Root` in `main.rs`), plus sidebar state (`open`, `width`, `active_panel`) and header search state.
    - **Header** (`HEADER_H = 40`, `bg-toolbar`, `border-b`): ☰ sidebar toggle, "Labonair" title, flex spacer, inline search field (shown on `Cmd-F`), `⋯` menu affordance (placeholder). Left inset `TRAFFIC_LIGHT_INSET = 78` (native titlebar still on, see deviation below).
    - **Body row**: optional left sidebar + `Workspace` (flex_1). Sidebar = a `RAIL_W = 44` panel-switcher rail (`SidebarPanel` enum: Explorer/Snippets/SourceControl/GitGraph/Ai — each a slot rendering "coming in a later phase") + the active panel body + a `cursor_col_resize` edge handle (drag via `on_drag(SidebarResize)` + `on_drag_move::<SidebarResize>` on the body row → `set_sidebar_width`, clamp 180–520). Toggle: ☰ button or `Cmd-B`. Clicking the active panel's rail icon collapses the sidebar.
    - **Status bar** (`STATUS_H = 32`, `bg-status-bar`, `border-t`): left = cwd breadcrumb built from `Workspace::active_cwd` (split on `/`, `~` home substitution via `dirs::home_dir`, last segment = `foreground`, rest muted, `/` separators); falls back to the active tab label when no cwd. Right = `N panes` when >1 + empty slots (connection / jump-host / AI badges land in Phase 06/10).
    - **Inline search**: `Cmd-F` opens a focusable field; typed chars go via `keystroke.key_char` (fallback to key name for single chars), `backspace`/`enter` re-run, `escape` closes + clears + refocuses the pane. Dispatched to `Workspace::search_active` → `TerminalView::search`.
  - **`crates/ui/src/window_state.rs`** (new) — minimal window geometry persistence (`<data_dir>/labonair/window.json`). `load()` on launch (in `main.rs`, feeds `WindowOptions.window_bounds`), `save()` from `AppShell` throttled per render (`SAVE_THROTTLE = 1s`, `bounds_differ` > 2px) **and** on `window.on_window_should_close`. `window_min_size` = 720×480. Overlaps intentionally with T14-001 (session persistence), which will supersede/extend this.
  - **`crates/ui/src/terminal.rs`** — added `TerminalView::search(query, cx) -> bool`: scans the visible screen (`SessionHandle::render().to_text()`) for the first `query` occurrence, selects it via `update_selection((col,row),(col+len,row))`; empty query clears selection. Visible-screen only — a full scrollback find widget with next/prev is a later search-module concern.
  - **`crates/ui/src/workspace.rs`** — **stripped its own chrome** (removed `render_header`/`render_sidebar`/`render_statusbar`, `sidebar_open`/`sidebar_width` fields, `SidebarResize`, `Cmd-B`, `HEADER_H`/`STATUS_H`/`SIDEBAR_*` consts). `Render` is now just tab bar + content + overlays. Added pub accessors for `AppShell`: `active_cwd`, `active_tab_label`, `active_pane_count`, `active_is_terminal`, `search_active`, `focus`.
  - **`crates/app/src/main.rs`** — deleted the old `Root` view; root is now `AppShell::new(...)`. Added `TitlebarOptions { title: "Labonair" }`, `window_min_size`, `window_state::load()` restore.
  - **`crates/ui/Cargo.toml`** — moved `serde_json` to deps, added `dirs`. **`crates/theme` → `crates/ui/src/theme.rs`** — new accessors `toolbar()`, `title_bar()`, `status_bar()`, `sidebar_bg()`, `sidebar_border()`, `sidebar_fg()`.
- **GPUI API used (gpui 0.2.2, verified in source):** `Window::on_window_should_close(cx, Fn(&mut Window, &mut App) -> bool)`; `Window::window_bounds() -> WindowBounds` (`Windowed(Bounds<Pixels>)` variant carries the restore rect); `WindowOptions.titlebar: Option<TitlebarOptions { title, appears_transparent, traffic_light_position }>`, `window_min_size: Option<Size<Pixels>>`. No `observe_window_bounds`/resize hook exists → geometry save is throttled-in-render + on-close. `Keystroke.key_char: Option<String>`.
- **Deviation / limits:** kept the **native macOS titlebar** (`appears_transparent: false`) rather than a custom transparent drag-region titlebar with tabs-in-titlebar (reference default). Traffic lights + native window drag work; the reference's custom-drawn titlebar + `data-tauri-drag-region` + window-move-on-drag was judged out of scope for this task — revisit if design parity demands it. Tab bar stays as its own strip below the header (reference `tabsLocation` supports a "pane" mode, so this is a supported layout, not a regression). Right sidebar still deferred (was already deferred in T04-002).
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (all 14 suites green; ui unchanged at 31 — no new unit tests, the feature is view/window wiring), `cargo build --bin labonair` — all green. Not visually run — user should `cargo run` and check: header + sidebar rail + tabs + statusbar all render; ☰ / `Cmd-B` toggle sidebar; rail icons switch panel; drag sidebar edge to resize; `Cmd-F` opens header search, typing scrolls/highlights a match in the active terminal, `Esc` closes it; statusbar breadcrumb tracks `cd`; resize/move the window, quit, relaunch → geometry restored.

### What's Next
- **T04-004 — Notifications & Toasts** (`tasks/phase-03-tabs-workspace/T04-004-notifications-toasts.md`). No blockers.

---

## Session: 2026-09-01 (T04-002 — split-pane layout & workspace shell)

### What Was Done
- **T04-002 ✅ Done.** Added the split-pane layout tree + full workspace window chrome (pure Rust + GPUI, ported from `reference-src/src/modules/tabs/types.ts` + `store/tabsStore.ts` `splitPane`/`closePane` and `WorkspacePane.tsx`).
  - **`crates/ui/src/pane.rs`** (new) — pure, serde-serializable pane tree:
    - `PaneNode` = `Pane { id }` | `Split { id, axis, ratio, first, second }` (`#[serde(tag="type")]`, axes `horizontal`/`vertical` lowercased). `SplitAxis::Horizontal` = children left→right. `MIN_RATIO = 0.1`.
    - `WorkspaceLayout { root, active }` (no id allocation — the `Workspace` view owns a process-wide `next_pane_id`; `split()`/`new()` take caller-allocated ids so pane ids stay unique across tabs). Ops: `new(first)`, `split(split_id, new_pane, axis)` (replaces active leaf with a 50/50 split, new pane becomes active — mirrors reference `splitPane`), `close(target) -> CloseOutcome::{Closed{new_active}, LastPane, NotFound}` (collapses parent split into sibling; new active = **promoted sibling subtree's first leaf**, matching reference `siblingLeaves[0]`, not the tree's global first leaf), `set_active`, `set_ratio`/`reset_ratio` (clamped `[MIN_RATIO, 1-MIN_RATIO]`), `leaves()`, `len()`.
    - 6 unit tests: split nesting/activation, close→collapse-into-sibling, close keeps a non-removed active valid, ratio clamping + reset, 8-pane deep nest build+teardown stays non-empty/consistent, JSON round-trip.
  - **`crates/ui/src/workspace.rs`** — `Workspace` now owns `layouts: HashMap<tab_id, WorkspaceLayout>` (survives tab switches — pane tree never lost) + `panes: HashMap<PaneId, PaneEntry{session_id, view}>` + `next_pane_id`. Replaced the old one-`TerminalView`-per-tab map.
    - `split_active(axis)` — spawns a new session in the active pane's cwd, `layout.split(...)`, builds a `TerminalView`, focuses the new pane. `close_active_pane` — `layout.close`; `LastPane` → `request_close(tab)`, else `retire_pane` (registry.close). `Cmd-W` = `close_active_pane_or_tab` (pane if split, else whole tab). `retire_tab` tears down **every** pane/session in the closed tab's layout.
    - `render_pane_node` recursively renders the tree: `Split` → flex row/col, `flex_basis(relative(ratio))` first child + `flex_grow` second, a draggable divider (`HANDLE = 6px`, `cursor_col_resize`/`row_resize`); drag handled by `on_drag(PaneResize{split_id})` + `on_drag_move::<DragMoveEvent<PaneResize>>` on the split group — computes new fraction from `ev.bounds` + `ev.event.position` along the axis → `resize_split`. Double-click the divider → `reset_split` (50/50). `Pane` leaf → its `TerminalView` (or empty), 1px border (accent when active+multi, else bg-coloured so no layout shift), click-to-activate.
    - **Workspace hull**: `render_header` (☰ sidebar toggle + "Labonair", `HEADER_H = 36`), body row = optional left sidebar + central column (tab bar over content), `render_statusbar` (active tab label + "· N panes", `STATUS_H = 24`). `render_sidebar` — `card`-bg column (default 260px, clamp 180–520) with an "EXPLORER" header + placeholder + a `cursor_col_resize` edge handle; drag via `on_drag(SidebarResize)` + `on_drag_move::<SidebarResize>` on the body row → `set_sidebar_width` (position.x − row origin.x). Toggle: header button or `Cmd-B`.
    - Keys added to `on_key_down`: `Cmd-D` split horizontal, `Cmd-Shift-D` split vertical, `Cmd-B` toggle sidebar. `Cmd-W` now pane-aware.
  - **`crates/ui/src/terminal.rs`** — `TerminalView` grids to **its own content area** now, not the whole window: new `measured: Option<Size<Pixels>>` field captured every paint by an absolute `canvas(|bounds,_,cx| this.measured = Some(bounds.size) + notify-on-change, |_,_,_,_| {})` child; `render` uses `self.measured.unwrap_or_else(|| window.viewport_size())` for `grid_size`. Needed so split-pane terminals don't oversize.
  - **`crates/ui/Cargo.toml`** — added `serde` (workspace) dep + `serde_json` dev-dep. **`crates/ui/src/lib.rs`** — `pub mod pane;` + re-exports `CloseOutcome, PaneId, PaneNode, SplitAxis, WorkspaceLayout`.
- **GPUI API used (gpui 0.2.2, verified in source):** `gpui::canvas(prepaint: FnOnce(Bounds<Pixels>, &mut Window, &mut App) -> T, paint: FnOnce(Bounds<Pixels>, T, ...))` — `impl Styled for Canvas<T>` so `.absolute().size_full()` chain works; capture `cx.weak_entity()` and `weak.update(cx, ...)` inside the prepaint closure (it gets `&mut App`). `InteractiveElement::on_drag_move::<T>(Fn(&DragMoveEvent<T>, &mut Window, &mut App))` — `DragMoveEvent { event: MouseMoveEvent, bounds: Bounds<Pixels> }`, `.drag(cx) -> &T` reads the active drag value; `cx.listener(...)` adapts directly. `gpui::relative(f32) -> DefiniteLength` for `flex_basis`. `ClickEvent::click_count()` for double-click. Cursor helpers `cursor_col_resize()` / `cursor_row_resize()`. `Pixels` arithmetic: `f32::from(a - b)`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 126 backend, 1 app_state, 22 theme, **31 ui (+6)**, 62 terminal. Not visually run — user should `cargo run` and check: `Cmd-D`/`Cmd-Shift-D` split the active pane (new shell, same cwd), drag a divider to resize (double-click = 50/50), click a pane to focus it, `Cmd-W` closes the focused pane (or the tab if it's the last pane), `Cmd-B` / the ☰ button toggle the sidebar and its edge is draggable, the status bar shows the pane count, switching tabs keeps each tab's split layout intact.

### Design notes / limits (T04-002)
- Pane content is terminal-only for now; other `TabKind`s still show the "coming in a later phase" placeholder (editor-as-pane-content is Phase 5). Sidebar content is a placeholder (Explorer = Phase 4). Header/statusbar are minimal shells (full chrome = T04-003).
- Layout persistence: `WorkspaceLayout` serializes to JSON (test proves round-trip); wiring it into a session snapshot is Phase 13. The `panes`/`session_id` ↔ pane-id mapping is view-side and not yet serialized.
- Only a left sidebar (task scope); the reference also has a right sidebar — deferred.
- `TabData.session_id` is still set by `open_workspace` for the initial pane (label/compat); teardown now goes through the layout's leaf list, so multi-pane tabs fully clean up.

### What's Next
- **T04-003** `tasks/phase-03-tabs-workspace/T04-003-app-shell-window-chrome.md` — app shell & window chrome. Dep: T04-002 (done).

---

## Previous Session: 2026-09-01 (T04-001 — tab bar & tab management)

### What Was Done
- **T04-001 ✅ Done.** Wired the T03-005 `TerminalRegistry` into the UI and built the tabbed workspace shell (pure Rust + GPUI, ported from `reference-src/src/modules/tabs/`).
  - **`crates/ui/src/tabs.rs`** (new) — the tab data model + `TabStore` GPUI entity:
    - `TabKind` (Home / Workspace / Editor / Preview / AiDiff / Sftp / GitGraph / GitDiff / CommitDiff) with `indicator()` glyph + `default_title()`. `TabData` = flat optional bag (session_id, cwd, process_title, path, host_id, repo_path, url) documented per-kind — deliberately not a per-kind enum so later phases add fields without churn. `Tab { id, kind, title, custom_title, dirty, peek, data }` with `label()` mirroring the reference `labelFor` (custom title → process title → cwd basename → fallback) and `needs_close_confirm()` (dirty editor).
    - `TabStore { tabs: Vec<Tab>, active_id, next_id }`, `EventEmitter<ActiveTabChanged>`. Ops: `open`/`open_workspace`, `set_active`, `cycle(forward)`, `close` (Home + last-tab guarded, returns removed `Tab` so caller tears down its session, active→left-neighbour like reference `closeTab`), `close_others`, `close_by_kind`, `reorder(dragged,target)` (array-move like reference `reorderTabs`), `tabs_by_kind`, field mutators (`set_title`/`set_custom_title`/`set_dirty`/`set_peek`/`set_path`), `sync_workspace_meta`. Every mutation `cx.notify()`s.
    - 7 `#[gpui::test]`s: add/switch/close, close-active→left-neighbour, last-tab+Home unclosable, title/label resolution, reorder, dirty-editor confirm flag, tabs_by_kind + close_by_kind.
  - **`crates/ui/src/workspace.rs`** (new) — `Workspace` GPUI entity: owns `Arc<TerminalRegistry>`, `Entity<TabStore>`, `HashMap<tab_id, Entity<TerminalView>>` (content views kept alive across switches — session lives in the registry regardless).
    - `open_terminal_tab`: inherits cwd from the active terminal's `.cwd()`, `registry.create(...)`, `open_workspace`, builds a `TerminalView` from the `SessionHandle`.
    - Close: `request_close` → dirty editor shows an inline confirm overlay (`confirm_close: Option<u64>`), else `do_close` → `TabStore::close` + `registry.close(session_id)` (no orphaned shells) + drop cached view + `focus_active`.
    - Tab bar render: horizontal `overflow_x_scroll()` strip, per-tab pill (indicator glyph + truncated `label()` + dirty dot + hover close ✕), active tab uses `theme.accent()`, `+` button. Drag-reorder via `on_drag(DraggedTab{id,label})` + `TabDragPreview` render view + `drag_over::<DraggedTab>` left-border highlight (live feedback) + `on_drop` → `TabStore::reorder`. Middle-click closes; right-click opens a context menu (`context_menu: Option<(u64, Point)>`) with Close / Close Others / Close All Of This Type + full-window click-catcher to dismiss.
    - Content area: active Workspace tab → its cached `TerminalView`; every other kind → `placeholder("… — coming in a later phase")`.
    - Keyboard (`on_key_down` on the workspace root, bubbled up from the focused terminal which returns `None` for Cmd combos): Cmd-T new, Cmd-W close (with confirm), Cmd-Shift-] / Cmd-Shift-[ next/prev; `cx.stop_propagation()` on each. Full configurability is Phase 12.
    - `_meta_sync` background task (400 ms) reads each terminal's `cwd()`/`shell_title()` into its tab via `sync_workspace_meta` so tab labels track the shell.
  - **`crates/ui/src/terminal.rs`** — `TerminalView` no longer spawns its own `TerminalSession`; `new()` now takes a `SessionHandle` (registry-backed). All session calls go through `handle` (`handle.with(|s| …)` for scroll/selection/mode/render/metadata, `handle.write`, `handle.resize`, `handle.set_colors`, `handle.drain_events`). Dropped the `Result<TerminalSession,String>` + spawn-failure render branch (registry owns spawn now). Added `handle()`, `focus(&mut Window)`, and a bottom "Shell exited (code) — press ⌘W to close" overlay when `handle.status()` is `Exited`.
  - **`crates/app/src/main.rs`** — `Root` now creates `Arc<TerminalRegistry>` + `Entity<Workspace>` instead of a bare `TerminalView`; renders the workspace full-window under the app-level background layer.
  - **`crates/ui/Cargo.toml`** — added `tracing` (workspace) dep. **`crates/ui/src/lib.rs`** — `pub mod tabs; pub mod workspace;` + re-exports `Tab`, `TabData`, `TabKind`, `TabStore`, `Workspace`.
- **GPUI API used (gpui 0.2.2, verified in source):** `InteractiveElement::on_drag<T,W: Render>(value, Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W>)` — build the preview with `cx.new(...)` on the `&mut App`; `InteractiveElement::drag_over<S: 'static>(Fn(StyleRefinement, &S, &mut Window, &mut App) -> StyleRefinement)`; `InteractiveElement::on_drop<T: 'static>(Fn(&T, &mut Window, &mut App))` — `cx.listener(...)` matches this signature directly (it returns `impl Fn(&E, &mut Window, &mut App)`); `ElementId: From<(&'static str, u64)>` (tuple ids with a `u64` work — no `usize` cast needed); `Styled`/FluentBuilder `when_some(Option<T>, FnOnce(Self,T)->Self)` needs `use gpui::prelude::FluentBuilder`; `ClickEvent` has **no** `stop_propagation` — use `cx.stop_propagation()`; `cx.new()` in a non-`Context` spot (`&mut App`) needs `use gpui::AppContext`.
- **Design note:** split-pane workspace tabs (multiple sessions per tab, `PaneNode` tree) are **T04-002** — a Workspace tab here owns exactly one session. Tab/Session separation from the reference is kept: closing/switching tabs never pauses a session, the registry is the sole session owner.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 126 backend, 1 app_state, 22 theme, **25 ui (+7)**, 62 terminal. Not visually run — user should `cargo run` and check: `+`/Cmd-T opens a new terminal tab (inherits cwd), tab click/Cmd-Shift-brackets switch, ✕/Cmd-W/middle-click close, drag to reorder, right-click menu, background terminals keep running while another tab is visible, closing a terminal tab kills its shell (no zombie).

### Current State
- Branch `master`, ~18 unpushed commits + this one. `crates/ui` owns the tab model + workspace shell; `TerminalView` is now registry-backed.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- One session per Workspace tab (splits = T04-002). No tab rename UI yet (`set_custom_title` exists, no inline `<input>` equivalent) — reference has it; deferred with the rest of the tab polish.
- Tab-bar overflow is a plain `overflow_x_scroll` (no sliding-pill animation, no scroll-active-into-view). Context menu is a hand-rolled panel (no `gpui-component` menu primitive in the workspace yet).
- The "shell exited" overlay is informational only — no in-place restart button (registry `SessionHandle::restart` exists; wire a KeepTerminal screen in a later polish pass).
- Editor/Sftp/Git/etc. tab kinds render a placeholder; their content + interactive creation come with their phases.

### What's Next
- **T04-002** `tasks/phase-03-tabs-workspace/T04-002-split-pane-layout-workspace.md` — split-pane layout & workspace (`PaneNode` tree, multiple sessions per tab). Dep: T04-001 (done).

### Blockers
- None.

---

## Session: 2026-09-01 (T02-006 — terminal/app background images)

### What Was Done
- **T02-006 ✅ Done.** Full parity port of the reference `backgrounds` feature (import/list/delete + opacity/blur/tint/fit/target), pure Rust + GPUI.
  - **`crates/backend/src/modules/backgrounds/mod.rs`** — de-Tauri'd: the 5 fns (`backgrounds_dir` now `pub`, `backgrounds_list`, `background_import`, `background_read_data_url`, `background_delete`) changed `async fn` → `fn` (they were only async for Tauri; no `.await` inside, no callers). Added the preferences layer:
    - `BackgroundSettings` (`#[serde(rename_all="camelCase")]`) = reference `preferencesStore` keys `backgroundImage` (filename, `""`=none), `backgroundOpacity` (u8 0–100, def 30), `backgroundBlur` (u8 px 0–100, def 0), `backgroundTintColor` (`#000000`), `backgroundTintOpacity` (u8, def 0) **plus** two keys the pure-Rust renderer needs that the reference did implicitly: `backgroundFit` (`BackgroundFit::{Cover,Contain,Tile}`, def Cover) + `backgroundTarget` (`BackgroundTarget::{Both,App,Terminal}`, def Both). Enums `#[serde(rename_all="lowercase")]`.
    - `background_settings_load()` / `background_settings_save(&BackgroundSettings)` read/merge the shared `config_dir()/labonair-settings.json` blob (same file `super::settings` uses) key-by-key so `barItemPlacements` etc. survive; atomic tmp+rename write; numbers clamped ≤100 on load. Internal `load_from(&Path)`/`save_to(&Path,..)` for tests.
    - 4 new tests (defaults match reference, missing-file → defaults, round-trip + preserves other keys + enum wire form, out-of-range clamp). Uses `std::env::temp_dir()`, not the real config dir.
  - **`crates/ui/src/background.rs`** (new) — `BackgroundStore` GPUI entity + `GlobalBackground` global (`init(cx)` / `background_store(cx)`):
    - Owns `BackgroundSettings` + one decoded `Arc<gpui::Image>`, rebuilt **only** when `(filename, blur)` changes (cache key) — never per frame (task warning). On decode failure it clears the selection and persists that (reference fallback behavior).
    - `load_processed_from(dir,filename,blur)`: `image::load_from_memory` → downscale to `MAX_DIM=2560` (Triangle) → `image::imageops::blur` (pre-blur, since GPUI has **no** blur filter) → re-encode (PNG if source had alpha, else JPEG q85). Fast path: no downscale + no blur → hand GPUI the untouched file bytes with format guessed from extension. AVIF: undecodable by the `image` crate here (no `avif` feature) → only works via the raw fast path (no blur); documented limitation.
    - Mutators `set_image/opacity/blur/tint_color/tint_opacity/fit/target` (persist + rebuild + notify), `import(PathBuf)`, `delete(&str)`, `available()`, `prompt_and_import(cx)` (native picker via `cx.prompt_for_paths(PathPromptOptions{files:true,..})` + `cx.spawn`).
    - `layer(LayerScope::{App,Terminal}) -> Option<AnyElement>`: absolutely-positioned `inset_0` non-interactive overlay, `img(image).object_fit(fit).size_full()`, wrapper `.opacity(backgroundOpacity/100 * 0.5)` (reference `BG_OPACITY_RENDER_FACTOR`), optional tint `div` on top. `App`/`Both` → App scope shows (window-wide, covers terminal too); `Terminal` → Terminal scope shows only. `Tile` → `ObjectFit::Cover` fallback (GPUI has no tiling).
    - 3 tests (fit mapping incl. Tile→Cover, extension→format, generated 3000px image → downscaled + blurred + re-encoded to JPEG).
  - **`crates/ui/Cargo.toml`** — added `image` (workspace) + `labonair-backend` path dep (no cycle; backend doesn't dep ui). **workspace `Cargo.toml`** — added `image = "0.25"` (`default-features=false`, features jpeg/png/gif/webp/bmp; matches gpui's image 0.25.10 in the lockfile).
  - **`crates/ui/src/terminal.rs`** — `TerminalView` gained a `background: Entity<BackgroundStore>` field + `new()` param, `cx.observe(&background)` → repaint, and `.children(self.background.read(cx).layer(LayerScope::Terminal))` on top of the cell/cursor layers.
  - **`crates/app/src/main.rs`** — `Root` holds `Entity<BackgroundStore>` (`labonair_ui::init_background(cx)`), observes it, passes it to `TerminalView::new`, renders `.relative()` + `.children(background.layer(LayerScope::App))` over the theme bg + terminal.
  - **`crates/ui/src/lib.rs`** — re-exports `BackgroundStore`, `BackgroundFit`, `BackgroundTarget`, `LayerScope`, `init as init_background`, `background_store`, `GlobalBackground`.
- **GPUI API used (gpui 0.2.2, verified in source):** `gpui::img(impl Into<ImageSource>) -> Img`; `ImageSource::From<Arc<gpui::Image>>`; `gpui::Image::from_bytes(ImageFormat, Vec<u8>)` (`ImageFormat::{Png,Jpeg,Webp,Gif,Bmp}` — **no Avif variant**); `StyledImage::object_fit(ObjectFit)` (needs the `StyledImage` trait, from `gpui::prelude`; `ObjectFit` has no `PartialEq`/`Debug` → test with `matches!`); `Styled::{opacity(f32), inset_0(), overflow_hidden(), absolute()}` (opacity nests/multiplies via `Window::with_element_opacity`); `App::prompt_for_paths(PathPromptOptions{files,directories,multiple,prompt}) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>>` (on `App`, reachable through `Context` deref); `.children(Option<AnyElement>)` works (Option: IntoIterator).
- **Design note:** the reference wallpaper is one full-window overlay painted *on top* of everything at low opacity (xterm's `<canvas>` is opaque, can't sit behind it). The GPUI port keeps that exact model — overlay on top, `pointer-events`-free, opacity ×0.5 — rather than truly compositing behind the terminal cells (which are also opaque `bg()` divs). "Behind terminal" in criteria == the reference's dimmed-overlay look.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: **126 backend (+4)**, 1 app_state, 22 theme, **18 ui (+3)**, 62 terminal. Not visually run — user should `cargo run`, import an image via a future settings pane (or temporarily call `background_store(cx).update(..set_image..)`), and confirm opacity/blur/fit/target behave.

### Current State
- Branch `master`, ~16 unpushed commits + this one. `crates/ui` now depends on `crates/backend` (first such edge; slower ui compiles). Background layer live in both Root and TerminalView.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- No settings UI yet — wiring `BackgroundStore` mutators + `prompt_and_import` into the Appearance pane is **T13-002**.
- AVIF images only load when no blur is requested (raw fast path); the `image` crate build here has no `avif` feature and GPUI's `ImageFormat` has no AVIF variant.
- `Tile` fit renders as `Cover` (GPUI `img()` has no tiling). The reference had no tile mode either.
- Re-encode (downscale/blur path) runs synchronously inside the GPUI update that changes the setting — fine for a one-shot user action; move to `spawn_blocking` if it ever stutters.
- Blur uses `image::imageops::blur` (true Gaussian, O(n·r)) applied once at load — not GPUI (no blur filter exists in 0.2.2).

### What's Next
- **T04-001** `tasks/phase-03-tabs-workspace/T04-001-*` — Tab-Leiste & Tab-Verwaltung (wires the T03-005 `TerminalRegistry` into the UI).

### Blockers
- None.

---

## Session: 2026-09-01 (T03-005 — local PTY sessions & multi-tab terminal)

### What Was Done
- **T03-005 ✅ Done.** Multi-session registry + lifecycle for local PTY terminals, ported from `reference-src/src-tauri/src/modules/pty/{mod.rs,session.rs}` (the `PtyState` HashMap + `pty_open`/`pty_close`/`pty_has_foreground_job`).
  - **`crates/terminal/src/registry.rs`** (new) — `TerminalRegistry`: `RwLock<HashMap<SessionId, Arc<Slot>>>` + `AtomicU64` id counter (starts at 1, never reused). `SessionId = u64`. API: `create(colors, dims, options) -> SessionId`, `handle(id) -> Option<SessionHandle>`, `ids()`, `len()`/`is_empty()`, `close(id)` (SIGHUP then drop → SIGKILL+join in `Drop`, returns promptly even with a wedged foreground job), `close_all()`.
  - **`SessionHandle`** — cheap `Clone` (`Arc<Slot>` + id). `write`/`resize`/`set_colors`/`with(|&TerminalSession|…)`/`drain_events`/`status`/`has_foreground_job`/`restart(dims)`. `drain_events` folds `Exit`/`ChildExit` into `SessionStatus::{Running, Exited(i32)}` as a side effect, so the existing UI poll loop gets shell-exit tracking for free. `restart` respawns in place with the stored `SessionOptions` + latest palette, same `SessionId` (→ same tab); errors if still running.
  - **`Slot`** — `Mutex<TerminalSession>` (Mutex only so `restart` can swap the whole value; hot path is `&self` on the inner session, no contention) + `Mutex<TerminalColors>` + `SessionOptions` + `Mutex<SessionStatus>`.
  - **`crates/terminal/src/session.rs`** — `SessionOptions.startup_command: Option<String>` (written to the PTY as input right after spawn, shell stays interactive — PTY buffers it until the shell reads, so ordering vs. shell-init is safe). New `TerminalSession::has_foreground_job()` (`#[cfg(unix)]`: `master.process_group_leader()` != `shell_pid`; `false` elsewhere) and `terminate()` (SIGHUP to `-pid` process group + `pid`; hard kill + join stays in `Drop`, so `drop` right after `terminate` can't hang).
  - **`crates/terminal/src/lib.rs`** — `pub mod registry` + re-export `SessionHandle, SessionId, SessionStatus, TerminalRegistry`.
  - **Deps:** added `libc = "0.2"` (workspace) + `[target.'cfg(unix)'.dependencies] libc` on `labonair-terminal` for SIGHUP. `portable_pty` already pulls libc transitively but doesn't re-export it.
  - **Tab-system seam (Phase 3)** documented in the `registry.rs` module doc: tab system holds `Arc<TerminalRegistry>` + a `SessionId` per local-terminal tab; calls `create` on open (inheriting cwd from the previous tab's `SessionMetadata::cwd`), `handle` for the visible tab's I/O, `status`/`has_foreground_job` for the "shell exited — click to restart" screen + close-confirm prompt, `restart` on click, `close` on tab close. Registry never pauses a session — visibility is purely UI.
  - 5 new registry tests: multiple independent sessions (output isolation), startup command runs, clean close with a live `sleep 300` foreground job (< 3s), background session keeps progressing while only another tab is polled, restart-in-place after `exit 7` (+ restart-on-running errors).
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, 22 theme, 15 ui, **62 terminal (+5)**. Not yet wired into the UI (`crates/ui/terminal.rs` still spawns a single `TerminalSession` directly — that migration is Phase 3 / T04-001). Not visually run.

### Current State
- Branch `master`, ~15 unpushed commits + this one. `crates/terminal` now owns the multi-session registry; UI still single-session until Phase 3.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### What's Next
- **T04-001** `tasks/phase-03-tabs-workspace/T04-001-*` — Tab-Leiste & Tab-Verwaltung. This is where the registry gets wired into the UI (replace the direct `TerminalSession` in `crates/ui/src/terminal.rs`).
- **T02-006** (terminal background images) also still unblocked.

### Blockers
- None.

---

## Session: 2026-09-01 (T03-004 — shell integration & CWD tracking)

### What Was Done
- **T03-004 ✅ Done.** OSC 7 + OSC 133 shell integration and session metadata tracking, ported from `reference-src` (`src-tauri/src/modules/pty/{shell_init.rs,scripts/*}` + `src/modules/terminal/lib/osc-handlers.ts`).
  - **`crates/terminal/src/scripts/`** (new) — verbatim copy of the reference shell-integration rc-files (`zshenv.zsh`, `zprofile.zsh`, `zlogin.zsh`, `zshrc.zsh`, `bashrc.bash`). They emit OSC 7 on every prompt, OSC 133 A/B/C/D around prompt/command/output, and reset OSC 0 at each prompt. Block-mode (`LABONAIR_BLOCKS`) branches kept intact.
  - **`crates/terminal/src/shell_integration.rs`** (new) — `Shell{Zsh,Bash,Other}` + `Shell::from_path`; `configure(&mut CommandBuilder, shell, blocks) -> Shell` sets `TERM/COLORTERM/TERM_PROGRAM=Labonair/LABONAIR_TERMINAL=1` (+ `LABONAIR_BLOCKS`), writes the rc-files atomically (tmp+rename) under `~/.cache/labonair/shell-integration/{zsh,bash}/`, sets `ZDOTDIR`/`LABONAIR_USER_ZDOTDIR` (+ `-l`) for zsh or `--rcfile <path> -i` for bash. Non-fatal on write failure (spawns without integration). No `dirs` dep — uses `std::env::var_os("HOME")`. 3 unit tests.
  - **`crates/terminal/src/engine.rs`** — `OscSniffer` (raw-stream tap, runs before the VTE parser) extended from OSC-7-only to also parse **OSC 133 A/B/C/D** and **OSC 0/1/2** titles. Internal `OscUpdate` enum; `feed()` folds updates into a new `SessionMetadata` and emits `TerminalEvent::{PromptStart,PromptEnd,CommandStart(Option<String>),CommandFinished(Option<i32>)}` (new variants) alongside the existing `Cwd`. OSC 7 now **percent-decoded** (`parse_osc7` mirrors the reference `^file://[^/]*(/.*)$` regex + `percent_decode`/`hex_val` helpers) and **gated**: an OSC 7 emitted while `in_command` (between 133;C and the next A/D) is ignored (untrusted-command-output parity with `registerCwdHandler`). `SessionMetadata{cwd,title,in_command,prompt_phase,last_exit_code,last_command}` + `PromptPhase{Unknown,PromptStart,Prompt,Executing}`. New `TerminalEmulator::{set_initial_cwd, metadata}`. OSC 0/2 title still also flows through alacritty's own `Title` event (sniffer only updates metadata, doesn't double-emit). 8 new engine tests (percent-decode, 133 lifecycle, bare C/D, in-command OSC7 gate, title set/reset, sequences-not-in-grid, initial cwd).
  - **`crates/terminal/src/session.rs`** — `SessionOptions.blocks: bool`; `spawn()` now calls `shell_integration::configure` instead of setting env inline, and seeds the emulator cwd from `options.working_directory` or `std::env::current_dir()`. New `TerminalSession::{metadata() -> SessionMetadata, cwd() -> Option<String>, ai_context(max_lines) -> TerminalContext}`; `TerminalContext{cwd,title,lines}` is the base data-holding for the Phase-10 AI live-context reader. 1 new session test (`/bin/bash` real spawn → `cd /tmp` → asserts OSC 7 tracked cwd + `ai_context`).
  - **`crates/terminal/src/lib.rs`** — `pub mod shell_integration`; re-exports `PromptPhase`, `SessionMetadata`, `TerminalContext`, `Shell`.
  - **`crates/ui/src/terminal.rs`** — `TerminalView::{cwd(), shell_title()}` accessors so the (not-yet-built) status bar / breadcrumb and new-tab logic can read them.
- **Design notes:** OSC 133/7 never reach the visible grid — alacritty's VTE parser consumes every OSC sequence (dispatches to a known set, silently drops the rest); the sniffer is a pure read-only tap on the raw bytes. Kept the sniffer approach from T03-001 rather than a `Term` handler because alacritty 0.24 has no OSC-7/133 hook.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, 22 theme, 15 ui, **57 terminal (+11: +8 engine, +3 shell_integration; the +1 session test replaced none)**. Not yet visually run — user should `cargo run` and check the status data once a status bar exists; for now confirm a zsh/bash shell still starts cleanly with the integration rc-files (no stray `]` / `133` leaking into the prompt, p10k/starship still fine).

### Current State
- Branch `master`, 14 unpushed commits + this one. `crates/terminal` owns shell-integration bootstrap + OSC parsing + `SessionMetadata`; `crates/ui` exposes `cwd()`/`shell_title()`.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- Block-terminal rendering (the reserved-row prompt + floating block header) is not built — only the `LABONAIR_BLOCKS` env plumbing + `CommandStart` command-text capture. Block UI is later-phase.
- Title still double-sourced (alacritty `Title` event + `SessionMetadata.title`); UI currently uses neither. Consolidate when the tab bar lands (Phase 03).
- `ai_context` returns the visible grid tail only (no scrollback, no prompt/command/output segmentation yet) — enough for Phase 10 to build on.
- OSC 633 (VS Code shell integration) is not parsed — the reference scripts only emit 7 + 133, so there is no path that needs it.

### What's Next
- **T03-005** `tasks/phase-02-terminal/T03-005-local-pty-sessions.md` — local PTY sessions & multi-tab terminal. Deps: T03-001–004 (all done).
- **T02-006** (terminal background images) also still unblocked.

### Blockers
- None.

---

## Session: 2026-09-01 (T03-003 — keyboard & mouse mapping)

### What Was Done
- **T03-003 ✅ Done.** Full GPUI-event → terminal-byte translation.
  - **`crates/terminal/src/input.rs`** (new, no gpui, no alacritty types leaking) — modular, framework-agnostic mappers driven by a `ModeState` snapshot:
    - `KeyInput { key: Key (Char|Named), mods: Modifiers{shift,alt,ctrl,logo}, text: Option<String> }`, `NamedKey` enum (Enter/Tab/Backspace/Escape/Space/arrows/Home/End/PageUp/PageDown/Insert/Delete/Function(u8 1–20)).
    - `key_to_bytes(&KeyInput, &ModeState) -> Option<Vec<u8>>`. Helpers: `char_key_bytes` (Ctrl+letter→C0 0x01–0x1a, Ctrl+symbol `[ \ ] ^ _ @ ?` + digit aliases, alt→ESC prefix, UTF-8 passthrough), `named_key_bytes`, `cursor_key` (DECCKM `ESC O`/`ESC [`, modified → `CSI 1;<mod><final>` with `mod = 1+shift+2*alt+4*ctrl`), `tilde_key` (`CSI n~` / `CSI n;mod~`), `function_key` (F1–F4 SS3 `ESC O P..S` unmodified else CSI; F5–F20 xterm code table 15/17/18/19/20/21/23/24/25/26/28/29/31/32/33/34). Shift+Enter→`ESC CR` (Claude-Code parity, from reference key handler), Shift+Tab→`CSI Z`, Ctrl+Backspace→0x08. Cmd/logo combos return `None` (app shortcuts).
    - `paste_payload(text, bracketed)` — `\r\n`/`\n`→`\r`, bracketed mode wraps in `ESC[200~`/`ESC[201~` and strips any embedded `ESC[201~`.
    - Mouse: `MouseInput{button,kind,col,row,mods}`, `mouse_report(&MouseInput,&ModeState)` → SGR (`CSI < b ; x ; y M|m`) when `sgr_mouse` else legacy `CSI M` with +32 bias; button codes L/M/R=0/1/2, wheel=64/65, +32 motion, +4/+8/+16 shift/alt/ctrl; returns `None` unless the matching mode (click/drag 1002/motion 1003) is active.
    - Wheel: `wheel_action(&WheelInput,&ModeState) -> WheelAction::{Bytes,Scrollback}` — mouse-mode→wheel button reports (one per line step), alt-screen+`alternate_scroll`→arrow keys, else scrollback.
    - 18 unit tests covering every category (basic/ctrl/alt/modified-cursor/function-both-forms/tilde/specials/bracketed-paste/SGR+legacy mouse/drag-mode-gating/wheel×3).
  - **`crates/terminal/src/engine.rs`** — new `ModeState` struct (app_cursor/app_keypad/bracketed_paste/insert/alt_screen/alternate_scroll/mouse_report_click/mouse_drag/mouse_motion/sgr_mouse/utf8_mouse/kitty_keyboard/report_all_keys_as_esc) + `.mouse_reporting()`. `TerminalEmulator::mode_state()` reads `term.mode()` (`TermMode` bitflags). `update_selection_viewport((col,row),(col,row))` — folds in `grid().display_offset()` so a drag stays buffer-anchored, picks anchor/head `Side` from column direction; `selection_text()` → `term.selection_to_string()` (empty→None). +1 engine test would be nice but covered via session.
  - **`crates/terminal/src/session.rs`** — passthroughs: `mode_state()`, `update_selection()`, `clear_selection()`, `selection_text()`.
  - **`crates/terminal/src/lib.rs`** — `pub mod input` + re-exports (`key_to_bytes`, `mouse_report`, `paste_payload`, `wheel_action`, `Key`, `KeyInput`, `Modifiers`, `MouseButton`, `MouseEventKind`, `MouseInput`, `NamedKey`, `WheelAction`, `WheelInput`, `ModeState`).
  - **`crates/ui/src/terminal.rs`** — replaced the minimal `keystroke_to_bytes` with `keystroke_to_input(&Keystroke) -> Option<KeyInput>` (GPUI key-name → `NamedKey`/`Char`, `f1..f20` parse, `key_char` preferred) + `to_term_mods`. `TerminalView` gained `cell_size: (f32,f32)` (stored each render) and `drag_anchor: Option<(usize,usize)>`. New handlers: `on_mouse_down(Left)` → `mouse_report` if mouse mode else start selection (clear old); `on_mouse_move` (pressed Left) → `session.update_selection(anchor, cell_at(pos))`; `on_mouse_up(Left)` → release report or copy-on-select (`copy_selection` → clipboard); `on_mouse_down(Right)` → `paste_from_clipboard` (right-click-pastes parity); `on_scroll_wheel` → `wheel_action` (Bytes vs Scrollback); `on_key_down` → Cmd+C/Cmd+V intercepted for clipboard, else `key_to_bytes(&input, &this.mode())`, write, snap-to-bottom, clear selection. `cell_at(Point<Pixels>)` maps window px → clamped grid cell (assumes terminal origin (0,0) — full-window; revisit when the app shell adds bounds). `mode()` helper reads `session.mode_state()` with default fallback. 3 old tests replaced with 5 (`keystroke_to_input`+`key_to_bytes` round-trips, app-cursor mode, platform-shortcut→None, f5).
- **Kitty keyboard protocol:** deliberately **not** emitted — `ModeState::kitty_keyboard` is observable but our DA responses never advertise support, so shells stay on the legacy sequences (per task warning; revisit if a real Kitty need appears).
- **GPUI API used (gpui 0.2.2):** `InteractiveElement::{on_mouse_move, on_mouse_up}`; `MouseMoveEvent{position, pressed_button: Option<MouseButton>, modifiers}`, `MouseUpEvent{button, position, modifiers}`; `App::{write_to_clipboard(ClipboardItem), read_from_clipboard() -> Option<ClipboardItem>}`, `ClipboardItem::new_string(String)` / `.text() -> Option<String>`; `Keystroke{key: String, modifiers: Modifiers, key_char: Option<String>}`; `gpui::Modifiers{shift,alt,control,platform,function}` (`platform` = Cmd).
- **alacritty_terminal 0.24.2:** `Term::mode() -> &TermMode` (bitflags u32, `APP_CURSOR=1<<1`, `BRACKETED_PASTE=1<<4`, `SGR_MOUSE=1<<5`, `MOUSE_REPORT_CLICK=1<<3`, `MOUSE_DRAG=1<<13`, `MOUSE_MOTION=1<<6`, `ALTERNATE_SCROLL=1<<15`, `KITTY_KEYBOARD_PROTOCOL` = OR of the 5 report flags); `Term::selection_to_string() -> Option<String>`; `Grid::display_offset() -> usize`; `Selection::new(SelectionType::Simple, Point, Side)` + `.update(Point, Side)`, assign to `term.selection`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, 22 theme, **15 ui**, **46 terminal (+18)**. Not yet visually run — user should `cargo run` and check: arrows/ctrl-combos/function keys in vim + bash, Cmd+C/Cmd+V, drag-select + copy-on-select, wheel scrollback vs. wheel in `less`/`htop`.

### Current State
- Branch `master`, 13 unpushed commits + this one. `crates/terminal::input` owns the pure mapping; `crates/ui` owns the GPUI event conversion.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- `cell_at` assumes the terminal element sits at window origin (0,0). Correct today (full-window); once tabs/splits land (Phase 03) it must use measured element bounds.
- Selection is `SelectionType::Simple` only — no double-click word / triple-click line select yet (T03-005 or polish).
- Kitty keyboard + OSC 52 clipboard: not implemented (Kitty intentionally; OSC 52 deferred — no shell path needs it yet).
- Numeric keypad application mode (DECKPAM) is tracked in `ModeState.app_keypad` but GPUI doesn't distinguish keypad keys, so no keypad-specific sequences are emitted.

### What's Next
- **T03-004** `tasks/phase-02-terminal/T03-004-shell-integration-cwd.md` — OSC 133 shell integration + CWD tracking. Deps: T03-002 (done).
- **T02-006** (terminal background images) also unblocked.

### Blockers
- None.

---

## Session: 2026-09-01 (T03-002 — GPUI terminal cell renderer)

### What Was Done
- **T03-002 ✅ Done.** The terminal is now a visible, interactive GPUI surface.
  - **`crates/terminal/src/render.rs`** (new, no GPUI) — pure prep helpers:
    - `batch_runs(&RenderableScreen) -> Vec<StyledRun>`: collapses the per-cell grid into per-row runs of identical `RunStyle` (fg/bg/bold/italic/underline/strikeout/hidden) on contiguous columns; a column gap always breaks a run so each run is positioned by `start_col` alone. Trailing blank cells share the default style and stay in one run (correct terminal behavior).
    - `grid_size(w_px, h_px, cell_w, cell_h) -> (cols, rows)`: floor division, min 1×1.
    - `RunStyle`, `StyledRun { line, start_col, text, style }` (+ `width()`).
    - 4 unit tests (run merge, style-change split w/ palette colors, bold breaks run, grid math).
  - **`crates/terminal/src/engine.rs`** — `RenderableScreen` gained `selection: Vec<SelectionSpan>` (per-visible-row spans, end-exclusive), populated in `render()` from `term.renderable_content().selection` (alacritty already resolves the `SelectionRange`; handles block vs linewise, clamps to `columns`). New `TerminalEmulator::set_selection((i32,usize),(i32,usize))` / `clear_selection()` (uses `alacritty_terminal::selection::{Selection,SelectionType}` + `index::Side`) — the mouse task (T03-003) will drive these; exposed now so the renderer can already draw selections. +1 engine test.
  - **`crates/terminal/src/lib.rs`** — re-exports `render::*`, `SelectionSpan`, and `alacritty_terminal::vte::ansi::{CursorShape, Rgb}`.
  - **`crates/ui/src/terminal.rs`** (new) — `TerminalView`, a GPUI entity (`Render` + `Focusable`):
    - `new(theme, window, cx)` spawns a local `TerminalSession` (80×24), focuses itself, `cx.observe`s the `ThemeStore` to re-color the running shell + repaint, and starts a poll `Task` (`cx.spawn` async closure + `cx.background_executor().timer(16ms)`) that drains `session.drain_events()` and only `cx.notify()`s when there were events; the loop stops on `Exit`/`ChildExit` or when the entity is dropped. Spawn failure is stored as `Err(String)` and rendered as a message instead of crashing.
    - `render()` derives cell metrics from the theme: `cell_w = text_system().ch_advance(resolve_font(terminal_font), font_size)`, `cell_h = ceil(font_size * terminal_line_height)`. Fits the grid to `window.viewport_size()` via `grid_size` and calls `session.resize(TermDimensions{..})` only when `(cols,rows)` changed. Paints a `relative` `size_full` container (bg = theme `terminal.background`) with: one absolutely-positioned `div` per `StyledRun` (`.left/.top` from `start_col*cell_w` / `line*cell_h`, `.w(run_width*cell_w)`, `.h(cell_h)`, `.bg(run.style.bg)`, `.text_color`, `.text_size(font_size)`, `.line_height(cell_h)`, `.whitespace_nowrap()`, bold→cloned `Font` with `weight = BOLD`, italic/underline/line_through toggles, hidden→fg=bg); selection spans as translucent overlays (`terminal.selection` @ `selection_alpha`); and a cursor overlay honoring `CursorShape` (Block/HollowBlock = 55%-alpha fill so the glyph shows through, Beam = 2px, Underline = 2px at cell bottom, Hidden/scrolled-out = none).
    - Input: `.on_scroll_wheel` → `Scroll::Delta(lines)` (`ScrollDelta::Lines` used directly, `Pixels` divided by `cell_h`); `.on_key_down` → `keystroke_to_bytes()` then `session.write()` + `Scroll::Bottom` snap; `.on_mouse_down(Left)` refocuses. `keystroke_to_bytes` is the minimal map (ctrl+letter→control byte, named keys enter/tab/backspace/arrows/home/end/delete→sequences, `key_char` passthrough, alt→ESC prefix). Full mapping is T03-003.
    - 4 plain `#[test]`s (ctrl-C, named keys, printable+alt, `to_hsla` black/white).
  - **`crates/ui/Cargo.toml`** — added `labonair-terminal` path dep (no cycle: terminal only deps theme). `crates/ui/src/lib.rs` re-exports `TerminalView`.
  - **`crates/app/src/main.rs`** — `Root` now holds `Entity<TerminalView>` (created with `window` in `Root::new`) and renders it full-window over the theme background; dropped the T02 swatch/sample demo. `Root::new` signature gained `&mut Window`.
- **GPUI API used (gpui 0.2.2, verified in source):** `App::text_system() -> &Arc<TextSystem>`; `TextSystem::{resolve_font(&Font)->FontId, ch_advance(FontId,Pixels)->Result<Pixels>}`; `f32::from(Pixels)` (the `.0` field is private in 0.2.2 — use the `From` impls); `Window::{viewport_size()->Size<Pixels>, focus(&FocusHandle)}`; `App::focus_handle()`; `Context::spawn(async move |WeakEntity<T>, &mut AsyncApp| ...)` (stable async closures) + `AsyncApp::background_executor()` + `BackgroundExecutor::timer(Duration)`; `WeakEntity::update(cx, |&mut T, &mut Context<T>|) -> Result<R>`; `div().id(_)` → `Stateful<Div>`, so a `Render` fn mixing an early `Div` return with a final `Stateful<Div>` must unify via `.into_any_element()` (keep the signature `-> impl IntoElement`, NOT `-> AnyElement`, or clippy's `refining_impl_trait` fires); `InteractiveElement::{track_focus, key_context, on_key_down, on_scroll_wheel, on_mouse_down}`; `Styled::{absolute, relative, whitespace_nowrap, italic, underline, line_through, line_height, text_size, font}`; `ScrollDelta::{Lines(Point<f32>), Pixels(Point<Pixels>)}`; `gpui::Rgba{r,g,b,a: f32}.into() -> Hsla`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Test counts: 122 backend, 1 app_state, 22 theme, **13 ui (+4)**, **28 terminal (+5)**. Not yet visually run — user should `cargo run` and confirm a live shell renders, is typable, scrolls, and resizes with the window.

### Current State
- Branch `master`, 12 unpushed commits + this one. `crates/ui` owns the GPUI terminal element; `crates/terminal` owns batching + selection snapshot. App root shows one interactive terminal.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (for next tasks, not blockers)
- Resize is driven by `window.viewport_size()` (terminal assumed full-window). Once the app shell / tabs exist (Phase 03) the view must measure its own element bounds instead.
- The renderer repaints the whole grid on any wakeup (no per-region damage tracking). Fine at 80×24–ish; revisit under T15-003 if scrolling large output stutters.
- Selection can be *drawn* but nothing *creates* one yet — `set_selection` is wired for T03-003 (mouse drag-select).
- Wide/CJK cells: the engine already drops `WIDE_CHAR_SPACER` cells; the wide glyph occupies one run cell but is positioned for a single column (double-width advance handled by GPUI text shaping, not by the cell grid). Verify under T03-003/T15-001.

### What's Next
- **T03-003** `tasks/phase-02-terminal/T03-003-keyboard-mouse-mapping.md` — full keyboard + mouse mapping (modifiers, mouse reporting, drag selection, copy/paste, bracketed paste). Deps: T03-002 (done).
- **T02-006** (terminal background images) is now unblocked (needed the renderer element).

### Blockers
- None.

---

## Session: 2026-08-31 (T03-001 — alacritty_terminal engine integration)

### Task selection note
- **T02-006** (terminal background images) was the next roadmap task, but its `## Abhängigkeiten` list `T02-002 (Theme-Store), Phase 2 (Terminal-Renderer — für die tatsächliche Darstellung)` and two of its acceptance criteria require a live terminal element to render into. Phase 02 did not exist. Per the session directive, **T02-006 was left untouched** (still `⏳ Pending`) and **T03-001** was done instead. Return to T02-006 after the Phase 02 renderer (T03-002) lands.

### What Was Done
- **T03-001 ✅ Done.** `crates/terminal` now embeds `alacritty_terminal` 0.24.2 as the emulation core — render-free logic only, no GPUI.
  - **`crates/terminal/src/engine.rs`** (new) — `TerminalEmulator`: wraps `alacritty_terminal::Term<EventProxy>` + `vte::ansi::Processor` (fed byte-by-byte; vte 0.13.1 `advance(&mut handler, u8)`). `feed(&[u8]) -> Vec<TerminalEvent>` runs the parser and returns a trailing `Wakeup` (Term never emits Wakeup itself — that was Alacritty's event-loop's job) plus any `Cwd` recovered by an OSC-7 sniffer that scans the raw stream *before* the parser (`alacritty_terminal` 0.24 ignores OSC 7; OSC 133 is deferred to T03-004). `render() -> RenderableScreen` walks `term.renderable_content().display_iter` and resolves every `vte::ansi::Color` (Named/Indexed/Spec, incl. DIM group + INVERSE swap) to a concrete `Rgb` via `TerminalColors` (T02-004) — never an Alacritty default. `TermDimensions` implements `alacritty_terminal::grid::Dimensions` (own type, not the crate's `term::test::TermSize`). `resize()`, `scroll(Scroll)`, `is_alt_screen()` (TermMode::ALT_SCREEN), `history_len()`, `set_colors()`, `take_pty_output()` (drains DA/DSR replies the `EventProxy` buffers). Config: `scrolling_history = 10_000`.
  - **`EventProxy`** impl `alacritty_terminal::event::EventListener` — maps `Event::{Wakeup,Title,ResetTitle,Bell,Exit,ChildExit,MouseCursorDirty}` onto a `std::sync::mpsc::Sender<TerminalEvent>`; `PtyWrite` bytes are appended to a shared `Arc<Mutex<Vec<u8>>>` (drained by the session's I/O thread, not sent to the UI). Clipboard/Color/CursorBlink/TextAreaSize events dropped. `Clone + Send` so `Term` can move to the reader thread.
  - **`crates/terminal/src/session.rs`** (new) — `TerminalSession::spawn(colors, dims, SessionOptions)`: opens a PTY via `portable-pty` `native_pty_system()`, `CommandBuilder` (default shell = `$SHELL` → `/bin/zsh`; env `TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=Labonair`, `LABONAIR_TERMINAL=1` — mirrors `reference-src/.../pty/shell_init.rs`). A dedicated `labonair-pty-reader` OS thread does blocking `reader.read()`, locks the emulator, `feed()`s, writes back any `take_pty_output()` replies, forwards events over the channel; on EOF sends `TerminalEvent::Exit`. UI-facing API: `write()`, `resize(&mut)` (both PTY + grid), `scroll()`, `set_colors()`, `render()`, `with_emulator()`, `drain_events()`, `recv_event_timeout()`, `shell_pid()`. `Drop` kills the child and joins the thread.
  - **`crates/terminal/examples/headless_dump.rs`** (new) — spawns a real shell, runs a colored `printf`, prints the grid to stdout with no GUI. Verified manually: prompt + `GREETING from alacritty_terminal` render correctly.
  - **`crates/terminal/src/lib.rs`** re-exports `engine::*`, `session::*`, and `alacritty_terminal::grid::Scroll`.
- **No new deps** — `alacritty_terminal`, `portable-pty`, `tokio` were already in `crates/terminal/Cargo.toml`; `labonair-theme` path dep already present (T02-004). `alacritty_terminal`'s own `tty`/`event_loop` modules are **not** used — PTY I/O is portable-pty + our thread (per the architecture doc + task note).
- **Tests:** 23 in `crates/terminal` (was 9). `engine::tests` (11): plain-text, SGR color→theme palette, cursor-move CSI, Wakeup on output, OSC 0 title event, OSC 7 cwd, resize, scrollback accumulate/scroll (`Scroll::Top`/`Bottom`), alt-screen `?1049h/l`, INVERSE fg/bg swap. `session::tests` (4, spawn real `/bin/sh`): runs a command & reads it back, ANSI color from shell output resolves to theme green, `stty size` reflects a resize, `exit` produces an Exit/ChildExit event.
- **GPUI API used:** none (this task is renderer-free).
- **alacritty_terminal 0.24.2 API (verified in `~/.cargo/registry/src/.../alacritty_terminal-0.24.2/src`):** `Term::new<D: Dimensions>(Config, &D, T)`, `Term::resize<S: Dimensions>(S)`, `term.renderable_content() -> RenderableContent { display_iter: GridIterator<Indexed<&Cell>>, display_offset, cursor: RenderableCursor { point: Point<Line,Column>, shape }, colors, mode }`, `term.grid().history_size()`, `term.scroll_display(Scroll)`, `term.mode().contains(TermMode::ALT_SCREEN)`. `Cell { c, fg, bg, flags: Flags }`; `Flags::{BOLD,ITALIC,DIM,INVERSE,STRIKEOUT,HIDDEN,ALL_UNDERLINES,WIDE_CHAR_SPACER,LEADING_WIDE_CHAR_SPACER}`. `event::{Event, EventListener, WindowSize}`. VTE parser = `alacritty_terminal::vte::ansi::Processor` (vte **0.13.1**, `advance(&mut H: Handler, byte: u8)` — single byte, not a slice). `NamedColor` discriminants: system 0–15 contiguous, `Foreground=256`, then Background/Cursor/Dim*/Bright*/Dim* — `named as usize` arithmetic is safe for the Bright/Dim ranges.

### Current State
- Branch `master`, 11 unpushed commits + this one. `crates/terminal` owns the emulation core + PTY sessions; nothing renders it yet.
- Pre-existing uncommitted `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from the commit, flag to user.
- `reference-src/` untouched.

### What's Next
- **T03-002** `tasks/phase-02-terminal/T03-002-gpui-terminal-renderer.md` — GPUI cell renderer consuming `engine::RenderableScreen`. Deps: T03-001 (done) + T02-004 (done).
- **T02-006** (terminal background images) is still pending and now only blocked on the renderer element from T03-002.

### Blockers
- None.

---

## Session: 2026-08-31 (T02-005 — font handling & bundling)

### What Was Done
- **T02-005 ✅ Done.** Bundled the reference app's font families natively and wired them into GPUI's text system.
  - **`crates/theme/assets/fonts/`** (new) — SIL OFL 1.1 font files committed into the repo: `InterVariable.ttf` + `InterVariable-Italic.ttf` (rsms Inter 4.1), `JetBrainsMono-{Regular,Medium,Bold,Italic,BoldItalic}.ttf` (JetBrainsMono 2.304). Family `name`-table values verified with fonttools: `"Inter Variable"` and `"JetBrains Mono"` (Medium carries typographic-family id16 `"JetBrains Mono"` so weight selection works). `LICENSE` + `Inter-OFL.txt` + `JetBrainsMono-OFL.txt` document redistribution rights. SF Mono / Menlo are runtime fallbacks only, never bundled.
  - **`crates/theme/src/fonts.rs`** (new module, `pub mod fonts`) — `embedded_fonts() -> Vec<Cow<'static,[u8]>>` via `include_bytes!` (7 files), plus `UI_FONT_FAMILY`/`MONO_FONT_FAMILY` consts and `UI_FONT_FALLBACKS` (`[".SystemUIFont","sans-serif"]`) / `MONO_FONT_FALLBACKS` (`["SFMono-Regular","Menlo","monospace"]` — mirrors the reference CSS stack). lib.rs re-exports all. 2 tests (all 7 assets are valid TrueType sfnt & >10KB; family-name stability).
  - **`crates/theme/src/tokens.rs`** — `Typography` gained font fields matching `preferencesStore` defaults: `ui_font_fallback`, `buffer_font_family` + `buffer_font_size` (13), `terminal_font_family`, `terminal_font_size` (14), `terminal_line_height` (1.05), `terminal_letter_spacing` (0), `terminal_font_weight` (new `MonoFontWeight` enum Normal/Medium/Bold), `mono_font_fallback`, `font_ligatures` (true — reference always loads xterm `LigaturesAddon`). `Typography::default()` fills them from `crate::fonts` consts; `Theme::light()/dark()` unchanged (both use the default). Existing `app_*` fields untouched.
  - **`crates/ui/src/theme.rs`** — `ThemeStore` accessors: `ui_font()` / `buffer_font()` / `terminal_font()` -> `gpui::Font` (family + `FontFallbacks::from_fonts` + weight; mono honors `font_ligatures` via `FontFeatures::disable_ligatures()`), `terminal_font_size()`, `terminal_line_height()`. New free fn **`init_fonts(cx: &App)`** = `cx.text_system().add_fonts(labonair_theme::embedded_fonts())` (non-fatal on error). lib.rs re-exports `init_fonts`. 1 new `#[gpui::test]`.
  - **`crates/app/src/main.rs`** — `labonair_ui::init_fonts(cx)` first thing in `Application::run`; `Root::render` now sets `.font(ui_font)` on the root and renders a mono/ligature sample line (`-> => != >= …`) in `terminal_font` for visual parity checks.
- **GPUI API used (gpui 0.2.2, verified in source):** `TextSystem::add_fonts(Vec<Cow<'static,[u8]>>)` (mac impl uses `CGFont::from_data_provider` → needs ttf/otf, **not** woff2/ttc); `App::text_system() -> &Arc<TextSystem>`; `gpui::font(family)` builder + `Font { features, fallbacks, weight, style, .. }`; `FontFallbacks::from_fonts(Vec<String>)` / `.fallback_list()`; `FontFeatures::disable_ligatures()` (sets `calt=0`) / `.is_calt_enabled()`; `FontWeight::{NORMAL,MEDIUM,BOLD}`; `Styled::font(Font)`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, **22 theme (+2)**, **9 ui (+1)**, 9 terminal. Not yet visually run — user should `cargo run` to confirm UI renders in Inter and the sample line shows JetBrains Mono ligatures.

### Current State
- Branch `master`, 10 unpushed commits + this one. `crates/theme` owns the font assets + `Typography` font fields; `crates/ui` exposes `gpui::Font` accessors + `init_fonts`.
- Pre-existing uncommitted `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from the commit, flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-006** `tasks/phase-01-theme/T02-006-terminal-background-images.md` — terminal background images. Deps: T02-002 (done) + Phase 2 (terminal engine, not yet started) — **may be blocked** until Phase 02 exists; check the task file's dependencies before starting.

### Blockers
- T02-006 lists a "Phase 2" dependency — the terminal engine (T03-*) does not exist yet. Next session must check whether T02-006 can proceed or whether Phase 02 comes first.

---

## Session: 2026-08-31 (T02-004 — terminal ANSI palette → engine bridge)

### What Was Done
- **T02-004 ✅ Done.** Built the theme → terminal-engine color bridge in **`crates/terminal/src/palette.rs`** (new module; `crates/terminal` gained a `labonair-theme` path dep).
  - **`Rgb`** is re-used straight from `alacritty_terminal::vte::ansi::Rgb` (`{r,g,b: u8}`) so Phase 02 integration is friction-free. `alacritty_terminal` re-exports `vte`, so the path is `alacritty_terminal::vte::ansi::{NamedColor, Rgb}` and `alacritty_terminal::term::color::Colors`.
  - **`TerminalColors`** (`Debug/Clone/Copy/PartialEq`, not `Eq` — carries an `f32`): `background`, `foreground`, `bright_foreground`, `dim_foreground`, `cursor`, `cursor_text` (= bg, xterm `cursorAccent`), `selection` (alpha stripped) + `selection_alpha: f32`, and `normal`/`bright`/`dim: [Rgb; 8]`. `from_theme(&Theme)` / `from_palette(&TerminalPalette)`. Row order = black,red,green,yellow,blue,magenta,cyan,white.
  - **`TerminalColors::ansi256(u8) -> Rgb`** — standard xterm 256 scheme: 0–7 normal, 8–15 bright, 16–231 6×6×6 cube (`cube_axis(n) = 0 if n==0 else 55+n*40`), 232–255 grayscale (`8 + 10*(i-232)`). Cube/ramp are derived, not from globals.css (per task note).
  - **`TerminalColors::to_alacritty_colors() -> Colors`** — fills all 256 ANSI slots + `Foreground`/`Background`/`Cursor` + `BrightForeground` (267) + `DimForeground` (268) + the `DimBlack..DimWhite` group (259–266). This is what a Phase-02 session hands the engine.
  - **`ansi_self_test() -> String`** — ANSI escape dump (16 system + 216 cube + 24 grayscale + SGR attrs incl. `\x1b[2mdim`) for visual parity checks against Labonair (`ls`/Vim/Git).
  - Conversion reuses `labonair_theme::to_rgb8` (same rounding as theme export) → "< 1/255" is exact by construction.
  - `crates/terminal/src/lib.rs` re-exports `TerminalColors`, `ansi_self_test`.
  - **9 tests** in `palette::tests`: three-rows-present, named-range lookup, cube+grayscale xterm scheme (16→black, 231→white, 196→red, 46→green, 232→#080808, 255→#eeeeee), exact conversion for all 8 normal colors, dark values vs globals.css (`--terminal-yellow`==primary #E6B450, `--terminal-red`==destructive #F26D78, bright white/fg ≈ #FFFFFF), light≠dark, `selection_alpha`==0.13, alacritty `Colors` list fully filled, self-test covers all 256 indices.
- **Deferred to Phase 02** (explicitly this task's "Weiterführende Tasks — Phase 2: Terminal-Engine übernimmt die Paletten-Integration in der Praxis"): binding `TerminalColors` into a live PTY session, and the `cx.observe(&theme_store)` re-color-on-theme-switch hook. The bridge + resolution + self-test are complete and green now.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all green. Counts: 122 backend, 1 app_state, 20 theme, 8 ui, **9 terminal (new)**.

### Current State
- Branch `master`, 9 unpushed commits + this one. `crates/terminal` now owns the palette bridge; `crates/theme` untouched.
- Uncommitted pre-existing `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched, flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-005** `tasks/phase-01-theme/T02-005-font-loading.md` — font handling / bundling (GPUI). Dep (T01-001) satisfied.
- Phase 02 (T03-001) will consume `TerminalColors` when it integrates alacritty_terminal.

### Blockers
- None.

---

## Session: 2026-08-31 (T02-003 — theme import/export for user themes)

### What Was Done
- **T02-003 ✅ Done** (functional layer; settings-UI wiring deferred to T13-002 — no shell/settings surface exists in Phase 1).
- **`crates/theme/src/import.rs`** (new) — the JSON ⇆ typed `Theme` conversion layer:
  - `ThemeFile` / `ThemeFileVariant` serde structs, Labonair-compatible (`name`/`author`/`author_url`/`version`/`description` + `variants: BTreeMap<String, {mode,label,colors}>`). `from_json` / `to_json` / `validate` (name non-empty, ≥1 `light` + ≥1 `dark` variant — matches backend `themes::Theme::validate`).
  - `Theme::from_theme_file(&ThemeFile, dark: bool) -> Result<(Theme, Vec<String>), String>` — resolves the mode's variant, starts from the built-in `Theme::light()/dark()` default, overlays every recognised token. Unknown token names + unparseable colors become **warnings**, never errors; omitted tokens keep their default. Color parsing reuses `color::parse_color` (hex/rgb/oklch/transparent).
  - `Theme::to_theme_file(name, author) -> ThemeFile` — serializes all `COLOR_TOKENS` to `#rrggbb(aa)` via new `color::to_hex`, writes the same set into both a `light` and `dark` variant so export always re-imports cleanly.
  - `pub const COLOR_TOKENS: &[&str]` — the 72 canonical dot-notation keys (core, sidebar.*, surface, border.*, status, cursor/selection, terminal.* + terminal.ansi.{,bright_,dim_}*). `set_token`/`get_token` are inverses; `set_token` also accepts a few legacy aliases (`terminal_red`, `card-foreground`, …).
  - lib.rs re-exports `ThemeFile`, `ThemeFileVariant`, `COLOR_TOKENS`, `to_hex`. Module-level doc block documents the full JSON schema.
  - 5 tests: parse+convert matching variant, bad/unknown → warnings not errors, validate rejects missing-mode/name, export round-trips (all 72 tokens within ±1/255), every canonical token is settable.
- **`crates/ui/src/theme.rs`** — `ThemeStore` gained `custom_file: Option<ThemeFile>` alongside `custom: Option<Theme>`:
  - `import_theme_file(ThemeFile, cx) -> Result<Vec<String>, String>` — validates, resolves for the current mode, stores both, `notify()`, returns warnings. Invalid file → `Err`, nothing activated.
  - `clear_custom_theme(cx)`, `active_theme_file(name) -> ThemeFile` (export).
  - `reresolve_custom()` re-derives `custom` from `custom_file` when the resolved mode changes — wired into `set_preference` + `set_system_appearance`. `set_custom_theme` now also clears `custom_file`.
  - lib.rs re-exports `ThemeFile` / `ThemeFileVariant`. 3 new `#[gpui::test]`s (import activates + follows mode switch + clear; invalid rejected without activating; export→import round-trip).
- **`crates/backend/src/modules/themes/mod.rs`** — persistence layer was already ported in T01-002 (file-based `config_dir()/themes/*.json`: `themes_get_all`, `theme_import`, `theme_export`, `theme_delete`, `theme_create`, download/index). Added `fn is_protected(id)` helper (used by `theme_delete`) + 3 tests (bundled default parses+validates, validate needs both variants, `default` id is protected).
  - NB: the T02-003 task text claims Labonair stores themes "in einer Datenbank" — the reference (`reference-src/src-tauri/src/modules/themes/mod.rs`) actually uses JSON files in a themes dir. Followed the reference.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all green. Counts now: 122 backend, 1 app_state, 20 theme, 8 ui.

### Current State
- Branch `master`, 8 unpushed commits + this one. Theme crate now owns import/export conversion; `ThemeStore` can activate imported themes and follows mode switches for them.
- Uncommitted pre-existing `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched, flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-004** `tasks/phase-01-theme/T02-004-terminal-palette.md` — integrate the terminal ANSI palette into the theme. Deps (T02-001) satisfied.
- Later: **T13-002** must wire `import_theme_file` / `active_theme_file` / backend `themes_*` into the Appearance settings pane (the deferred criterion 7 of T02-003).

### Blockers
- None.

---

## Session: 2026-08-31 (T02-002 — runtime theme provider + store)

### What Was Done
- **T02-002 ✅ Done.** Added the runtime theme layer in `crates/ui/src/theme.rs`.
  - `ThemeStore` — GPUI entity, the single source of truth for the active theme. Holds `preference: ThemePreference` (System/Light/Dark), `system_mode: ThemeMode` (resolved from `WindowAppearance` via `ThemeMode::from_appearance`, Vibrant* mapped to Light/Dark), the default `light`/`dark` `Theme`s built once from `labonair-theme` (never recomputed), and `custom: Option<Theme>` for imported themes (T02-003).
  - `theme(&self) -> &Theme` returns the custom theme if set, else the default for the resolved mode — cheap, no alloc. `mode()` resolves preference against `system_mode`.
  - Mutators each guard on equality and only `cx.notify()` on real change: `set_preference`, `set_system_appearance` (only re-renders when preference == System), `set_custom_theme(Option<Theme>)`.
  - Convenience accessors: `background/foreground/card/muted/muted_foreground/border/primary/accent -> Hsla`, `radius() -> RadiusScale`, `shadows() -> &Shadows`, `animation() -> &Animation`.
  - Global: `GlobalTheme(pub Entity<ThemeStore>)` + `init(appearance, &mut App) -> Entity<ThemeStore>` (creates entity + `set_global`), `theme_store(&App)`, `active_theme(&App) -> &Theme`.
  - 5 `#[gpui::test]`s (preference switching, system-appearance follow, custom override+fallback, accessor parity, global access). `crates/ui/Cargo.toml` gained `[dev-dependencies] gpui = { features = ["test-support"] }`.
  - `crates/app/src/main.rs`: window init calls `labonair_ui::init_theme(window.appearance(), cx)`, wires `window.observe_window_appearance(..)` → `store.set_system_appearance(..)`. `Root` view now holds `Entity<ThemeStore>`, `cx.observe`s it for re-render, and renders sample swatches (primary/accent/muted/destructive/border) + a card surface using live theme colors.
- **Verified:** `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (119 backend + 1 app_state + 15 theme + 5 ui) — all green. `cargo build --bin labonair` OK. Not yet visually run — user should `cargo run` to confirm swatches + system dark/light.

### Current State
- Branch `master`, 7 unpushed commits + this one. `crates/ui` now owns the theme provider; `crates/theme` unchanged.
- GPUI API used: `Window::observe_window_appearance(FnMut(&mut Window, &mut App)) -> Subscription` (1 arg, no cx), `App::window_appearance()` / `Window::appearance() -> WindowAppearance` (gpui 0.2.2). `#[gpui::test]` needs `gpui` dev-dep feature `test-support`.
- Uncommitted `CLAUDE.md` edit (pre-existing, not mine) still garbles Next Task Protocol steps 1 & 3 (status vocabulary removed, "to ." dangling). Left untouched — flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-003** `tasks/phase-01-theme/T02-003-theme-import-export.md` — theme import/export for user themes. Deps (T02-001, T02-002) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (T02-001 — extract theme tokens from globals.css)

### What Was Done
- **T02-001 ✅ Done.** Transcribed every design token from `reference-src/src/styles/globals.css` into typed Rust in `crates/theme/`.
  - `crates/theme/src/color.rs`: Oklch→sRGB→`gpui::Hsla` conversion via the `palette` crate (`oklch(l%, c, h)` / `oklch_a(..., alpha)`), `transparent()`, `to_rgb8()`, and a `parse_color()` accepting `oklch(...)`, `#rgb/#rgba/#rrggbb/#rrggbbaa`, `rgb()/rgba()` (space or comma sep, `/ alpha` as number or %), and `transparent` — for later user-theme import (T02-003).
  - `crates/theme/src/tokens.rs`: `Theme` struct with named typed fields for every category — `CoreColors` (incl. `charts: [Hsla;5]`), `SidebarColors`, `SurfaceColors`, `BorderVariants`, `StatusColors`, `InteractionColors`, `TerminalPalette` (`AnsiColors` ×3 for normal/bright/dim + bg/fg/bright_fg/dim_fg + cursor/selection), `RadiusScale` (base 5px from `0.3125rem`@16px, sm..xl4 via the `calc()` multipliers, `window`=12px), `Shadows` (`Vec<ShadowLayer>{x,y,blur,spread,color}` per tier — row/popover/modal), `Animation` (`Duration` 160/240/320ms + two `CubicBezier`), `Typography` (Inter Variable, 13px, 1.5).
  - `Theme::light()` / `Theme::dark()` factory methods with values transcribed 1:1 from the `:root` / `.dark` blocks. `var(--x)` refs resolved to their concrete values; `color-mix()` only appears in the non-token `.themed-scrollbar` rule so nothing to resolve.
  - 15 unit tests: oklch extremes/grayscale/stability, design-hex spot checks (#E6B450, #F26D78, #3C3C3C, #2B2B2B within ±3/255), parse round-trips, radius multipliers, shadow layer counts, animation durations, terminal palette distinctness.
  - Deps: `palette = "0.7"` added to workspace + `crates/theme/Cargo.toml`.
- **Verified:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (119 backend + 1 app_state + 15 theme) — all green.

### Current State
- Branch `master`, 6 unpushed commits + this one. `crates/theme` is the design-token source of truth; not yet consumed by any UI.
- Uncommitted `CLAUDE.md` edit (pre-existing, not mine) garbles the Next Task Protocol step 1/3 wording (status vocabulary removed, "to ." dangling). Left untouched — flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-002** `tasks/phase-01-theme/T02-002-theme-provider-store.md` — runtime theme provider + store (GPUI global). Deps (T02-001) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (T01-005 — CI pipeline)

### What Was Done
- **T01-005 ✅ Done.** Added `.github/workflows/ci.yml`: triggers on push to `master` + all PRs, `concurrency` cancel-in-progress, `RUSTFLAGS: -D warnings`.
  - `macos-latest` job (timeout 60m): `dtolnay/rust-toolchain@stable` (rustfmt+clippy) + `Swatinem/rust-cache@v2` → `cargo fmt --all --check` · `cargo check --workspace --all-targets` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace`.
  - `ubuntu-latest` job: `cargo check` only, `continue-on-error: true` (Linux is "later", must not block).
- `.github/dependabot.yml`: dropped the npm ecosystem + the stale `/src-tauri` cargo dir; now one `cargo` entry at workspace root `/` + `github-actions`.
- `.github/labeler.yml`: rewritten from `src/**` / `src-tauri/**` globs to the `crates/**` structure (backend, ui, theme, terminal, editor, ai, ssh/sftp, hosts, git, snippets, ci, dependencies).
- `.github/release.yml`: changelog categories updated (added Git, renamed Frontend→UI/Theme, `frontend`→`ui` label).
- Old web workflows remain only under `reference-src/.github/workflows/` (untouched, reference only). `CODEOWNERS` unchanged (`* @Snenjih` still correct).
- **Verified locally:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (119 backend + 1 app_state) — all green. No Rust code touched.

### Current State
- Branch `master`, 6 unpushed commits + this one. CI config in place but **not yet run on GitHub** (push only on user request) — acceptance criterion "CI grün auf Test-PR" still open.
- `reference-src/` untouched.

### What's Next
- **Phase 00 complete.** Next is **T02-001** `tasks/phase-01-theme/T02-001-*.md` — extract design tokens from `reference-src/src/styles/globals.css`. Dep (T01-001) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (T01-004 — typed event system + logging)

### What Was Done
- **T01-004 ✅ Done.** Built a typed routing layer on top of the existing string `EventBus` from T01-002 — additive, no churn to the ~40 ported `app.emit("name", json)` call sites.
- `crates/backend/src/events.rs`: renamed the transport struct `AppEvent` → **`RawEvent`** (name + `serde_json::Value`, still what the broadcast channel carries). Added the typed **`AppEvent`** enum (variants: Transfer{Progress,Completed}, FileConflict, Ssh{SessionEstablished,AuthRequired,PassphraseRequired,KnownHostsWarning,ConnectionLost}, DirChanged, MenuActivated, Mcp{OpenTabRequest,CloseTabRequest,GrantExpired}). `AppEvent::event_name()` maps each variant → the exact string the existing call site emits; `AppEvent::from_raw(&RawEvent) -> Option<AppEvent>` decodes by name + serde (tolerates extra payload fields via `#[serde(default)]` on Options, ignores unknowns).
- `EventBus::emit_event(AppEvent)` — serializes externally-tagged, unwraps to the flat field object so typed + string emitters share one wire shape. `App::emit_event()` forwards to it.
- **One call site converted to typed** (acceptance criterion): `modules/fs/watcher.rs` `fs:dir-changed` → `AppEvent::DirChanged` (dropped the local `DirChangedPayload` struct).
- `lib.rs` now re-exports `RawEvent` alongside `AppEvent`.
- `crates/app/src/main.rs`: `init_logging()` with `EnvFilter` (`warn,labonair=debug,labonair_backend=debug`, `RUST_LOG`-overridable, ANSI+target). `main()` now builds a `tokio::runtime::Runtime`, constructs `Backend::new(dirs::data_dir()/labonair)`, calls `spawn_workers()`, and `spawn_event_logger()` subscribes to the bus and logs every event (typed via `from_raw`, else trace). Runtime is `mem::forget`-kept for process lifetime. Then the GPUI window opens as before.
- Workspace `tracing-subscriber` gained `features = ["env-filter"]`; `crates/app` gained `dirs`.
- 3 unit tests in `events.rs` (typed round-trip, extra-field decode, unknown-name → None).
- **Verified:** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (119 backend + 1 app_state) — all green. `cargo build --bin labonair` OK.

### Current State
- Branch `master`, 4 unpushed commits + this one. Backend event layer is typed; app wires bus + logging.
- **Not yet visually run** — user should `cargo run` to confirm the startup log line prints and the window still opens.
- `reference-src/` untouched.

### What's Next
- **T01-005** `tasks/phase-00-setup/T01-005-ci-pipeline.md` — GitHub Actions: cargo check/clippy/test/fmt on macOS. Deps (T01-001) satisfied. Last task in phase 00.

### Blockers
- None.

---

## Session: 2026-08-31 (T01-003 — verify reference-src + project docs)

### What Was Done
- **T01-003 ✅ Done.** Verified all reference paths under `reference-src/` exist and are readable: `src/styles/globals.css`, `src/modules/` (23 feature modules), `src-tauri/src/modules/` (all listed backend modules incl. `secrets.rs`, `dock_menu.rs`, `menu_sync.rs`, `errors.rs`), `src-tauri/Cargo.toml`, `src-tauri/src/modules/pty/scripts/` (zshrc/bashrc + z* init scripts). Nothing missing.
- **README.md** already existed (from T01-001) and already describes the hard-fork / read-only `reference-src/` character. Added a "Goal" (full feature parity, web-preview → native markdown) and "Status" (links to ROADMAP + handshake) section.
- **.gitignore** confirmed: `reference-src/` is NOT ignored (tracked); no external symlink entry.
- Reworded 3 historical lines in `handshake.md` that still contained the literal old `../Labonair` path so `git grep -n "\.\./Labonair"` is now 0 hits.
- **Verified:** `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` (117 tests) — all green.

### Current State
- Branch `master`, 3 unpushed commits + this one. No code changes — docs only.
- `reference-src/` untouched.

### What's Next
- **T01-004** `tasks/phase-00-setup/T01-004-event-system.md` — typed event routing/logging on top of `crate::events`. Deps (T01-001, T01-002) satisfied.
- Then T01-005 (CI).

### Blockers
- None.

---

## Session: 2026-08-31 (T01-002 — full backend port from reference-src, Tauri stripped)

### What Was Done
- **T01-002 ✅ Done.** Ported the entire Rust backend from `reference-src/src-tauri/src/modules/` into `crates/backend/src/modules/` (18 modules: ssh, sftp, git, fs, pty, hosts, credentials, snippets, secrets, shell, themes, backgrounds, fonts, scrollback, terminal_exec, settings, mcp, errors). ~16k LOC, ~150 `#[tauri::command]` fns.
- **Approach: verbatim copy + mechanical Tauri strip** (perl passes in `/tmp/detauri.pl`) rather than hand-retyping — preserves logic 1:1. Transformations: drop `#[tauri::command]`; `tauri::State<'_,T>` → `&T`; `tauri::AppHandle` → `crate::App`; `tauri::ipc::Channel<T>` → `crate::events::EventChannel<T>`; `use tauri::*` removed; `.inner().clone()` → `.clone()`; `tauri::async_runtime::spawn` → `tokio::spawn`; `app.state::<T>()` → `&app.<field>` (5 sites, hand-fixed); `app.emit(...)` → `crate::App::emit`.
- **New infra:** `crates/backend/src/events.rs` (`EventBus` broadcast + `EventChannel<T>` point-to-point sink — forward-compatible with T01-004), `crates/backend/src/app.rs` (`App`/`AppState` = `Arc<AppInner>` holding every sub-state + `EventBus`; `App::new(&Path)` opens SQLite + builds all state; `App::spawn_workers()` starts the SFTP transfer worker + MCP auto-revoke sweeper). `lib.rs` re-exports `App`, `AppState`, `AppError` (= `LabonairError`, `From` impls extended with `serde_json::Error` + `String`), `AppResult`.
- **Hand-rewritten:** `settings/mod.rs` (`tauri_plugin_store` → plain atomic JSON read/merge/write against `config_dir()/labonair-settings.json`). `modules/mod.rs` (dropped `dock_menu`/`menu_sync` → deferred to T04-005).
- **Deps added to `crates/backend/Cargo.toml`:** russh(+rsa), russh-sftp, rusqlite, reqwest(rustls), portable-pty, tokio-util, thiserror, log, aes-gcm, rand, md5, openssl(vendored), base64, ignore, grep-regex, grep-searcher, globset, notify, notify-debouncer-mini, flate2, vte 0.15, rmcp 2.2, axum 0.8, schemars, fontdb, socket2, libc, dirs, uuid. (No git2 — git via CLI. No keyring — secrets use aes-gcm local store.)
- **One pre-existing broken test fixed:** `git::tests::branches_parses_current_and_upstream_tracking` used `|` separators but `parse_branches` splits on `\0` (matches `BRANCH_FORMAT` `%00`) — test data corrected, no logic change.
- **Verified:** `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` — all green. 116 backend unit tests + `tests/app_state.rs` (AppState init acceptance test) pass.

### Current State
- Branch `master`, 2 unpushed commits ahead + this one. `crates/backend` fully ported and green. `crates/app` still just the empty GPUI window (does not wire `AppState` yet — that's later phases).
- `reference-src/` untouched.

### What's Next
- **T01-003** `tasks/phase-00-setup/T01-003-reference-symlink.md` — verify `reference-src/` is intact + write project README/.gitignore notes. Deps (T01-001) satisfied.
- Then T01-004 (event system — build typed routing on top of `crate::events`), T01-005 (CI).

### Blockers
- None. Note for later: SSH/SFTP/git/MCP got a *mechanical* Tauri strip — runtime behaviour needs the user's testing once a UI wires them (per the project's KI-builds / user-tests workflow).

---

## Session: 2026-08-31 (T01-001 — Cargo workspace scaffolded, empty GPUI window runs)

### What Was Done
- **T01-001 ✅ Done.** Scaffolded the cargo workspace: root `Cargo.toml` (resolver 2, `[workspace.dependencies]`) + 7 crates under `crates/`: `app` (bin `labonair`), `ui`, `theme`, `terminal`, `editor`, `backend`, `ai`. Placeholder `lib.rs` per crate (doc comment only — **not** the `pub mod ...;` stubs the task listed, since those module files don't exist yet and would break `cargo build`; later phases add them).
- **`crates/app/src/main.rs`** — GPUI entry: `Application::new().run(...)`, `Bounds::centered`, `cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), .. }, |_, cx| cx.new(|_| Root))`, `cx.activate(true)`. `Root` is a `Render` view: full-size `div` bg `0x1a1b26`, child text "Labonair-rust — ready for development" in `0xc0caf5`. `tracing_subscriber::fmt::init()`.
- **Deps corrected vs task file**: `gpui = "0.2.2"` (real crates.io crate; `gpui-component` is a **separate** crate `0.5.1`, NOT a gpui feature — deferred to T04+). Dropped `alacritty_config_derive`, `fontdb`, `rsa` feature on russh (not needed yet). russh resolved to 0.62.7, russh-sftp 2.4.0.
- **Metal Toolchain installed** — `cargo build` of gpui failed with "missing Metal Toolchain"; fixed via `xcodebuild -downloadComponent MetalToolchain` (688 MB, one-time, machine-level).
- Root `README.md` added (workspace layout + commands + Metal Toolchain note).
- Verified: `cargo check`, `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` all green. `cargo run` opens the window — **user visually confirmed** "ready for development" text renders.

### Current State
- Branch `master` (still 1 unpushed planning commit ahead + this one). `Cargo.lock` committed.
- Working GPUI window. `reference-src/` untouched.

### What's Next
- **T01-002** `tasks/phase-00-setup/T01-002-extract-backend-logic.md` — port backend modules from `reference-src/src-tauri/src/modules/` into `crates/backend/`, stripping Tauri/IPC. Deps (T01-001) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (Planning correction: hard-fork restructure, reference-src, roadmap gaps closed)

### What Was Done
- **Hard fork clarified & enforced.** User: Labonair-rust must be fully decoupled — no symlink/submodule/path-dependency to the original repo. Memory written (`memory/hard-fork-reference-src.md`).
- **Moved the copied web app into `reference-src/`** (`git mv` of `src/`, `src-tauri/`, `docs/`, `scripts/`, all web configs, `CHANGELOG/CONTRIBUTING/SECURITY/README`, and the old `.github/workflows/`). Repo root is now clean: `CLAUDE.md`, `handshake.md`, `tasks/`, `reference-src/`, `LICENSE`, `.github/` (templates only), `.gitignore`.
- **Swept old external relative paths → `reference-src`** across CLAUDE.md, handshake.md, ROADMAP.md, and all task files (0 remaining hits for the old parent-dir path).
- **`.gitignore` rewritten** for Rust (`/target`, `.claude/`, `session-*.md`); `reference-src/` is tracked.
- **T01-001 deps corrected** to match `reference-src/src-tauri/Cargo.toml`: `russh 0.62.2` (ring), `russh-sftp 2.3.0`, `rusqlite 0.40`, `portable-pty 0.9`; **removed `git2`** — git runs via the `git` CLI (local + remote-over-SSH), same as the original. Same fix applied in T01-002 and T09-001.
- **T01-003 rewritten** — no longer a symlink-to-parent-repo task; now "verify reference-src + write README + .gitignore". Standalone.
- **Roadmap gaps closed** (user wants full feature parity, nothing out of scope). Added tasks:
  - `T01-005` CI pipeline (cargo check/clippy/test/fmt on macOS)
  - `T02-005` font handling/bundling · `T02-006` terminal background images
  - `T04-003` app-shell & window chrome (header/statusbar/sidebar/root coordinator) · `T04-004` notifications/toasts · `T04-005` native macOS menus (app menu + dock menu)
  - `T11-005` MCP bridge server · `T11-006` MCP bridge UI/grants
  - `T15-005` auto-updater (Sparkle) · `T15-006` feature-parity acceptance checklist
  - ROADMAP: added "Feature-Parität" section + success criteria 16–21. Only accepted deviation: web-preview tab → native markdown + open-in-system-browser (GPUI has no WebView).

### Current State
- Branch `master`. This planning-correction commit is **local, not yet pushed** (push when user asks).
- `reference-src/` holds the full frozen original. No cargo workspace scaffolded yet.
- Roadmap now has **55 task files** across 15 phases + expanded ROADMAP.md.

### What's Next
1. Execute **`tasks/phase-00-setup/T01-001-setup-cargo-workspace.md`** per the Next Task Protocol: scaffold the cargo workspace (7 crates), empty GPUI window, `cargo check`/`clippy`/`run` green → mark `✅ Done` → update handshake → commit.
2. Then T01-002 (extract backend from `reference-src/`) → T01-003 (verify reference-src + README) → T01-004 (events/logging) → T01-005 (CI).

### Blockers
- None. T01-001 has no dependencies.

---

## Session: 2026-08-31 (Roadmap complete + repo created + CLAUDE.md Next Task Protocol)

### What Was Done
- **Roadmap built out fully.** `tasks/ROADMAP.md` defines the 15-phase port (Setup → Theme → Terminal → Tabs → Explorer → Editor → SSH → SFTP → Git → Git-Graph → AI → Snippets/Palette → Settings → Session → Testing/Polish). 43 task files exist under `tasks/phase-*/` (each is instruction-only: context, goal, instructions, acceptance criteria, notes, warnings — no code). Naming: `T{NN}-{OOO}.md` where NN = phase 01–15, OOO = task number (files currently use the sequential scheme, e.g. `T01-001-setup-cargo-workspace.md`, `T13-001-settings-preferences.md`, `T15-004-packaging-release.md`). Next Task Protocol in CLAUDE.md consumes these in order.
- **Initial commit** `ead881a` committed the full repo (including the copied original source, kept as reference).
- **GitHub repo created & linked**: `gh repo create Labonair-rust --public --source=. --remote=origin --push` → remote `origin` set, `master` tracked to `origin/master`, URL https://github.com/Snenjih/Labonair-rust
- **CLAUDE.md rewritten** for the port (was stale Tauri/web content). Now documents: Rust/GPUI commands (`cargo check/build/run/clippy/test/fmt`), the single-binary GPUI architecture, Critical Rules (no web tech, reference-only original, GPU API must be source-verified), **Next Task Protocol** (the core task-by-task workflow), session start/end protocols, bug-memory rule, reference usage, language protocol.
- **handshake.md reset** for the port project (this file).

### Current State
- Branch `master`, repo pushed to `origin/master`.
- **First implementation task** (`T01-001-setup-cargo-workspace.md`) is the next to work — nothing implemented yet (only roadmap + config docs). `cargo` project has NOT yet been scaffolded.
- The original source files (src/, src-tauri/, docs/, package.json, etc. from the clone) still sit in the repo as an untracked-from-the-web-app reference. **These are reference artifacts for the port to read, NOT the app to extend.** Future phases will replace the web-tech portions.

### What's Next
1. Execute **T01-001-setup-cargo-workspace.md** per the Next Task Protocol in CLAUDE.md: scaffold the cargo workspace, set it up, verify (`cargo check`), mark `✅ Done`, update this handshake, commit.
2. Then T01-002 → T01-003 (reference symlink → `reference/`) → T01-004, and continue through the phases in order.

### Blockers
- None. Next task is dependency-clean (T01-001 has no dependencies).

---

## Previous Session: (none — port infrastructure only)

Project start. Created repo clone, initialized git, wrote the full task roadmap + task files. No application code written yet.
