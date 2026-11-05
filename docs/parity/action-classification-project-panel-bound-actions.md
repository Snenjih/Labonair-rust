# Default-Bound ProjectPanel Action Classification

**Status:** Static action, keymap, Settings, and route classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Pinned ProjectPanel owns project-tree selection and file-operation behavior with Workspace action adapters. Workspace owns panel focus, preview/tab, diff, and cross-surface routes; pinned Search owns directory-scoped search. Labonair Explorer owns its tree-local selection, clipboard, and filesystem UI; Workspace owns tabs, docks, notifications, and SearchOverlay presentation; Editor owns search-query/result models and document history.

## Evidence boundary

This packet classifies all 20 default-macOS-bound project_panel action IDs in the focused slice. Their default macOS, Linux, and Windows bindings and action arguments remain in [action-requirements.md](action-requirements.md). The overlay registers contain 41 macOS-profile entries across 16 of these IDs, 20 shared Linux/Windows-profile entries across 14 IDs, and 10 conditional Vim/Helix entries across 10 IDs. They are static keymap evidence only. No action is marked runtime-observed.

The adjacent [ProjectPanel unbound-action packet](action-classification-project-panel-actions.md) classifies 25 registered-unbound actions and their related profile bindings. Absolute and relative path-copy actions remain in the separate [Workspace path-copy packet](action-classification-workspace-path-copy.md). Same-session source analysis and independently authored requirements are redevelopment, not separated-room clean-room implementation.

## Action semantics and Labonair routes

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-project_panel-CollapseAllEntries](action-requirements.md) | Collapse expanded directories for every visible worktree. A single-worktree project keeps its root expanded; with multiple worktrees the expanded-directory set is cleared. | Explorer can toggle individual directory rows; it has no collapse-all action or typed ProjectPanel dispatch. |
| [ACT-MACOS-project_panel-CollapseSelectedEntry](action-requirements.md) | Recompress one level of a compacted ancestor row when possible; otherwise collapse the selected directory or nearest expanded ancestor. It does not select a parent as a fallback. | Explorer toggles the clicked directory row, with no keyboard-selected tree row or ancestor-fold action. |
| [ACT-MACOS-project_panel-CompareMarkedFiles](action-requirements.md) | Open a file diff for the last two marked entries that resolve to files; fewer than two eligible files is a no-op. | Explorer supports path selection for its own operations but has no marked-file comparison action. Workspace ProjectDiff covers Git WorkingTree and Commit sources, not arbitrary two-file comparison. |
| [ACT-MACOS-project_panel-Copy](action-requirements.md) | Copy disjoint selected entries, excluding worktree roots, to the system file clipboard and retain an internal ProjectPanel copy record. | Explorer has an app-local copy buffer and context/key routes; it does not place selected file paths on the system file clipboard or expose this typed action. |
| [ACT-MACOS-project_panel-Cut](action-requirements.md) | Cut the eligible disjoint selection to the system file clipboard and retain a transient cut record for a later move. | Explorer has an app-local cut buffer and context/key routes; it does not share the pinned system-clipboard or action contract. |
| [ACT-MACOS-project_panel-Delete](action-requirements.md) | Permanently delete eligible selected entries, excluding worktree roots. Confirm by default with names and dirty-buffer count unless skip_prompt is true. Partial failures continue and are reported; Delete is not undoable. | Explorer's context-menu Delete requires confirmation and calls filesystem deletion directly. It handles one target and has no ProjectPanel action argument, batch result, or delete undo history. |
| [ACT-MACOS-project_panel-Duplicate](action-requirements.md) | Copy the selected entry or entries and paste them into the selected directory, or the source file's parent. Name collisions create a copy-suffixed path; a single collision opens inline rename for adjustment. | Explorer exposes Copy and Paste, which can resemble a duplicate in some cases; there is no Duplicate action or matching collision/rename behavior. |
| [ACT-MACOS-project_panel-ExpandAllEntries](action-requirements.md) | Recursively expand every visible worktree root and its directories, then refresh the flattened tree. | Explorer loads and expands directories individually; no expand-all action exists. |
| [ACT-MACOS-project_panel-ExpandSelectedEntry](action-requirements.md) | Unfold one compacted ancestor level; otherwise expand a collapsed directory. If the directory is already expanded, move selection to the next entry. | Explorer toggles a clicked directory and has no keyboard selection or expand-next action. |
| [ACT-MACOS-project_panel-NewDirectory](action-requirements.md) | Start an inline directory-name entry in the selected directory, selected file's parent, or last worktree root when nothing is selected. | Explorer toolbar and context menu offer New Folder with inline naming. This is a direct UI route without a typed ProjectPanel action and does not implement pinned worktree-root selection semantics for all states. |
| [ACT-MACOS-project_panel-NewFile](action-requirements.md) | Start an inline file-name entry in the selected directory, selected file's parent, or last worktree root when nothing is selected. A created file opens when project_panel.auto_open.on_create is enabled. | Explorer toolbar/context menu creates and reloads a file but does not auto-open it or expose the same setting/action contract. |
| [ACT-MACOS-project_panel-NewSearchInDirectory](action-requirements.md) | Open Search with the selected directory as its scope; a selected file scopes to its parent. Multi-worktree projects include the root name. A root-level file dispatches Search without an explicit directory filter. | Workspace SearchOverlay supports current-file/project search through Editor-owned query models, but it receives no selected-directory filter and Explorer has no action route. |
| [ACT-MACOS-project_panel-Open](action-requirements.md) | On a selected file, open using preview_tabs.enable_preview_from_project_panel; when the setting disables preview, focus the opened item. On a directory, toggle expansion. | Explorer Space opens a preview, Enter opens permanently, and either toggles a directory. These routes are UI-local and have no matching preview setting or typed action. |
| [ACT-MACOS-project_panel-Paste](action-requirements.md) | Paste eligible OS file paths into the selected directory, or the last worktree root when there is no selection; internal paste uses the selected entry. A selected file targets its parent. Internal Cut renames/moves; Copy creates entries. A single pasted file may open per project_panel.auto_open.on_paste; collisions can enter inline rename. Changes are recorded for undo; Cut becomes Copy after the first paste. | Explorer pastes only from its app-local buffer, moves cut paths or copies them off-thread, and reloads the tree. It does not consume OS file paths, apply the auto-open setting, or record undoable filesystem changes. |
| [ACT-MACOS-project_panel-Redo](action-requirements.md) | Reapply the latest available ProjectPanel file-operation change from its transient undo history; concurrent changes can make it fail. | Editor has text-buffer redo. Explorer has no filesystem redo stack or matching action. |
| [ACT-MACOS-project_panel-Rename](action-requirements.md) | Edit the selected path inline. When marks exist the Workspace adapter first selects the first marked entry and clears the marks. The filename stem is selected for files while the extension remains; directory names are selected in full. Hidden/sole project-root restrictions apply. | Explorer context menu starts inline rename for its target, then mutates the filesystem and reloads. It has no ProjectPanel selection adapter or matching filename selection/root contract. |
| [ACT-MACOS-project_panel-RevealInFileManager](action-requirements.md) | Reveal the selected entry in the platform file manager; with no selected sub-entry, use the last worktree root when available. | Explorer has a per-row Reveal in Finder context-menu route. There is no selected-entry action route or Workspace action binding. |
| [ACT-MACOS-project_panel-ToggleFocus](action-requirements.md) | Workspace toggles focus on ProjectPanel. The action is bound in Workspace and AgentPanel contexts; it is distinct from the ProjectPanel Toggle action's close-on-unfocus behavior. | Workspace docks and Explorer controls can show/focus the panel, but no matching typed ProjectPanel focus action is registered. |
| [ACT-MACOS-project_panel-Trash](action-requirements.md) | Move eligible selected entries to system trash. Confirm unless skip_prompt is true; include dirty-buffer context, continue after partial failures, and record eligible successful trash operations for undo when collaboration history is supported. | Explorer Delete presents a confirmation dialog and permanently deletes its target. It has no OS-trash route, multi-entry ProjectPanel action, or undoable trash record. |
| [ACT-MACOS-project_panel-Undo](action-requirements.md) | Apply the latest available ProjectPanel create, rename, paste, or trash change in transient history. History is disabled for collaboration guests; operations may fail if concurrent filesystem work changes the target. | Editor owns text-buffer undo. Explorer has no filesystem undo history or ProjectPanel action route. |

## File-operation and state contract

- Copy, Cut, and Paste operate on disjoint marked entries while excluding worktree roots. The internal ProjectPanel clipboard and selected/marked rows are transient. Copy and Cut also expose file paths to the operating-system clipboard.
- Internal paste uses the selected directory or the parent of a selected file; external OS file paths are copied into the selected directory, or the last worktree root when no entry is selected. Internal Cut moves by rename and becomes Copy after one successful paste; internal Copy duplicates. Paste/Trash/create/rename changes that the undo manager can invert are transient history over persistent filesystem mutations.
- Delete permanently removes selected entries; Trash moves them into system trash and can be undone. Both confirm by default and account for dirty buffers. The Delete action has a permanent-removal path, not a hidden trash alias.
- NewFile and NewDirectory use inline name editing and derive their target from selection or the last worktree root. Rename edits one resolved path; its Workspace adapter converts the first mark to the active selection before starting.
- CompareMarkedFiles compares the last two marked file paths in FileDiffView. NewSearchInDirectory adapts the selected ProjectPanel path to Search. OpenedEntry and SplitEntry events are separate actions and are not substitutes for this Open action.
- ProjectPanel UndoManager is in-memory and records eligible changes; Editor document undo/redo remains a separate owner and state history.

## Settings effects

| Requirement | Pinned effect | Labonair relationship |
|---|---|---|
| [SET-window-layout-preview-tabs-enable-preview-from-project-panel](settings-requirements.md) | Default true; ProjectPanel Open allows preview when enabled and focuses the item when disabled. | Explorer uses Space for preview and Enter for permanent open without this setting: no matching value or action route. |
| [SET-panels-project-panel-auto-open-on-create](settings-requirements.md) | Default true; open a newly created file after successful creation. | Explorer creates the file and refreshes its tree without opening it; no matching setting. |
| [SET-panels-project-panel-auto-open-on-paste](settings-requirements.md) | Default true; open an eligible single pasted file, including the Paste used by Duplicate. | Explorer paste and copy/paste do not automatically open the resulting file; no matching setting. |

All three are dynamic USER Settings. Static defaults and handler reads do not demonstrate Settings UI editing, save/reload, or live update behavior.

## Keymap overlays

The default macOS, Linux, and Windows chords and action arguments remain in [action-requirements.md](action-requirements.md). The focused action subset has 41 macOS profile entries across 16 IDs and 20 shared Linux/Windows profile entries across 14 IDs. The conditional Vim/Helix layer contributes 10 entries: percent for NewFile, slash for NewSearchInDirectory, d for NewDirectory, h/l for collapse/expand, Shift-D for Delete, Shift-R for Rename, p for Open, x for RevealInFileManager, and z d for CompareMarkedFiles. Profile-only bindings and contexts remain pending runtime observation in the [macOS overlay](keymap-overlay-requirements.md) and [non-macOS layer](keymap-nonmacos-requirements.md) registers.

Delete and Trash use prompt arguments in the default maps. Alternate profiles include a no-prompt Trash binding in TextMate; profile-specific routes do not change the distinction between recoverable Trash and permanent Delete.

## Required user-flow and state acceptance

- Exercise Open with preview tabs enabled/disabled for files and directories; verify selection, focus, preview replacement, and permanent activation.
- Exercise create, rename, duplicate, copy, cut, OS-file paste, internal paste, collisions, and auto-open Settings with files, directories, multiple selection, worktree roots, excluded paths, and permission failures.
- Verify Delete and Trash prompts, dirty-buffer messaging, skip-prompt arguments, partial failures, filesystem outcomes, and Undo/Redo eligibility and recovery. Confirm that permanent Delete does not appear in undo history.
- Compare two, one, and more than two marked entries; include directories, stale entries, and cross-worktree files. Verify the last-two-file ordering and FileDiffView target values.
- Invoke NewSearchInDirectory on a directory, nested file, root-level file, and multi-worktree entry; verify the Search query scope and path style.
- Exercise Collapse/Expand Selected and All with compacted ancestor rows, expanded directories, empty selection, one/multiple worktrees, and changes while operations are pending.
- Exercise Reveal with selected file/directory, no selection, removed path, and unavailable file manager; verify Workspace/AgentPanel focus contexts and profile chord precedence.
- Capture direct macOS action registration and paired ProjectPanel/Explorer states. Static Linux registry data and keymap files do not satisfy native dispatch, file recovery, or visual acceptance.

## Acceptance evidence still required

Static source review establishes 20 default-macOS-bound action intents, their cross-platform chords, the profile counts above, three pinned Settings effects, and current Explorer/Workspace/Editor/Search route gaps. Labonair has useful Explorer pointer and custom keyboard operations, but those do not supply the typed ProjectPanel actions, system file clipboard contract, recoverable trash/history, marked-file comparison, scoped directory search, or identical preview/open Settings. Native macOS registration/dispatch, prompt and cancellation behavior, file recovery, Settings save/reload, profile precedence, focus, and paired visual evidence remain open.

## Source records inspected

- Pinned actions and handlers: zed-refrence/zed/crates/project_panel/src/project_panel.rs, project_panel_settings.rs, and undo.rs.
- Pinned file and preview Settings: zed-refrence/zed/assets/settings/default.json, zed-refrence/zed/crates/workspace/src/item.rs, and [settings-requirements.md](settings-requirements.md).
- Pinned default and profile maps: default macOS/Linux/Windows keymaps, [keymap-overlay-requirements.md](keymap-overlay-requirements.md), and [keymap-nonmacos-requirements.md](keymap-nonmacos-requirements.md).
- Labonair Explorer actions and UI: crates/panel-explorer/src/panel_explorer.rs, crates/explorer-host/src/lib.rs, Workspace Editor/Search/Diff routes, and Settings catalog.
- Related classifications: [ProjectPanel unbound actions](action-classification-project-panel-actions.md) and [Workspace path-copy actions](action-classification-workspace-path-copy.md).
