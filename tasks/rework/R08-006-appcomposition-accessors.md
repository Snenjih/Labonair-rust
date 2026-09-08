# R08-006 — Remove the AppComposition blanket Deref; capability accessors only

## Status

`✅ Done`

## Owner

- Module: `shell` (application composition)
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::composition` / `::bootstrap`

## Goal

`AppComposition` is a construction bundle, not a service facade. No caller can
treat it as a general capability bag; each capability is reached through an
explicit accessor, and the SSH transport adapters carry capability-specific
names.

## Scope

- In scope: `crates/shell/src/composition.rs`, `crates/shell/src/bootstrap.rs`,
  `crates/app/src/main.rs`, `crates/ssh-transport/src/contract.rs`, and a
  handful of stale doc comments referencing the deleted backend crate.
- Out of scope: the composition→integration-sibling construction edge (B07,
  retain by design); reworking how `bootstrap` builds each entity.

## Contracts and ownership

- Public domain values: none.
- Service traits or typed events: `AppComposition` now exposes `events()`,
  `db()`, `secrets()`, `ssh()`, `trust()`, `tunnels()`, `snippet_run()`,
  `mcp()`, `transfer()` — each returning `&T`. `AppCompositionInner` and its
  fields are private; the `impl Deref` is gone.
- Registry contributions: none.
- UI surface: none.
- Shared UI-kit components: none.

## Dependencies

- Existing edges removed: none.
- New edges: none.
- Dependency verifier change: none.

## Persistence and migration

- Settings: None.
- Storage: None.
- Compatibility: None — internal composition refactor only.

## User-visible behavior

- None.

## Implementation plan

1. Make `AppCompositionInner` private, drop `impl Deref`, add one accessor per
   field; fix the two internal `self.<field>` uses → verify: `cargo check`.
2. Rewrite the ~46 `backend.<field>` sites in `bootstrap` to
   `composition.<field>()`; rename the `backend` param/local to `composition`
   in `bootstrap` and `main.rs` → verify: `cargo check --workspace`.
3. Rename `BackendSsh*` → `Ssh*Adapter` across `ssh-transport/contract.rs` +
   `bootstrap`; fix stale "backend facade / crates/backend/…" doc comments →
   verify: `cargo test --workspace`, `cargo clippy -D warnings`.

## Acceptance criteria

- [x] No feature constructor accepts `AppComposition` as a general service
      (it is confined to `labonair-shell` / `labonair` and has no `Deref`).
- [x] Composition exposes explicit capability accessors only.
- [x] No broad backend facade or backend-shaped feature owner reappears; the
      SSH transport adapters are `Ssh*Adapter`, not `BackendSsh*`.
- [x] Active comments describe the native architecture (the
      `crates/backend/src/modules/ssh/client.rs` path reference and the
      "aggregate backend App" / "legacy backend event stream" comments are
      corrected).
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`, and
      `git diff --check` pass.

## Notes and follow-ups

Remaining loose "backend" comments (`backend SSH session id`, `backend
worker`, `the SSH backend`) describe the remote/transport side, not the
deleted crate, and are left as accurate native terminology. Next task:
R08-007 (P1.6, delete the inert `ShortcutId` / `SHORTCUTS` keymap model).
