# Terminal Clear Action Classification

**Status:** Pinned action and local command route classified; native dispatch, scrollback, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned `terminal_view::TerminalView` dispatches `terminal::Clear` to the terminal model. Labonair `labonair-terminal` owns the `ClearTerminal` command; Workspace supplies the active terminal pane and performs the PTY write.

## Evidence boundary

The pinned action is bound in the `Terminal` context: macOS `cmd-k`; Linux and Windows `ctrl-shift-l`. The pinned default keymap search found no additional profile-overlay binding for this action. Local Labonair registers the Clear Terminal command in the Terminal and SSH Terminal palette contexts and maps its command ID to the `terminal::Clear` name, but has no matching built-in default shortcut. Static declarations do not prove current native dispatch.

## Pinned action behavior

The pinned `TerminalView::clear` handler sets the view's scroll position to the top, invokes the terminal model's `clear`, and notifies the view. Processing the model's Clear event calls `clear_saved_screen`, resets current-working-directory history, and emits a wakeup. The saved-screen helper clears the saved terminal screen, resets the region above the cursor, moves the current row to the first row, and places the cursor on that row. This is a terminal-model operation; it does not enter a shell command into the PTY.

## Labonair command route

`TerminalCommandProvider` contributes `Clear Terminal` for Terminal and SSH Terminal contexts. Its handler calls the Workspace `TerminalCommandTarget`; Workspace locates the active terminal pane and writes `\x1b[2J\x1b[H` to its terminal handle. The write result is ignored. This route sends ANSI input through the active PTY and does not call a model-level saved-screen or cwd-history clear operation. The command's exact effect on local scrollback, alternate-screen applications, SSH sessions, and a running foreground process has not been observed; do not infer parity from the sequence spelling alone.

## Binding and route comparison

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-terminal-Clear](action-requirements.md) | Clear the terminal's saved screen through the model, reset its cwd-history tracking, and wake the view. The default shortcut is scoped to a focused Terminal. | A palette command writes an ANSI clear-screen/home sequence to the active terminal PTY. It has no local default chord and does not directly invoke a model-level clear API. |

## Acceptance evidence still required

- Invoke `cmd-k` with a focused terminal on macOS and verify the visible screen, scrollback, cwd-history navigation, cursor placement, and view scroll position. Confirm no shell input was sent.
- Invoke the pinned Linux and Windows `ctrl-shift-l` bindings and check their context precedence against Editor Select All Matches.
- Invoke Labonair Clear Terminal in local and SSH terminals, with idle shells, a running foreground process, and an alternate-screen application. Record the PTY write result, visible output, scrollback, cwd history, and behavior when there is no active terminal.
- Compare the Labonair palette route with the pinned model-level action and document whether Labonair needs a terminal-owned clear operation and a default shortcut.
- Capture the native macOS action registration and dispatch plus paired terminal states. Source review alone does not establish that the route is reachable or visually equivalent.

## Source records inspected

- Pinned action and dispatch: `zed-refrence/zed/crates/terminal/src/terminal.rs`, `zed-refrence/zed/crates/terminal_view/src/terminal_view.rs`, and `zed-refrence/zed/crates/terminal/src/alacritty.rs`.
- Pinned bindings: `zed-refrence/zed/assets/keymaps/default-macos.json`, `default-linux.json`, and `default-windows.json`; profile keymaps were searched for the action name.
- Labonair command: `crates/terminal/src/command_provider.rs`, `crates/command-palette-core/src/lib.rs`, `crates/workspace/src/command_provider.rs`, and `crates/workspace/src/workspace.rs`.
- Related registers: [action requirements](action-requirements.md), [parity packet index](README.md), and the [Terminal capability crosswalk](feature-crosswalk.md).
