# Vim EndOfLine Action Classification

**Status:** Logical/display-row routes, counted endpoints, inclusive motion, and Helix adjustment classified; platform and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns line endpoints, display geometry, selection goals, and operator ranges; Keymap owns the shared Vim/Helix profile; Settings owns mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::EndOfLine` with an optional Boolean `display_lines` argument, defaulting to `false`. No platform-default map binds it. In the shared `VimControl && !menu` context, `end` and `$` use the default argument, while `g $` and `g end` set `display_lines: true`. Helix Normal and Select add `g l` with the default argument. The separate `g _` action is `vim::EndOfLineDownward`, not EndOfLine. The Vim action palette may list the action, but availability and direct invocation are unverified.

## Pinned motion behavior

The action consumes the pending count, defaulting to one, and dispatches `Motion::EndOfLine`. The motion is inclusive for Vim operators. With `display_lines: false`, it targets the end of the current logical buffer row; a count greater than one first moves to the corresponding later buffer row. With `display_lines: true`, it targets the end of the rendered display row containing the cursor. Even with this argument, a count greater than one advances by buffer rows first, then ends at the display segment containing that target point; it does not count each wrapped segment as a separate counted row.

The motion sets the horizontal selection goal to infinity. Subsequent vertical movement therefore continues to target line ends until another motion resets the goal. The pinned vertical-motion fixture checks this retention and that horizontal movement resets it. Empty rows, final newlines, inlay hints, clipping, and different wrap geometries affect the endpoint and remain acceptance cases.

## Vim selections, operators, and Helix

The default `EndOfLine` motion kind is inclusive. Vim Visual modes extend the active selection to the endpoint. Visual Block has a separate vertical/block path for the display-row variant; the ordinary buffer-row EOL variant follows the regular selection path. Pending delete/change/yank and other motion operators consume the inclusive endpoint; forced-motion operator state can alter range handling and must be checked separately.

Helix binds `g l` to the same action with the default buffer-row argument. The Helix Normal adapter moves left from the calculated EOL position so the cursor rests on the final character. Helix Select makes the same endpoint adjustment before extending its range, avoiding selection of the following newline. On an empty line it remains at column 0. The `g $`/`g end` display-row routes are distinct profile rows and should be checked separately in Helix modes.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the Vim/Helix profile; Settings owns mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's local Vim resolver maps `$` to a counted inclusive endpoint on the current or a later logical row. This partially covers the default Vim motion. It has no typed `vim::EndOfLine` action, `end` binding, shared profile, Helix `g l` adapter, or `display_lines` variant. Its `$` route sets the cursor directly and does not retain an end-of-line selection goal for later vertical motion. No focused local EndOfLine Vim fixture was found.

## Pinned fixtures inspected

`test_end_of_line_with_times` and `test_end_of_line_with_neovim` check counted logical-row endpoints and clipping at the last row. `test_wrapped_lines` distinguishes `g $`'s display-row endpoint from `$`'s logical-row endpoint. `test_end_of_line_with_vertical_motion` checks that EOL goal persists through vertical movement and resets after horizontal movement. `test_clipping_with_inlay_hints_end_of_line` covers clipping around inlays. `test_g_l_end_of_line` checks the Helix last-character position with ASCII, multibyte characters, EOL, and an empty row; `test_helix_select_end_of_line` covers Helix selection. Delete/change fixtures and `test_forced_motion_delete_to_end_of_line` cover consuming operator paths. These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence.

## Acceptance still required

- Verify macOS action registration, palette behavior, and `end`, `$`, `g $`, `g end`, and Helix `g l` dispatch by mode. Verify menu, user-override, and platform-keymap precedence; keep `g _` mapped to EndOfLineDownward.
- Verify default and counted endpoints on short, long, empty, and final rows; count clipping at EOF; trailing-newline behavior; and `display_lines: true` count semantics when wrapped rows and buffer rows differ.
- Compare logical-row and display-row endpoints under soft wrapping, resize/reflow, tabs, wide glyphs, folds, inlay hints, excerpts, and multiple selections. Check cursor visibility and the saved horizontal goal after `$`, `$ k`, repeated vertical movement, and horizontal motion.
- Verify Vim Visual, Visual Line, Visual Block, Helix Normal, and Helix Select endpoints, selection direction, newline inclusion, empty rows, and multi-cursor behavior.
- Verify delete/change/yank/indent/conversion/shell operator ranges, inclusive register contents, forced-motion handling, caret/mode transitions, undo grouping, repeat, and recovery at empty rows and EOF.
- Dispatch the shared profiles on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, action/argument, line-wrap geometry, count, result, theme, and scale recorded.

## Source records inspected

- Action schema and shared/Helix routes: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/assets/keymaps/vim.json`.
- Action registration, motion kind, endpoint calculation, and selection goal: `zed-refrence/zed/crates/vim/src/motion.rs` and `zed-refrence/zed/crates/vim/src/normal.rs`.
- Visual/Visual Block and Helix last-character handling: `zed-refrence/zed/crates/vim/src/visual.rs` and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Count, wrap, goal, inlay, selection, operator, and forced-motion fixtures: `zed-refrence/zed/crates/vim/src/test.rs`, `zed-refrence/zed/crates/vim/src/motion.rs`, `zed-refrence/zed/crates/vim/src/normal/delete.rs`, `zed-refrence/zed/crates/vim/src/normal/change.rs`, and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Local `$` resolver and cursor placement: `crates/editor/src/vim.rs` and `crates/editor/src/document.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
