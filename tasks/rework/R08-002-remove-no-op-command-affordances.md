# R08-002 — Remove visible no-op command affordances

## Status

`✅ Done`

## Owner

- Modules: `workspace`, `editor`, `command-palette`, `shell` (menu/composition)
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::commands` / `labonair-shell::menu`

## Goal

Every visible, actionable command has an owner handler and a focused execution
test, or is absent from the visible surface. No normal palette / menu / keymap
route silently does nothing.

## Scope

- In scope: the `Zoom In` / `Zoom Out` / `Reset Zoom` commands, the
  `Adjust Font Size…` submenu trigger and its `Font Size` palette page, and
  the `Format Document` editor command — all of which had no execution
  handler.
- Out of scope: implementing font zoom or a document formatter (no current
  product decision); the inert `ShortcutId` / `SHORTCUTS` cheat-sheet model
  (P1.6 follow-up).

## Contracts and ownership

- Public domain values: `CommandId` in `labonair-command-palette-core` — the
  `ZoomIn`, `ZoomOut`, `ZoomReset`, `AdjustFontSize`, and `FormatDocument`
  variants are removed together with their `ACTION_NAMES` entries.
- Registry contributions: `WorkspaceCommandProvider` / `EditorCommandProvider`
  no longer emit those descriptors; `CommandSubmenu::Zoom` and the palette
  `Page::Zoom` are removed.
- UI surface: native `View` menu (menu.rs), command palette.
- Shared UI-kit components: unchanged.

## Dependencies

- Existing edges removed: none.
- New edges: none.
- Dependency verifier change: none.

## Persistence and migration

- Settings: None.
- Storage: None.
- Compatibility: the `view.zoomIn` / `view.zoomOut` / `view.zoomReset` legacy
  keybind slugs are dropped from `migrate_v2::SLUG_TO_ACTION`; an old binding
  to one of them is now silently discarded rather than migrated to a dead
  action. The default keymap assets no longer bind `cmd-=/-/0`.

## User-visible behavior

- Canonical entry point: n/a — the affordances are removed.
- Notifications: none.
- Inline errors/toasts: none.

## Implementation plan

1. Remove the `CommandId` / `CommandSubmenu` / `Page` variants and their
   `ACTION_NAMES` rows → verify: `cargo check --workspace`.
2. Remove the descriptors, the `zoom_submenu` provider, the native-menu items
   and `actions!` entries, and the default-keymap bindings → verify:
   `cargo test -p labonair-shell` (`menu::tests::default_keymap_bindings_load`
   counts action coverage).
3. Update the legacy keybind slug map and its migration test → verify:
   `cargo test -p labonair-settings`.

## Acceptance criteria

- [x] Every visible actionable command has an owner handler and a focused
      execution test, or is absent from the visible surface.
- [x] `Zoom In` / `Zoom Out` / `Reset Zoom` / `Adjust Font Size…` /
      `Format Document` no longer appear in the palette, native menu, or
      default keymap.
- [x] `menu::tests::default_keymap_bindings_load` still proves every shipped
      default binding resolves to a concrete action.
- [x] No normal palette / menu / keymap route silently does nothing (the
      remaining `SwitchTab` / `OpenShortcuts` ids are navigators, covered by
      `commands::tests::navigator_ids_resolve_to_no_run`).
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`, and
      `git diff --check` pass.

## Notes and follow-ups

If font zoom is ever wanted, it returns as a new bounded task with a product
decision on scope (which surface, persistence, per-project) — not as a
resurrected no-op. Next task: R08-003 (panel-snippets → workspace decoupling).
