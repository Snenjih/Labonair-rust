# Git Stash Workflow Action Classification

**Status:** Static action and route classification; native dispatch, stash outcomes, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns stash operations and stash-specific behavior. Workspace
hosts picker and diff items. Keymap owns default bindings.

## Evidence boundary

This packet independently states the stash creation, list, apply, pop, drop,
and review workflows, then records current Labonair routes and gaps. Static
action metadata, keymaps, pinned source, and local contracts establish the
requirements and code paths; they do not prove native dispatch, successful
repository operations, or visual acceptance. Research and requirements were
produced in the same session, so this is independent redevelopment, not
separated-room clean-room implementation. Paired macOS interaction evidence
remains pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Create stashes | From the Git change-list context menu, `Stash All` is available when new or tracked changes exist. `Stash Tracked` is offered for Status grouping when tracked changes exist. `Stash Staged` is offered for Staging grouping when staged changes exist. Each opens a focused message prompt; confirming an empty message lets Git supply its usual WIP description, while cancel leaves the working tree unchanged. Labonair has one optional-message stash form and no explicit tracked-only or staged-only operation. |
| Apply or pop the latest stash | `StashApply` applies the current repository's latest stash and keeps it. `StashPop` applies and removes the latest stash; the change-list context menu disables Pop when there are no stashes. These workspace actions have no default key binding; the explicit menu route is Pop. Labonair's list can apply or pop a selected entry, but does not expose the same latest-stash actions. |
| Open the stash list | `ViewStash` opens the Git Picker on its Stashes tab from the change-list menu. `ActivateStashTab` switches an already open Git Picker to that tab (`cmd-2` on macOS; `alt-2` on Linux and Windows). A selected entry can be filtered by its message; normal picker confirmation applies it, while secondary confirmation pops it. Labonair has no Git Picker or stash-tab keyboard action. |
| Review one stash | `ShowStashItem` opens the selected stash as a diff item. The diff is associated with both the selected stash identity and its current list index. Labonair has no stash-specific diff entry point or review item. |
| Apply, pop, or drop a reviewed stash | In the stash diff, Apply, Pop, and Drop each ask for confirmation naming the selected `stash@{index}`. Confirm performs the operation; Cancel leaves it open. The selected stash identity is checked against the current index before mutation. Successful operations close the diff. A stale identity closes the diff and reports an error; an operation failure is reported and leaves the review available. Labonair has no stash diff actions. |
| Drop from the list | `DropStashItem` drops the selected list entry asynchronously without a second confirmation and keeps the picker available. Errors are presented to the user. Labonair instead asks for confirmation before dropping an entry from its Source Control list. |
| Git behavior and failures | Stash operations run asynchronously against the active repository. Apply/pop conflicts are visible failures; pop retains the stash when Git cannot complete the operation. Labonair already explains that a conflicted apply/pop keeps the stash and asks the user to resolve the working-tree conflicts. Native recovery and retry behavior still need interaction evidence. |
| Owner and crates | Git is the capability owner and `labonair-git` its canonical contract crate. `labonair-git-transport` performs local or remote operations, and `labonair-panel-scm` is a Git-owned sibling UI crate. Workspace hosts future picker/diff items; Keymap supplies shortcut resolution. A stash diff must use the accepted shared DiffView contract rather than establish a second diff owner. |
| Contracts, events, and registries | Labonair already exposes path/message/include-untracked stash push and hash-addressed apply, pop, and drop operations through `labonair-git`; transport refreshes repository state. This classification adds no event, registry, command, or action contract. A future stash review route needs a typed Git-to-DiffView request and a Workspace host adapter. |
| State and persistence | Stash objects and order persist in Git for the selected repository. Search query, selected row, open prompt, and review item are transient UI state. No Settings value or separate stash database is required. |
| Notifications and UI kit | Labonair user-facing errors go through the Notifications owner. Existing Source Control stash controls use `labonair-ui-kit`; any future picker, confirmation, or diff controls must use shared UI-kit components. |
| Current Labonair route | Open the Source Control panel and expand its Stashes section. The `+` control focuses an optional-message field; Stash submits asynchronously. Each entry has Apply, Pop, and Drop controls, with confirmation before Drop. The list shows an empty state and refreshes after operations. The transport supports local and remote sessions. No default stash shortcuts or palette actions, stage-specific stash creation, Git Picker Stashes tab, stash diff item, or diff actions were found. The current push passes no include-untracked override, which the transport treats as including untracked files. |
| Settings and dependencies | No Settings changes or dependency additions. Preserve Git repository/session boundaries and asynchronous I/O. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-git-ApplyCurrentStash`](action-requirements.md) | `ctrl-space` in `StashDiff > Editor` on macOS, Linux, and Windows; confirm before applying the stash currently shown. | No stash diff item or context-bound action route. |
| [`ACT-MACOS-git-DropCurrentStash`](action-requirements.md) | `ctrl-shift-backspace` in `StashDiff > Editor` on all three platforms; confirm before dropping the reviewed stash. | No stash diff item or reviewed-stash drop route. |
| [`ACT-MACOS-git-PopCurrentStash`](action-requirements.md) | `ctrl-shift-space` in `StashDiff > Editor` on all three platforms; confirm before applying and removing the reviewed stash. | No stash diff item or context-bound action route. |
| [`ACT-MACOS-git_picker-ActivateStashTab`](action-requirements.md) | `cmd-2` in `GitPicker` on macOS; `alt-2` on Linux and Windows. Select the Stashes tab in the open Git Picker. | No Git Picker, stash tab, or default shortcut. |
| [`ACT-MACOS-stash_picker-DropStashItem`](action-requirements.md) | `ctrl-shift-backspace` in `StashList` or its nested Picker/Editor context on all platforms. Drop the selected stash without a confirmation prompt. | A Source Control row can drop by pointer after confirmation; no selected-row action or keyboard binding. |
| [`ACT-MACOS-stash_picker-ShowStashItem`](action-requirements.md) | `ctrl-shift-v` in `StashList` or its nested Picker/Editor context on all platforms. Open the selected stash as a diff. | No stash review/diff route or shortcut. |
| [`ACT-REGISTRY-LINUX-git-StashAll`](registered-actions-unbound.md) | Registered workspace action; change-list context menu opens the optional-message prompt when new or tracked changes exist. | Generic stash form only; no same action or context-dependent availability evidence. |
| [`ACT-REGISTRY-LINUX-git-StashApply`](registered-actions-unbound.md) | Registered workspace action applies the latest stash. No default key binding or GitPanel context-menu entry was found. | Per-entry Apply exists, but no latest-stash action or matching global command route. |
| [`ACT-REGISTRY-LINUX-git-StashPop`](registered-actions-unbound.md) | Registered workspace action pops the latest stash; change-list context menu exposes it and disables it when no stash exists. | Per-entry Pop exists, but no matching latest-stash action or availability state. |
| [`ACT-REGISTRY-LINUX-git-StashStaged`](registered-actions-unbound.md) | Registered workspace action opens the optional-message prompt for staged changes; offered from Staging grouping only when staged changes exist. | No staged-only stash operation or matching action. |
| [`ACT-REGISTRY-LINUX-git-StashTracked`](registered-actions-unbound.md) | Registered workspace action opens the optional-message prompt for tracked changes; offered from Status grouping only when tracked changes exist. | No tracked-only stash operation or matching action. |
| [`ACT-REGISTRY-LINUX-git-ViewStash`](registered-actions-unbound.md) | Registered workspace action opens the Git Picker at Stashes; also offered in the change-list context menu. | The Stashes section is visible in Source Control, but no picker route or matching action exists. |

## Required user-flow and state acceptance

- From each Git change-list grouping, verify the Stash All, Stash Tracked, and
  Stash Staged menu visibility and disabled states against new, tracked,
  staged, conflicted, and clean changes. Verify message focus, editing,
  empty-message behavior, confirmation, cancellation, asynchronous completion,
  and errors.
- Apply and pop the latest stash from their action/menu routes. Verify a clean
  repository, no-stash state, operation failure, conflict, list refresh, and
  that pop keeps the entry if applying it conflicts.
- Open the Git Picker from View Stash and by its existing-tab shortcut. Verify
  tab selection, no repository, empty/loading/error states, message filtering,
  row selection, normal apply, secondary-confirm pop, dismissal, and refresh.
- Show and drop the selected stash. Verify diff contents, file navigation,
  the selected stash's identity under list reorder/removal, direct list drop,
  failure presentation, and picker behavior after a successful drop.
- From a stash diff, test Apply, Pop, and Drop with confirmation, cancellation,
  successful completion, stale index/hash, missing repository, Git failure,
  and conflict. Verify whether the diff closes or stays available and that the
  correct stash is retained or removed.
- Reproduce the equivalent Local and SSH/remote-session workflows in Labonair.
  Verify message and include-untracked semantics, apply/pop conflict recovery,
  drop confirmation, async loading, refresh, errors through Notifications, and
  no-repository/empty states.
- Capture paired native macOS change-list context menus, stash message prompt,
  Git Picker Stashes tab, stash diff, confirmations, disabled/empty/error/
  conflict states, and the Labonair equivalents. Record OS, window bounds,
  theme, scale, repository state, stash messages/order, grouping, and focus.
  Verify Linux and Windows key dispatch separately.

## Acceptance evidence still required

Static review establishes the twelve action meanings, default bindings,
context-menu conditions, async operation targets, current Labonair stash API,
and panel controls. It does not establish native key dispatch, command
discoverability, exact success/conflict recovery, SSH behavior, or paired
visual parity. Labonair's current list workflow is partial support; the picker,
stash diff, stage-specific creation, action discovery, and pinned latest-stash
routes remain open.
