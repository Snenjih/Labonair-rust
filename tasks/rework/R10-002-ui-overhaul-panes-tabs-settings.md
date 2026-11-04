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

- [ ] WS1 Panes: all kinds tile; pane bar; zoom; keybinds; session v3.
- [ ] WS1 DnD: tab<->pane, reorder, group tab name/icon, hover list.
- [ ] WS2 Spaces dropdown restyled, no colour badges.
- [ ] WS3 Floating tab blocks (horizontal and sidebar).
- [ ] WS4 Motion engine, `animations` setting, exit animations for overlays.
- [ ] WS5 Settings: cards, tiles, new tree, Appearance, Themes, inline background.
- [ ] WS5 Themes require light+dark; backgrounds visible.
- [ ] Focused tests pass; docs/ADRs/inventory/catalog updated.
- [ ] `cargo fmt --check`, `cargo check --workspace --all-targets` pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes (known
      pre-existing lint at `workspace.rs:3602` reported separately).
- [ ] `cargo test --workspace` passes (known 2 `project_multibuffer` failures
      reported separately).
- [ ] `scripts/check-crate-deps.sh` and `git diff --check` pass.

## Notes and follow-ups

See the approved plan for edge cases per workstream.
