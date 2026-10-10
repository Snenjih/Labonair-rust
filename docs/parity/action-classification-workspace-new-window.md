# Workspace New Window Action Classification

**Status:** Static intent, owner, and route crosswalk for one default-macOS
action; pinned dispatch, native interaction, and paired visual acceptance
remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the action and initial
workspace content; application composition owns the native window host

## Evidence boundary

This packet records the user outcome and acceptance requirements for creating
a fresh application window through `workspace::NewWindow`. The exact default
bindings are preserved in the [action requirements register](action-requirements.md).
Static source and Settings inventory checks do not prove key dispatch, window
activation, or initial content in either runtime.

This is same-session independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux
host without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace — `labonair-workspace` owns the New Window intent, Workspace state, and initial view. Application composition creates the native window and composes the Workspace. |
| Composition | The Workspace owner contributes the action through its typed command/action surface. It emits a typed request to the application composition root with the initial-content intent. The composition root supplies the native window host and mounts the requested Workspace; it does not own the new-window policy or Settings value. |
| Canonical user entry point | File > New Window and `Cmd+Shift+N` in the Workspace context on macOS. Linux and Windows defaults are retained in the action register. A platform app-menu entry may also expose the action when no app window is open; verify availability per platform. |
| Public contract | Add a Workspace-owned `WorkspaceEvent::NewWindowRequested` carrying a typed initial-content choice (`Launchpad` or `EmptyEditorTab`). The app composition root consumes it to open and activate a native window, create a fresh Workspace, and report creation errors. Keep the platform operation narrow and typed; do not add a shell command table or broad window registry. |
| Initial content | Create and activate a new application window. The `On New Window` setting chooses between the Launchpad with recent projects and an empty untitled Editor buffer. The default is Launchpad. The new window starts without implicitly copying the requesting Workspace's tabs or project root. |
| Existing windows | Leave the requesting window, its active Workspace, tabs, and persisted state intact. Repeated invocations create separate windows unless the platform prevents them; verify this behavior. |
| Registry contributions | Workspace owns the action/command contribution and `NewWindowRequested` event. Shell or application composition supplies only the window host. This inventory slice adds no command, action, event, or registry entry. |
| Dependencies | Settings supplies `on_new_window`; Workspace builds the initial view; Editor provides the empty untitled buffer; application composition creates and activates the native window. |
| Persistence | The `on_new_window` value is user-scoped and must survive save, reload, restart, and reset through the Settings owner. The initial view is per newly created window; verify how unsaved empty buffers and closed windows participate in session restoration. |
| Notifications and errors | A failed window creation or activation must preserve existing windows, report a recoverable error through the notification center, and allow retry. Cancellation is not applicable because this action selects no path. |
| Security | Do not copy project paths, project trust, credentials, or other workspace-specific state into the new window unless the user selects a project there. |
| UI-kit | Reuse existing Launchpad and Editor surfaces. Do not add a second app launcher or a feature-local window control. |

## Action requirement

| Requirement ID | Default route and context | User goal and required outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-NewWindow`](action-requirements.md) | macOS: `Cmd+Shift+N` (`Workspace`). Linux: `Shift+New` and `Ctrl+Shift+N` (`Workspace`). Windows: `Ctrl+Shift+N` (`Workspace`). | Create and activate a separate application window. Show the configured initial content, keep existing windows and Workspace state intact, and apply the user's `On New Window` preference. | **Missing.** Labonair has no user-facing Workspace New Window action or command. Startup creates the initial window; the Shell's debug-only Component Gallery can open a separate window, but that is not this workflow. |

## Settings and lifecycle requirements

| Requirement | Target behavior | Current Labonair status |
|---|---|---|
| [`SET-general-on-new-window`](settings-requirements.md) | `on_new_window` is user-scoped, defaults to `launchpad`, and accepts `launchpad` or `empty_tab`. `launchpad` opens the recent-project Launchpad; `empty_tab` opens a new untitled Editor buffer. | The parity inventory contains the target row. No matching `on_new_window` value or user-facing new-window route is wired in Labonair. Validate options, default, save/load, reload, reset, migration, and behavior in Settings and Workspace. |

The preference changes only the contents of windows created after the setting
is applied. Verify whether changing it updates an already open Launchpad or
empty tab; do not assume that it does. Closing a fresh window, restoring the
previous session, and opening another new window are separate lifecycle
transitions and require acceptance evidence.

## Required user-flow and state acceptance

- Invoke through File > New Window and `Cmd+Shift+N`; verify the default key in
  the Workspace context, keyboard discoverability, and any platform app-menu
  route that remains available without a window.
- With `on_new_window` set to `launchpad`, verify the new window activates and
  shows recent projects. With `empty_tab`, verify it activates with one empty,
  editable untitled buffer and the expected initial focus.
- Invoke from standalone and project Workspaces, with one and several existing
  windows. Verify none of their tabs, active project roots, split layout,
  focus state, or saved session data changes.
- Invoke repeatedly. Verify each result is a distinct window, all windows can
  be focused and closed independently, and no duplicate singleton view is
  substituted for a new window.
- Test saved, reloaded, restarted, and reset `on_new_window` values. Verify a
  setting change affects subsequent windows, and record its effect on windows
  already open.
- Cover no existing window, restricted displays, window creation failure,
  activation failure, and retry. Verify the existing session remains usable
  and failures are reported through the notification center.
- Record keyboard and menu focus, initial view focus, accessible window names,
  platform titlebar behavior, window bounds/minimum size, display placement,
  and narrow-window behavior for both initial views.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`main.rs`](../../crates/app/src/main.rs) calls `cx.open_window` during application startup. | Initial application window | Proves one startup window is composed; it does not provide a user action that creates more windows. |
| [`gallery.rs`](../../crates/shell/src/gallery.rs) exposes `open_gallery_window` for the debug-only Component Gallery command. | Separate-window framework evidence | The Gallery is not Workspace content, is debug-only, and does not implement `workspace::NewWindow` or `on_new_window`. |
| Local code search finds no `CommandId::NewWindow`, `workspace::NewWindow`, or `on_new_window` wiring. | Missing product route and Settings behavior | Absence in source search is static evidence only. It does not verify an equivalent native menu or keyboard route at runtime. |

No Labonair test covers the pinned action, window creation through Workspace,
new-window content selection, or preservation of existing Workspace state.
There is no pinned macOS dispatch evidence, native interaction artifact, or
paired visual capture.

## Acceptance evidence still required

Record the exact pinned and Labonair builds, OS, screen arrangement, window
bounds, scale, theme, `on_new_window` value, entry route, initial view, focus,
existing window state, and close/reopen behavior. Attach native interaction
and paired captures for both setting values. Linux source checks do not satisfy
the macOS-first acceptance requirement; Windows behavior also remains open.
