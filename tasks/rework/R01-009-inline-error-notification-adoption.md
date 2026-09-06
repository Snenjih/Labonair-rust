# R01-009 — Route inline user-facing errors through Notifications

## Status

`⬜ Todo`

## Owner

- Module: `notifications` plus the affected feature owners
- Canonical crates: `labonair-notifications`, `labonair-notifications-core`
- Related matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)

## Goal

Complete the Notifications product contract: user-facing failures are retained
in the statusbar notification dropdown instead of being rendered as feature-
local error banners, rows, or form messages.

## Scope

- Inventory every remaining feature-local user-facing error surface before
  editing it.
- Route errors from Explorer, SFTP, Preview, Git Graph, SCM, Hosts, and other
  active views through the notification registry with useful titles, details,
  source IDs, and deduplication keys.
- Keep internal error state only when it is required for retry or control flow;
  do not render duplicate inline error text.
- Preserve genuine task controls such as transfer conflict resolution dialogs.
- Add regression coverage for notification publication and the absence of
  feature-local error rendering.

## Dependencies

- `R01-008-notification-surface-owner.md` — Done

## Acceptance criteria

- [ ] No active product view renders a user-facing error banner, error row, or
  inline error message for an operation that is already represented by a
  notification.
- [ ] Notification records retain a meaningful title and expandable details
  for migrated failures.
- [ ] Repeated watcher/retry failures are deduplicated by source and operation.
- [ ] Internal retry state and actionable task dialogs remain functional.
- [ ] Inventory, capability matrix, and roadmap reflect the completed Phase 3
  exit condition.
- [ ] Full workspace verification gates pass.

## Removal condition

The task is complete only when the inventory has no remaining active
user-facing inline error surface outside an explicitly documented actionable
dialog or field-validation affordance.

## Outcome

Pending.
