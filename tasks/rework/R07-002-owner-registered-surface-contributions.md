# R07-002 — Move surface contributions to owning modules

## Status

`⏳ Planned`

## Owner

- Module: application composition with the affected capability owners
- Capability-matrix rows: panels, status items, command palette, keymap
- Composition entry point: `labonair-shell`

## Dependencies

- `R07-001-product-surface-acceptance`
- `R03-001-command-palette-provider-registry`
- `R03-002-keymap-runtime-and-editor`

## Goal

Close any owner-contribution residue discovered by the R07-001 acceptance
audit. Capability modules must register their command execution and status
items through typed owner APIs; `labonair-shell` may only assemble those
contributions and connect native/debug actions that have no capability owner.
Panel contributions and all currently migrated product commands/status items
already follow this boundary and are the reference pattern.

## Scope

- In scope: any remaining feature-owned residue in
  `shell/src/commands.rs` or shell status-item registration, owner
  contribution APIs, command execution callbacks, and dependency/documentation
  evidence.
- Out of scope: redesigning command behavior, adding new commands, changing
  product surfaces, or introducing extension/marketplace infrastructure.

## Contracts and ownership

- The command-palette core remains the single metadata registry.
- Each owning module supplies its command metadata and execution contribution;
  execution must not be duplicated in a shell-wide command table.
- Panel registrations are contributed by their owning modules and consumed by
  the generic workspace registry. Status-item registrations must follow the
  same pattern.
- The shell remains responsible only for composition, native-window actions,
  and wiring typed services/events.

## Persistence and migration

No persisted format changes are expected. Existing command IDs, keymap files,
panel layout data, and statusbar placement data must remain compatible.

## User-visible behavior

This is an ownership refactor. Command search, keybindings, panel layout,
statusbar placement, and native-window behavior must remain unchanged.

## Verification

- Add or update focused registry/contribution tests.
- Run the full repository verification gates from
  [`feature-lifecycle.md`](../../docs/feature-lifecycle.md).
- Recheck that no duplicate shell-wide command, panel, or status-item list
  remains and update the capability matrix and architecture inventory.

## Removal condition

The task is complete when adding an owner command or status item does not
require editing a shell feature table, while the shell still composes and the
full verification gates pass.
