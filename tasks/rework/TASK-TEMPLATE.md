# Rework Task Template

Copy this file for every new task under `tasks/rework/` and replace the
bracketed values. Task files are written in English so they remain usable as
machine-checkable project documentation.

## Status

`⬜ Todo`

## Owner

- Module: `[owning module]`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `[shell/application registration function]`

## Goal

[One observable architectural or product outcome.]

## Scope

- In scope: `[explicit files/modules]`
- Out of scope: `[nearby work intentionally deferred]`

## Contracts and ownership

- Public domain values: `[crate/path]`
- Service traits or typed events: `[crate/path]`
- Registry contributions: `[registry and stable IDs, or None]`
- UI surface: `[crate/path]`
- Shared UI-kit components: `[components to reuse or add]`

## Dependencies

- Existing edges removed: `[edges]`
- New edges: `[edges and why they are allowed]`
- Dependency verifier change: `[script entry or None]`

## Persistence and migration

- Settings: `[keys/defaults/migration or None]`
- Storage: `[schema/query/data migration or None]`
- Compatibility: `[temporary adapter and explicit removal condition, or None]`

## User-visible behavior

- Canonical entry point: `[surface and action]`
- Notifications: `[kind/title/details/action behavior]`
- Inline errors/toasts: `[must be None for new user-visible messages]`

## Implementation plan

1. `[contract/model]` → verify: `[focused test]`
2. `[owner implementation]` → verify: `[focused test or UI check]`
3. `[composition and migration]` → verify: `[dependency or integration check]`

## Acceptance criteria

- [ ] Ownership and public contract are explicit.
- [ ] No unrelated module imports private implementation state.
- [ ] Registry and command/keymap contributions use stable IDs.
- [ ] Shared controls use `labonair-ui-kit`.
- [ ] Notifications use `labonair-notifications-core`; no new toast or inline
      feature error surface exists.
- [ ] Focused tests pass.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace --all-targets` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `scripts/check-crate-deps.sh` and `git diff --check` pass.

## Notes and follow-ups

[Non-obvious API constraints, deferred migrations, and the next task ID.]
