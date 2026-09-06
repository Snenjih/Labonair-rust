# R01-003 — Make transfers a first-class capability registry

## Status

`✅ Done`

## Owner

- Module: `transfers`
- Capability matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Canonical surface: statusbar Transfers item and dropdown

## Goal

Replace the workspace/backend transfer split with a retained, typed transfer
registry that owns transfer identity, lifecycle, progress, cancellation,
conflict state, and history. Retry remains a separate capability addition once
the worker has a typed retry contract; this task must not invent a second
execution path for it.

## Scope

- In scope: transfer domain values, registry, typed events, backend worker
  adapter, statusbar UI, and migration from existing transfer events.
- Out of scope: new download marketplaces or unrelated notification styling.

## Contracts and ownership

- Core registry must be UI-free and expose immutable snapshots.
- Worker implementations are injected adapters for SFTP/SSH capabilities.
- Statusbar is the canonical presentation; no toast or inline duplicate
  transfer status is allowed.
- Transfer actions use stable command IDs and UI-kit controls.

## Persistence and migration

- Preserve active transfer behavior and existing user data.
- Define whether history is retained in memory or persisted before adding a
  schema; do not silently discard existing records.
- Keep an explicit compatibility adapter for legacy `AppEvent` payloads until
  all producers publish typed transfer events.

## Acceptance criteria

- [x] One registry owns all transfer lifecycle state.
- [x] Statusbar shows active progress, failures, cancellation, and history.
- [x] Workspace no longer owns transfer domain state.
- [x] Producers do not render transfer-specific toasts or inline errors.
- [x] Focused registry/event tests and full task verification gates pass.
