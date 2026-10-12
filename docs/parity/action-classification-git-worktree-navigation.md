# Git Worktree Navigation and Creation Action Classification

**Status:** Static picker and typed-action requirements; runtime operations,
Workspace restoration, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns worktree discovery and lifecycle operations. Workspace
owns project activation and window hosting. Keymap owns the Workspace
shortcut.

## Evidence boundary

This packet independently states the Worktree Picker, create, switch, and
open-in-new-window workflows and current Labonair routes. Static action
metadata, keymaps, typed fields, and local Git contracts identify targets and
ownership gaps; they do not establish native dispatch, operation success,
state restoration, or visual acceptance. Work in this session is independent
redevelopment, not separated-room clean-room implementation, as recorded in
the [parity README](README.md#provenance-and-source-boundary). Paired macOS
picker and Workspace evidence remains pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Worktree Picker | `git::Worktree` opens the picker from the titlebar worktree trigger or its Workspace shortcut. It lists non-bare worktrees in the active repository, marks worktrees already active in the project, and offers create targets. Labonair has no worktree metadata API, picker, titlebar trigger, or shortcut. |
| Create worktree | `CreateWorktree` accepts an optional worktree name and a CurrentBranch, ExistingBranch, or RemoteBranch target. It creates at the selected ref in detached HEAD and opens/activates a Workspace on the new linked worktree. A name may be generated when omitted; remote targets may require fetching. Creation is unavailable for collaborative projects and when the Project has no Git repository. Labonair exposes only a branch-switch command; it has no worktree creation route. |
| Switch worktree | `SwitchWorktree` takes a linked-worktree path and display name and switches the current Workspace to that worktree. Switching is unsupported in collaborative projects and is guarded while a worktree creation is active. Labonair has no linked-worktree inventory, active-worktree switch, or Workspace state-transfer path. |
| Open in new window | `OpenWorktreeInNewWindow` takes a worktree path. The picker exposes it for a non-current worktree and opens that path in a separate Workspace window; remote projects reuse their remote connection options. Labonair has no worktree-targeted new-window route. |
| Related Worktree Picker actions | Delete/Force Delete and Open Worktree Setup Tasks are classified separately in the [deletion packet](action-classification-git-worktree-delete.md) and [task-definition entry packet](action-classification-task-definition-entry.md). Keep their confirmation, trust, and task-seeding contracts intact when composing the picker. |
| Owner and crates | Git (`labonair-git`) is the existing owner and canonical contract crate, but it currently has no worktree lifecycle API; no worktree UI crate or implementation exists. Workspace remains the host and does not become a second owner. |
| Events and registries | The pinned picker dispatches typed create, switch, and open-window actions. This inventory adds no Labonair event or registry; define the smallest operation contract and consumers before implementation. |
| State and persistence | Worktree membership/ref state persists in Git. Switching must account for the current Workspace's open items, docks, and focus. Creation opens a new Workspace and restores the relevant layout/items. Window opening creates a separate Workspace. There is no worktree-specific Settings value. |
| Notifications and recovery | Repository absence, collaboration restrictions, remote fetch/open failures, path conflicts, concurrent operations, and cancellation require visible recovery. Labonair routes user-visible errors through Notifications. Observe the pinned runtime's exact failure and retry behavior before claiming parity. |
| UI kit | The picker uses shared list, search, buttons, and icon controls; any Labonair controls use `labonair-ui-kit`. |
| Current Labonair route | `labonair-git` has status and branch value contracts, and the palette has `Git: Switch Branch…`. It has no worktree APIs or action handlers; shell/workspace exposes no Worktree Picker, creation, switch, or open-in-new-window route. Branch checkout is not worktree switching. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-git-Worktree`](action-requirements.md) | `cmd-ctrl-w` (`Workspace`) on macOS; `alt-ctrl-shift-w` on Linux; `shift-alt-w` on Windows. Open Worktree Picker from the titlebar trigger or shortcut. | No picker, titlebar trigger, worktree catalog, or default action binding. |
| [`ACT-REGISTRY-LINUX-git-CreateWorktree`](registered-actions-unbound.md) | Typed optional name plus branch target; create detached at current, local, or remote ref, then open/activate the new Workspace. | No Git worktree creation contract or operation. |
| [`ACT-REGISTRY-LINUX-git-SwitchWorktree`](registered-actions-unbound.md) | Typed worktree path/display name; switch the current Workspace to the selected linked worktree. | `Git: Switch Branch…` changes a branch in place and does not switch worktrees. |
| [`ACT-REGISTRY-LINUX-git-OpenWorktreeInNewWindow`](registered-actions-unbound.md) | Typed path; open the existing selected worktree in a separate local or remote Workspace window. | No worktree-aware window-opening route. |

## Required user-flow and state acceptance

- Open the picker by titlebar and shortcut. Verify repository selection,
  worktree grouping/filtering, active-worktree indication, and empty/loading/
  error states.
- Create from the current ref, default ref, an existing local branch, and a
  remote-tracking branch. Verify detached starting state, generated/custom
  name, path collisions, asynchronous progress, cancellation, and that the
  resulting Workspace becomes active with the expected layout and items.
- Select an existing linked worktree to switch. Verify open items, dirty state,
  dock/focus state, branch/repository identity, and recovery after a failed or
  canceled operation.
- Open a non-current worktree in a new window for local and remote projects.
  Verify remote connection reuse, independent window focus, and existing
  workspace/session state.
- Verify create is unavailable for collaborative projects or a project with
  no Git repository. Verify switching is rejected in collaborative projects
  and while creation is active. Capture remote fetch failures, duplicate paths,
  concurrent creation, and menu/picker dismissal behavior.
- Capture paired native macOS titlebar picker, list/search, create targets,
  switch, new-window, disabled/error, and restored-state interactions. Record
  OS, repository identity, worktrees/refs, selected path, window count, and
  focus. Verify Linux and Windows bindings separately.

## Acceptance evidence still required

Static metadata and code review establish action parameters, picker targets,
creation guards, and current local API absence. They do not establish shortcut
dispatch, fetch/create/switch outcomes, Workspace restoration, remote window
behavior, failure recovery, or paired visual acceptance. All runtime actions
and the current Git owner boundary remain open.
