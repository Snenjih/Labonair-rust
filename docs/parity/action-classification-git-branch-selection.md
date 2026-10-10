# Git Branch Selection and Rename Action Classification

**Status:** Static action/schema and branch-route classification; native interaction, checkout recovery, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns branch refs and checkout. Workspace hosts the GitPicker and rename modal; Source Control owns Labonair's branch dropdown and its palette contribution.

## Evidence boundary

This packet classifies the pinned `CheckoutBranch`, `Switch`, `RenameBranch`, and `CopyBranchName` actions and compares their GitPicker/Workspace routes with Labonair. Static registration and source establish which views and Git operations are called, not native focus, dirty-worktree protection, confirmation, clipboard effects, or visual parity. The `RenameBranch` schema has an optional branch argument, but its Workspace handler ignores that value and targets the current branch. Research and requirements were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Open branch selection | `CheckoutBranch` and `Switch` are both registered without default bindings. Each opens the GitPicker modal on its Branches tab for the active repository. The branch list supports filtering and selecting a local or remote branch; selecting the current branch simply dismisses. Selecting another branch calls the repository branch-change operation and reports failure through the Git error prompt. Labonair has a Source Control dropdown with a filter and local/remote rows, but not these registered action routes or the GitPicker modal. |
| Select or create a branch | The Branches list supports branch selection and query-based creation; its secondary confirmation path can use the configured default branch as the new branch base. GitPanel and CommitModal also embed a narrower BranchList popover for branch selection. Labonair's dropdown has a separate form with branch name, optional base ref, and a Checkout after create choice. These are analogous workflows with different entry points, state, and layout. |
| Rename branch | `RenameBranch` is registered without a default binding. Its schema accepts an optional `branch` field documented to default to the current branch. The Workspace action registration discards the action value and always opens a modal for the current branch. The modal pre-fills that name; confirming empty or unchanged text closes without a mutation, while a changed name calls Git rename and reports asynchronous failure. Thus a supplied branch argument is not honored in the pinned handler. |
| Copy current branch name | `CopyBranchName` is registered without a default binding. Its Workspace handler reads the active GitPanel's active repository and writes the current branch name as a string to the OS clipboard; it silently returns without a GitPanel or current branch. |
| Current Labonair routes | The Source Control branch dropdown lists local and remote branches; clicking a different row calls `checkout_branch`, while clicking the current row is a no-op. The dropdown has inline rename controls on local branch rows, a name filter, and branch creation fields for a name/base plus optional checkout. The command palette owns a separate `source-control` SwitchBranch(name) action that calls the panel checkout method; it is not a binding or dispatch of pinned `git::Switch` or `git::CheckoutBranch`. The Git trait supports checkout/create/rename operations, but there is no pinned GitPicker modal, current-branch RenameBranch action, or equivalent branch-name clipboard route. |
| Ownership, contract, and persistence | `labonair-git` owns repository branch operations; `labonair-git-transport` performs checkout/create/rename commands; `labonair-panel-scm` owns the current dropdown and form. Workspace owns the pinned modal/action routes. Branch refs and current HEAD are repository-owned; picker, filter, form, and modal input are transient. No Settings, persistence, event, registry, action, or dependency change is proposed. |
| Failure and safety | Pinned selection skips a branch-change request when the selected ref is already current and presents a Git error on failed checkout. Dirty worktree protection and recovery must be verified at runtime. Labonair maps checkout errors to an inline message and notification, and refreshes after success. Both flows need tests for dirty files, remote branch checkout, invalid names, and branch-name collisions; this packet makes no runtime claim. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-REGISTRY-LINUX-git-CheckoutBranch`](registered-actions-unbound.md) | Registered without a default binding; open GitPicker on the Branches tab. Despite the name, it opens the picker rather than directly checking out a branch. | Branch dropdown and `source-control` SwitchBranch(name) palette contribution can select/check out branches, but no matching action or GitPicker modal. |
| [`ACT-REGISTRY-LINUX-git-Switch`](registered-actions-unbound.md) | Registered without a default binding; open the same GitPicker Branches tab as CheckoutBranch. | Same local dropdown/palette routes; no matching action dispatch or modal. |
| [`ACT-REGISTRY-LINUX-git-RenameBranch`](registered-actions-unbound.md) | Registered without a default binding; optional `branch` argument is accepted by the schema but ignored by the handler, which opens a prefilled rename modal for the current branch. Empty/unchanged confirmation dismisses; changed name renames current branch. | Inline rename is available on local branch rows, including non-current branches; no matching action, current-branch modal, or argument behavior. |
| [`ACT-REGISTRY-LINUX-git-CopyBranchName`](registered-actions-unbound.md) | Registered without a default binding; copy the active GitPanel repository's current branch name to the OS clipboard. Missing panel/current branch is a silent no-op. | No branch-name clipboard action or route was found; existing path-copy actions copy file paths and are not equivalent. |

## Required user-flow and state acceptance

- Invoke CheckoutBranch and Switch through every registered menu/palette route with an active repository, no active repository, local branch, remote branch, current branch, and no matches. Confirm both actions open the same GitPicker tab and do not directly checkout by name.
- Filter local/remote branches, select a branch, cancel the picker, create a branch from the current/default/typed base, and verify HEAD, tracking state, index, worktree, focus restoration, and error recovery.
- Switch with staged and unstaged edits, conflicts, detached HEAD, a branch checked out in another worktree, missing refs, and ambiguous names. Record prompt/no-prompt behavior and preserved data.
- Invoke RenameBranch with no argument, current branch name, another branch name, empty text, unchanged text, invalid names, and collisions. Verify whether the optional argument is ignored, rename target, current/upstream refs, modal dismissal, and error reporting.
- Invoke CopyBranchName with and without an active GitPanel, repository, and current branch; verify the exact clipboard string and that no-op cases leave the clipboard unchanged.
- Compare the Labonair dropdown and SwitchBranch palette path with the pinned modal. Verify local and remote rows, filter state, create-base and checkout toggles, inline rename scope, current row, no repository, loading/error states, keyboard-only selection, and narrow layout.
- Capture paired native macOS branch picker, embedded popover, rename modal, create flow, and dirty-worktree error states. Record exact builds, branch/ref fixture, dirty/index state, window size, scale, theme, focus, and result. Keep Linux/Windows action registration and any overlay bindings separate from default-map acceptance.

## Acceptance evidence still required

Static review establishes four registered-unbound action identities, that Switch and CheckoutBranch open the same GitPicker Branches tab, the BranchList selection/create routes, RenameBranch's optional argument schema and current-branch-only behavior, CopyBranchName's active-GitPanel clipboard route, and Labonair's distinct dropdown/palette contracts. It does not establish native invocation, keyboard focus, clipboard contents, dirty-worktree protection, error visibility, branch tracking after checkout/create/rename, or paired visual parity. Runtime, implementation, and macOS acceptance remain open.
