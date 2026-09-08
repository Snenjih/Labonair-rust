# R08-010 — Actionable notifications with stable action ids

## Status

`⏳ Planned`

## Owner

- Module: `notifications`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell` command/palette action registry

## Goal

When a feature needs a notification the user can act on (for example "Open
logs", "Retry", "Reveal file"), the action is a stable id + typed payload
dispatched through the owning module or the command registry — never a stored
closure.

## Activation condition

This task is `Planned` and has **no current consumer**. Activate it only when
a concrete feature workflow needs an actionable notification. Until then,
`labonair-notifications-core` keeps its data-only `NotificationAction`
(id + label) and the GPUI adapter stores no callbacks (R08-009).

## Scope (when activated)

- `labonair-notifications-core`: keep the data-only action record; add a
  stable `action_id` (string or typed enum) alongside the label.
- The statusbar dropdown: render the action and emit the `action_id` +
  notification id as a typed event.
- Composition: route the event to the owning module's command/palette action
  handler (the same `PaletteActionHandlerRegistry` / `CommandHandlerRegistry`
  pattern the palette already uses), not to notification-held state.

## Acceptance criteria (when activated)

- [ ] Persistent notification records contain no closures.
- [ ] Every actionable notification carries a stable `action_id`; the dropdown
      dispatches it through the owning module / command registry.
- [ ] Notification history, details, read state, dedup, and the existing
      non-actionable path stay intact.
- [ ] Focused tests cover dispatch and the round-trip from publish to action.
- [ ] Full verification suite passes.

## Notes and follow-ups

Prior context: R08-009 removed the never-used callback adapter.
