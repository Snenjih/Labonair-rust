# Git Repository and Remote Management Action Classification

**Status:** Static action, project-lifecycle, and repository-selection classification; native interaction and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns repository detection and remote configuration. Workspace owns clone destinations, project/worktree creation, and activation. Source Control hosts GitPanel actions.

## Evidence boundary

This packet independently states the pinned Clone, Init, SelectRepo, CreateRemote, and FilterRemotes actions and compares their GitPanel, Workspace, and BranchPicker routes with Labonair's Git contracts. Source establishes URL/destination prompts, repository selection, configured initialization branch, and two action IDs without a matching handler; native prompts, project restoration, and remote outcomes remain unobserved. Research and requirements were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Clone | Registered `git::Clone` opens a URL modal. Confirm requests a destination directory, runs clone asynchronously, then adds the repository to the current project or opens it in a new project; an empty project skips that choice and adds it. Cancel dismisses; clone errors appear in a status toast. Labonair has no Git clone contract or reachable clone route in the inspected crates. |
| Initialize | Registered `git::Init` acts from GitPanel's no-repository state. It uses the only visible worktree directly, warns when none are open, or asks which directory when several are visible. It creates a repository with `git_panel.fallback_branch_name` (default `main`); errors are shown in the Git error surface. Labonair has a `git_init` transport helper, but no GitService method, adapter route, or caller was found. |
| Select repository | Registered `git::SelectRepo` opens a modal listing repositories in the active Project, sorted by display name, marks the active repository, filters as the user types, and sets the chosen entry as active on confirm. Labonair's Source Control view is scoped to one repository root and has no matching Git repository inventory/selection contract or action. |
| CreateRemote action | The registered `git::CreateRemote` action has no matching Workspace/GitPanel handler in the inspected source. Separately, BranchPicker can transition from a newly entered remote URL to a remote-name field and call `Repository::create_remote`; this is a BranchPicker query workflow, not evidence that the registered action dispatches. Labonair has no remote-create GitService method or route. |
| FilterRemotes action | The registered `git::FilterRemotes` action has no matching handler in the inspected source. BranchPicker separately filters branch entries among Local, Remote, and All; this is branch-list filtering and is not connected to the registered action. Labonair has branch-list filtering and remote branch display, but no remote-filter action or remote configuration list. |
| Settings and persistence | `git_panel.fallback_branch_name` defaults to `main` and supplies the new repository's branch name during Init. Cloning creates files at the user-selected destination; selecting a repository changes active in-memory GitStore state; remote creation changes repository Git configuration. Labonair has no matching clone/init/remote operations or active repository selector. |
| Ownership, contract, and current routes | Labonair's canonical Git capability is `labonair-git`; `labonair-git-transport`, `labonair-panel-scm`, and `labonair-panel-git-graph` are Git-owned siblings. `GitService` exposes repository detection, root, state, branch and log operations, but no clone, init, repository enumeration/selection, or remote list/mutation. A `git_init` helper exists only in `labonair-git-transport` and has no caller; Source Control uses one repository root, and Workspace has no Git clone event route. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-REGISTRY-LINUX-git-Clone`](registered-actions-unbound.md) | Registered without a default binding; enter repository URL, choose destination, clone, then add to the current project or open a new one. | No Git clone contract, URL/destination flow, or Workspace route found. |
| [`ACT-REGISTRY-LINUX-git-CreateRemote`](registered-actions-unbound.md) | Registered without a default binding; no action handler found. BranchPicker has a separate URL-then-name remote creation workflow. | No typed remote-create method or route found. |
| [`ACT-REGISTRY-LINUX-git-FilterRemotes`](registered-actions-unbound.md) | Registered without a default binding; no action handler found. BranchPicker's Local/Remote/All branch filter is a distinct surface. | Branch lists can distinguish/filter local and remote branches, but there is no matching action or remote configuration filter. |
| [`ACT-REGISTRY-LINUX-git-Init`](registered-actions-unbound.md) | Registered without a default binding; initialize the selected visible worktree using the configured fallback branch name. | A transport `git_init` helper exists, but is not part of GitService and has no caller; no Source Control action route exists. |
| [`ACT-REGISTRY-LINUX-git-SelectRepo`](registered-actions-unbound.md) | Registered without a default binding; filter and activate a repository in the active Project. | Source Control has no matching multi-repository selector or action route. |

## Required user-flow and state acceptance

- Clone valid HTTPS and SSH URLs to a new and existing destination; test empty/malformed URL, authentication, unavailable destination, existing directory, network failure, cancellation, and interrupted clone. Verify the choice between adding a worktree to the current project and opening a new project, including the empty-project shortcut.
- Initialize one visible worktree, no worktree, and multiple worktrees; verify the chosen directory, fallback branch setting (including non-default name), existing `.git` handling, permissions, cancellation, success refresh, and error recovery.
- Select a repository in a multi-root project; verify sort order, type-to-filter, active marker, keyboard/pointer confirmation, empty list, repository removal while modal is open, and correct Source Control scope after selection.
- Invoke the registered CreateRemote and FilterRemotes actions to establish whether they have effects. Separately test BranchPicker remote URL/name creation, duplicate/invalid names, remote configuration persistence, and the Local/Remote/All branch-list filters. Do not treat the alternate picker paths as action dispatch evidence.
- Verify action discoverability, focus transfer, operation progress, errors via Labonair's statusbar notification center, and narrow layouts. Capture paired native macOS results with exact builds, project/worktree count, remotes, URLs with secrets removed, destination, fallback branch value, window size, scale, theme, and result.

## Acceptance evidence still required

Static review establishes the Clone and Init flows, SelectRepo's active Project picker, Init's fallback branch setting, and separate BranchPicker remote creation/branch-filter routes. It also establishes that the registered CreateRemote and FilterRemotes actions have no matching handler in the inspected pinned source. Labonair has no clone/init/remote CRUD or multi-repository selection contract; its `git_init` helper is not connected to GitService. This does not establish action runtime dispatch, clone credentials/filesystem outcomes, active project restoration, remote config changes, or paired visual parity. Runtime, implementation, and macOS acceptance remain open.
