# R08-004 — Anchor trigger-opened popovers to their trigger bounds

## Status

`✅ Done`

## Owner

- Modules: `shell` (titlebar), `notifications`, `workspace` (status items),
  `settings-ui`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: n/a — per-view rendering

## Goal

Every trigger-opened popover (as opposed to a right-click context menu) drops
from its trigger's rendered bounds, satisfying the normative rule in
`docs/design-system.md` ("A dropdown or popover must be positioned from the
triggering element's bounds in the same window coordinate space").

## Scope

- In scope: the titlebar global `⋯` menu (`shell/src/titlebar.rs`), the
  Notifications statusbar dropdown (`notifications/src/status_item.rs`), the
  Agent Access statusbar dropdown (`workspace/src/status_items.rs`), and the
  Settings `Select` / `FontFamily` field dropdowns
  (`settings-ui/src/{view.rs,panes/generic.rs}`).
- Out of scope: right-click context menus (they correctly open at the pointer);
  the tab / new-tab menus (already fixed in the preceding worktree commit); the
  `ui-kit` `context_menu` / `popover` / `select_popover` primitives (already
  bounds-capable — only the callers passed click coordinates).

## Contracts and ownership

- Public domain values: none.
- Service traits or typed events: none.
- Registry contributions: unchanged.
- UI surface: the four views above.
- Shared UI-kit components: `popover_menu`, `popover`, `select_popover`
  (unchanged; they already take a window-space anchor point).

## Dependencies

- Existing edges removed: none.
- New edges: none.
- Dependency verifier change: none.

## Persistence and migration

- Settings: None.
- Storage: None.
- Compatibility: None.

## User-visible behavior

- Canonical entry point: unchanged.
- Notifications: unchanged.
- Inline errors/toasts: none.

## Implementation plan

1. Each view records its trigger's window-space `Bounds` from the last paint
   via an absolute `canvas` probe (the pattern already used by
   `workspace/src/views/editor.rs`) → verify: `cargo check`.
2. The open handler anchors the popover at `bounds.bottom_left()`, falling
   back to the click position only before the first paint → verify:
   `titlebar::tests::global_menu_anchor_*`.
3. Settings keys the recorded bounds by `json_path` in a `HashMap` since a
   page has many select fields → verify: `cargo test -p labonair-settings-ui`.

## Acceptance criteria

- [x] The titlebar `⋯` menu anchors to the trigger's bottom-left, not the
      click x + a hard-coded `HEADER_H`.
- [x] Notifications and Agent Access statusbar dropdowns anchor to their
      trigger bounds.
- [x] Settings `Select` / `FontFamily` dropdowns anchor to the trigger bounds.
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`, and
      `git diff --check` pass.
- [ ] Right-edge / narrow-window collision behavior is confirmed with native
      visual evidence. *Deferred into the R07-001 visual matrix.*

## Notes and follow-ups

`anchored().snap_to_window()` in the shared primitives already flips a popover
back into the viewport near an edge; this task only fixes the anchor point fed
to it. Next task: R08-005 (P1.5, move Theme management policy out of
Settings UI).
