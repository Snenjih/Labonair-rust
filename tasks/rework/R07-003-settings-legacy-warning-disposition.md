# R07-003 — Resolve legacy Settings warning-path policy

## Status

`⏳ Planned`

## Owner

- Module: Settings persistence and migration
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-settings::SettingsStore::load_user`

## Goal

Make the treatment of removed or already-migrated Settings keys explicit and
quietly compatible. A normal launch must not repeatedly report known legacy
input as an accidental unknown-key defect, while genuinely unsupported future
keys must remain visible as non-fatal warnings.

## Scope

- In scope: the Settings migration output, schema-warning classification,
  startup warning emission, tests, and the related migration documentation.
- Out of scope: adding removed Settings values back to the typed model,
  changing the Settings UI, or deleting user configuration data.

## Contracts and ownership

- The Settings crate owns the distinction between migration-only input,
  intentionally ignored legacy keys, and unknown future keys.
- Migration-only keys may be consumed or recorded in the migration envelope;
  they must not be presented as active Settings fields.
- Unknown future keys remain non-fatal and diagnosable through the existing
  Settings validation contract.
- No other module may special-case Settings key names.

## Dependencies

- Existing edges removed: none expected.
- New edges: none expected.
- Dependency verifier change: none expected.

## Persistence and migration

- Settings: preserve the current v1/v2 migration behavior and idempotence.
- Storage: do not rewrite or delete user data solely to silence a warning.
- Compatibility: document every intentionally consumed or retained legacy key
  and define when its compatibility handling can be removed.

## User-visible behavior

- Canonical entry point: Settings load/validation diagnostics and the Settings
  JSON editor's warning model.
- Notifications: no new notification is required for a known migration-only
  key; genuine malformed values keep the existing Settings diagnostic path.
- Inline errors/toasts: do not add a toast or feature-local error surface.

## Implementation plan

1. Inventory the keys emitted by current real-world legacy config fixtures →
   verify with migration and schema-warning tests.
2. Define and implement the classification boundary in the Settings owner →
   verify known legacy keys are quiet while unknown future keys warn.
3. Document the compatibility envelope and removal condition → verify the
   settings audit and full repository gates.

## Acceptance criteria

- [ ] Known removed/migration-only keys have an explicit disposition.
- [ ] Unknown future keys remain non-fatal warnings.
- [ ] Migration remains idempotent and does not delete user data.
- [ ] No unrelated module imports Settings implementation details.
- [ ] Focused migration and schema-warning tests pass.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace --all-targets` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `scripts/check-crate-deps.sh` and `git diff --check` pass.

## Notes and follow-ups

The R07-001 native launch on 2026-09-07 reported removed keys such as
`hosts`, `hostsMigrated`, `statusBarItemPlacements`, and workspace bookmark
values as unknown. This task must first confirm whether those warnings come
from user-file migration input or the active Settings validation path before
choosing the smallest compatible fix.

Initial diagnosis: the observed user file is already marked
`schemaVersion: 2`/`sparsified: true`, and `migrate_settings_v1_to_v2` returns
early for that state. The old root keys therefore survive into
`SettingsStore::reload_user_layer`, where the generic schema walk reports them.
The implementation must address this post-migration v2 compatibility case
explicitly and idempotently.
