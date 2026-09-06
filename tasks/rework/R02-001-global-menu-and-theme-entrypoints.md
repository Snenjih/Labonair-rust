# R02-001 — Global menu and theme entry points

## Status

`✅ Done`

## Owner

- Modules: `shell`, `command-palette`, and `themes`
- Canonical crates: `labonair-shell`, `labonair-command-palette-core`,
  `labonair-command-palette`, and `labonair-theme`
- Related contracts: [`../../docs/architecture.md`](../../docs/architecture.md),
  [`../../docs/design-system.md`](../../docs/design-system.md)

## Goal

Make the single titlebar menu the stable global entry point for values and
capability pickers that do not belong in Settings. The titlebar must publish
typed navigation events; it must not know how Settings, the keymap editor, or
the command palette execute their work.

## Scope

- Replace the placeholder titlebar profile item with `Settings`, `Keymap`,
  `Themes`, `Icon Themes`, and `Hosts` actions.
- Route those actions through a typed titlebar event subscribed by the shell
  composition root.
- Keep the shared `popover_menu` primitive as the only menu surface and use a
  window-coordinate anchor under the clicked button.
- Add an `Icon Themes` palette page backed by the theme registry, searchable,
  selectable with Enter, and previewed while the highlighted row moves.
- Keep host Enter/Shift+Enter behavior and app-theme preview behavior intact.
- Do not move command execution or host management into the titlebar.

## Dependencies

- `R01-004-command-palette-keymap-registry.md` — Done
- `R01-005-settings-ownership-and-reduction.md` — Done
- `R01-009-inline-error-notification-adoption.md` — Done

## Acceptance criteria

- [x] The titlebar menu contains only the documented global actions and no
  placeholder profile action.
- [x] Every menu action is emitted as a typed event and handled by the shell
  composition boundary.
- [x] The menu opens below the clicked button at the right edge, including
  when the window is resized or the button is near the edge.
- [x] App Themes and Icon Themes are separate searchable palette pages.
- [x] Selecting an icon theme with Enter activates and persists it; moving the
  selection previews it and leaving/canceling restores the persisted theme.
- [x] Existing host SSH/SFTP and app-theme behavior remains functional.
- [x] Documentation, capability inventory, tests, and dependency checks agree
  with the new ownership and entry points.
- [x] Full workspace verification gates pass.

## Removal condition

This task is complete only when the titlebar no longer owns feature behavior,
the old profile placeholder is gone, and no second global menu or direct
feature-specific menu implementation is needed for these entry points.

## Outcome

The titlebar now exposes Settings, Keymap, Themes, Icon Themes, and Hosts
through one shared, correctly anchored menu. Navigation is emitted through
`TitlebarEvent` and handled by shell composition. The palette has a separate
registry-backed Icon Themes page with search, Enter activation, and transient
preview/revert behavior. App-theme preview and host SSH/SFTP secondary actions
remain unchanged.
