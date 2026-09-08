# R01-004 — Replace static commands with palette and keymap registries

## Status

`✅ Done`

## Owner

- Modules: `command-palette`, `keymap`
- Capability matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Canonical surfaces: titlebar command menu, global palette, and Keymap settings

## Dependencies

- `R01-003-typed-transfer-registry`

## Goal

Make command discovery and keyboard shortcuts extensible module-owned
capabilities. The command palette must consume one typed registry/provider
surface instead of a static multi-level table, while the keymap owns binding
storage, contexts, precedence, conflicts, and editing.

## Scope

- Extract the UI-free command descriptors and provider registration API.
- Keep search, filtering, navigation, preview, and confirmation in the palette
  UI crate.
- Register commands from their owning modules through stable action IDs.
- Separate keymap descriptors and resolution from palette rendering.
- Remove duplicated command definitions and obsolete palette entries; retain
  only the first agreed command/submenu set.
- Preserve existing keymap migration data while making future bindings
  discoverable from registered commands.

## Ownership and dependency rules

- A feature owns the command action and its execution context.
- The palette owns presentation and selection, never feature state.
- The keymap owns binding resolution, never command implementation.
- Cross-module execution uses stable IDs and typed context, not a central
  feature-specific dispatch table.
- New palette UI uses `labonair-ui-kit` controls.

## Acceptance criteria

- [x] One command registry/provider surface is the only source of palette
      entries (`CommandId` + `CommandRegistry` provider contributions).
- [x] Dynamic submenus can supply immutable filtered snapshots and typed
      selection actions (`SubmenuSnapshot` + `PaletteAction`).
- [x] Keymap resolution and conflict detection are independent of GPUI
      (`labonair-keymap` is a UI-free crate).
- [x] Owning modules register commands without editing palette internals
      (per-module `CommandProvider` impls composed by the shell).
- [x] Legacy duplicate/static entries are removed or explicitly migrated.
      Runtime keymap resolution and the palette shortcut hint now derive from
      owner `with_default_binding` metadata and the `CommandId`-keyed
      `KeybindDisplay` map. The legacy `ShortcutId` / `SHORTCUTS` table is
      inert (its `with_shortcut` metadata is never rendered) but is still
      physically present and re-exported; deleting it is tracked by the P1.6
      follow-up task.
- [x] Focused registry/keymap tests and all workspace verification gates pass.
