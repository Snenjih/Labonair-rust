# Terminal Clipboard Action Classification

**Status:** Pinned action, Settings effects, and local clipboard routes classified; native clipboard and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned TerminalView handles action dispatch and clipboard-item branching; the terminal model owns selection copy and paste encoding. Labonair `labonair-terminal` owns selection/input helpers, Workspace renders the Terminal view, and Settings owns local preferences.

## Evidence boundary

The three pinned action rows and contexts remain in [action-requirements.md](action-requirements.md): macOS `cmd-c` Copy, `cmd-v` Paste, and `ctrl-cmd-v` PasteText; Linux Copy has `copy`, `ctrl-insert`, and `ctrl-shift-c`, Paste has `paste`, `shift-insert`, and `ctrl-shift-v`, with no default PasteText binding; Windows Copy has `ctrl-insert`, `ctrl-shift-c`, and `ctrl-c` only when a selection exists, Paste has `shift-insert`, `ctrl-v`, and `ctrl-shift-v`, and PasteText has `ctrl-alt-v`. No profile-overlay occurrence of these action names was found in the pinned keymap sources. Static bindings do not establish native dispatch.

## Pinned clipboard behavior

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-terminal-Copy](action-requirements.md) | Copy selected terminal text to the OS clipboard. With no explicit keep-selection argument, `terminal.keep_selection_on_copy` decides whether Copy clears the selection; the pinned default is true. `editor::Copy` is also handled so Edit > Copy remains enabled in the Terminal context. | The Terminal view reads its current selection and writes a string to the OS clipboard. It does not clear the selection. Labonair has no explicit keep-selection setting, so the pinned configurable false behavior is missing. |
| [ACT-MACOS-terminal-Paste](action-requirements.md) | Reads the first clipboard entry. Non-empty images forward Ctrl+V to the terminal application; external paths are shell-quoted, focused into the Terminal, and pasted; otherwise text is sent through terminal paste handling. `editor::Paste` also routes to this handler. | The Terminal view reads clipboard text and sends it to the active session. It has no explicit image or external-path clipboard branch in this route. Its platform-modifier C/V handling and context-menu item provide local entry points, but no pinned action registration or matching built-in default keymap row was found. |
| [ACT-MACOS-terminal-PasteText](action-requirements.md) | Reads only clipboard text and sends it through terminal paste handling; it bypasses the image and external-path branches of Paste. | No distinct PasteText action or route was found. The local paste helper is text-only, but normal Cmd/Ctrl+V uses that same path. |

### Paste encoding and safety

The pinned terminal model wraps text with `\x1b[200~` and `\x1b[201~` when the terminal mode requests bracketed paste and removes every ESC character from the clipboard text first. Without bracketed paste, it converts CRLF and LF line breaks to CR. The Labonair `paste_payload` helper normalizes CRLF/LF in either mode; when bracketed mode is active, it removes the exact `\x1b[201~` closing marker from the payload and wraps the result with the same markers. It does not remove other ESC characters. These source differences need runtime acceptance with shell prompts and terminal applications; this packet does not infer their user-visible or security effect.

### Copy-related Settings

| Requirement | Pinned effect | Labonair relationship |
|---|---|---|
| [SET-terminal-terminal-copy-on-select](settings-requirements.md) | Defaults false. When enabled, selecting terminal text copies it while preserving the selection. | `terminal.copy_on_select` maps to the local `terminal_copy_on_select` value, defaults false, and copies after double-click word selection or drag selection completes. |
| [SET-terminal-terminal-keep-selection-on-copy](settings-requirements.md) | Defaults true. Copy retains selection when true and clears it when false. | The local copy handler currently leaves selection intact but has no matching configurable value; only the pinned default behavior is represented. |

## Labonair clipboard route

`TerminalView::copy_selection` reads the selection from the active session and writes it to the OS clipboard; an empty or unavailable selection is a no-op. `paste_from_clipboard` reads text only, ignores missing/empty text, checks the session's bracketed-paste mode, and sends bytes produced by `paste_payload`. Raw platform C/V key handling and the terminal context menu use these same helpers. Local selection copying is controlled by `terminal.copy_on_select`; the current handler does not clear the selection after copy. There is no distinct local PasteText path.

## Acceptance evidence still required

- Exercise Copy with no selection and a selection, with pinned `keep_selection_on_copy` true and false. Compare clipboard content, selection highlight, and Edit menu enablement. Exercise Labonair's default and configurable copy-on-select behavior and record its missing keep-selection value.
- Exercise text, image, external-path, empty, and absent clipboard entries through Paste and PasteText. Verify image forwarding, safely quoted paths, focus, and whether a command is executed or only inserted.
- Exercise multiline text and embedded ESC/`ESC[201~` payloads with bracketed paste on/off in local and pinned terminals. Record the exact bytes and resulting shell/TUI behavior without exposing clipboard contents in logs.
- Verify macOS, Linux, and Windows default contexts, the Windows selection-only `ctrl-c` branch, profile precedence, alternate-screen behavior, and local shortcut reachability.
- Capture paired native macOS selection, clipboard-type, and paste states. Source review and existing tests, which were inspected but not run, do not establish the native clipboard behavior.

## Source records inspected

- Pinned handlers: `zed-refrence/zed/crates/terminal_view/src/terminal_view.rs` and `zed-refrence/zed/crates/terminal/src/terminal.rs`.
- Pinned Settings/keymaps: `zed-refrence/zed/crates/terminal/src/terminal_settings.rs`, `zed-refrence/zed/assets/settings/default.json`, and default platform keymaps.
- Labonair routes: `crates/workspace/src/views/terminal.rs`, `crates/terminal/src/input.rs`, `crates/settings-content/src/terminal.rs`, and `crates/settings/src/concrete.rs`.
- Related registers: [action requirements](action-requirements.md), [Settings requirements](settings-requirements.md), and the [Terminal capability crosswalk](feature-crosswalk.md).
