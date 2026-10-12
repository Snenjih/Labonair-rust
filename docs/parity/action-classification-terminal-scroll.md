# Terminal Scroll Action Classification

**Status:** Pinned action and history-setting effects classified; Labonair pointer routes recorded; native keyboard and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned TerminalView owns scroll action dispatch, viewport offsets, and inline-block handling; the terminal model owns scrollback offsets. Labonair `labonair-terminal` owns session/emulator history; Workspace embeds and renders the Terminal view.

## Evidence boundary

The six default-bound actions and platform contexts remain in [action-requirements.md](action-requirements.md). Default maps use `shift-up/down` for line scrolling, `shift-pageup/pagedown` for page scrolling, and `shift-home/end` for oldest/newest history on all platforms. macOS adds `cmd-up/down` for page actions and `cmd-home/end` for top/bottom. The JetBrains macOS profile maps `cmd-up/down` to line actions and `shift-pageup/pagedown` to page actions; its Linux/Windows profile maps `ctrl-up/down` to line actions and retains the `shift-pageup/pagedown` page bindings. No other profile action occurrence was found. Static maps do not establish native dispatch or page size.

## Pinned action behavior

All six TerminalView handlers propagate without changing terminal scrollback in alternate-screen mode. Otherwise they update scrollback line, page, or edge position and notify the view. The base model maps line actions to a one-line delta, page actions to viewport PageUp/PageDown operations, and top/bottom to the oldest/newest available history. When output includes a block below the terminal cursor, line/page/top/bottom handlers also adjust the block's separate viewport offset instead of moving only the terminal model. Page-up accounts for lines already visible from that block offset; page-down clamps the block to its maximum offset.

## Requirement crosswalk

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-terminal-ScrollLineUp](action-requirements.md) | Move one line toward older terminal output, or move the inline block viewport. Propagate in alternate-screen mode. | No matching action or default binding was found. Local Terminal exposes wheel-driven `Scroll::Delta` and scrollbar/session scrolling. |
| [ACT-MACOS-terminal-ScrollLineDown](action-requirements.md) | Move one line toward newer output, or move the inline block viewport. Propagate in alternate-screen mode. | Same pointer/session routes; no matching action or default binding. |
| [ACT-MACOS-terminal-ScrollPageUp](action-requirements.md) | Move toward older output by a viewport page, accounting for partial inline-block visibility. Propagate in alternate-screen mode. | No named page action or matching default binding; page keys follow the local Terminal input path unless another local binding handles them. |
| [ACT-MACOS-terminal-ScrollPageDown](action-requirements.md) | Move toward newer output by a viewport page and clamp an inline block at its maximum offset. Propagate in alternate-screen mode. | No named page action or matching default binding. |
| [ACT-MACOS-terminal-ScrollToTop](action-requirements.md) | Move to oldest available history, including the inline block's maximum upward offset. Propagate in alternate-screen mode. | No matching top action or default binding was found. |
| [ACT-MACOS-terminal-ScrollToBottom](action-requirements.md) | Return to newest output, and expose the bottom of an inline block. Propagate in alternate-screen mode. | Local input/paste snaps to bottom and wheel can scroll to current output; no matching user action or default binding was found. |

## History settings

| Requirement | Pinned effect | Labonair relationship |
|---|---|---|
| [SET-terminal-terminal-max-scroll-history-lines](settings-requirements.md) | `terminal.max_scroll_history_lines` defaults to 10,000 rows of terminal history available for scrolling. | Live emulator history is set by `terminal_scrollback` (default 5,000); `session_scrollback_lines` (default 1,000) bounds local history saved and replayed after a session restart. These values have different owners and scopes from the pinned maximum; do not assume numeric equivalence. |

## Labonair scroll routes

The local Terminal view sends pointer-wheel movement to `wheel_action`: depending on terminal mouse/alternate-screen mode it sends bytes to the program or changes local scrollback with `Scroll::Delta`. The view also exposes session scrolling and snaps to bottom after text or key input. No named line/page/top/bottom action handler or matching Terminal default binding was found. Labonair's emulator uses its own live scrollback cap and separately persists/replays a bounded portion of local session history; remote SSH history is not persisted by the local scrollback store.

## Acceptance evidence still required

- Exercise all six pinned actions at the oldest/newest boundary and at intermediate offsets; record exact page size, viewport and selection behavior.
- Use a Terminal with an inline block below the cursor and verify partial block visibility, page-up/down clamping, and top/bottom routing.
- Run an alternate-screen application and confirm the actions propagate as intended without changing scrollback or sending unwanted input.
- Compare macOS defaults and both JetBrains overlay maps, including `cmd-up/down` override precedence, with local wheel/scrollbar behavior and local key input.
- Verify pinned max history at 10,000 against Labonair's 5,000 live cap and 1,000 persisted/replayed lines across local and remote sessions.
- Capture paired native macOS top, middle, bottom, active-output, inline-block, alternate-screen, and restored-history states. Static source and existing tests were inspected, not run.

## Source records inspected

- Pinned handlers: `zed-refrence/zed/crates/terminal_view/src/terminal_view.rs` and terminal model scroll methods in `zed-refrence/zed/crates/terminal/src/terminal.rs`.
- Pinned bindings: default macOS/Linux/Windows keymaps and macOS/Linux JetBrains overlays.
- Labonair routes: `crates/workspace/src/views/terminal.rs`, `crates/terminal/src/session.rs`, `crates/terminal/src/engine.rs`, and `crates/settings-content/src/terminal.rs`.
- Related registers: [action requirements](action-requirements.md), [Settings requirements](settings-requirements.md), and the [Terminal capability crosswalk](feature-crosswalk.md).
