# R10-002 — UI overhaul: universal panes, floating tabs, motion, Settings

## Status

`⏳ Planned`

## Owner

- Modules: `workspace` (panes, tabs, spaces), `ui-kit` (pane bar, motion,
  settings widgets, floating tabs), `settings` / `settings-ui` (page tree,
  widgets, new keys), `theme` (light+dark requirement), `background`
  (visibility, inline surface)
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair_workspace::Workspace` (layout),
  `labonair_settings_ui::pages` (Settings tree)
- Decisions: [ADR 0005](../../docs/adr/0005-universal-panes.md),
  [ADR 0006](../../docs/adr/0006-settings-hosts-owner-surfaces-inline.md)
- Plan: orchestrated in waves by role (K = kit/chrome/motion,
  S = settings/themes, P = panes); the orchestrator alone edits this file.

## Goal

Every tab kind can be tiled as a pane; a tab is a pane group; panes float or
sit compact by setting; spaces and tabs use the Ciel look with floating tab
blocks; the UI has small consistent motion; Settings use Apple-style grouped
cards with preview tiles and the new category tree.

## Scope

- In scope: workstreams WS1-WS5 below.
- Out of scope: new icon set, remote editor collaboration, animating hover
  (GPUI 0.2.2 has no hover transitions) and pane/tab removal.

## Contracts and ownership

- Public domain values: `PaneKind`, pane tree, `PaneId`-keyed store
  (`workspace`), `MotionPrefs` (`ui-kit` Palette), `ThemeFamilyContent::validate`.
- Service traits or typed events: `pane::*` commands, `DraggedPane`.
- Registry contributions: `pane::*` command IDs and keymap defaults; inline
  `SettingsSurface` render mode.
- UI surface: `workspace/src/{pane_items,tiling,tab_strip,spaces_ui}`, `ui-kit`
  `pane_bar`, `settings_group`, `settings_row`, `preview_tile`.
- Shared UI-kit components: pane bar, settings group/row/tile, slider row,
  category chip, floating tab item, motion helpers.

## Dependencies

- Existing edges removed: Editor split tree -> removed.
- New edges: none planned. Dependency verifier change: None.

## Persistence and migration

- Settings: `appearance.paneLayout` (comfortable|compact),
  `appearance.animations` (system|full|reduced|off; `reduceMotion` migrates to
  `off`). Glassy/Classic reuses `appearance.translucentSurfaces`.
- Storage: session snapshot v3 (migration from v2); `SpaceEntry.color` dropped,
  old snapshots still load.
- Compatibility: none kept after migration.

## User-visible behavior

- Canonical entry points: pane bar, `pane::*` commands, Settings pages.
- Notifications: no-room split, rejected theme file, via the notification
  center. No toasts.

## Implementation plan

Wave 1 (parallel): K1 kit foundation · S1 settings data/themes/backgrounds ·
P1 pane model (additive).
Wave 2 (parallel): P2 pane integration · S2 settings UI · K2 motion sweep.
Wave 3: K3 spaces + tabs, then P3 DnD + group tabs; S3 closing sweep.
Gates run by the orchestrator after every wave.

## Acceptance criteria

- [x] WS1 Panes: all kinds tile; pane bar; zoom; keybinds; session v3.
- [x] WS1 DnD: tab<->pane, reorder, group tab name/icon, hover list.
- [x] WS2 Spaces dropdown restyled, no colour badges.
- [x] WS3 Floating tab blocks (horizontal and sidebar).
- [x] WS4 Motion engine, `animations` setting, exit animations for overlays.
- [x] WS5 Settings: cards, tiles, new tree, Appearance, Themes, inline background.
- [x] WS5 Themes require light+dark; backgrounds visible.
- [x] Focused tests pass; docs/ADRs/inventory/catalog updated.
- [x] `cargo fmt --check`, `cargo check --workspace --all-targets` pass.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace` passes (known 2 `project_multibuffer` failures
      reported separately).
- [ ] `scripts/check-crate-deps.sh` and `git diff --check` pass.

## Notes and follow-ups

- Checked boxes mean implemented, compiled and unit-tested. **No wave was
  verified visually or interactively** (screenshots were black in the agent
  environment): run [`../../docs/qa/r10-002-visual-checklist.md`](../../docs/qa/r10-002-visual-checklist.md)
  before closing this task.
- Editor panes: only the focused pane of a document is interactive; other
  panes of the same document are read-only mirrors until clicked (two
  `EditorView`s cannot share a `Document`). A shared-document editor refactor
  is a separate task.
- `appearance.animations = system` currently behaves like `full` (needs an
  `objc2-app-kit` edge in `theme-ui`; owner decision).
- Editor panes shadow `cmd-alt-up/down` and `cmd-[ ]`; SSH/SFTP split opens a
  new connection.
- Pre-existing, unrelated: 2 failing `project_multibuffer` tests, 2 crate-deps
  violations (`editor-ui`->`panel`, `shell`->`editor-ui`), ~60 undocumented
  `CommandId`s in `commands.toml`, `check_crate_deps.py` metadata decode.
