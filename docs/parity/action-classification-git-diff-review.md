# Git Diff and Review Action Classification

**Status:** Static action, diff-base, and review-route classification; native interaction and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns repository comparison and Git diff production. Workspace hosts diff items and action composition. Agent consumes branch diffs for review.

## Evidence boundary

This packet independently states the pinned diff-base, working/staged/unstaged, branch comparison, active-file diff, and branch review workflows and compares Labonair's typed diff request and Source Control routes. Source establishes action registration, setting updates, item selection, and Agent dispatch; native focus, setting reload, diff rendering, and review interaction remain unobserved. Research and requirements were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Configured diff base | `git.diff_base` defaults to `head`. `git::Diff` displays worktree changes against HEAD or all branch changes against the repository's default branch. Repositories without a resolvable default branch fall back to HEAD behavior. The setting also identifies the base used by Git gutter and file-color features. Labonair's current diff request has no base-ref or merge-base field. |
| Toggle diff base | Registered `git::ToggleDiffBase` switches the persisted setting between HEAD and default branch by updating the settings file. Labonair has no matching setting/action route found; no local diff-base field was found in the inspected crates. |
| Diff and DiffHead | `git::Diff` follows the configured base; `git::DiffHead` explicitly opens working changes against HEAD. The Git action opens or activates the Workspace diff item. Labonair can open one WorkingTree ProjectDiff from Source Control, but does not expose these actions or configured/default-branch semantics. |
| Branch diff | Registered `git::DiffBranch` (deprecated alias `git::BranchDiff`) opens the current branch diff against the repository default branch. If no default branch can be resolved, the handler falls back to the ordinary working-tree ProjectDiff. Labonair has no branch-base comparison route. |
| Compare with a branch | Registered `git::CompareWithBranch` opens branch selection, preselecting the current comparison ref when the active item is already a merge-base diff. Choosing a branch opens or activates a diff against that ref; canceling leaves the current comparison unchanged. An active repository is required. Labonair has no typed selected-branch diff request or picker route. |
| Working, unstaged, and staged views | `ViewUncommittedChanges` opens the whole working diff, `ViewUnstagedChanges` opens only unstaged changes, and `ViewStagedChanges` opens only staged changes. The pinned handlers use separate ProjectDiff, UnstagedDiff, and StagedDiff workspace items. Labonair's ProjectDiff source is either all current WorkingTree changes or a selected Commit; the inspected panel path has no staged-only or unstaged-only diff source/action. |
| Active-file diff | Registered `OpenFileDiff` is handled in the active Editor when its project path maps to a Git status entry. It opens or focuses a SoloDiffView for that repository file. The GitPanel's secondary-confirm route also opens the selected row's solo diff. Labonair's changed-file selection carries a selected path within its ProjectDiff request, but no matching editor action or per-file solo diff route was found. |
| Branch review | `ReviewDiff` is bound in Editor and handled by BranchDiff for a merge-base comparison. It loads the merge-base diff asynchronously, dispatches a typed `ReviewBranchDiff`, focuses AgentPanel, and starts an auto-submitted review thread containing a GitDiff resource. A non-merge-base item or missing Agent panel returns without opening review. Labonair has no matching branch-review action/dispatch or GitDiff review resource route. |
| Ownership, contract, and persistence | Labonair's canonical Git capability is `labonair-git`; `labonair-git-transport` and `labonair-panel-scm` are Git-owned siblings. The display request is `ProjectDiffRequest` in `labonair-panel` and contains repository root, session ID, `DiffSource` (`WorkingTree` or `Commit { hash, subject }`), file metadata, selected path, and mode. Workspace hosts the serializable ProjectDiff item. No local typed diff-base/merge-base or branch-review contract was found. No contract changes are proposed in this inventory slice. |
| Current Labonair routes | Source Control changed-file selection emits a WorkingTree ProjectDiff across current changes; history selection emits a read-only Commit-source ProjectDiff. These are partial display routes, not action-parity routes. Labonair has no `git.diff_base`, branch comparison, staged/unstaged split-diff source, or branch-to-Agent review dispatch in the inspected crates. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-git-Diff`](action-requirements.md) | `shift-ctrl-d` in `AcpThread > Editor` and `ctrl-g d` in GitPanel on macOS; corresponding platform bindings open the configured-base diff. | Source Control opens a WorkingTree ProjectDiff, but the action, persisted diff base, and branch/default-branch comparison are absent. |
| [`ACT-MACOS-git-ReviewDiff`](action-requirements.md) | `cmd-alt-g r` in Editor on macOS; available for a branch merge-base diff and starts an Agent review thread with the diff attached. | No branch review action, typed review dispatch, or GitDiff Agent resource path was found. |
| [`ACT-REGISTRY-LINUX-git-CompareWithBranch`](registered-actions-unbound.md) | Registered without a default binding; choose a branch from the picker to show changes against that ref. | No selected-branch diff request or branch comparison picker. |
| [`ACT-REGISTRY-LINUX-git-DiffBranch`](registered-actions-unbound.md) | Registered without a default binding; show current branch changes from the default branch. `git::BranchDiff` is a deprecated alias. | No branch/default-base diff route; current display source is WorkingTree or Commit. |
| [`ACT-REGISTRY-LINUX-git-DiffHead`](registered-actions-unbound.md) | Registered without a default binding; explicitly show working changes relative to HEAD. | A generic WorkingTree diff is available from Source Control, but the pinned action route is absent. |
| [`ACT-REGISTRY-LINUX-git-OpenFileDiff`](registered-actions-unbound.md) | Registered without a default binding; in a changed file's Editor, open or focus that file's Git diff. | ProjectDiff can select a changed path, but no matching action or solo file diff route was found. |
| [`ACT-REGISTRY-LINUX-git-ToggleDiffBase`](registered-actions-unbound.md) | Registered without a default binding; persistently switch `git.diff_base` between HEAD and default branch. | No matching field or action route was found. |
| [`ACT-REGISTRY-LINUX-git-ViewStagedChanges`](registered-actions-unbound.md) | Registered without a default binding; open a diff containing only staged changes. | No staged-only display source or action route was found. |
| [`ACT-REGISTRY-LINUX-git-ViewUncommittedChanges`](registered-actions-unbound.md) | Registered without a default binding; open the complete working diff. | A WorkingTree ProjectDiff exists as a Source Control route, without this action entry point. |
| [`ACT-REGISTRY-LINUX-git-ViewUnstagedChanges`](registered-actions-unbound.md) | Registered without a default binding; open a diff containing only unstaged changes. | No unstaged-only display source or action route was found. |

## Required user-flow and state acceptance

- Open the configured Diff with `git.diff_base` set to HEAD and to default branch; toggle the setting, restart/reload, and verify Diff, Git gutter, and file colors use the selected base. Test repositories with no default branch and verify fallback to HEAD.
- Open DiffHead while the configured base is default branch; open DiffBranch; compare two different selected refs; cancel branch selection; verify active repository, existing comparison item reuse, labels, navigation history, and recovery when a ref disappears.
- Open whole working, staged-only, and unstaged-only changes with empty, staged, unstaged, mixed, untracked, and conflicted repositories. Verify file ordering, read-only/editable behavior, hunk navigation, and refresh after index/worktree changes.
- From a changed file's Editor invoke OpenFileDiff; from GitPanel secondary-confirm open the selected file diff. Test deleted, renamed, untracked, and no-longer-changed files, multiple repositories, missing repository, repeated activation, and error recovery.
- From a branch merge-base diff invoke ReviewDiff. Verify diff generation, focus transfer, review thread creation, attached GitDiff resource, auto-submit, Agent unavailable/model failure behavior, and no-op behavior when the active diff is not a merge-base comparison.
- Verify platform-specific key contexts, command discovery for unbound actions, keyboard focus, pointer entry points, loading/error/empty states, persisted setting failure, and narrow layout. Capture paired native macOS states with exact builds, repository/ref graph, staged state, setting value, Agent availability, window size, scale, theme, and outcome.

## Acceptance evidence still required

Static review establishes two bound and eight registered-unbound actions, action contexts, the persisted HEAD/default-branch setting behavior, the branch-picker and merge-base review route, and Labonair's current WorkingTree/Commit request sources. It does not establish native key dispatch, settings save/reload, rendered diff states, file diff behavior, Agent thread interaction, recovery, or paired visual parity. Labonair's generic ProjectDiff display does not establish comparison or review parity. Runtime, implementation, and macOS acceptance remain open.
