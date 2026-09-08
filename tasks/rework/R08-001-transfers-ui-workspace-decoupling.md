# R08-001 — Remove Workspace entity coupling from Transfers UI

## Status

`✅ Done`

## Owner

- Module: `transfers`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap` / `shell::status_items`

## Goal

`labonair-transfers-ui` no longer depends on `labonair-workspace`. The
statusbar transfer item emits its typed completion signal and the composition
root routes it to the SFTP pane refresh, matching `docs/registries.md`.

## Scope

- In scope: `crates/transfers-ui/src/status_item.rs`,
  `crates/transfers-ui/Cargo.toml`, `crates/shell/src/status_items.rs`,
  `crates/shell/src/bootstrap.rs`, `scripts/check_crate_deps.py`,
  `docs/audits/architecture-inventory.md`.
- Out of scope: transfer lifecycle, progress, history, resolution dialogs
  (all already owned by `labonair-transfers` / the `TransfersView`).

## Contracts and ownership

- Public domain values: `TransferUiEvent::Completed { session_id, direction }`
  (already defined in `labonair-transfers-ui`).
- Service traits or typed events: the existing `EventEmitter<TransferUiEvent>`
  on `TransfersView`.
- Registry contributions: `StatusItemRegistration` id `"transfers"` (unchanged).
- UI surface: `crates/transfers-ui`.
- Shared UI-kit components: unchanged.

## Dependencies

- Existing edges removed: `labonair-transfers-ui → labonair-workspace`.
- New edges: none.
- Dependency verifier change: drop `labonair-workspace` from the
  `labonair-transfers-ui` allow-list set in `scripts/check_crate_deps.py`.

## Persistence and migration

- Settings: None.
- Storage: None.
- Compatibility: None — the `TransferUiEvent::Completed` contract already
  exists; only its subscriber moves.

## User-visible behavior

- Canonical entry point: statusbar transfer item + dropdown (unchanged).
- Notifications: unchanged.
- Inline errors/toasts: none.

## Implementation plan

1. Move the `TransferUiEvent::Completed → refresh_sftp_after_transfer`
   subscription out of `TransfersStatusItem` and into `bootstrap` → verify:
   `cargo test -p labonair-transfers-ui -p labonair-shell`.
2. Drop the `workspace` field/param and the `labonair-workspace` dependency
   → verify: `cargo check -p labonair-transfers-ui`.
3. Update the verifier allow-list and architecture inventory → verify:
   `bash scripts/check-crate-deps.sh`.

## Acceptance criteria

- [x] `transfers-ui` has no `labonair-workspace` dependency or Workspace
      entity field (`Cargo.toml` + `check_crate_deps.py` allow-list updated).
- [x] Transfer completion still refreshes the affected SFTP pane
      (`bootstrap` subscribes to `TransferUiEvent::Completed`).
- [x] A focused test proves completion dispatch without constructing Workspace
      (`view::tests::completion_event_maps_only_completed_updates`).
- [x] Registry documentation and the Cargo graph describe the same boundary
      (`docs/registries.md`, `docs/audits/architecture-inventory.md`).
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`, and
      `git diff --check` pass.

## Notes and follow-ups

The `cx.observe(&workspace)` in `TransfersStatusItem` was inert boilerplate —
the item renders only from `TransfersView` + `ThemeStore` state. Next task:
R08-002 (panel-snippets → workspace decoupling).
