# R07-004 — Extract the Explorer host contract

## Status

`⏳ Planned`

## Owner

- Module: Explorer with the Workspace composition boundary
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell` / Workspace construction

## Goal

Remove the remaining feature-view dependency from `labonair-panel-explorer`
to `labonair-workspace`. Explorer must express open-file, open-terminal,
preview, and drag/preview intents through a narrow typed host contract rather
than storing or calling a Workspace entity directly.

## Dependencies

- `R07-001-product-surface-acceptance` must be complete.
- The current edge and its removal condition are recorded in
  [`../../docs/audits/remaining-boundaries.md`](../../docs/audits/remaining-boundaries.md).

## Scope

- In scope: `panel-explorer` host intents, shared drag/preview values, the
  Workspace adapter, composition wiring, focused tests, and dependency
  documentation.
- Out of scope: Explorer feature redesign, filesystem API redesign, new
  panels, or changes to the user-visible Explorer workflow.

## Contracts and ownership

- Explorer owns file-tree state, selection, and Explorer commands.
- Workspace owns tab/pane creation and decides how an Explorer intent is
  fulfilled.
- A UI-free Explorer host contract owns only the intents required by the
  Explorer view. It must not expose `Entity<Workspace>` or Workspace-private
  state.
- Shared drag/preview values move to a lower-level contract location only when
  both consumers can use that location without importing a feature owner.
- Existing command IDs, keymap behavior, notifications, and UI-kit controls
  remain unchanged.

## Dependencies and migration

- Remove `panel-explorer → workspace` from Cargo metadata and the dependency
  verifier allow-list after every consumer is migrated.
- Add only the narrow contract edge required by Explorer and its Workspace
  adapter.
- Keep composition in `labonair-shell`; do not introduce a replacement
  aggregate backend or a second Explorer owner.

## Persistence and migration

- Settings: none.
- Storage: none; existing Explorer and Workspace persistence formats remain
  unchanged.
- Compatibility: none expected. If a compatibility adapter is unavoidable,
  document its exact consumer and removal condition in the task before adding
  it.

## User-visible behavior

- Canonical entry point: the existing Explorer dock panel.
- Notifications: preserve the current owner-routed operation notifications.
- Inline errors/toasts: no new surface; existing actionable controls remain
  only where the user must make a decision.

## Implementation plan

1. Inventory the current Explorer-to-Workspace calls and define the smallest
   typed contract → verify with contract tests.
2. Move shared drag/preview values and inject the Workspace adapter → verify
   Explorer focused tests and the existing Workspace integration tests.
3. Remove the direct dependency and update the inventory/backlog evidence →
   verify the dependency checker and source audit.
4. Run the native Explorer visual states from the acceptance matrix → verify
   the narrow, focused, empty, loading, error, long-list, and overlay states.

## Acceptance criteria

- [ ] Explorer has no direct Workspace entity or private-state dependency.
- [ ] The host contract is UI-free, narrow, and owned by the Explorer boundary.
- [ ] Workspace fulfills the contract only through composition injection.
- [ ] Existing commands, persistence, notifications, and user behavior remain
      compatible.
- [ ] Shared controls continue to come from `labonair-ui-kit`.
- [ ] Focused tests pass.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo check --workspace --all-targets` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace --no-fail-fast` passes.
- [ ] `scripts/check-crate-deps.sh` and `git diff --check` pass.
- [ ] The Explorer visual states are recorded with native evidence.

## Removal condition

This task is complete only when the direct `panel-explorer → workspace` edge
is absent from source and metadata, the injected host contract is the sole
integration path, and the focused plus full verification and visual checks
pass.

## Notes and follow-ups

This task is intentionally planned while R07-001 remains active; it must not
be marked in progress until the product-surface acceptance gate is complete.
The next boundary after this one is B02 (`workspace → background`) only after
this task has been completed and re-evaluated.
