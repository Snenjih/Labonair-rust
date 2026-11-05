# Workspace Close Window Action Classification

**Status:** Static intent, Workspace ownership, key contexts, and current
Labonair close routes; native dispatch, save/cancel outcomes, session restore,
and paired visual acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns preparing and closing a
Workspace window. Application composition owns process-level quit policy.
Settings and Skill Creator own their secondary windows.

## Evidence boundary

This packet distinguishes closing one window from closing a tab, closing a
project, and quitting the app. It records the pinned platform/context bindings,
the save and active-call gates, and Labonair's native-window callback and
current tab/pane shortcut routes. Static inspection does not establish native
dispatch, modal appearance, or cross-platform window policy.

The requirements are independently stated for Labonair's Workspace model.
This is same-session independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux host
without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) coordinates Workspace close preparation and window removal. Application composition decides what happens after the last window closes. Secondary-window owners close their own Settings or Skill Creator surface. |
| Canonical user entry points | The pinned File menu exposes Close Window. macOS binds `Cmd+Shift+W` globally; Linux and Windows bind `Ctrl+Shift+W` globally. SettingsWindow also binds `Cmd/Ctrl+W` and Escape; SkillCreator and its Editor bind `Cmd/Ctrl+W`. Windows additionally binds `Alt+F4` in Terminal. Labonair has the OS window close affordance, but no Workspace Close Window command or File-menu item. |
| Public typed contract | Close the current Workspace window only after its close-preparation flow succeeds. Dirty items may be serialized for hot exit when the Workspace remains reachable; remaining items require save/discard handling. Cancelling keeps the window and dirty items open. A failed save may offer Close Without Saving or Cancel. If this is the only Workspace in an active call, ask whether to close and hang up; Cancel keeps the window open. Close Project and app Quit are separate operations. |
| Window and process scope | Closing a window removes that window after successful preparation. Other windows remain. Closing the final window can exit the process according to platform/application policy, but the action itself targets one window. Verify the final-window result on each supported platform. |
| State and persistence | The close flow may serialize dirty content, update session membership, flush window/workspace state, and then remove the window. Session restore and the final-window policy affect whether state is available when reopening. Verify close, cancellation, and relaunch outcomes separately. |
| Settings | No new setting. Record existing session-restore and final-window settings where relevant. Labonair's `confirmQuitWithActiveSessions` is a running-shell safeguard in the native-window close callback; it is not a dirty-editor save prompt and does not establish the pinned Workspace close flow. |
| Cancellation and failure | A cancelled active-call or dirty-item prompt aborts closure and leaves the window usable. A failed close must clear any transient “removing” state so a later close attempt can proceed. Test save failure, serialization failure, multiple dirty items, and a second open window. |
| Events and registry contributions | Workspace handles its lifecycle; shell/application composition handles window creation and process policy. Add no duplicate window registry or shell-wide close table for this inventory slice. |
| Notifications and UI-kit | Reuse shared dialogs and the notification center. A close-preparation prompt is distinct from a transient notification. Do not add local close-window styling in this inventory slice. |

## Action requirements

| Requirement ID | Platform binding and context | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-CloseWindow`](action-requirements.md) | macOS: `Cmd+Shift+W` (`<global>`); `Cmd+W` and `Escape` (`SettingsWindow`); `Cmd+W` (`SkillCreator` and `SkillCreator > Editor`). Linux: `Ctrl+Shift+W` (`<global>`); `Ctrl+W` and `Escape` (`SettingsWindow`); `Ctrl+W` (`SkillCreator` and its Editor). Windows: `Ctrl+Shift+W` (`<global>`); `Alt+F4` (`Terminal`); `Ctrl+W` (`SettingsWindow`, `SkillCreator`, and its Editor). | Close the current window after its owning surface's close flow succeeds. The Workspace path handles dirty items and, for a sole Workspace in an active call, asks before hanging up. | Native OS close is intercepted for active terminal/SSH shells and session snapshot handling. No Workspace close-window action or File-menu route exists. `Cmd+W` closes the active tab; in Terminal context `Cmd+Shift+W` closes the active pane, rather than the window. Settings has its own Escape/header close behavior. |

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`window.on_window_should_close` in shell bootstrap](../../crates/shell/src/bootstrap.rs) | Persists window geometry. When `confirmQuitWithActiveSessions` is enabled and a local/SSH shell is running, the first attempt posts a notification and blocks closure; retrying within three seconds proceeds. It saves or clears the session snapshot according to `sessionRestore`, then flushes settings. | This is a native window-close callback, not a Workspace action with dirty-editor/call preparation. Runtime behavior and settings combinations remain unverified. |
| [`CloseTab` and `ClosePane` descriptors](../../crates/workspace/src/command_provider.rs) | `Cmd+W` closes the active universal tab; Terminal-context `Cmd+Shift+W` closes the active split pane. | Neither closes a Workspace window. The latter conflicts with the pinned global close-window shortcut while Terminal is focused. |
| [`SettingsView::request_close`](../../crates/settings-ui/src/view.rs) | Escape and the header close button dismiss Settings; a windowed Settings view removes its own window. | It does not expose the pinned `CloseWindow` Workspace lifecycle action. |
| Workspace close-request flow | Dirty Editor tabs and configured running Terminal tabs can require per-tab confirmation. | This flow is used when closing tabs; the native window-close callback does not route through it. |

The action is not equivalent to `CloseActiveDock`, `CloseTab`,
`CloseProject`, or Quit. In particular, Labonair's OS close behavior can warn
about running shells while its tab-close behavior prompts for dirty Editor
content; neither alone implements the pinned Workspace close lifecycle.

## Required user-flow and state acceptance

- Invoke the File menu and global shortcut with one Workspace window, then
  with another window left open. Confirm only the targeted window closes and
  the other window retains its content and focus.
- Exercise clean content, one dirty item, multiple dirty items, serializable
  dirty content, serialization failure, save success/failure, Save all,
  Discard all, and Cancel. Verify the window remains usable after cancellation
  and that no dirty data disappears unless the user discards it.
- With an active call, verify the close-and-hang-up prompt and cancellation
  when this is the only Workspace; verify the prompt condition when another
  Workspace window exists.
- Verify closing the final window under each supported platform's process
  policy, both with session restore enabled and disabled. Reopen the app and
  confirm the documented Workspace, window geometry, tabs, panes, and dirty
  content recovery outcome.
- Exercise each `SettingsWindow`, `SkillCreator`, and Windows Terminal context
  binding. Confirm each closes only its intended window/surface. Compare
  `Cmd+Shift+W` in Labonair Terminal (currently Close Active Pane) with the
  pinned global Close Window behavior; check user overrides and conflicts.
- Compare native titlebar close, menu, and keyboard paths. Record exact binary,
  OS, window count, active context, dirty/session/call state, settings, outcome,
  theme, scale, and window bounds. Capture paired macOS states for confirmation,
  cancellation, successful close, and remaining-window focus.

## Acceptance evidence still required

For each trace, record build and commit, platform, input route and action
context, Workspace/window count, active view and session/call state, dirty
items, save/hot-exit choice, prompt result, session snapshot, surviving windows,
process state, and restart result. Include paired native macOS captures for the
menu, prompt, cancelled close, and remaining-window focus. The OS close button,
session notification, tab close command, and Terminal pane shortcut do not
establish acceptance for the pinned Workspace action.
