# Git Add and Ignore Action Classification

**Status:** Static action, handler, and ignore-file classification; native interaction and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns index and ignore-file operations. Source Control owns the selected file context and its controls. Workspace hosts the action/menu surfaces.

## Evidence boundary

This packet independently states the pinned Add, AddToGitignore, and AddToGitInfoExclude actions and compares their GitPanel handlers with Labonair's typed stage/ignore operations. Source establishes action registration, created-entry gating, repository-relative ignore writes, deduplication, and current local menu/API routes. Native command dispatch and user-visible refresh remain unobserved. Research and requirements were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| `git::Add` action | The action metadata describes adding files to the Git staging area. The only inspected registered handler is declared in ProjectDiff and calls `ProjectDiff::deploy_at`; it opens the working diff and does not stage a file. No second handler for this action was found in the inspected GitPanel/Workspace routes. Treat the name/description versus handler as an unresolved pinned source discrepancy; dispatch behavior has not been observed. Labonair has typed `stage_file`/`stage_all` operations and Source Control stage controls, but no matching Add action route. |
| Add to `.gitignore` | In GitPanel's ChangesList context menu, enabled only for one selected created/untracked entry (not a bulk set). The handler adds that repository-relative path to the repository `.gitignore`; tracked entries do not show an enabled action. Labonair has a Source Control context-menu route and typed GitService operation, but its menu is rendered for file rows beyond newly created entries. Its transport writes a root-anchored `/<path>` pattern, while the pinned writer appends the path without a leading slash. |
| Add to `.git/info/exclude` | In the same created-entry/single-selection state, add the repository-relative path to the repository-local exclude file rather than shared `.gitignore`. Labonair has a typed operation and menu route; its adapter appends to `.git/info/exclude`. This is repository-local ignore state and does not alter application Settings. |
| Duplicate and file behavior | Pinned local ignore-file append helpers avoid duplicating an existing pattern and add a newline only when required. Labonair's local/remote adapters also deduplicate exact normalized entries and append a missing newline. Verify pattern matching for nested paths, existing negation/glob rules, tracked files, directories, and remote repositories; the leading-slash difference changes `.gitignore` matching scope. |
| Ownership, contract, and persistence | Labonair's canonical capability is `labonair-git`; `labonair-git-transport` executes local/remote work and `labonair-panel-scm` owns the Source Control file menu. `GitService` exposes `stage_file`, `stage_all`, `add_to_gitignore`, and `add_to_exclude` with repository root, relative path, and optional session ID. Staging mutates the Git index; the two ignore operations write repository files. No contract or persistence change is proposed. |
| Current Labonair routes | Source Control row controls invoke typed file staging. Its file context menu invokes the typed `.gitignore` and `.git/info/exclude` operations; the generic operation runner serializes operations, routes errors through Labonair's notification path, clears cached history, refreshes repository state, and updates the panel. These local UI controls are not matching pinned action dispatch routes. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-REGISTRY-LINUX-git-Add`](registered-actions-unbound.md) | Registered without a default binding; action metadata says stage files, but its inspected registered handler opens ProjectDiff without a staging call. Pinned dispatch semantics remain unresolved. | File and all-stage controls exist through GitService, but there is no matching Add action. |
| [`ACT-REGISTRY-LINUX-git-AddToGitInfoExclude`](registered-actions-unbound.md) | Registered without a default binding; from one created GitPanel entry add its repository-relative path to `.git/info/exclude`. | Typed operation and Source Control menu exist; menu scope is broader and the action ID is not connected. |
| [`ACT-REGISTRY-LINUX-git-AddToGitignore`](registered-actions-unbound.md) | Registered without a default binding; from one created GitPanel entry add its repository-relative path to `.gitignore`. | Typed operation and Source Control menu exist; menu scope is broader, path anchoring differs, and the action ID is not connected. |

## Required user-flow and state acceptance

- Invoke `git::Add` through command discovery and any action context. Establish whether the user-visible effect is staging or opening ProjectDiff; verify index state before/after and resolve the source/metadata mismatch before implementing parity.
- For each ignore action, test no repository, no selection, one created/untracked file, tracked modified/deleted file, directory, multi-selection, duplicate pattern, already ignored path, nested path, existing `.gitignore` with/without final newline, negation/glob rule, read-only repository, and remote repository.
- Verify the pinned action is disabled or unavailable for tracked entries and bulk selection, then compare Labonair's currently broader Source Control menu behavior. Confirm ignoring a tracked file does not silently untrack it.
- Verify `.gitignore` affects shared/project-visible Git behavior while `.git/info/exclude` stays repository-local; compare the exact pattern (including anchoring), path normalization, deduplication, success refresh, status changes, failure recovery, and retry.
- Verify command discovery, context focus, keyboard/pointer invocation, loading/error states, and narrow layout. Capture paired native macOS states with exact builds, repository/worktree, status entry, ignore-file contents, window size, scale, theme, and result.

## Acceptance evidence still required

Static review establishes three registered-unbound actions and their current source routes, including a mismatch between the Add action's staging description and its ProjectDiff handler. It also establishes GitPanel's created/single-entry gating for the two ignore actions and Labonair's typed stage/ignore APIs, broader file-menu exposure, and root-anchored `.gitignore` pattern. It does not establish actual action dispatch, resulting index/status changes, glob behavior in native repositories, remote failure recovery, or paired visual parity. Runtime, implementation, and macOS acceptance remain open.
