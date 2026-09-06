# R01-008 — Make Notifications own its statusbar surface

## Status

`✅ Done`

## Owner

- Module: `notifications`
- Canonical crates: `labonair-notifications-core`, `labonair-notifications`
- Related matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)

## Goal

Keep the notification registry and its only global presentation surface in the
Notifications capability. The shell may compose and register the statusbar
item, but must not own notification-specific rendering or lifecycle behavior.

## Scope

- Move the statusbar notification item out of `shell/src/status_items.rs`.
- Keep the statusbar dropdown scrollable, expandable, and backed by retained
  registry records.
- Render the existing notification action affordance from the stable record ID
  and preserve clear/read/dismiss behavior.
- Add only the capability dependencies required by that UI surface.
- Do not migrate every producer's inline error state in this task; that is the
  next notification-surface adoption task.

## Acceptance criteria

- [x] The Notifications capability owns the statusbar item and dropdown.
- [x] Shell only constructs and registers the capability-owned item.
- [x] Notifications retain titles, summaries, optional details, actions, read
  state, and scrolling behavior.
- [x] No toast renderer or second global notification surface is introduced.
- [x] Capability matrix, inventory, and dependency verifier describe the new
  ownership.
- [x] Full workspace verification gates pass.

## Removal condition

The temporary callback-backed notification action adapter can be removed after
all producers publish stable command IDs and the command registry owns action
dispatch.

## Outcome

`NotificationsStatusItem` now lives in `labonair-notifications`. The shell
registers the public item with the statusbar registry but no longer contains
notification-specific UI code. The dropdown also exposes retained action
labels and dispatches them through the existing adapter while stable command
IDs are migrated.
