# ProjectPanel Action Classification

**Status:** Static ProjectPanel behavior, alternate keymap, Settings, and Explorer-route classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned ProjectPanel owns tree selection and commands, with Workspace action adapters. Labonair keeps tree behavior in Explorer, project/tab/dock state in Workspace, diagnostics in Editor, Git state in Git, and persisted values in Settings.

## Evidence boundary

This packet classifies all 25 registered-unbound `project_panel` actions in the pinned Linux action inventory. None is bound in the three default platform maps. Fifteen action IDs have 17 bindings in the conditional Vim/Helix layer; `OpenPermanent` and `ScrollCursorCenter` each have two chords. The overlay and action records describe intended profile use, but Linux action registration does not prove macOS registration or runtime dispatch. No action is marked runtime-observed.

Source research and independently written requirements were produced in the same session. This is independent redevelopment, not separated-room clean-room implementation. Static source does not establish native keyboard focus, Settings save/reload behavior, preview/tab placement, transfer recovery, or visual parity.

## Owner and workflow contract

| Contract | Pinned behavior | Labonair route and gap |
|---|---|---|
| Expand and collapse a selected subtree | `ExpandSelectedEntryAndChildren` recursively expands the selected directory and descendants; `CollapseSelectedEntryAndChildren` collapses that subtree. Missing selection is a no-op. | Explorer's disclosure pointer toggles one directory at a time. No recursive typed action route was found. |
| Fold and unfold compact directory chains | `FoldDirectory` and `UnfoldDirectory` update the selected path's folded-chain state and its single-child ancestors/descendants. This changes compact-chain presentation, not ordinary subtree expansion. | Explorer can automatically compress single-child chains when `file_manager.explorerFoldSingleChildDirs` is enabled. There is no selected-entry Fold/Unfold action. |
| Open a file | `OpenPermanent` opens the selected file as a permanent editor item and focuses it. `OpenMarkdownPreview` opens only a selected Markdown file in the Markdown preview view. | Explorer Enter/double-click and its Open menu use Workspace's permanent file route. The Preview menu supports Markdown and other previewable formats; no Markdown-only action dispatch exists. |
| Open in a split | `OpenSplitVertical` and `OpenSplitHorizontal` permanently open the selected file in a new split using the pinned vertical/horizontal direction. | Workspace can split the active pane, but `ExplorerHost::open_file` has no split intent and Explorer exposes no selected-file split action. |
| Navigate the tree | `ScrollUp` and `ScrollDown` move selection by half the rendered entry count. Cursor Top/Center/Bottom align the current selection without changing it. Directory actions select the next/previous visible directory. Git-entry actions select the next/previous file whose status reports modified index or worktree counts, expanding ancestors. Parent selects the containing directory, or the first visible entry when no selected child has a parent. | Explorer supports pointer selection, active-file reveal, and list scrolling. Its key handler covers open/preview, clipboard, and Escape; no matching navigation, selection, alignment, or action-dispatch routes were found. Git decorations exist. The typed diagnostic decoration hook has no in-repository provider and currently supplies no diagnostics. |
| Navigate diagnostics | Next/previous diagnostic actions scan files in tree order and select a file matching the `severity` argument. The argument accepts one severity or a range; its default spans Error through Hint. | Editor has a document diagnostic/navigation route, but Explorer's diagnostic hook is empty and is not connected to a project-wide file diagnostic source. |
| Change file visibility | `ToggleHideGitIgnore` and `ToggleHideHidden` update and persist `project_panel.hide_gitignore` and `project_panel.hide_hidden`; both default to `false`. Settings changes drive ProjectPanel visibility. | Explorer has a transient toolbar toggle for dot-prefixed hidden names. `file_manager.explorerShowHiddenByDefault` is declared with default `false`, but no Explorer consumer was found. Git-ignored entries can be rendered muted; no hide-ignored control or matching action was found. |
| Toggle or remove the panel/project entry | Workspace's `Toggle` toggles ProjectPanel focus and closes the panel if focus toggling returns false. `RemoveFromProject` removes the selected root worktree from the current project; the context menu offers it on non-collaborative roots. It does not delete files. | Workspace dock controls can show or close Explorer, but no matching focus-toggle action was found. Labonair currently routes a project around one active root; no selected-worktree removal route was found. |
| Download remote entries | `DownloadFromRemote` collects selected files and recursively collects files below selected directories, preserving the selected directory name and relative paths. It prompts for a destination, creates parents, downloads files sequentially, and reports progress/completion through the pinned toast route. A cancelled picker or empty selection has no download effect. | ExplorerHost's remote contract reads directory entries; Explorer reports remote open/preview operations unsupported and exposes no selected-entry download route. SFTP/Transfers are separate owners and have no Explorer selection adapter. Any future progress message must use Labonair's statusbar notification center, which has no toast system. |

The two pinned visibility settings are recorded as `SET-panels-project-panel-hide-gitignore` and `SET-panels-project-panel-hide-hidden`. Labonair's declared `file_manager.explorerShowHiddenByDefault` is a different value and currently has no Explorer consumer; the Explorer toolbar toggle changes transient model state.

## Vim/Helix profile bindings

The conditional Vim/Helix overlay (`ProjectPanel && not_editing`) maps 15 action IDs across 17 entries:

- `Enter` and `t` open permanently; `v` opens a vertical split; `o` opens a horizontal split.
- `] c` / `[ c` select next/previous Git-modified entries; `] d` / `[ d` select next/previous diagnostics.
- `}` / `{` select next/previous directories; `-` selects the parent.
- `Ctrl-U` / `Ctrl-D` move selection by half a page.
- `z t`, `z z`/`z c`, and `z b` align the selection at top, center, and bottom.

The 25 actions are absent from the three default platform maps that define the unbound catalog. These Vim/Helix entries are recorded in [`keymap-overlay-requirements.md`](keymap-overlay-requirements.md), but the pinned Linux runtime dump does not verify action registration on macOS. Labonair has no corresponding typed action map or native invocation evidence.

## Action requirements

| Requirement | Pinned intent | Labonair route and current gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-project_panel-CollapseSelectedEntryAndChildren](registered-actions-unbound.md) | Collapse the selected directory and all expanded descendants; no selection is a no-op. | Explorer can collapse one directory by pointer; no recursive action route. |
| [ACT-REGISTRY-LINUX-project_panel-DownloadFromRemote](registered-actions-unbound.md) | Download selected remote files or recursive directory contents into a user-selected local destination, with progress and completion feedback. | No selected Explorer-to-SFTP/Transfers download route; remote Explorer currently lists directories only. |
| [ACT-REGISTRY-LINUX-project_panel-ExpandSelectedEntryAndChildren](registered-actions-unbound.md) | Recursively expand the selected directory and its descendants; no selection is a no-op. | Explorer expands one directory at a time; no recursive action route. |
| [ACT-REGISTRY-LINUX-project_panel-FoldDirectory](registered-actions-unbound.md) | Fold the selected compact single-child directory chain in the visible ProjectPanel. | Explorer supports setting-controlled automatic chain compression; no selected-entry fold action. |
| [ACT-REGISTRY-LINUX-project_panel-OpenMarkdownPreview](registered-actions-unbound.md) | Open the selected Markdown file in the Markdown preview view; other selections are ignored. | Generic Explorer Preview supports Markdown but also other formats; no matching action route. |
| [ACT-REGISTRY-LINUX-project_panel-OpenPermanent](registered-actions-unbound.md) | Open the selected file permanently and focus it. | Enter, double-click, and the row menu open permanently; no matching typed action dispatch. |
| [ACT-REGISTRY-LINUX-project_panel-OpenSplitHorizontal](registered-actions-unbound.md) | Permanently open the selected file in a horizontal split. | Workspace can split panes, but Explorer has no split-open host intent or action. |
| [ACT-REGISTRY-LINUX-project_panel-OpenSplitVertical](registered-actions-unbound.md) | Permanently open the selected file in a vertical split. | Workspace can split panes, but Explorer has no split-open host intent or action. |
| [ACT-REGISTRY-LINUX-project_panel-RemoveFromProject](registered-actions-unbound.md) | Remove each selected project-root worktree from the project without deleting its files. | No selected-root removal route for Labonair's single-root project workflow. |
| [ACT-REGISTRY-LINUX-project_panel-ScrollCursorBottom](registered-actions-unbound.md) | Scroll the selected entry to the bottom of the ProjectPanel viewport; no selection is a no-op. | No selected-row alignment action; ordinary list scrolling and active-file reveal are separate. |
| [ACT-REGISTRY-LINUX-project_panel-ScrollCursorCenter](registered-actions-unbound.md) | Center the selected entry in the ProjectPanel viewport; no selection is a no-op. | No selected-row alignment action. |
| [ACT-REGISTRY-LINUX-project_panel-ScrollCursorTop](registered-actions-unbound.md) | Scroll the selected entry to the top of the ProjectPanel viewport; no selection is a no-op. | No selected-row alignment action. |
| [ACT-REGISTRY-LINUX-project_panel-ScrollDown](registered-actions-unbound.md) | Move selection downward by half the currently rendered entries. | No Explorer action or keyboard selection-navigation route. |
| [ACT-REGISTRY-LINUX-project_panel-ScrollUp](registered-actions-unbound.md) | Move selection upward by half the currently rendered entries. | No Explorer action or keyboard selection-navigation route. |
| [ACT-REGISTRY-LINUX-project_panel-SelectNextDiagnostic](registered-actions-unbound.md) | Select and reveal the next file with a matching diagnostic. `severity` accepts one severity or a range and defaults from Error through Hint. | No project-wide Explorer diagnostic provider; current Explorer diagnostic hook is empty. |
| [ACT-REGISTRY-LINUX-project_panel-SelectNextDirectory](registered-actions-unbound.md) | Select and reveal the next visible directory. | No Explorer action or keyboard directory-navigation route. |
| [ACT-REGISTRY-LINUX-project_panel-SelectNextGitEntry](registered-actions-unbound.md) | Select and reveal the next file with modified index/worktree status. | Git decorations are rendered, but Explorer has no Git-entry navigation action. |
| [ACT-REGISTRY-LINUX-project_panel-SelectParent](registered-actions-unbound.md) | Select the selected entry's parent; if the selected entry has no parent, select the first visible entry. | Pointer tree selection exists; no parent-selection action route. |
| [ACT-REGISTRY-LINUX-project_panel-SelectPrevDiagnostic](registered-actions-unbound.md) | Select and reveal the previous file with a matching diagnostic. `severity` accepts one severity or a range and defaults from Error through Hint. | No project-wide Explorer diagnostic provider; current Explorer diagnostic hook is empty. |
| [ACT-REGISTRY-LINUX-project_panel-SelectPrevDirectory](registered-actions-unbound.md) | Select and reveal the previous visible directory. | No Explorer action or keyboard directory-navigation route. |
| [ACT-REGISTRY-LINUX-project_panel-SelectPrevGitEntry](registered-actions-unbound.md) | Select and reveal the previous file with modified index/worktree status. | Git decorations are rendered, but Explorer has no Git-entry navigation action. |
| [ACT-REGISTRY-LINUX-project_panel-Toggle](registered-actions-unbound.md) | Toggle ProjectPanel focus; close the panel when the focus toggle returns false. | Workspace dock controls open/close Explorer, but no matching focus-toggle action route. |
| [ACT-REGISTRY-LINUX-project_panel-ToggleHideGitIgnore](registered-actions-unbound.md) | Toggle and persist `project_panel.hide_gitignore` (default `false`). | No local hide-ignored setting or action; ignored rows may be muted as decoration. |
| [ACT-REGISTRY-LINUX-project_panel-ToggleHideHidden](registered-actions-unbound.md) | Toggle and persist `project_panel.hide_hidden` (default `false`). | Toolbar toggles dot-hidden visibility transiently; the declared default setting has no Explorer consumer. |
| [ACT-REGISTRY-LINUX-project_panel-UnfoldDirectory](registered-actions-unbound.md) | Reveal the selected compact single-child directory chain and its eligible ancestors. | Explorer can automatically compress single-child chains through a setting; no selected-entry unfold action. |

## Required user-flow and state acceptance

- Invoke tree subtree expansion/collapse and Fold/Unfold separately on leaf, empty, single-child, branching, nested, and unloaded directories. Verify how recursive expansion loads descendants and how selection/viewport behave.
- Open text and Markdown files permanently and in preview, then use both split orientations. Verify tab reuse, active pane, preview replacement, unsupported extensions, errors, and narrow-window layout.
- Navigate next/previous directory, Git-modified entry, and diagnostic with default and explicit severity filters. Verify traversal wrap/bounds, ancestor expansion, absent providers, and no-match behavior.
- Align a selected row to top/center/bottom and use half-page movement with short, long, and filtered lists; confirm whether scrolling changes selection or only viewport position.
- Toggle ignored and hidden visibility, then save/reload Settings. Verify persisted pinned values, runtime refresh, and Labonair's distinct transient toolbar and currently unconsumed default setting.
- Remove project roots without deleting files. Download one file and nested directories, then verify destination prompt cancellation, path preservation, conflicts, progress/completion, permission errors, cancellation, and credential/path redaction through the owning transfer flow.
- Exercise the Vim/Helix chords under `ProjectPanel && not_editing`, verify chord precedence and unaffected text-editing contexts, and capture direct macOS registration and dispatch. Default macOS has no binding for these actions.

## Acceptance evidence still required

Static review establishes 25 action semantics, two persistent pinned Settings effects, the 15-action/16-binding Vim/Helix profile, and local Explorer/Workspace owner gaps. No Labonair ProjectPanel action dispatch was added or observed. Native macOS action registration, profile dispatch, Settings persistence/refresh, split and preview placement, diagnostic/Git navigation, remote transfer and recovery, focus, and paired visual captures remain open. Pinned remote-download progress uses a toast; a Labonair implementation must follow its statusbar notification-center contract.

## Source records inspected

- Pinned ProjectPanel declarations, actions, settings handlers, tree selection, open/split/preview, remote download, and worktree removal: `zed-refrence/zed/crates/project_panel/src/project_panel.rs` and `project_panel_settings.rs`.
- Pinned diagnostic argument schema and severity default: `zed-refrence/zed/crates/project/src/project_settings.rs` and `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Pinned Vim/Helix bindings: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Labonair Explorer tree, keyboard, hidden-file toggle, folding, preview, diagnostics hook, and remote error behavior: `crates/panel-explorer/src/panel_explorer.rs`.
- Labonair Explorer host operations: `crates/explorer-host/src/lib.rs`; Workspace file/preview/pane/project routes: `crates/workspace/src/workspace.rs`.
- Labonair Explorer and remote-transfer owners: `docs/capabilities.md`; Settings values: `crates/settings/src/concrete.rs` and `docs/parity/settings-requirements.md`.
