# R07-005 — Remove the Workspace → BackgroundStore dependency

## Status

`⏳ Planned`

> Structural migration landed ahead of formal activation, the same way
> [`R07-004`](R07-004-explorer-host-contract.md) did for B01. The direct
> `workspace → background` edge is removed, the `BackgroundHost` contract and
> composition-time adapter are in place, and every code/dependency/test gate
> passes. The task stays `Planned` (not `Done`) only because its native
> visual-state evidence (terminal background layer, App-scope layer) folds
> into the still-open R07-001 visual matrix.

## Owner

- Module: Backgrounds, with the Workspace/Terminal presentation boundary
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell` / `bootstrap::bootstrap`

## Goal

Remove `labonair-workspace`'s direct dependency on `labonair-background`
(`Entity<BackgroundStore>`), described as B02 in
[`../../docs/audits/remaining-boundaries.md`](../../docs/audits/remaining-boundaries.md),
while keeping all image storage, import/delete behavior, persistence,
decoding, and rendering policy in the Backgrounds module.

## Scope

- In scope: `labonair-workspace` (`workspace.rs`, `views/terminal.rs`), the
  new `labonair-background-host` presentation-contract crate, `labonair-background`'s
  injection point, and the shell composition wiring.
- Out of scope: Background feature redesign, new background capabilities, or
  changes to the user-visible background/appearance workflow.

## Contracts and ownership

- Public domain values: `labonair_background_host::{LayerScope, BackgroundHost, BackgroundPulse}`
- Service traits or typed events: `BackgroundHost::layer(&self, cx, scope) -> Option<AnyElement>`;
  `BackgroundHost::pulse(&self) -> &Entity<BackgroundPulse>` for `cx.observe`-based repaint.
- Registry contributions: None.
- UI surface: unchanged (`crates/workspace/src/views/terminal.rs` terminal
  overlay, `crates/shell/src/app_shell.rs` app-wide overlay).
- Shared UI-kit components: unchanged.

## Dependencies

- Existing edges removed: `labonair-workspace → labonair-background`.
- New edges: `labonair-workspace → labonair-background-host`,
  `labonair-background → labonair-background-host` (new leaf crate, only
  `gpui`). `labonair-shell` keeps its existing `labonair-background` edge —
  the composition root is allowed to construct the concrete store.
- Dependency verifier change: `scripts/check_crate_deps.py` — new
  `labonair-background-host` entry (leaf); `labonair-workspace` and
  `labonair-background` allow-list entries updated.

## Persistence and migration

- Settings: none; `BackgroundSettings` persistence is unchanged and stays
  entirely inside `labonair-background`.
- Storage: none.
- Compatibility: none. No temporary adapter was needed.

## User-visible behavior

- Canonical entry point: unchanged — the terminal background overlay and the
  app-wide window overlay render exactly as before.
- Notifications: unchanged.
- Inline errors/toasts: none; no new user-visible surface.

## Implementation plan

1. Inventory current `Entity<BackgroundStore>` consumers in `labonair-workspace`
   (`workspace.rs::new_terminal_view`, `views/terminal.rs::TerminalView`) →
   verified by source audit (two call sites, both presentation-only).
2. Define `labonair-background-host` (`LayerScope`, `BackgroundPulse`,
   `BackgroundHost`) → verify with a `gpui::test` for host dispatch.
3. `labonair-background` re-exports `LayerScope`/`BackgroundHost`/`BackgroundPulse`
   and adds `host(&Entity<BackgroundStore>, &mut App) -> BackgroundHost`,
   wiring the pulse to `cx.observe` on the concrete store → verified by the
   existing `labonair-background` test suite (unchanged) plus the new host test.
4. Migrate `labonair-workspace` (`Workspace` struct/constructor,
   `TerminalView` struct/constructor/render) to `BackgroundHost` → verify
   `cargo check -p labonair-workspace`.
5. Shell composition (`bootstrap.rs`) builds `labonair_background::host(&background, cx)`
   once and injects it into `Workspace::new` → verify `cargo check -p labonair-shell`.
6. Remove the `labonair-workspace → labonair-background` Cargo edge and update
   the dependency verifier, architecture inventory, remaining-boundary
   backlog (B02 → Done), capability matrix, and roadmap → verify
   `scripts/check-crate-deps.sh`.
7. Run the full test/lint/format suite → verify all gates pass.

## Acceptance criteria

- [x] `labonair-workspace` has no direct dependency on `labonair-background`
      or `BackgroundStore`; `Workspace` and `TerminalView` hold a
      `BackgroundHost` instead.
- [x] The presentation contract (`labonair-background-host`) is UI-rendering-only
      and carries no image storage, import, delete, or persistence behavior.
- [x] `labonair-background` continues to own image storage, import/delete,
      persistence, decoding, and rendering policy (`BackgroundStore::layer`
      is unchanged; only its type/export location moved).
- [x] Repaint-on-change is preserved: `Workspace` and `TerminalView` observe
      `BackgroundHost::pulse()`, which `labonair_background::host()` bumps on
      every `BackgroundStore` notification.
- [x] Existing commands, persistence, notifications, and user behavior remain
      compatible; no command IDs or settings changed.
- [x] Shared controls continue to come from `labonair-ui-kit` (unaffected).
- [x] Focused tests pass (`labonair-background-host` adds a `gpui::test`).
- [x] `cargo fmt --all -- --check` passes.
- [x] `cargo check --workspace --all-targets` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] `cargo test --workspace --no-fail-fast` passes.
- [x] `scripts/check-crate-deps.sh` and `git diff --check` pass (54 crates,
      222 edges, acyclic).
- [ ] The Workspace/Terminal background-layer visual states are recorded with
      native evidence. Deferred into the R07-001 visual matrix.

## Removal condition

This task is complete only when the direct `workspace → background` edge is
absent from source and metadata, `BackgroundHost` is the sole integration
path for presentation, and the focused plus full verification and visual
checks pass. Only the native visual-state recording remains outstanding.

## Notes and follow-ups

This task was sequenced to start only after R07-004 (B01). Its structural
migration was implemented and verified in the same session, immediately
after R07-004, on explicit direction to prioritise the boundary backlog over
the visual-matrix capture. Remaining before this task is marked `Done`:
record the native background-layer visual states, the same capture effort
blocked in R07-001. Backlog items B03–B06 are review points, not automatic
extraction mandates, and are re-evaluated separately (see
[`../../docs/audits/remaining-boundaries.md`](../../docs/audits/remaining-boundaries.md)).
