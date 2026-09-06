# R03-001 — Make command discovery provider-owned

## Status

`⏳ Planned`

## Owner

- Module: `command-palette`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Goal

Make the command palette a registry consumer rather than a second application
dispatcher. Every palette command and dynamic submenu must be contributed by
the owning module through one typed registry, while execution remains in that
owner.

## Scope

- In scope: `labonair-command-palette-core`, palette UI, shell command wiring,
  command descriptors, provider snapshots, stable action IDs, and the first
  owner registrations for workspace, terminal, editor, hosts, themes, and
  settings.
- Out of scope: implementing new feature behavior, redesigning the keymap
  editor, remote theme downloads, and changing the user-facing workflow of an
  existing command.

## Contracts and ownership

- Public domain values: `CommandId`, `CommandDescriptor`, submenu/provider
  snapshots in `labonair-command-palette-core`.
- Service boundary: owners expose typed command handlers or action requests;
  the palette never receives a feature entity or private state.
- Registry contributions: one command/provider registry with duplicate-ID
  rejection and stable serialization names.
- UI surface: `labonair-command-palette` owns search, focus, selection,
  submenu navigation, and Enter/secondary-action dispatch.
- Shared UI-kit components: search field, list, list row, key hint, empty
  state, scrolling container, and popover/modal primitives.

## Dependencies

- Existing edges removed: the shell's duplicate behavior table and palette
  feature-specific dispatch; legacy entries that have no current workflow.
- New edges: owner-to-registry contract crates only; no palette-to-feature UI
  dependency is allowed.
- Dependency verifier change: remove each obsolete shell/palette edge and
  record any temporary adapter with a named removal condition.

## Persistence and migration

- Settings: none. Command metadata and registration are not settings values.
- Storage: preserve stable action names in existing keymap files.
- Compatibility: accept old action names only through a documented alias map;
  remove an alias after all shipped keymap migrations and consumers are
  covered by tests.

## User-visible behavior

- Canonical entry point: global command-palette action and the titlebar global
  menu's palette-backed submenus.
- Notifications: registration or provider failures are diagnostic
  notifications with source and details; selection failures use the same
  registry. No passive toast is emitted.
- Inline errors/toasts: none for palette discovery or command execution.

## Implementation plan

1. Define the provider snapshot, stable IDs, and typed execution request →
   verify duplicate rejection, filtering, submenu ordering, and alias tests.
2. Move registrations into owning modules and make the palette render only
   registry snapshots → verify each initial provider in isolation.
3. Delete the duplicate shell table and obsolete palette entries → verify
   dependency graph, keymap compatibility fixtures, and visual palette states.

## Acceptance criteria

- [ ] One typed registry/provider surface is the only source of palette rows.
- [ ] Owning modules register commands without editing palette internals.
- [ ] Dynamic submenus provide immutable searchable snapshots and typed
      primary/secondary actions.
- [ ] Command execution does not move feature state into the palette or shell.
- [ ] Stable action IDs and compatibility aliases are covered by tests.
- [ ] Shared palette controls use `labonair-ui-kit`.
- [ ] No new toast or duplicate inline error surface exists.
- [ ] Focused tests and all repository verification gates pass.
- [ ] Normal, empty, focused, filtered, and long-list palette states are
      visually checked.

## Removal condition

This task is complete only when no active palette row is declared in a
shell-wide feature table and no command handler is implemented in the palette
or shell outside composition wiring.

## Notes and follow-ups

This task precedes `R03-002`. Keymap runtime dispatch may consume the same
stable command IDs, but keymap state and conflict logic remain owned by the
keymap module.
