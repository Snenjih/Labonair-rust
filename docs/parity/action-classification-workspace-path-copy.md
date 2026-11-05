# Workspace Path Copy Action Classification

**Status:** Static intent, selection/path behavior, owner routes, and current
Labonair Explorer menu; native dispatch, cross-platform clipboard behavior,
and paired visual acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the shared path-copy action
intent and key route. Explorer, Outline, and Git surfaces provide their
selected eligible path or paths.

## Evidence boundary

This packet distinguishes an absolute filesystem path from a path relative to
the selected project/worktree. It records each surface's selection behavior,
the project path style, and Labonair's current Explorer-only equivalent.
Static inspection does not establish native key dispatch or clipboard
behavior on macOS, Linux, or Windows.

The requirements are independently stated for Labonair's Workspace model.
This is same-session independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux host
without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) defines Copy Path and Copy Relative Path; Explorer, Outline, and Git consumers resolve their own current selection. No new registry is needed. |
| Canonical user entry points | Context menus expose both actions for eligible filesystem rows. macOS binds `Cmd+Alt+C` and `Cmd+Alt+Shift+C` in OutlinePanel when not editing, ProjectPanel, and Git ChangesList outside branch/repository selectors. Linux binds Copy Path to `Alt+Insert` or `Ctrl+Alt+C` in Git ChangesList; Copy Relative Path to `Alt+Shift+Insert` or `Alt+Ctrl+Shift+C` in OutlinePanel, ProjectPanel, and Git ChangesList. Windows binds Copy Path to `Shift+Alt+C` in Git ChangesList; Copy Relative Path to `Ctrl+Shift+Alt+C` in OutlinePanel and `Ctrl+K`, then `Ctrl+Shift+C`, in ProjectPanel and Git ChangesList. |
| Public typed contract | Copy the selected eligible path string to the system clipboard without changing files. Copy Path produces an absolute path. Copy Relative Path produces a path from the selected project's worktree/repository root, formatted with that project's path style. ProjectPanel copies each effective selected entry as one line; OutlinePanel and Git ChangesList copy their single eligible selected entry. |
| Selection and empty state | ProjectPanel supports its effective multi-selection. OutlinePanel uses its selected entry and its keyboard binding is disabled while editing; relative paths are unavailable for search and outline-symbol rows. Git uses the selected ChangesList row, while branch and repository selectors are excluded from the key context. If no eligible path resolves, leave the clipboard unchanged or allow the action to propagate; never write a stale path. |
| State and persistence | The only state change is replacing system clipboard contents. No setting, saved preference, filesystem mutation, or notification is introduced. A successful copy does not close or change the selection. |
| Path formatting | Absolute paths include the worktree/repository root and use the path's display form. Relative paths are rooted at the selected worktree/repository and use the project's configured path style (including separators). Multi-selection output is newline-separated; acceptance must record item ordering and line endings. |
| Failure and security | Missing selection, unsupported row, or path lookup failure must not produce a stale/partial value. Copying is an explicit user disclosure to the operating system clipboard; do not log the path or copy it automatically. |
| Events, registry, and UI-kit | Reuse the existing Workspace action names and shared context-menu controls. Add no clipboard registry, duplicate global action table, local menu style, or dependency. |

## Action requirements

| Requirement ID | Binding contexts | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-CopyPath`](action-requirements.md) | See the [full platform map](action-requirements.md): absolute-path copy in OutlinePanel, ProjectPanel, and Git ChangesList on macOS; Git ChangesList bindings on Linux and Windows. | Copy the selected eligible absolute path to the clipboard. ProjectPanel copies all effective selected entries separated by newlines; Outline and Git use one selected row. | Explorer context menu copies one right-clicked path. No matching shared key action, Outline route, or Git ChangesList route is present. |
| [`ACT-MACOS-workspace-CopyRelativePath`](action-requirements.md) | See the [full platform map](action-requirements.md): project-relative copy in all three panel contexts on macOS and Linux; Outline plus Project/Git platform sequences on Windows. | Copy selected path(s) relative to the containing worktree/repository, using project path style. Unsupported Outline row types have no relative path. | Explorer context menu copies one right-clicked path relative to its root. No matching shared key action, Outline route, or Git ChangesList route is present. |

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`PanelExplorer` row context menu](../../crates/panel-explorer/src/panel_explorer.rs) | Copy Path writes the target path; Copy Relative Path writes the path prepared relative to the Explorer root. Both close the context menu after copying. | The route is a pointer menu for one row, not the pinned shared keyboard action or a Git/Outline selection route. Multi-selection and platform path style require runtime checks. |
| [`SftpView` path menu](../../crates/workspace/src/views/sftp.rs) | Copies a selected remote SFTP entry path. | This is an SFTP-owned remote path action with no relative-path counterpart; it does not establish the Workspace action contract. |
| `panel-scm` current source-control surface | No Copy Path or Copy Relative Path command/menu route was found. | Git ChangesList path copying remains missing. |
| Current Outline surface | No matching OutlinePanel path-copy action route was found. | The pinned OutlinePanel actions remain missing locally. |

## Required user-flow and state acceptance

- In each eligible context, select one file and one directory, invoke both
  actions from keyboard and context menu, and compare the exact clipboard text.
- In ProjectPanel, select multiple files/directories across visible worktrees.
  Verify one path per line, deterministic selection order, absolute roots, and
  the relative root used for each item.
- In OutlinePanel, test a filesystem entry, a folded directory, an outline
  symbol, a search result, no selection, and inline edit mode. Confirm the
  keyboard action only runs in its declared context and unavailable relative
  paths do not overwrite the clipboard.
- In Git ChangesList, test files, directories, and repository/worktree edge
  cases. Verify the selected entry's repository-relative path and absolute
  path. Confirm actions do not trigger from branch or repository selectors.
- Test POSIX and Windows separators, multi-root workspaces, remote roots,
  paths containing spaces or non-ASCII characters, clipboard write failures,
  and a previously populated clipboard after an ineligible selection.
- Compare paired macOS context-menu/focus states for Explorer, OutlinePanel,
  and Git ChangesList. Record exact build, OS, selected rows, clipboard text,
  path style, theme, scale, and window bounds.

## Acceptance evidence still required

For each trace, record build and commit, platform, action/key context, panel,
selection, worktree/repository roots, project path style, output string,
clipboard result, focus/selection after copying, and any fallback/propagation
when no path exists. Include paired native macOS captures for each panel and
keyboard context. Labonair's one-row Explorer menu does not establish the
multi-selection, Outline, Git, or shared-key behavior.
