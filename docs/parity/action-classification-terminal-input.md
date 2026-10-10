# Terminal Text and Keystroke Action Classification

**Status:** Pinned input paths and Labonair owner gap classified; native dispatch and PTY acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned `TerminalView` dispatches both actions and the Terminal model maps `SendKeystroke` according to live terminal state. Labonair's canonical owner is the Terminal capability (`labonair-terminal`); the current Workspace-hosted Terminal view translates physical key events and writes input to that capability.

## Evidence boundary

The two bound action IDs keep their existing Terminal contexts and platform bindings in [action-requirements.md](action-requirements.md). The default macOS map contains 13 `SendKeystroke` and six `SendText` entries; Linux and Windows each contain 14 and four, respectively. The macOS and Linux Emacs overlays document that `SendKeystroke` accepts one key stroke and null compound sequences so those sequences reach the terminal application. No active JetBrains or other profile binding for either Terminal action was found. Static maps do not prove action registration or native dispatch.

## Pinned input contract

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-terminal-SendKeystroke](action-requirements.md) | Parse the string payload as one GPUI keystroke, then pass it to the Terminal model. The model applies current terminal modes and may encode it as input; vi mode consumes it as a local motion. Invalid or unsupported input produces no PTY bytes. Supported Alt combinations use the pinned `option_as_meta` behavior. | The focused Workspace Terminal view translates actual GPUI key events to `KeyInput`, asks `labonair-terminal` to encode them using live `ModeState`, then writes the bytes to the active session. No named `SendKeystroke` payload action or matching default binding was found, so user keymaps cannot dispatch this action contract. |
| [ACT-MACOS-terminal-SendText](action-requirements.md) | Write the string payload's UTF-8 bytes directly to Terminal input after clearing the bell and pausing cursor blinking. This bypasses keystroke parsing, terminal-mode key encoding, and paste wrapping; the default bindings use explicit escape sequences for word movement/deletion. | The Terminal view has a private byte-writing helper used by existing input and paste routes, but no named `SendText` action or matching default binding was found. There is no local action route for configured arbitrary text payloads. |

The two actions are intentionally distinct: `SendKeystroke` asks the Terminal to interpret a named key using its live input mode; `SendText` sends the supplied bytes as-is. In vi mode the pinned `try_keystroke` route handles the key as a vi motion instead of forwarding it. When not in vi mode, unsupported keys are not sent.

## Bindings, profiles, and setting

The exact key names, arguments, and `Terminal` contexts remain recorded in the two action rows. On macOS, `cmd-backspace`, `cmd-delete`, `cmd-left`, and `cmd-right` map to control-key payloads; navigation keys, Enter, Escape, Ctrl+C, Ctrl+R, and Ctrl+Backspace map to `SendKeystroke`. `alt-delete`, Alt+Left/Right/B/F, and Ctrl+Delete map to direct `SendText` escape sequences. Linux and Windows map Control letters and navigation/Enter/Escape to `SendKeystroke`, and Alt+B/F/period plus Ctrl+Delete to `SendText`.

The macOS and Linux Emacs profile maps null compound sequences including `ctrl-x ctrl-c`, `ctrl-x ctrl-f`, `ctrl-x ctrl-s`, `ctrl-x ctrl-w`, and `ctrl-x s`. Their comments distinguish those multi-stroke bindings from the single-stroke `SendKeystroke` action. Other inspected profile maps have no active Terminal send-action bindings.

`terminal.option_as_meta` is a pinned USER setting with default `false`, tracked as [SET-terminal-terminal-option-as-meta](settings-requirements.md). On macOS, enabling it allows supported Alt keystrokes to pass through `SendKeystroke` as meta input. Linux and Windows treat supported Alt combinations as meta without this macOS-specific gate. `SendText` bypasses the setting because its payload already contains the bytes to send. No corresponding Labonair setting was found in the Terminal and Settings crates inspected for this slice.

## Labonair ownership and acceptance

The local path is `Workspace TerminalView` → `keystroke_to_input` → `labonair-terminal::key_to_bytes` using the session's mode snapshot → `send_input` to the active session. This supports mode-aware handling of actual key events, but does not provide the two pinned action IDs or string-payload keymap dispatch. Keep terminal byte encoding and session writes in the Terminal capability, with Workspace limited to hosting the view and forwarding input.

Acceptance evidence still required:

- Dispatch each action from a focused Terminal keymap binding and verify the target local or remote PTY receives the intended bytes; verify no action fires from other contexts.
- Compare `SendKeystroke` for control keys, navigation keys, unsupported keys, alternate-screen/application cursor modes, and vi mode. Verify invalid payloads produce no input.
- Compare `SendText` with printable text, control bytes, and escape sequences; confirm it does not gain paste wrappers or reinterpret bytes through terminal modes.
- Exercise Alt-as-meta with `option_as_meta` false/true on macOS and the pinned Linux/Windows behavior. Confirm it has no effect on raw `SendText` payloads.
- Verify default platform keymaps, Emacs compound-sequence null overrides, custom keymap payload validation, disconnected/exited sessions, and remote-session delivery.
- Capture paired native macOS focused-Terminal and keymap-dispatch states. Source review and existing tests, which were not run, do not establish native dispatch or PTY behavior.

## Source records inspected

- Pinned action definitions and handlers: `zed-refrence/zed/crates/terminal_view/src/terminal_view.rs`.
- Pinned mode-aware input: `zed-refrence/zed/crates/terminal/src/terminal.rs`, `terminal/src/mappings/keys.rs`, Terminal Settings, and default macOS/Linux/Windows plus Emacs keymaps.
- Labonair input: `crates/workspace/src/views/terminal.rs`, `crates/terminal/src/input.rs`, `crates/terminal/src/engine.rs`, and `crates/settings-content/src/terminal.rs`.
- Related registers: [action requirements](action-requirements.md), [Settings requirements](settings-requirements.md), and the [Terminal capability crosswalk](feature-crosswalk.md).
