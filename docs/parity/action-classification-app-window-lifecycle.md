# Application and Window Lifecycle Action Classification

**Status:** Static action, settings, and current-route requirements; native
dispatch, quit-preparation interaction, and paired acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Platform integration supplies native app/window behavior. Workspace
owns the fullscreen presentation contract and Settings values remain with
Settings. `labonair-shell` currently composes the menu and window adapters; it
does not become a separate lifecycle capability owner.

## Evidence boundary

This packet independently states the app/window lifecycle workflows and
current Labonair routes. Static action metadata, keymaps, Settings defaults,
and source review establish the action contexts, platform bindings, quit
preparation, and present handlers; they do not establish native dispatch or
user-observed behavior. Work in this session is independent redevelopment,
not separated-room clean-room implementation, as recorded in the
[parity README](README.md#provenance-and-source-boundary). Native macOS and
Linux/Windows runtime acceptance and paired captures remain pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | `Hide`, `HideOthers`, `Minimize`, and `Quit` invoke stateless native application/window behavior through platform integration. The current tree has no separate lifecycle capability crate; `labonair-shell` composes the native actions and remains an adapter, not a feature owner. `ToggleFullScreen` is a Workspace-context window action; Settings owns its value and platform integration chooses the window mode. No new product capability or crate is assigned. |
| App lifecycle entry | The application menu exposes Hide, Hide Others, Show All, and Quit. Hide does not quit the process. Hide Others leaves Labonair visible and hides other applications. Minimize affects the current window. Show All is a separate registered menu action without a default keybinding. |
| Fullscreen entry | `ToggleFullScreen` is available in the View menu and the `Workspace` key context. It acts on the current window and is distinct from Workspace Pane/Panel zoom and the native window Zoom action. |
| Keyboard defaults | macOS binds Hide `cmd-h`, Hide Others `alt-cmd-h`, Minimize `cmd-m`, and Quit `cmd-q`; Workspace Full Screen binds `fn-f` and `ctrl-cmd-f`. Linux and Windows bind Quit `ctrl-q` and Workspace Full Screen `f11`; the other three defaults are macOS-only. Terminal contexts can shadow `ctrl-q` so it reaches the PTY. |
| Quit settings and preparation | The app-global JSON setting `confirm_quit` defaults to `false` and controls the optional application confirmation prompt. After confirmation, Zed prepares each Workspace window for close, awaits save/close decisions, and flushes session serialization before quitting. A canceled or failed preparation keeps the app open. `confirm_quit` has no Settings-page control and is added to the JSON-only Settings requirements. |
| Fullscreen setting | `fullscreen_mode` defaults to `native`. On macOS, `native` uses system fullscreen in a separate Mission Control Space; `simple` resizes the window across the screen bounds. The setting is on Window & Layout and is marked macOS-only. Other platforms use native fullscreen behavior. |
| Labonair state and failure | Labonair's Quit action calls `cx.quit()` directly. Its native-window close callback separately checks `confirmQuitWithActiveSessions` (default true): a running terminal/SSH shell blocks the first close request and posts a notification; retry within three seconds proceeds. This is not Zed's all-window dirty-buffer/save preparation. Labonair's current editor has no dirty-buffer close workflow to compare. |
| Events and registry contributions | No cross-capability event or discovery registry is needed. The app menu supplies native actions; Keymap supplies configurable action bindings where allowed. Labonair currently contributes menu actions and a shell native-window command adapter. |
| Settings and persistence | Zed `confirm_quit` and `fullscreen_mode` are Settings values, not action state. Fullscreen mode selection and quit confirmation persist in the user Settings document. Add `SET-settings-json-only-workspace-confirm-quit`; the Window & Layout `fullscreen_mode` requirement already exists. Labonair's terminal-session guard remains a separate setting. |
| UI kit | No product control is required. Use the native app menu and platform window controls; do not add shell chrome for these actions. |
| Notification and recovery | Hide, Hide Others, Minimize, and fullscreen do not need a notification. Quit cancellation, a declined save prompt, or preparation failure leaves the application running. Labonair's active-session notification is a distinct close safeguard. |
| Labonair route | Hide maps to the app menu's Hide Labonair action and `cmd-h`; Hide Others and Show All are menu actions, but Hide Others lacks the pinned `alt-cmd-h` default; Minimize is in the Window menu with fixed `cmd-m`; Quit is in the app menu with fixed `cmd-q` but lacks the non-macOS `ctrl-q` default and Zed's prepare-all-workspaces path. Full Screen appears in View and the command palette with fixed `ctrl-cmd-f`, but lacks the pinned `fn-f`/`f11` bindings and native/simple mode setting. Runtime behavior has not been observed. |

## Action requirements and current route

| Requirement ID | Pinned entry and effect | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-zed-Hide`](action-requirements.md) | `cmd-h`, global, macOS App menu. Hide the current application while leaving it running. | App menu and fixed `cmd-h` route exist. macOS native hide behavior is pending acceptance. |
| [`ACT-MACOS-zed-HideOthers`](action-requirements.md) | `alt-cmd-h`, global, macOS App menu. Hide other applications. | App menu action exists; the default shortcut is not bound in Labonair. |
| [`ACT-MACOS-zed-Minimize`](action-requirements.md) | `cmd-m`, global, macOS Window menu. Minimize the current window. | Window menu and fixed `cmd-m` route exist; native outcome is pending acceptance. |
| [`ACT-MACOS-zed-Quit`](action-requirements.md) | `cmd-q` on macOS and `ctrl-q` on Linux/Windows, global. Optionally ask to confirm, prepare each Workspace window, honor save/cancel results, flush session state, and then quit. | App menu and fixed `cmd-q` route exist. Non-macOS `ctrl-q`, dirty-buffer preparation, multi-window save/cancel handling, and the pre-quit persistence flow are missing or unverified. The active-shell close guard is not equivalent. |
| [`ACT-MACOS-zed-ToggleFullScreen`](action-requirements.md) | `fn-f` and `ctrl-cmd-f` in `Workspace` on macOS; `f11` in `Workspace` on Linux/Windows. Toggle the current window using the platform/mode policy. | View menu, command-palette action, and fixed `ctrl-cmd-f` route exist. `fn-f`, `f11`, the macOS `simple` mode, and native paired acceptance remain open. |

`zed::ShowAll` is a related registered action without a default binding: its
macOS app-menu entry unhides other applications. Labonair has a Show All app
menu action. Its route is recorded in the
[registered unbound action inventory](registered-actions-unbound.md).
`zed::Zoom` is the separate native window zoom action and remains in the
[Workspace zoom packet](action-classification-workspace-zoom.md); it is not
fullscreen or Pane/Panel zoom.

## Required user-flow and state acceptance

- Use Hide and restore the application from the OS. Use Hide Others and Show
  All with another visible application. Confirm app process and workspace
  state remain available.
- Minimize the active window and restore it. Verify another Labonair window,
  if present, is not minimized with it.
- Quit with `confirm_quit` false and true. With multiple windows, test active
  and background dirty buffers, successful save, save failure, cancel, and
  final session restore state. Confirm the application stays open after any
  canceled/failed preparation.
- Test macOS native and simple fullscreen modes, repeated toggle/restore,
  window bounds, menu/titlebar behavior, and state after switching Spaces.
  Test `fn-f` and `ctrl-cmd-f`; test `f11` on Linux and Windows.
- In Labonair, verify `confirmQuitWithActiveSessions` with no live sessions,
  one running local shell, one running SSH shell, notification dismissal, and
  a retry inside/outside three seconds. Keep these results separate from dirty
  document preparation.
- Capture paired native macOS app/window menus and all action states. Record
  build, OS, number of windows, active Workspace, Settings values, dirty item
  state, live-shell state, and fullscreen mode. Capture non-macOS shortcut
  routing and fullscreen behavior separately.

## Acceptance evidence still required

Static metadata and local code establish the listed routes and differences;
they do not establish app-menu accelerators, hide/minimize semantics, quit
prompts, multi-window save behavior, fullscreen rendering, or cross-platform
dispatch. Native and paired acceptance must be recorded before parity is
claimed.
