# Workspace Add Folder to Project Action Classification

**Status:** Static action scope, multi-root project behavior, persistence path,
and current Labonair route; native picker behavior, partial failures, and paired
visual acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the action and adds roots to
the active project. Filesystem supplies path and watcher behavior. Explorer
owns the presentation of resulting project roots.

## Evidence boundary

This packet distinguishes adding folders to an existing project from opening a
folder as a new project or Space. The action accepts multiple directories,
preserves the current project and its open content, and makes selected roots
visible in that project's Explorer. A collaboration-owned project rejects the
operation. Static inspection establishes the action handler's path selection
and project-root calls, but not native picker interaction, remote-root success,
or user-visible recovery for individual path failures.

The requirements are independently stated for Labonair's Workspace,
Filesystem, and Explorer contracts. This is same-session independent
redevelopment, not separated-room clean-room work, as recorded in the
[parity README](README.md#provenance-and-source-boundary). Pinned macOS
interaction and paired captures are unavailable on this Linux host without
`DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) owns action dispatch and the active project's root set. Filesystem (`labonair-filesystem`) supplies filesystem and watcher contracts; Explorer (`labonair-panel-explorer`) presents the resulting roots. |
| Canonical user entry points | Pinned File > Add Folder to Project; the ProjectPanel project-root context menu's Add Folders to Project; macOS `Cmd+Shift+A`, Linux and Windows `Ctrl+Shift+A` in `RecentProjects` or its nested `Picker > Editor` context. Labonair's File menu offers Open Project, which changes the active Space's single project root; its Explorer has no add-root action. |
| Selection | Present a directory-only picker that accepts multiple folders. Each selected directory is attempted as an additional visible root in the current project. Preserve existing roots, open tabs, and current project identity. Selecting a folder already covered by a project root must not create a duplicate root. |
| Cancellation and result | Cancelling selection makes no project changes. For accepted selections, keep successful additions if another selected directory fails; report partial completion and retain enough information to retry failed paths without duplicating successes. The pinned route attempts paths individually and logs path errors; user-visible partial-failure recovery is still unverified. |
| Collaboration and remote projects | A project shared through collaboration rejects adding folders with an error. The pinned action has no equivalent explicit guard for other remote projects; verify whether their project-aware picker can select and add remote roots and how permission and transport failures appear. |
| Project and Explorer state | Newly accepted paths join the current project's visible root/worktree set; they do not replace the project root, create a new Workspace, or open each directory as an Editor item. The Explorer reflects every successful root and the final selection deterministically. Opening/removing roots through other flows must leave this action's state consistent. |
| Settings and persistence | No action-specific setting. Added roots participate in the existing project/worktree path and Workspace grouping/persistence lifecycle. Verify the root set after project/window restore, and recovery when a saved path moved or disappeared. |
| Events and registry contributions | Keep the action owned by Workspace. Root changes use the existing typed project/worktree lifecycle and Explorer projection; add no general shell file-operation table or second Explorer action owner. |
| Notifications and failure | Cancellation is silent. The pinned collaboration restriction is reported to the user; the handler logs per-path failures, but the visible outcome of failed native prompts or root creation requires runtime verification. Do not present a partial result as complete without recording which roots succeeded. |
| UI-kit | Use the native or project-aware shared directory picker and existing Explorer/workspace components. Add no local duplicate picker or root-list style. |

## Action requirement and current route

| Requirement ID | Pinned bindings and contexts | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-AddFolderToProject`](action-requirements.md) | macOS `Cmd+Shift+A`; Linux and Windows `Ctrl+Shift+A`; context `RecentProjects` or `RecentProjects > Picker > Editor` | Choose one or more directories and add them as visible roots to the current project; reject a collaboration-owned project. | No Add Folder to Project command or menu action. The nearest local route is Open Project, which replaces the active Space's one root; it does not add multiple roots to the same project. |

The project-root context menu is a separate pointer entry point using the same
Workspace action. The pinned File menu entry is also distinct from Open, Open
Files, Open Recent, Open Remote, and New Window. Do not treat those routes as
equivalent to adding roots to the current project.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`WorkspaceContext`](../../crates/workspace/src/context.rs) and [`Workspace::open_project_in_place`](../../crates/workspace/src/workspace.rs) | Own the current project identity and the Open Project transition. | Labonair records one root for each Project Space. Open Project re-roots that Space instead of retaining multiple roots. |
| [`WorkspaceEvent::OpenProject`](../../crates/workspace/src/workspace.rs) and the shell [`open_project_picker`](../../crates/shell/src/bootstrap.rs) | Reuse the native directory picker and route the chosen root to Workspace. | The picker allows one folder and labels it Open Project; it does not add roots or preserve a multi-root project set. |
| [`WorkspaceCommandProvider`](../../crates/workspace/src/command_provider.rs) and [`File menu command registration`](../../crates/shell/src/menu.rs) | Supply the current Open Project entry point and Workspace-owned command route. | No Add Folder to Project descriptor, action handler, File menu item, or default key binding is registered. |
| [`Explorer capability contract`](../capabilities.md) | Explorer owns the file-tree surface and filesystem presentation. | There is no Explorer action for adding multiple project roots. |

The current local Open Project and New Space from Folder flows are useful
related directory-picker routes, but neither adds folders to an existing
project. The accepted universal-tab model does not change this root ownership
gap.

## Required user-flow and state acceptance

- Add one directory and several directories to a project with existing tabs
  and roots. Verify the Workspace identity and open content remain unchanged
  while all successful roots appear in Explorer.
- Select nested directories, an existing root, a folder already inside a
  root, and the same folder twice. Verify root de-duplication, stable ordering,
  and the selected Explorer entry.
- Cancel the picker, deny filesystem access, choose a deleted or unreadable
  path, and inject a root-creation failure. Verify no partial state on cancel;
  for multi-select failures, record successful roots, visible errors, and
  retry behavior.
- Exercise a collaborative project and verify the explicit rejection. Test
  local and non-collaborative remote projects separately, including access
  loss during selection and scan.
- Restart and reopen the project/window. Verify added roots are restored,
  missing paths are reported and recoverable, and removing one root does not
  discard other roots or project data.
- Invoke via the File menu, project-root context menu, and each platform
  binding. Verify keymap context, user overrides, conflict behavior, and that
  Open Project and New Space retain their distinct meaning.
- Compare paired native macOS captures for both menus, directory picker,
  multi-root Explorer layout, partial-error feedback, focus, and narrow
  window behavior.

## Acceptance evidence still required

For each trace, record build and commit, platform, entry point and context,
project identity, original and selected root paths, picker result or
cancellation, per-root success/failure, visible ordering and selection,
notifications, persisted path set after restore, and recovery behavior. Include
paired macOS captures with window bounds, theme, scale, and project fixture.
Static handler inspection does not establish native picker, remote, persistence,
or partial-failure acceptance.
