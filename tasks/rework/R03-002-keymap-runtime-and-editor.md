# R03-002 — Build the module-owned keymap runtime and editor

## Status

`✅ Done`

## Owner

- Module: `keymap`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Dependencies

- `R03-001-command-palette-provider-registry`
- `R02-003-project-entry-and-workspace-transitions`

## Goal

Make keyboard navigation a first-class, module-owned capability. Most
meaningful actions—including workspace, tabs, panes, panels, editor, terminal,
hosts, transfers, and palette navigation—must expose stable command identities
and resolve through one context-aware keymap runtime.

## Scope

- In scope: keymap descriptors, context precedence, default bindings,
  user-file persistence, conflict resolution, runtime installation, keymap
  editor UI, and registration guidance for new panels/actions.
- Out of scope: changing every default shortcut in one pass, editor command
  semantics, platform-specific OS menu accelerators, and modal task controls
  that intentionally capture input.

## Contracts and ownership

- Public domain values: command IDs from the command registry, binding values,
  context identifiers, conflict records, and immutable resolved snapshots in
  `labonair-keymap`.
- Service boundary: the keymap resolves a command request; owning modules
  execute it. The keymap must not import feature state or UI entities.
- Registry contributions: every module registers its available commands and
  optional default bindings through the command/keymap contract.
- UI surface: a dedicated keymap management view, reachable from the titlebar
  global menu and command palette.
- Shared UI-kit components: search field, grouped list, editable shortcut row,
  conflict indicator, key hint, dialog, and scroll container.

## Dependencies

- Existing edges removed: the temporary shell-only keybinding installation and
  palette-owned shortcut table after all consumers migrate.
- New edges: the GPUI adapter may depend on `labonair-keymap` and the command
  registry; feature modules depend only on their own public command contracts.
- Dependency verifier change: remove the temporary palette-to-keymap adapter
  once runtime installation is owned by the keymap boundary.

## Persistence and migration

- Settings: no keybindings are stored in Settings; bindings live in the
  versioned user keymap file with optional context scopes.
- Storage: preserve existing shortcut slugs and migrate old action names using
  the explicit alias map from `R03-001`.
- Compatibility: conflicting or unknown entries remain readable and are
  surfaced as diagnostics; they must not silently erase user data.

## User-visible behavior

- Canonical entry point: global menu → Keymap and the global palette action.
- Notifications: malformed files, unknown commands, and write failures use
  structured notifications with expandable details. Conflict resolution is an
  actionable keymap control, not a passive duplicate error.
- Inline errors/toasts: no passive inline error banner or toast; field-level
  validation may identify the row currently being edited.

## Implementation plan

1. Define contexts, precedence, resolved snapshots, and migration-safe file
   format → verify deterministic resolution and conflict tests.
2. Install the runtime adapter and migrate tab/pane/panel/editor/terminal
   navigation registrations → verify keyboard integration without feature
   state in the keymap crate.
3. Implement the keymap editor and remove the legacy shortcut surface/table →
   verify persistence fixtures, malformed-file notifications, and visual
   focused/filtered/conflict states.

## Progress

The keymap module now owns a UI-free deterministic resolver in
`keymap::runtime`. It accepts typed `CommandId` bindings, applies global and
active-context precedence, replaces later declarations at equal precedence,
and preserves first-seen keystroke order. Existing file parsing and GPUI
installation remain adapters around this contract. The GPUI menu adapter now
resolves persisted action names through `keymap::runtime::command_for_action`
before mapping typed IDs to concrete actions; migration aliases therefore have
one owner and cannot drift between adapters. Rich GPUI context predicates stay
in the adapter while portable contexts remain typed in the runtime. The next
slice is to replace the shell-owned keymap reload/install entry point with a
keymap-owned adapter and expose diagnostics through the notification contract.
The JSONC parser, default assets, merge model, validation, and user-file path
have now moved from `labonair-settings` into `labonair-keymap::file`; Settings
no longer owns or imports the keymap module.
Last-good user-file recovery, validation issue retention, and default/user
layer composition now also live in `labonair-keymap::file`. The shell's
remaining keymap loader only supplies the command vocabulary, installs GPUI
bindings, publishes display hints, and connects the existing filesystem
watcher. The next slice is to replace that remaining reload/install
orchestration with an explicit Keymap adapter contract before implementing the
editor surface.

## Acceptance criteria

- [ ] A new action can register a stable command ID and default binding without
      editing shell or palette tables.
- [ ] Resolution is deterministic across contexts, user overrides, and
      conflicts, with focused UI-free tests.
- [ ] Tab, pane, panel, editor, terminal, host, transfer, and palette actions
      have command identities; missing defaults are intentional and documented.
- [ ] The keymap editor reads/writes the canonical file without losing unknown
      or invalid entries.
- [ ] The keymap module does not own feature execution or feature UI state.
- [ ] Shared controls use `labonair-ui-kit`; no toast or duplicate inline error
      surface exists.
- [ ] Focused tests and all repository verification gates pass.
- [ ] Keymap normal, filtered, conflict, empty, and malformed-file states are
      visually checked.

## Removal condition

This task is complete only when no active feature relies on a private shortcut
table or shell-only keybinding path, and the old Settings/Shortcuts management
surface is no longer reachable except through explicit data migration.

## Notes and follow-ups

Theme, host, transfer, and workspace modules remain responsible for their
commands. They contribute metadata and handlers; they do not move behavior
into the keymap runtime.

Command descriptors now carry typed owner defaults through
`with_default_binding`. Keymap materializes those contributions as a default
file layer before applying the user file; the shipped JSONC remains a
compatibility layer while the remaining defaults migrate. The provider and
shell adapter are equality-checked so their command metadata cannot diverge.

`keymap::adapter::load` now returns an immutable module-owned snapshot of
effective bindings and diagnostics from the command registry. Shell retains
only GPUI installation, display-hint publication, and file-watch wiring; the
next slice is the lossless keymap management/editor contract.

The first owner-default migration is now broad enough to cover command palette,
editor search, workspace tabs/panes/sidebar/zoom, settings Zen mode, and the
shell's debug actions. The shell adapter derives matching metadata only while
the transitional execution registrations remain; provider equality tests catch
drift.

The palette, panel tooltips, and tab context-menu hints now render from the
effective `CommandId` map rather than the legacy `ShortcutId` table. The old
table remains exported only for migration compatibility and is not an active
UI source.

`keymap::file::KeymapDocument` now provides the lossless editor contract: raw
JSONC source remains authoritative while parse and validation results are
derived views. Invalid or unknown entries can be displayed and saved without
being erased; runtime activation still uses the last valid layer.

The first dedicated management surface is now implemented in the real
`labonair-keymap-ui` sibling crate. It loads the document and runtime snapshot
off the GPUI thread, renders searchable command rows and diagnostics from
shared UI-kit primitives, and exposes raw JSONC editing through an injected
composition callback. The titlebar and command-palette Keymap entrypoints
now open this surface; shell remains responsible only for the callback that
opens the existing editor tab. Each row now has an inline rebind editor with
GPUI keystroke validation, same-context conflict detection, explicit unbind,
and a lossless JSONC append-only override writer. Keymap diagnostics and
load/save failures are published through the app-wide retained notification
registry; the management view does not render a duplicate passive error
banner. The user confirmed the native visual state, and the full workspace
verification gates pass.
