# Git Panel Action Classification

**Status:** Static GitPanel action, settings, and diff-route classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns Source Control behavior and repository state. Workspace composes the panel and its dock lifecycle. Settings owns the persisted GitPanel view values.

## Evidence boundary

This packet classifies twelve registered-unbound `git_panel` actions and compares their pinned handlers with Labonair's Source Control panel. Pinned source establishes panel event routes, settings writes, transient commit state, and selected-entry staged/unstaged diff calls. It does not establish native invocation, settings save/reload behavior, visible menu geometry, or paired macOS parity. Research and requirements were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Close or toggle the panel | `git_panel::Close` emits `PanelEvent::Close`. Workspace registers `git_panel::Toggle`: it tries to toggle panel focus and closes the GitPanel if that call returns false. These actions do not share the separate `git_panel::ToggleFocus` action. Labonair's Workspace `FocusSourceControl` command opens/focuses the panel; no matching close/toggle action route was found. |
| Open the panel menu | The action declaration documents `OpenMenu` as opening the GitPanel menu, but no handler or action call site was found in the inspected pinned source. The menu still has pointer entry points. Labonair's menus are pointer-driven; no matching action route exists. Keep the action result unresolved. |
| Group and sort changes | `SetGroupByNone`, `SetGroupByStatus`, and `SetGroupByStaging` write `git_panel.group_by` as `none`, `status`, or `staging`. `SetSortByPath` and `SetSortByName` write `git_panel.sort_by` as `path` or `name`; those menu options are shown only in flat view. `ToggleTreeView` inverts persisted `git_panel.tree_view` and switches the view model. Settings changes are observed and rebuild visible entries. Labonair has a Flat/Tree pointer menu backed by the separate `scm_file_tree` setting, a path-sorted list, and fixed conflict/staged/changes/untracked sections; it has no matching settings or action routes for this GitPanel grouping/sorting model. |
| Include co-author trailers | `ToggleFillCoAuthors` changes the panel's transient `add_coauthors` flag, initialized to enabled. If potential collaborators are available, the commit editor shows a toggle; committing with it enabled adds the suggested `Co-authored-by` trailers to the message. This is not a Settings value. No matching Labonair co-author suggestion or commit-message route was found. |
| View one entry's staged or unstaged changes | `ViewStagedChanges` and `ViewUnstagedChanges` read the selected GitPanel status entry, if any, and pass it to `StagedDiff::deploy_at` or `UnstagedDiff::deploy_at`. These `git_panel` actions are distinct from the `git::ViewStagedChanges` and `git::ViewUnstagedChanges` whole-view actions. Labonair changed-file selection opens a ProjectDiff; the inspected route has no staged-only or unstaged-only selected-entry diff source. |
| Ownership, contract, and persistence | Pinned behavior belongs to GitPanel; Workspace supplies dock focus/close, and SettingsStore persists the three view values. Labonair owners are `labonair-git`, `labonair-panel-scm`, and `labonair-workspace`; the local `scm_file_tree` setting is not the same key or behavior as `git_panel.tree_view`. No local contract, setting, action, registry, or dependency change is proposed. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-REGISTRY-LINUX-git_panel-Close`](registered-actions-unbound.md) | In GitPanel context, emit the panel Close event. | Dock close controls exist, but no matching action dispatch route was found. |
| [`ACT-REGISTRY-LINUX-git_panel-OpenMenu`](registered-actions-unbound.md) | Action documentation says open the GitPanel menu; no handler or call site was found in inspected source. | Pointer menus exist; no matching action route. Static intent remains unresolved. |
| [`ACT-REGISTRY-LINUX-git_panel-SetGroupByNone`](registered-actions-unbound.md) | Persist `git_panel.group_by = none`. | Fixed status sections; no configurable grouping setting/action. |
| [`ACT-REGISTRY-LINUX-git_panel-SetGroupByStatus`](registered-actions-unbound.md) | Persist `git_panel.group_by = status` (Tracked & Untracked). | Fixed status sections; no matching grouping value/action. |
| [`ACT-REGISTRY-LINUX-git_panel-SetGroupByStaging`](registered-actions-unbound.md) | Persist `git_panel.group_by = staging` (Staged & Unstaged). | Fixed status sections; no matching grouping value/action. |
| [`ACT-REGISTRY-LINUX-git_panel-SetSortByName`](registered-actions-unbound.md) | Persist name sorting in flat view. | Current list sorts by path; no name-sort setting/action. |
| [`ACT-REGISTRY-LINUX-git_panel-SetSortByPath`](registered-actions-unbound.md) | Persist path sorting in flat view. | Path sorting exists, but no matching setting or action route. |
| [`ACT-REGISTRY-LINUX-git_panel-Toggle`](registered-actions-unbound.md) | Toggle GitPanel focus; close the panel if focus toggling reports false. | Workspace opens/focuses Source Control but has no matching toggle-and-close action route. |
| [`ACT-REGISTRY-LINUX-git_panel-ToggleFillCoAuthors`](registered-actions-unbound.md) | Toggle transient automatic co-author trailers for subsequent commits; initially enabled. | No co-author suggestion or commit trailer route found. |
| [`ACT-REGISTRY-LINUX-git_panel-ToggleTreeView`](registered-actions-unbound.md) | Toggle and persist tree/flat GitPanel view in `git_panel.tree_view`. | Flat/Tree pointer control exists under `scm_file_tree`; no matching action or setting contract. |
| [`ACT-REGISTRY-LINUX-git_panel-ViewStagedChanges`](registered-actions-unbound.md) | Open StagedDiff for the selected status entry, if present. | ProjectDiff is available, but no selected-entry staged-only diff route. |
| [`ACT-REGISTRY-LINUX-git_panel-ViewUnstagedChanges`](registered-actions-unbound.md) | Open UnstagedDiff for the selected status entry, if present. | ProjectDiff is available, but no selected-entry unstaged-only diff route. |

## Required user-flow and state acceptance

- Invoke Close and Toggle with the GitPanel closed, open, focused, and unfocused; verify dock state, focus restoration, and the separate ToggleFocus behavior.
- Invoke OpenMenu through command discovery and panel context, and verify whether any dispatch route exists. Do not infer behavior from its action description alone.
- Change grouping among none/status/staging, flat sorting between name/path, and tree view. Verify SettingsStore persistence, reload, visible ordering/group membership, hidden sort options in tree mode, and failed-settings-write recovery.
- Toggle co-author trailers with no collaborators and with one or multiple suggested co-authors; verify initial state, exact generated message trailers, commit failure, and state lifetime after panel/repository changes.
- Invoke the two selected-entry diff actions on staged-only, unstaged-only, partially staged, conflicted, untracked, and no-selection states. Verify which entry and diff scope are opened, Workspace item reuse, and error recovery.
- Compare native menus, keyboard discovery, focus, loading/empty/error states, and narrow layout with Labonair's pointer view-options menu, fixed sections, local tree setting, and ProjectDiff route.

## Acceptance evidence still required

Static review establishes twelve registered-unbound action IDs, the Close/Toggle handlers, the unresolved OpenMenu declaration, the three persisted view settings and menu conditions, transient co-author state, and selected-entry staged/unstaged diff dispatch. Labonair has a distinct tree setting and pointer menu, fixed status sections, and WorkingTree/Commit ProjectDiff sources. Native action dispatch, settings reload, message construction, selected-entry scope, focus, errors, and paired macOS visual parity remain open.
