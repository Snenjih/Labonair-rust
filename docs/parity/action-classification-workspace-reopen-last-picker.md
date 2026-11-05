# Workspace Reopen Last Picker Action Classification

**Status:** Static modal eligibility, stash scope, state/focus intent, and
current Labonair route; native dispatch and paired visual acceptance remain
pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the action and modal
lifecycle. Each eligible picker owner owns its query, selection, and other
view state. The shell remains a composition host.

## Evidence boundary

This packet defines the user-visible behavior of reopening the most recently
dismissed eligible picker in the current Workspace window. Eligibility is
independent of the particular picker workflow: generic pinned Picker views
are eligible by default, while the one-shot Command Palette opts out. Static
inspection does not establish native keyboard dispatch or macOS focus and
visual behavior.

The requirements are independently stated for Labonair's Workspace and modal
contracts. This is same-session independent redevelopment, not separated-room
clean-room work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux
host without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) owns `ReopenLastPicker`, eligibility coordination, and the one-entry per-window modal stash. Picker owners retain their own transient view state. |
| Canonical user entry points | Pinned macOS `Cmd+K Cmd+P`, Linux `Ctrl+K Ctrl+P`, and Windows `Ctrl+K Ctrl+P`, all in Workspace context. Labonair has no registered action or published default binding for this behavior. |
| Eligible picker | Reopen the most recently dismissed modal picker marked eligible for reopening. Generic pinned Picker views are eligible by default. A one-shot Command Palette is explicitly ineligible. Picker-specific opt-outs remain possible. |
| Restored state | Reveal the same picker view with its prior query, selected row, preview, and other transient state intact. Transfer keyboard focus into the restored picker so its normal key navigation resumes. Do not construct a fresh picker or reset its query. |
| Stash replacement and retention | Keep at most one dismissed eligible picker per Workspace window. A newly dismissed eligible picker replaces the previous one. Dismissing an ineligible modal leaves the previous eligible picker available. Revealing consumes the stash; dismissing that picker later can make it the latest eligible picker again. |
| Empty and active-modal behavior | With no eligible picker stashed, the action is a silent no-op. If another modal is active when the action fires, reveal the stashed picker after the active modal closes. An asynchronous Command Palette dismissal must not race away the reveal. |
| Scope and lifetime | Stash is local to the current Workspace window and exists only in memory. It is not shared between windows or restored after closing/restarting the app. Normal dismissal restores focus according to the modal lifecycle. |
| Settings and persistence | No setting, database value, or session persistence is introduced by this action. |
| Events and registry contributions | Workspace owns the action. Picker owners identify whether their modal may be reopened; the palette opts out. Add no second shell-wide action table or per-picker duplicate reopen command. |
| Failure and cancellation | No eligible stash leaves the visible UI unchanged. A picker with a blocked or pending dismissal must preserve its own decision; verify that a queued reveal neither bypasses that decision nor exposes stale content. |
| UI-kit | Reveal the existing picker and reuse its shared input/list/preview controls and modal focus behavior. Do not create a local replacement picker or duplicate control styling. |

## Action requirement and current route

| Requirement ID | Pinned bindings | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-ReopenLastPicker`](action-requirements.md) | macOS `Cmd+K Cmd+P`; Linux and Windows `Ctrl+K Ctrl+P`; `Workspace` context | Reopen the most recently dismissed eligible picker in the current window, preserving its view state and restoring focus. | No Reopen Last Picker action or default binding is registered. Workspace's modal host keeps only an active modal and drops it on dismissal; opening File Finder creates a new view. |

The action is distinct from reopening a file, project, command palette, or
picker search result. It reveals a previously dismissed eligible modal. It
does not reopen the selected file or execute the selected row; those happen
through the picker owner's existing confirmation flow.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`ModalLayer`](../../crates/workspace/src/modal_layer.rs) | Owns the shared active-modal slot, modal focus, and dismissal lifecycle. | It drops dismissed modal views and has no last-picker eligibility or stash state. |
| [`AppShell::toggle_file_finder`](../../crates/shell/src/actions.rs) and [`FileFinderView`](../../crates/workspace/src/file_finder.rs) | The existing `OpenFile` command callback shows the Workspace File Finder in the shared modal host. | A fresh FileFinderView is built when opened; query and selection are not restored after dismissal. No Reopen Last Picker route exists. |
| [`CommandPalette` modal route](../../crates/shell/src/actions.rs) | The palette uses the same modal host and selects an action through its existing command flow. | It does not expose a previous-picker action and does not preserve an earlier modal. |
| [`WorkspaceCommandProvider`](../../crates/workspace/src/command_provider.rs) and [`builtin command registration`](../../crates/shell/src/commands.rs) | Provide typed Workspace command descriptors and composition callbacks. | There is no descriptor, handler, menu item, or keymap action for ReopenLastPicker. |

The current Workspace File Finder, Search Overlay, and Command Palette have
different roles. Confirm which additional Labonair modal picker owners should
contribute to the shared reopenable behavior when implementation is planned;
do not infer that every dialog or overlay is a picker.

## Required user-flow and state acceptance

- Open an eligible picker, enter a query, move selection, and dismiss it.
  Invoke Reopen Last Picker and verify the same content, query, selected row,
  preview, and keyboard focus return.
- Dismiss one eligible picker, then a different eligible picker. Verify only
  the latest one reopens. Dismiss a non-picker modal between those steps and
  verify it does not erase the saved picker.
- Invoke with no eligible picker available. Verify there is no newly created
  empty modal and normal focus remains unchanged.
- Invoke from the Command Palette while a picker is stashed. Verify palette
  dismissal and the reopen action can occur in either event order without
  losing the picker or leaving two active modals.
- Invoke while an unrelated modal is active and verify the picker appears
  after the active modal closes. Exercise a modal whose dismissal is pending
  or declined and confirm its cancellation decision is respected.
- Verify separate Workspace windows do not exchange picker state. Close the
  window and confirm the in-memory stash does not reappear in a later session.
- Verify the macOS, Linux, and Windows Workspace bindings, keymap overrides,
  chord continuation/cancellation, and conflict behavior.
- Compare paired native macOS captures for the dismissed picker, restored
  query and selection, focus ring, active-modal transition, empty stash, and
  narrow window behavior.

## Acceptance evidence still required

For each trace, record build and commit, platform, binding and active context,
window identity, picker eligibility, query and selected row before dismissal,
intervening modal actions, reopened view identity and state, focus target,
empty/blocked outcomes, and any errors. Include paired macOS captures with
window bounds, theme, scale, and fixture picker recorded. Static modal code
inspection does not establish native dispatch or user-visible focus parity.
