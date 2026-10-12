# Terminal Character Palette and Vi Mode Action Classification

**Status:** Pinned action and key routing classified; local action routes and native macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned TerminalView dispatches both actions; the Terminal model owns vi-mode state and motions. Labonair `labonair-terminal` owns session input and `labonair-workspace` renders the terminal; no local action owner route for either behavior was found.

## Evidence boundary

The two actions remain bound in the `Terminal` context: macOS uses `ctrl-cmd-space` for `terminal::ShowCharacterPalette` and `ctrl-shift-space` for `terminal::ToggleViMode`; Linux and Windows use `ctrl-alt-space` and `ctrl-shift-space`. The pinned keymap search found these action names only in default platform maps, with no profile overlays. Keymap declarations and handler source do not establish native registration or interaction.

## Action behavior and local routes

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-terminal-ShowCharacterPalette](action-requirements.md) | On normal screen, call `Window::show_character_palette`: pinned GPUI opens the native palette on macOS and its generic platform implementation is a no-op on Linux/Windows. On alternate screen, parse and send `ctrl-cmd-space` through `Terminal::try_keystroke`, using `terminal.option_as_meta`. | No Terminal character-palette handler was found. Terminal key input otherwise passes through the local keystroke-to-PTY path; native palette and alternate-screen routing remain absent/unverified. |
| [ACT-MACOS-terminal-ToggleViMode](action-requirements.md) | Toggle model vi-mode state and cursor mode. While enabled, Terminal consumes key strokes as vi motions against terminal history instead of sending them to the PTY. The Terminal key context adds `vi_mode`. | No local vi-mode state, action, mode-specific cursor, or vi-motion handler was found. Local key input otherwise goes through the normal PTY input mapping. |

### Alternate-screen character palette path

The pinned handler checks `Modes::ALT_SCREEN`. On the normal screen it calls `Window::show_character_palette`; the pinned macOS GPUI implementation asks `NSApplication` to open the character palette, while GPUI's generic platform method does nothing. On alternate screen the handler asks the terminal model to try `ctrl-cmd-space` and passes the current `option_as_meta` preference. The related `terminal.option_as_meta` setting is tracked in [Settings requirements](settings-requirements.md). Linux/Windows bind `ctrl-alt-space` even though the alternate-screen handler parses `ctrl-cmd-space`; verify this platform difference at runtime.

### Vi-mode transition

The TerminalView action toggles the model flag and notifies the view. Processing flips `vi_mode_enabled` and updates the terminal's vi cursor mode. Subsequent `try_keystroke` calls are consumed by `vi_motion` while vi mode is on; they do not pass through the ordinary PTY keystroke encoder. Turning the action off returns input to normal terminal handling. The mode is transient model state, not a Settings value.

## Acceptance evidence still required

- On macOS, invoke the character palette with normal screen and alternate-screen applications; confirm the OS palette opens only in normal mode and the intended key reaches the TUI in alternate mode.
- Repeat on Linux and Windows; confirm the pinned normal-screen platform API is a no-op and resolve any runtime effect of their `ctrl-alt-space` bindings versus the alternate-screen parsed `ctrl-cmd-space` keystroke.
- Toggle vi mode on/off while reading long output. Verify cursor shape/location, supported motions, selection behavior, history boundaries, and that vi keystrokes never reach the foreground shell while enabled.
- Check mode state after session restart, alternate-screen transition, focus change, and terminal pane recreation.
- Capture the native macOS shortcuts, palette, normal/alternate screens, and vi cursor/motion states. Source review and existing tests, which were inspected but not run, do not satisfy runtime acceptance.

## Source records inspected

- Pinned actions and handlers: `zed-refrence/zed/crates/terminal/src/terminal.rs`, `zed-refrence/zed/crates/terminal_view/src/terminal_view.rs`, generic GPUI `PlatformWindow`, and macOS GPUI Window implementation.
- Pinned bindings: default macOS/Linux/Windows keymaps; profile keymaps searched for both action names.
- Labonair routes: `crates/workspace/src/views/terminal.rs`, `crates/terminal/src/input.rs`, and terminal engine/session input handling; no vi-mode or character-palette route was found.
- Related registers: [action requirements](action-requirements.md), [Settings requirements](settings-requirements.md), and the [Terminal capability crosswalk](feature-crosswalk.md).
