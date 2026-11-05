# Workspace Collaborator Follow Action Classification

**Status:** Static active-pane, target cycling, and disconnect intent; no
Labonair collaboration runtime, native dispatch, or paired visual acceptance

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the action and pane follow
projection. The planned Collaboration capability must own participant
identity, call membership, and follow transport; it has no canonical Labonair
crate yet.

## Evidence boundary

This packet records two Workspace actions: cycle the collaborator followed by
the active center pane, and stop following that pane's current leader. It
separates following a participant from joining/leaving a call or sharing a
project. Labonair currently has no collaboration runtime, so this packet maps
the owner gap without claiming a local user route or behavior.

The requirements are independently stated for Labonair's Workspace and future
Collaboration contracts. This is same-session independent redevelopment, not
separated-room clean-room work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux
host without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Workspace (`labonair-workspace`) owns Follow Next, Unfollow, and the pane's local follow projection. Planned Collaboration owns participants, call state, and follow/unfollow transport. Define a narrow typed contract between them before implementation; no canonical Collaboration crate currently exists. |
| Follow Next entry points | Pinned global binding: macOS `Ctrl+Alt+Cmd+F`, Linux `Ctrl+Alt+Shift+F`, Windows `Ctrl+Shift+Alt+F`. When Vim/Helix mode is active, `[` then `f` and `]` then `f` also invoke the action in normal, visual, and operator modes. Labonair has no collaboration action or default binding. |
| Unfollow entry point | Pinned `Escape` in Workspace context on macOS, Linux, and Windows. More specific modal, search, Editor, and panel Escape handlers may take precedence; verify the effective context. Labonair's Escape routes do not stop following because no follow state exists. |
| Target pane | Both actions use the active center pane to identify follow state. Moving focus into a dock does not by itself select a different center pane. Verify this with multiple center panes and a focused dock. |
| Follow Next when following | Move to the next currently available project collaborator, wrapping to the first when needed. Record the order users see and the sequence followed; the action contract does not promise alphabetical ordering. Starting a follow synchronizes the active pane to the participant's active view, with cursor/selection/scroll behavior governed by the collaboration view contract. |
| Follow Next when not following | Restore that pane's previous leader if the participant is still available; otherwise select the first available participant. If there is no candidate, leave follow state unchanged. Treat Agent as a separate candidate case and verify its ordering and availability. |
| One-collaborator edge case | Static intent appears to resolve the next candidate to the current participant when only one is available, then stop that follow instead of restarting it. Verify the pinned runtime outcome and record whether its user-facing effect is to unfollow. |
| Unfollow | Stop the follow session associated with the active center pane. If not following, do nothing. Keep the call and project sharing active. If that session also backs another follower surface, return that surface to local state as well. |
| Other state changes | A collaborator leaving clears their follow state. Local editing can independently leave follow mode; distinguish this from a failed Follow Next or Escape action. Project switch, pane removal, call leave, and connection loss must not leave stale participant UI. |
| Settings and persistence | No action-specific setting. Current/previous follow targets are transient interaction state, not Editor content or durable project preferences. Verify behavior after disconnect, reconnect, project switch, pane close, and Workspace restart. Account, call, and project-sharing settings remain Collaboration-owned. |
| Events and registry contributions | Workspace sends typed follow intent for the active pane; Collaboration resolves participant availability and executes transport. Keep membership and credentials out of shell-wide conditionals and Workspace private state. |
| Failure and recovery | With no available participant, Follow Next is a no-op. Pinned asynchronous follow and network Unfollow errors are logged by the action path; determine whether call/presence UI informs the user and how stale follow state is retried or cleared. Do not show a participant as followed after a rejected or interrupted request. |
| UI-kit | Reuse shared participant, active-view, cursor, and status surfaces from their owners. Do not add a local presence badge or duplicate follow control in this inventory slice. |

## Action requirements and current route

| Requirement ID | Pinned bindings and contexts | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-FollowNextCollaborator`](action-requirements.md) | macOS `Ctrl+Alt+Cmd+F`, Linux `Ctrl+Alt+Shift+F`, Windows `Ctrl+Shift+Alt+F`, global. Vim/Helix overlay: `[ f` and `] f`, normal/visual/operator modes. | Cycle the active center pane's participant follow target, restoring its previous target or choosing the first available participant when not following. | No collaboration runtime, participant list, pane follow state, command route, or default action binding. |
| [`ACT-MACOS-workspace-Unfollow`](action-requirements.md) | `Escape` in Workspace context on macOS, Linux, and Windows. | Stop the follow session associated with the active center pane; no-op if not following. Do not leave the call or unshare the project. | No collaboration runtime, follow state, or default Unfollow route. Other Escape handlers are unrelated. |

The Vim/Helix keys are separately inventoried as
[`KEYBIND-VIM-HELIX-3203a0243935`](keymap-overlay-requirements.md) and
[`KEYBIND-VIM-HELIX-63e23c14e4e0`](keymap-overlay-requirements.md). They invoke
the Follow Next action only when the mode-layer context is active; they do not
make collaboration available when no call or participant exists.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`Workspace` capability record](../capabilities.md) | Defines `labonair-workspace` as the owner of Workspace state and entry points. | Collaboration remains a planned capability; no canonical runtime crate or typed follow contract exists. |
| [`collaboration` feature crosswalk](feature-crosswalk.md) | Records collaboration as missing and assigns its account, participant, and offline contracts to the planned Collaboration owner. | No participant list, call state, follow transport, cursor/view synchronization, or leave/recovery route is implemented. |
| [`Workspace::active_pane` and Workspace action dispatch](../../crates/workspace/src/workspace.rs) | Provide the current Workspace pane and command/action composition model. | No collaborator identity or following state is present in Labonair Workspace. |
| [`CommandId`](../../crates/command-palette-core/src/lib.rs) and [`WorkspaceCommandProvider`](../../crates/workspace/src/command_provider.rs) | Define current typed command discovery and Workspace contributions. | Neither follow action has a Labonair descriptor, handler, menu route, or keymap binding. |

The action packet identifies an unresolved owner boundary rather than assigning
network state to Workspace. R07-001 must define the typed Collaboration
contract and canonical owner before implementation work for these workflows.

## Required user-flow and state acceptance

- With several active participants, follow each in the presented order and
  verify wrapping, participant removal, and a participant joining during
  cycling. Record the exact order rather than assuming a sort order.
- Start from no follow state. Verify restoration of a still-available previous
  leader, fallback when it has left, and a no-op when no participant is
  available.
- Test one participant while following it, an Agent target, disconnected and
  reconnecting participants, and a follow request rejected or delayed by the
  network. Verify local and remote follow indicators recover consistently.
- Follow from one of several center panes, focus a dock, then invoke Follow
  Next and Unfollow. Verify which pane is affected and whether a linked follower
  surface returns to local state.
- Press Escape when following, when not following, and while a more specific
  modal/search/Editor/panel Escape handler is active. Confirm precedence and
  focus behavior without dismissing unrelated UI.
- Edit locally while following and verify the automatic leave-follow behavior
  is distinct from explicit Unfollow and does not create a stale leader view.
- Leave the call, switch projects, remove the pane, lose the connection, and
  restart the Workspace. Verify participant state, previous target, and remote
  subscription cleanup for each transition.
- Verify default macOS/Linux/Windows bindings, the Vim/Helix overlay layer,
  profile selection, user overrides, shadowing, and conflicts.
- Compare paired native macOS captures for participant identity, active view,
  cursor/selection, focus, Escape precedence, one/no-participant states, and
  disconnect recovery in single- and multi-pane layouts.

## Acceptance evidence still required

For each trace, record build and commit, platform, binding and context, active
Workspace/pane, participant list and visible order, prior/current leader,
request result and latency, active-view/cursor state, local edit or disconnect
events, notifications, stale-state cleanup, and restart outcome. Include
paired macOS captures with window bounds, theme, scale, call fixture, and
participant fixture. Static source inspection does not establish native
dispatch, actual ordering, network recovery, or visual parity.
