# Workspace New Terminal Action Classification

**Status:** Static intent, owner, settings, and route crosswalk for one
default-macOS action; pinned dispatch, native interaction, and paired visual
acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Terminal (`labonair-terminal`) owns shell configuration, process
sessions, and terminal rendering. Workspace (`labonair-workspace`) hosts the
terminal in the active tab, center pane, or dock panel and owns the local
`New Terminal Tab` route.

## Evidence boundary

This packet describes the user outcome of opening another shell in the active
workspace, including whether that shell runs in the project's local or remote
environment. Static source checks identify the action data and current local
routes; they do not prove action dispatch, shell creation, focus placement,
remote execution, or rendered behavior.

The requirements are independently stated for Labonair's existing universal
tab architecture. This is same-session independent redevelopment, not
separated-room clean-room work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux
host without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Terminal — `labonair-terminal` owns shell settings, process/session state, terminal output, and terminal view behavior. Workspace — `labonair-workspace` hosts a terminal in a tab or pane and currently owns the menu/command route. |
| Composition | Terminal owns shell defaults and terminal session lifecycle. Workspace owns the local command route, execution context, and host placement. The current route starts in Workspace and calls the typed Terminal registry for local sessions. A remote shell must use the owning SSH/remote transport contract to supply terminal I/O. Shell composes the native menu and command provider only. |
| Canonical user entry points | Pinned `Ctrl+~` in Workspace and `Cmd+N` in Terminal on macOS; `Ctrl+~` and `Ctrl+N` on Linux; `Ctrl+Shift+Backtick` and `Ctrl+N` on Windows. The Terminal context menu also exposes New Terminal. Labonair provides File > New Terminal Tab and Command Palette `New Terminal Tab`, bound to `Cmd+T` on macOS and `Ctrl+T` on Linux/Windows. |
| Public typed contract | The current local route is `CommandId::NewTerminalTab` plus `SessionOptions` and `TerminalRegistry::create`. The transport-backed session entry accepts remote input/output adapters supplied by the owning SSH/remote contract. Workspace owns tab/pane placement. Any future owner-crossing request must stay typed; this inventory adds no API or registry. |
| Initial state and placement | Create exactly one fresh shell session and preserve existing sessions. In the pinned flow, if the center pane has focus and its active item is a Terminal, add the new Terminal in the center area. Otherwise create a Terminal Panel shell and reveal the panel. Focus the new terminal after creation. |
| Local versus remote | The pinned action has a default-false `local` field. With `local=false`, use the active project's execution environment, including its remote host when the project is remote. With `local=true`, explicitly create a local shell even in a remote project. Labonair's New Terminal Tab always starts a local process; New SSH Tab is a separate saved-host workflow and does not establish a project-scoped remote shell. |
| Working directory | The pinned default is `current_project_directory`; the setting also supports current-file directory, first project directory, always home, and an explicit directory. Remote projects do not apply local home fallback or local path expansion/checks. Labonair uses the active terminal's live cwd, then the active project root, and otherwise passes no explicit cwd. Its actual inherited process cwd in that last case needs runtime verification. It has no matching configurable terminal working-directory setting. |
| Shell defaults | The pinned `terminal.shell` default is `system`; arguments, environment, and explicit program fields are separate Settings values. Labonair's `terminalShell` defaults to an empty string, meaning the OS login shell, with optional shell arguments and extra environment variables. Its launch path passes those typed values into `SessionOptions`. |
| Registries and events | The current local `CommandId::NewTerminalTab` is registered by the Workspace provider and calls `Workspace::new_terminal_tab`, which calls `TerminalRegistry`. This inventory slice adds no route, event, registry, setting, or binding. |
| Persistence | Keep existing terminal tabs and sessions intact while creating the new session. Record whether the new session is restored after close/restart separately from creation acceptance. Labonair has terminal-session persistence, but this action's end-to-end restore behavior is not verified here. |
| Notifications and failure | A failed shell spawn must preserve existing tabs and provide a recoverable notification with retry. Labonair currently logs a `TerminalRegistry::create` error and returns without a visible notification; this is a user-facing gap. |
| Security | Keep local and remote execution environments separate. A forced-local shell in a remote project must use local cwd and environment only. Acceptance traces record the target host/local choice, executable, arguments, cwd, and environment sources without capturing secret values. |
| UI-kit | Reuse the Terminal view, existing Workspace tab/pane, Terminal dock, and shared menu primitives. Do not create feature-local tab or menu styling. |

The action namespace is `workspace::NewTerminal`, but its primary behavior
belongs to Terminal. It is distinct from Workspace's local `New Terminal Tab`,
which always opens a new universal workspace tab. The Terminal context binding
also differs from New File: on macOS `Cmd+N` creates a Terminal only while the
Terminal context is active; the New File binding is excluded there.

## Action requirement

| Requirement ID | Default route and context | User goal and pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-NewTerminal`](action-requirements.md) | macOS: `Ctrl+~` (`Workspace`); `Cmd+N` (`Terminal`). Linux: `Ctrl+~` (`Workspace`); `Ctrl+N` (`Terminal`). Windows: `Ctrl+Shift+Backtick` (`Workspace`); `Ctrl+N` (`Terminal`). | Open and focus a fresh shell in the active workspace. Use the project execution environment by default; honor an explicit local-shell request in remote projects. Place the new Terminal beside the active center Terminal when it has focus, otherwise reveal the Terminal dock panel. | **Partial, distinct route.** Labonair's `New Terminal Tab` creates a new local terminal in a universal Workspace tab and focuses it. File menu and palette invoke `CommandId::NewTerminalTab`; defaults are `Cmd+T`/`Ctrl+T`. It does not implement focus-dependent center/dock placement, the typed `local` choice, or a remote project shell. |

## Settings and session requirements

| Requirement | Target behavior | Current Labonair status |
|---|---|---|
| [`SET-terminal-terminal-shell`](settings-requirements.md), [`SET-terminal-terminal-shell-5c271ac7`](settings-requirements.md), and [`SET-terminal-terminal-shell-program`](settings-requirements.md) | Resolve the selected terminal shell/program; the pinned default is `system`, with a separately inventoried explicit program value. | Local `terminalShell` uses the OS login shell when unset/empty and has no separate `terminal.shell.program` field. Exact pinned/local field-to-widget effects remain under Settings acceptance. |
| [`SET-terminal-terminal-shell-args`](settings-requirements.md) | Append configured arguments to the shell launch. | Local `terminalShellArgs` is parsed into arguments passed through `SessionOptions`. |
| [`SET-terminal-terminal-shell-title-override`](settings-requirements.md) | Override the title shown for a terminal shell. | No matching local setting is inventoried; verify terminal tab and panel naming separately. |
| [`SET-terminal-terminal-env`](settings-requirements.md) | Apply configured environment values to the shell. | Local `terminalEnvironmentVariables` is parsed and passed through `SessionOptions`. Do not include secret values in evidence. |
| [`SET-terminal-terminal-working-directory`](settings-requirements.md) and [`SET-terminal-terminal-working-directory-always`](settings-requirements.md) | Select the cwd policy and, for an explicit path, the configured directory. | No matching local setting exists. The current local route uses live Terminal cwd, then project root, then passes no explicit cwd; the inherited process cwd needs runtime verification. |

## Required user-flow and state acceptance

- Activate the Workspace shortcut with an Editor or other non-Terminal center
  view focused. Verify one terminal is created in the dock, the dock is shown,
  and focus enters the new shell.
- Activate the Terminal-context shortcut from a focused center Terminal.
  Verify one terminal is created in the center area, existing terminals and
  their input state remain intact, and focus enters the new shell.
- Open the Terminal context menu and activate New Terminal. Verify it follows
  the same typed action and placement path as the shortcut.
- Test a standalone workspace, local project, remote project, and no active
  project. Verify default shell target and cwd in each case, including remote
  home fallback. Set `local=true` in a remote project and verify the process,
  cwd, and environment are local.
- Change shell, shell arguments, environment, cwd policy, and explicit cwd.
  Verify values take effect on the next new shell and do not mutate an already
  running session. Record current unsupported settings as gaps.
- Test repeated invocations, slow startup, missing/invalid shell, invalid cwd,
  user cancellation where applicable, remote disconnection, and spawn failure.
  Existing tabs and session output must remain intact; failures must be visible
  and retryable.
- Verify `Cmd+N` dispatches New Terminal only in Terminal context and New File
  only in Workspace/Welcome contexts. Verify `Ctrl+~`, `Ctrl+N`, and
  `Ctrl+Shift+Backtick` against their platform/context records and user keymap
  overrides.
- Verify terminal close confirmation, session persistence, and restart restore
  independently from the action that creates a new shell.
- Inspect accessibility labels, keyboard focus, narrow-window dock layout,
  center terminal layout, and light/dark themes. Capture pinned and Labonair
  states under matching native macOS conditions.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| `CommandId::NewTerminalTab` descriptor and handler in [`command_provider.rs`](../../crates/workspace/src/command_provider.rs) and [`workspace.rs`](../../crates/workspace/src/workspace.rs) | New local terminal command and universal-tab host path | Static source proves the handler calls `open_terminal_tab`; no pinned context dispatch, focus placement, or remote shell path is verified. |
| `TerminalSettings` and `SessionOptions` in [`concrete.rs`](../../crates/settings/src/concrete.rs) and [`session.rs`](../../crates/terminal/src/session.rs) | Shell, arguments, environment, and typed launch options | Cwd policy differs from the pinned setting. Config value-to-process behavior has not been action-accepted. |
| `TerminalRegistry::create` and its transport-backed session entry in [`registry.rs`](../../crates/terminal/src/registry.rs) | Local process creation or terminal session registration for caller-supplied transport I/O | The local action calls `create`; it does not select an SSH/remote target. A local spawn error is logged and returned to Workspace, which silently leaves the current view unchanged. |
| `terminal_spawn_cwd` and `open_terminal_tab` in [`workspace.rs`](../../crates/workspace/src/workspace.rs) | Local cwd fallback and new-tab placement | This proves local static routing only; no live cwd, local/remote selection, or native layout behavior is verified. |

No Labonair test or runtime trace exercises `ACT-MACOS-workspace-NewTerminal`
end to end. Existing `New Terminal Tab` tests establish menu visibility and
keymap descriptor values only; they do not establish this action's distinct
Terminal context behavior. Pinned dispatch, local/remote execution, native
placement, notifications, and paired visual evidence remain pending.

## Acceptance evidence still required

For each pinned and Labonair trace, record build and commit, OS, window and
pane dimensions, scale, theme, active context, exact menu/key route, local or
remote target, configured shell/cwd, resulting session placement, existing
session state, focus, and spawn result. Capture matched Workspace-focus,
Terminal-focus, local-project, and remote-project states. Do not infer dispatch
or remote behavior from action declarations, descriptors, or a successful
local `New Terminal Tab` launch.
