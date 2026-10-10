# Git Worktree Deletion Action Classification

**Status:** Static requirements and owner route; Git lifecycle implementation,
native dispatch, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git (`labonair-git`) owns worktree discovery and deletion.
Workspace provides the typed open-worktree view needed to protect live project
roots. `labonair-git-transport` is the Git execution adapter.

## Evidence boundary

This packet independently states user goals and observable behavior. Static
inspection of the pinned action map, Worktree Picker, and Git operation
establishes its route and behavior but does not prove native dispatch or
rendered interaction. Labonair source inspection found no worktree listing,
picker, or removal API. Same-session research and packet authoring are
independent redevelopment, not separated-room clean-room work, as recorded in
the [parity README](README.md#provenance-and-source-boundary). Native macOS
runtime and paired capture access are unavailable in this session.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Git owns worktree listing and lifecycle operations. Workspace supplies which worktree paths are open across related windows so Git can prevent removal of a live project root. Keep the lifecycle operation in `labonair-git`; execute through the configured Git adapter. The current `GitService` has no worktree list/remove operations. |
| Picker entry | The pinned titlebar worktree selector opens a compact picker; the registered Git Worktree action opens its modal form. Both show eligible worktree rows and their trash affordance. The modal also has a footer Delete button; the compact picker omits that footer. In Labonair, retain the existing Git / Source Control and Command Palette entry points; do not add permanent shell chrome without an ADR. |
| Keyboard entry | The actions apply only in `WorktreePicker || (WorktreePicker > Picker > Editor)`. macOS normal delete is `Command+Shift+Backspace`; Linux and Windows use `Ctrl+Shift+Backspace`. Force delete is `Command+Option+Shift+Backspace` on macOS and `Ctrl+Alt+Shift+Backspace` on Linux and Windows. Verify user overrides, profile differences, and actual focus dispatch. |
| Mouse entry | The trash control on an eligible row performs normal deletion. Holding Alt while clicking it switches the action and tooltip to Force Delete, with destructive styling. In modal form, the footer Delete button invokes the normal action. Other row controls open a worktree in another window or remove an already-open worktree from a window; those do not delete it. |
| Eligible target | Only an existing non-main worktree that is not visible in any Workspace belonging to the same project group can be deleted. Main and open worktrees remain listed but have no Delete affordance. A selection that is not a worktree, a stale path, and an already pending deletion are no-ops. |
| Normal removal | Remove the selected worktree and its working directory. A clean worktree is removed without another confirmation. If Git reports modified or untracked files, explain the risk and offer Force Delete or Cancel. Cancel leaves the worktree and its files intact. Do not ask for force after unrelated Git errors. |
| Forced removal | The explicit Force Delete action immediately requests forced removal, without a second warning prompt. It can permanently discard modified and untracked files in the selected worktree. The same action is available from the row trash control while Alt is held. No branch-deletion operation is part of this workflow; verify the branch remains available afterward. |
| Operation state | Git I/O runs asynchronously. Immediately show the selected row as deleting, prevent duplicate requests and opening that row, and show a pending footer state when available. Clear pending state on error, cancellation, or before waiting for the dirty-tree confirmation. After confirmation, recheck that the worktree still exists and that no deletion is already pending before retrying once with force. |
| Success and refresh | Remove the successful worktree from the picker and its local catalog, adjust selection to a surviving row, and refresh from Git's worktree-list change event. A restart must not rediscover the removed worktree. Other windows must observe the repository change without invalidating unrelated workspaces. |
| Failure and recovery | On non-dirty errors, clear pending state, keep the row, and send the attempted operation and Git error through the notification center. The user can retry after correcting the cause. A failed forced retry follows the same error route. If another actor removed the worktree while confirmation was open, do not issue a duplicate operation. |
| Settings and persistence | No application setting controls this workflow. Git's worktree registration and filesystem are changed by successful removal and persist across restart. Forced removal's local-data loss is consequential and must be explicit in the force affordance and dirty-tree prompt. |
| Events and registries | Register both actions in the Git Worktree Picker context and use the Git worktree-list change event to refresh rows. Use a typed Git operation and a typed Workspace open-path snapshot. One configured Git service is sufficient; do not add a generic registry solely for these actions. |
| UI kit | Use shared Picker, ListItem, Button, IconButton, tooltip, loading, warning-dialog, and notification-center components. Keyboard and mouse inputs must call the same typed operation and expose the same normal/force distinction. |

## Action requirements and current Labonair route

| Requirement ID | Binding and context | User-visible outcome | Current Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-worktree_picker-DeleteWorktree`](action-requirements.md) | `Command+Shift+Backspace` on macOS; `Ctrl+Shift+Backspace` on Linux/Windows; WorktreePicker context | Remove the selected eligible worktree. If it contains modified or untracked files, ask whether to force-delete or cancel. | The `GitService` contract has no worktree listing/removal API, and no Git worktree picker or default action route exists. |
| [`ACT-MACOS-worktree_picker-ForceDeleteWorktree`](action-requirements.md) | `Command+Option+Shift+Backspace` on macOS; `Ctrl+Alt+Shift+Backspace` on Linux/Windows; WorktreePicker context | Force-remove the selected eligible worktree directly; its uncommitted and untracked files may be lost. | No Git worktree operation, force confirmation flow, or default action route exists. |

Both actions are limited to the Worktree Picker. The picker is reachable in
the pinned build from the titlebar worktree selector and the registered Git
Worktree action. The row trash button supports Alt-to-force, and the modal
footer exposes normal Delete. Removing another open worktree from a window
only closes that Workspace view; it does not remove the Git worktree.

## Required user-flow and state acceptance

- Open both picker presentations and verify the compact picker and modal
  expose the correct entry controls, focus, and selection.
- Delete a clean linked worktree normally. Verify its directory and Git
  worktree registration disappear while its branch remains available, the
  selection moves to a surviving row, and restart does not restore it.
- Attempt normal deletion with modified and untracked files. Verify the
  warning describes data loss; Cancel preserves the row and files; Force
  Delete removes them. Invoke explicit keyboard and Alt-click force paths and
  verify they do not show a redundant confirmation.
- Confirm the main worktree and worktrees visible in this or another related
  window cannot be deleted. Verify “Remove Worktree from Window” only closes
  the other Workspace and does not delete its directory or registration.
- Exercise empty/non-worktree selection, duplicate pending action, row opening
  during deletion, a stale worktree while the prompt is open, a failed normal
  operation, a failed forced retry, and a successful retry after recovery.
  Confirm pending state clears and the user can retry after each recoverable
  failure.
- Check macOS/Linux/Windows default bindings and user keymap overrides,
  WorktreePicker focus contexts, keyboard-only use, Alt tooltip and destructive
  styling, loading, disabled rows, narrow layout, notification recovery, and
  the absence of an unrelated branch deletion.
- Capture paired native macOS picker, selected/disabled row, Alt force,
  warning, pending, and error states. Record exact builds, OS, window bounds,
  theme, scale, repository state, focus, and selected worktree.

## Acceptance evidence still required

Static source inspection establishes the picker routes, deletion restrictions,
force path, and error/retry structure only. Record actual keyboard dispatch,
prompt choice, data effects, branch state, open-worktree protections,
cross-window refresh, failure recovery, and paired visual results on the pinned
build and Labonair. The Linux host cannot supply native macOS evidence. No
runtime or visual parity claim is made from this packet.
