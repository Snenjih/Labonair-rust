# Workspace File and Project Open Action Classification

**Status:** Static intent, owner, and route crosswalk for one macOS-bound and
one Linux/Windows-only action; runtime dispatch, native interaction, and
paired visual acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns path selection and workspace
identity/window transitions; Editor (`labonair-editor`) owns opened document
state

## Evidence boundary

This packet specifies two distinct user workflows: opening files or folders
through the general Open flow, and explicitly opening files through the
platform Open Files flow. Their action IDs, default bindings, and contexts are
in the [action requirements register](action-requirements.md). Static source
and catalog checks support the behavior summary below; they do not prove key
dispatch, rendered picker behavior, or parity on any platform.

The requirements are written as user outcomes and acceptance checks. This is
same-session independent redevelopment, not separated-room clean-room work, as
recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS runtime interaction and paired captures are unavailable on this
Linux host without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace — `labonair-workspace` owns choosing paths, applying project/workspace identity changes, and selecting the target window. Editor — `labonair-editor` owns opening selected files as documents. |
| Composition | Shell (`labonair-shell`) composes menus, platform prompts, and application windows. It must not own file-open policy or workspace identity. |
| Canonical user entry points | On macOS, File > Open… and `Cmd+O` invoke general Open. On Linux and Windows, File > Open File… and `Ctrl+O` invoke Open Files; File > Open Folder… invokes general Open. The Linux/Windows general Open action also has the listed `Ctrl+K Ctrl+O` route. |
| Public contract | Preserve the typed Workspace path-open and identity-transition boundary. The current Labonair public path supports opening one project directory in the active workspace. Labonair has no equivalent typed multi-path open request or default-open-behavior value. |
| Editor handoff | Selected files become Editor documents in the workspace chosen by the Workspace open flow. Folder selections change or create project workspaces according to the selected action and window behavior. Exact mixed-selection outcomes require runtime acceptance. |
| Registries and commands | Workspace owns any future Open/Open Files command contributions. Shell composes those contributions. This inventory slice adds no command, keybinding, event, or registry entry. |
| Dependencies | Settings supplies the two General values below; the platform prompt supplies paths; Workspace applies the window/project transition; Editor opens selected files. Use existing `labonair-ui-kit` controls for any built-in picker surface. |
| Persistence | Persist user Settings values through the Settings owner. Persist opened Workspace identity and session state through existing Workspace lifecycle contracts. A dismissed picker changes neither. |
| Notifications and errors | Cancellation is a normal no-change result. Report prompt failures, inaccessible or missing selected paths, and failed window/workspace creation through the notification center. A failure must leave the prior workspace usable and allow retry. |
| Security | Test access to selected paths outside the current project, revoked or unavailable paths, and remote paths. Do not weaken File Finder's separate root-containment checks to implement general Open. |
| UI-kit | Use the platform dialog when configured. Any built-in keyboard-first path picker must use shared UI-kit input, list, button, focus, and empty/error states; do not add a feature-local replacement control. |

## Action requirements

| Requirement ID | Default route and context | User goal and required outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-Open`](action-requirements.md) | macOS: `Cmd+O` (global). Linux: `open` and `Ctrl+K Ctrl+O` (global). Windows: `Ctrl+K Ctrl+O` (global). | Choose one or more files and/or folders. Open the selected paths in the current or a new window according to an explicit action argument or the user's Default Open Behavior setting. | **Missing.** The local Open Project flow only selects one folder and re-roots the active Workspace. The local Open File route opens File Finder. Neither implements this general multi-path flow. |
| [`ACT-PLATFORM-workspace-OpenFiles`](action-requirements.md) | Linux and Windows: `Ctrl+O` (global). No default macOS binding. | Choose multiple files, and folders where the platform picker supports mixed file/folder selection, then open the selection in a new window. | **Missing.** Labonair has no multi-path picker or new-window Open Files route. Local Editor `CommandId::OpenFile` opens File Finder and is not this action. |

The pinned Linux action catalog records `editor::OpenFile` as a deprecated
alias for `workspace::OpenFiles`. That catalog alias is not a separate
default-map binding and does not make Labonair's Editor-owned `editor::OpenFile`
command equivalent: Labonair routes it to File Finder. Keep this action
identity distinct from [`file_finder::Toggle`](action-classification-file-finder.md).

## Settings and persistence requirements

| Requirement | Target behavior | Current Labonair status |
|---|---|---|
| [`SET-general-default-open-behavior`](settings-requirements.md) | `default_open_behavior` is user-scoped, defaults to `existing_window`, and accepts `existing_window` or `new_window`. For general Open, an explicit `create_new_window` argument overrides the setting; when omitted, the setting selects the target window behavior. Opening a project with `existing_window` adds a new Workspace in the current window; `new_window` opens a new window. Open Files creates a new window regardless of this value. | No corresponding Labonair value is wired. Validate UI/schema options, save/load, reload, reset, migration, and the resulting window behavior in the Settings and Workspace owners. |
| [`SET-general-use-system-path-prompts`](settings-requirements.md) | `use_system_path_prompts` is user-scoped and defaults to `true`. When enabled, local Open flows use system path dialogs. When disabled, use the built-in keyboard-first path picker. Non-local path sources use the built-in picker even when the setting is enabled. | No corresponding Labonair value is wired. Verify prompt choice for local and remote workspaces, focus behavior, saving/reloading, and reset through the Settings and Workspace owners. |

The picker must preserve no-change cancellation, selected-path order where the
platform provides it, and the user's configured window behavior across
restarts. Runtime persistence and ordering details are pending; do not infer
them from the static action registration.

## Required user-flow and state acceptance

### General Open — `ACT-MACOS-workspace-Open`

- Activate through the macOS File menu and `Cmd+O`; verify Linux/Windows menu
  and `Ctrl+K Ctrl+O` routes where available. Confirm the action is global and
  does not steal input from a focused text field unexpectedly.
- Choose a file, a folder, several files, several folders, and a supported
  mixture of files and folders. Verify the active tab, project identity,
  window, and recent/session state after each choice.
- Test both setting values, both explicit `create_new_window` argument values,
  and the omitted argument. Verify explicit action intent overrides the
  setting; `existing_window` adds a project as a new Workspace in the current
  window, while `new_window` opens a new window.
- Cover no project, standalone mode, an existing project, a path already open,
  a path inside the current project, a local path outside it, and a remote
  workspace.
- Verify native and built-in picker behavior with `use_system_path_prompts`
  enabled and disabled. Record keyboard navigation, pointer selection, initial
  focus, focus restoration, accessible labels, narrow-window behavior, and
  disabled/loading states if the built-in picker is used.
- Verify cancel/dismiss leaves the current tabs, project root, window, and
  persisted session unchanged. Exercise empty selection, missing or moved
  paths, inaccessible paths, prompt errors, and failed window creation; report
  recoverable failures and allow retry.

### Open Files — `ACT-PLATFORM-workspace-OpenFiles`

- On Linux and Windows, activate File > Open File… and `Ctrl+O`; confirm the
  action is separate from general Open and from File Finder.
- Select multiple files. Test directory-only and mixed file/folder selection
  on each platform, recording when the native prompt cannot represent a mixed
  selection. Verify selected files open in a new window and do not replace the
  active workspace.
- Cover no project, standalone mode, current-project and outside-project
  files, already-open files, cancellation, unavailable paths, prompt errors,
  and recovery. Confirm the Open Files window behavior remains independent of
  `default_open_behavior`.
- Verify keyboard and pointer navigation, prompt focus, focus after dismissal,
  and Settings behavior for system versus built-in path prompts.

## Existing Labonair route and test traceability

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| `CommandId::OpenFile` is Editor-owned, has local default `Cmd+Shift+O`, and resolves to `editor::OpenFile` in [`command_provider.rs`](../../crates/editor/src/command_provider.rs). The shell File menu contributes “Open File…” in [`menu.rs`](../../crates/shell/src/menu.rs). | Both action requirements | Opens File Finder; it does not implement either Workspace native path prompt. |
| [`file_finder.rs`](../../crates/workspace/src/file_finder.rs) and the Editor query model | Related file selection | Covers a workspace-root fuzzy query and one candidate at a time, not arbitrary paths, folders, multiple selection, or window creation. See the [File Finder packet](action-classification-file-finder.md). |
| `WorkspaceEvent::OpenProject`, [`WorkspaceContext`](../../crates/workspace/src/context.rs), and the shell `open_project_picker` in [`bootstrap.rs`](../../crates/shell/src/bootstrap.rs) | Related folder transition for general Open | Current picker accepts one directory and re-roots the active workspace. Context transition tests do not exercise native picker dispatch or the target multi-path/window behavior. |
| `open_file_is_not_a_second_workspace_descriptor` in [`command_provider.rs`](../../crates/workspace/src/command_provider.rs), and `context_applies_only_explicit_identity_transitions` in [`context.rs`](../../crates/workspace/src/context.rs) | Typed owner-boundary evidence | The tests establish descriptor ownership and explicit project-identity transitions only. No test verifies either pinned action ID, platform binding, native/built-in picker, multi-path selection, or new-window behavior. |

No Labonair test exercises either pinned action end to end. There is no pinned
macOS or Windows runtime dispatch evidence, native interaction artifact, or
paired visual capture. Linux source/catalog checks are static evidence only.

## Acceptance evidence still required

Record one trace per action and platform: exact build, OS, window dimensions,
scale, theme, active workspace/project, Settings values, entry route, selected
paths, window/tab/project result, cancellation or failure path, and artifact or
test link. Verify key dispatch and menu behavior against the pinned build, then
compare the same flow in Labonair. Linux host runtime access does not satisfy
the macOS-first acceptance requirement. Windows behavior also remains
unverified.
