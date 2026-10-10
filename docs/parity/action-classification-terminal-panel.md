# Terminal Panel Toggle and Focus Action Classification

**Status:** Pinned focus/visibility actions, Settings effects, and local owner gap classified; native dispatch and universal-tab adaptation pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned `terminal_panel::Toggle` and `ToggleFocus` are registered on Workspace and target the TerminalPanel contributed by terminal_view; Workspace owns dock visibility and focus transitions. Labonair Workspace owns universal tabs and docks, while `labonair-terminal` owns terminal sessions. Labonair has no TerminalPanel or matching action route.

## Entry points and contexts

`terminal_panel::Toggle` is bound to <code>ctrl-&#96;</code> in the Workspace context on the default macOS, Linux, and Windows maps. Both JetBrains platform profiles bind `alt-f12` in `Workspace || Editor`. The Terminal menu exposes “Terminal Panel”; the action registry names it `terminal panel: toggle`; Vim provides the `te rm` and `T erm` command aliases. `terminal_panel::ToggleFocus` is registered as an unbound action in the Linux action catalog and has no default or profile key binding.

## Pinned focus and visibility behavior

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-terminal_panel-Toggle](action-requirements.md) | Calls `Workspace::toggle_panel_focus<TerminalPanel>`. When TerminalPanel is not focused, Workspace activates and opens its dock and focuses the Terminal input. If it is already focused, the action explicitly closes the panel. When an empty TerminalPanel becomes active, its activation path starts the default shell asynchronously. | Labonair exposes Terminal as a universal Workspace tab with `NewTerminalTab`; its statusbar controls toggle registered dock panels and its sidebar command targets the primary dock. No typed action focuses/reveals an existing Terminal tab or toggles a TerminalPanel. Do not add a second Terminal dock to copy the pinned chrome. |
| [ACT-REGISTRY-LINUX-terminal_panel-ToggleFocus](registered-actions-unbound.md) | Calls the same Workspace focus toggle without the explicit close branch. With `close_panel_on_toggle=false`, toggling from Terminal input to center leaves the panel open; when true, `Workspace::toggle_panel_focus` closes it. | No separate local focus-only action for a Terminal surface was found. The local tab strip selects terminal tabs, but does not expose this action contract. |

The two actions therefore differ when `close_panel_on_toggle` is false: `Toggle` closes an already-focused TerminalPanel; `ToggleFocus` returns focus to center and leaves the panel visible. The Workspace setting defaults false.

### Duplicate `ToggleFocus` registration

Pinned source registers `ToggleFocus` twice: `terminal_panel::init` adds a Workspace callback that checks terminal support, and Zed's central Workspace setup later adds another callback without that guard. `Workspace::register_action` appends listeners. The TerminalPanel callback is attached first; GPUI runs matching bubble listeners in registration order and stops propagation after the first handler unless it explicitly propagates. The first guarded callback therefore handles the action in the inspected source path, and the later duplicate is shadowed. This is a source-level dispatch finding; native key/menu/palette invocation was not run.

## Settings and persistence

| Setting | Pinned effect | Default |
|---|---|---|
| [SET-panels-terminal-dock](settings-requirements.md) | Places TerminalPanel on the left, bottom, or right dock. | `bottom` |
| [SET-panels-terminal-starts-open](settings-requirements.md) | Controls whether TerminalPanel starts open in a Workspace. | `false` |
| [SET-window-layout-terminal-button](settings-requirements.md) | Controls whether the TerminalPanel status item is shown. | `true` |
| [SET-window-layout-close-panel-on-toggle](settings-requirements.md) | When true, the shared focus-toggle method closes the panel while returning focus to center. | `false` |

Workspace serializes panel visibility and active dock state. Dock position, startup visibility, and status-item visibility are USER Settings. In Labonair, Terminal is a tab and tab/layout state is Workspace-owned; the dock-specific effects have no direct local equivalent.

## Labonair owner and acceptance

The product owner for a local entry action is Workspace because it selects or reveals the active Terminal tab; Terminal owns session construction and focused input. The accepted universal tab strip remains the Terminal surface. Before implementation, specify how the pinned close/unfocus states map to tabs without closing or losing an existing session. A new panel registry entry or shell-wide conditional route is not indicated by this inventory.

Acceptance evidence still required:

- Invoke Toggle from center focus, another dock, an open but unfocused TerminalPanel, and a focused TerminalPanel. Verify panel visibility, selected Terminal, focus target, and close behavior.
- Invoke ToggleFocus from center and Terminal focus with `close_panel_on_toggle` false and true. Verify its focus-only distinction and restored dock state.
- Verify the two `ToggleFocus` handler registrations and first-listener behavior against native dispatch; confirm unsupported-terminal workspaces do not invoke an unguarded duplicate route.
- Open an empty TerminalPanel and confirm one default shell starts; then repeatedly toggle/focus and confirm no duplicate session starts. Cover spawn failure, unsupported projects, closed sessions, and Workspace restoration.
- Verify `terminal.dock`, `terminal.starts_open`, `terminal.button`, and `close_panel_on_toggle` defaults, scopes, save/reload/reset, and their effects. Exercise left/bottom/right, hidden status item, narrow width, and other docks open.
- Verify default `<code>ctrl-&#96;</code>`, JetBrains `alt-f12`, menu, Command Palette, Vim aliases, and the unbound `ToggleFocus` action's discoverability and conflicts.
- Under Labonair's universal tab architecture, verify one canonical Workspace entry focuses or opens Terminal while preserving existing terminal sessions. Do not create a second dock-style Terminal surface.
- Capture paired native macOS visible/hidden, focused/unfocused, empty-shell startup, repositioned, and narrow states. Static source and map review do not establish rendered layout or action reachability.

## Source records inspected

- Pinned action declarations and module handlers: `zed-refrence/zed/crates/terminal_view/src/terminal_panel.rs`; central Workspace registration: `zed-refrence/zed/crates/zed/src/zed.rs`.
- Pinned focus/dispatch: `zed-refrence/zed/crates/workspace/src/workspace.rs` and `zed-refrence/zed/crates/gpui/src/elements/div.rs`, `gpui/src/window.rs`.
- Pinned entry points: default macOS/Linux/Windows maps, macOS/Linux JetBrains maps, `zed/src/zed/app_menus.rs`, Command Palette action discovery, and `zed-refrence/zed/crates/vim/src/command.rs`.
- Pinned Settings: `zed-refrence/zed/assets/settings/default.json`, Terminal Settings, and Workspace Settings.
- Labonair routes: `crates/workspace/src/workspace.rs`, `command_provider.rs`, menu and dock controls; `docs/capabilities.md`, [`workspace.docks` crosswalk](feature-crosswalk.md), and Settings requirements.
- Related registers: [bound action requirements](action-requirements.md) and [registered-unbound actions](registered-actions-unbound.md).
