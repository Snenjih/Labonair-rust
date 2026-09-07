# R07-004 — Extract the Explorer host contract

## Status

`⏳ Planned`

> Structural migration landed ahead of formal activation (see
> [Notes and follow-ups](#notes-and-follow-ups)). The direct
> `panel-explorer → workspace` edge is removed, the `ExplorerHost` contract and
> Workspace adapter are in place, and every code/dependency/test gate passes.
> The task stays `Planned` (not `Done`) only because its native Explorer
> visual-state evidence folds into the still-open R07-001 visual matrix.

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

- [x] Explorer has no direct Workspace entity or private-state dependency.
      `ExplorerView` holds a `labonair_explorer_host::ExplorerHost` (four
      injected callbacks); the `pub(crate) mod workspace` / `mod preview`
      shims and the `labonair-workspace` dependency are gone.
- [x] The host contract is UI-free, narrow, and owned by the Explorer boundary.
      `labonair-explorer-host` is a leaf crate (only `gpui`) that carries the
      four intents plus the `DraggedPaths` / `is_previewable` values shared
      with the terminal and preview views.
- [x] Workspace fulfills the contract only through composition injection.
      `crates/shell/src/bootstrap.rs` builds the Workspace-backed `ExplorerHost`
      and re-notifies the panel on active-editor changes.
- [x] Existing commands, persistence, notifications, and user behavior remain
      compatible. No command IDs, keymaps, settings, or persistence formats
      changed; the open/preview/terminal call sites forward to the same
      Workspace methods.
- [x] Shared controls continue to come from `labonair-ui-kit` (unchanged).
- [x] Focused tests pass. `labonair-explorer-host` adds unit coverage for the
      moved value types and a `gpui::test` for `ExplorerHost` dispatch.
- [x] `cargo fmt --all -- --check` passes.
- [x] `cargo check --workspace --all-targets` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] `cargo test --workspace --no-fail-fast` passes.
- [x] `scripts/check-crate-deps.sh` and `git diff --check` pass; the
      `panel-explorer → workspace` allow-list entry is replaced by
      `panel-explorer → explorer-host`.
- [ ] The Explorer visual states are recorded with native evidence. Deferred
      into the R07-001 visual matrix.

## Removal condition

This task is complete only when the direct `panel-explorer → workspace` edge
is absent from source and metadata, the injected host contract is the sole
integration path, and the focused plus full verification and visual checks
pass.

## Notes and follow-ups

This task was originally sequenced to start only after R07-001's visual
acceptance gate. On explicit direction to prioritise the boundary work over
the visual-matrix capture, the structural migration was implemented and
verified early:

- new crate `crates/explorer-host` (`labonair-explorer-host`): `ExplorerHost`
  (open-file / open-terminal / open-preview / active-file-path callbacks),
  plus `DraggedPaths` / `shell_quote` / `quote_paths` / `is_previewable` /
  `PREVIEW_EXTENSIONS` moved down from `labonair-workspace`;
- `labonair-panel-explorer`: dropped `labonair-workspace`; `ExplorerView` holds
  an `ExplorerHost`; `on_workspace_changed` → public
  `notify_active_file_changed` driven by the composition root;
- `labonair-workspace`: deleted `src/drag.rs`; `views/terminal.rs` and
  `views/preview.rs` re-import the shared values from `labonair-explorer-host`;
- `crates/shell/src/bootstrap.rs`: constructs the Workspace-backed
  `ExplorerHost` and folds an `e.notify_active_file_changed(cx)` call into the
  existing workspace observer;
- `scripts/check_crate_deps.py`, `docs/audits/architecture-inventory.md`,
  `docs/audits/remaining-boundaries.md` (B01 → **Done**), `docs/capabilities.md`,
  and `docs/rework-roadmap.md` updated in the same change.

Remaining before this task is marked `Done`: record the native Explorer visual
states, which is the same capture effort blocked in R07-001. The next boundary
after this one is B02 (`workspace → background`).
