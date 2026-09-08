# R08-009 — Remove the unused notification callback-action adapter

## Status

`✅ Done`

## Owner

- Module: `notifications`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-notifications::init`

## Goal

Persistent notification records and the GPUI adapter contain no closures. The
callback-action compatibility path is gone because nothing produced one.

## Scope

- In scope: `crates/notifications/src/notifications.rs` (delete
  `ActionCallback`, `NotificationAction`, `Notification.action`,
  `NotificationCenter.actions`, the `if let Some(action)` push branch,
  `trigger_action`, `NotificationSnapshot.action_label`),
  `crates/notifications/src/status_item.rs` (delete the trailing action
  button), `docs/registries.md`.
- Out of scope: `labonair-notifications-core`'s data-only `NotificationAction`
  (id + label), which is the correct model and is retained; the notification
  history / details / read-state / dedup behaviour (untouched).

## Contracts and ownership

- Public domain values: `Notification` builder (`message` / `info` /
  `success` / `warning` / `error` / `source` / `details` / `dedupe_key`) —
  the `.action(...)` builder is removed.
- Service traits or typed events: none.
- Registry contributions: none.
- UI surface: the statusbar dropdown (unchanged apart from the never-rendered
  action button).
- Shared UI-kit components: `ListItem` (unchanged).

## Dependencies

- Existing edges removed: none.
- New edges: none.
- Dependency verifier change: none.

## Persistence and migration

- Settings: None.
- Storage: None.
- Compatibility: None — the removed API had no callers, so no behaviour
  changes. `labonair-notifications-core` records were already closure-free.

## User-visible behavior

- None — no notification ever attached an action, so no action button ever
  rendered.

## Implementation plan

1. Delete the callback types and fields; simplify `insert` / `dismiss` /
   `clear_all` / `snapshots` → verify: `cargo test -p labonair-notifications`.
2. Delete the trailing action-button block in `status_item.rs` → verify:
   `cargo clippy -p labonair-notifications -- -D warnings`.
3. Update `docs/registries.md` → verify: `python3 scripts/check_documentation.py`.

## Acceptance criteria

- [x] Persistent notification records contain no closures (true before; now
      also true of the GPUI adapter — the `HashMap<u64, NotificationAction>`
      of closures is gone).
- [x] The callback adapter is removed rather than left as an unbounded
      compatibility path; there is a named follow-up (R08-010) for when an
      actionable notification is actually needed.
- [x] Notification history, details, read state, and deduplication stay
      intact (`labonair-notifications-core` unchanged).
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`,
      `python3 scripts/check_documentation.py`, and `git diff --check` pass.

## Notes and follow-ups

Next task: R08-010 (actionable notifications with stable ids — Planned, no
current consumer).
