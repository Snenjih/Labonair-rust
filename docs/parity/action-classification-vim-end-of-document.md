# Vim EndOfDocument Action Classification

**Status:** Vim column/count behavior, Helix `ge` override, and operator routes classified; command parsing, platform dispatch, and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns document targets, cursor/selection effects, and linewise operator ranges; Keymap owns the shared profile and mode overrides; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::EndOfDocument` as a unit action with no arguments or deprecated aliases. No platform-default map binds it. In the shared `VimControl && !menu` context, `Shift-G` binds this action. In Helix Normal and Helix Select, a later profile layer overrides `Shift-G` with the distinct `vim::HelixGotoLine` action. Helix `g e` binds EndOfDocument in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`; in ordinary Vim the same `g e` sequence remains `vim::PreviousWordEnd`.

The Vim Ex command table also associates `$` and `%` tokens with EndOfDocument. Their command-line parsing and user-visible dispatch were not verified. The Vim action palette may expose the registered action, but availability and direct invocation are unknown. Static profile and command-table records do not prove runtime behavior on any platform.

## Pinned Vim behavior

The action dispatches `Motion::EndOfDocument`. It is linewise for Vim operators. Without a count it moves to the final buffer row, preserving the current column and clipping that column to the destination row. With a count it treats the count as a one-based buffer row, preserving the current column there as well. The target is clipped to the available buffer rows. Soft wrapping affects the displayed caret location, not the buffer-row destination.

Vim Visual and Visual Line send the motion through the visual-selection path. A pending delete, change, yank, indent, conversion, or other motion operator consumes the linewise range; its edit, register, mode, and caret effects come from that operator. A bare movement changes the cursor or selection and does not edit text or registers.

## Pinned Helix behavior and keymap precedence

Helix uses `g e` for EndOfDocument. Its mode adapter moves to column 0 of the final buffer row regardless of the starting column or pending count. In Helix Normal, an existing selection collapses to that target. In Helix Select, the active range extends to column 0 of the final row while preserving its direction/anchor behavior.

Helix Normal and Select override `Shift-G` with `vim::HelixGotoLine`; that action has separate count and selection semantics. The EndOfDocument `Shift-G` row applies to Vim Normal, Visual, and operator contexts where `VimControl` matches, but not after the Helix override. Likewise, ordinary Vim `g e` remains PreviousWordEnd; the Helix-specific `g e` row replaces it only in Helix Normal/Select. The two routes must be accepted independently.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim/Helix profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's local Vim resolver maps `G` to a counted linewise range ending at the counted row, or the last row when no count is given. It preserves the current column subject to the destination line length, so the basic Vim Normal route is partially present. There is no typed `vim::EndOfDocument` action, shared profile entry, Helix `ge` route, HelixGotoLine distinction, or matching `$`/`%` command-table route. The local `move_to` path extends only ordinary Visual mode; in Visual Line it calls `Document::set_caret` without extension and clears the active selection.

## Pinned fixtures inspected

`test_end_of_document_710` checks the default final row and a counted jump to row one while retaining the current column. `normal.rs::test_end_of_document` covers cursor positions across blank rows and counted targets. `test_delete_end_of_document` and `test_change_end_of_document` cover linewise operators. `test_helix_goto_line` establishes that Helix `Shift-G` is a distinct action with count-dependent targets. `test_helix_end_of_document` covers Helix `g e`, column 0, ignored counts, Normal collapse, and Select extension. `test_wrapped_delete_end_document` checks an operator path with wrapped content. These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence.

## Acceptance still required

- Verify macOS action registration, palette availability/focus behavior, Vim `Shift-G`, menu exclusion, pending operators, and the `g e` distinction from PreviousWordEnd. Verify Helix `g e` routes to EndOfDocument while Helix `Shift-G` routes to HelixGotoLine in both Helix Normal and Select.
- Verify Vim no-count final-row movement and one-based count targets while preserving the starting column, including long-to-short rows, empty rows, empty documents, final trailing newlines, very large counts, wrapped lines, multibuffer excerpts, and targets outside the current excerpt.
- Verify Visual, Visual Line, Visual Block, and reversed-selection behavior; pending delete/change/yank/indent/conversion effects; linewise registers, caret placement, mode transitions, count handling, undo grouping, repeat, and EOF recovery.
- Verify Helix `g e` always reaches column 0 of the final row and ignores counts, including active Normal selections, forward/reversed Select ranges, multiple cursors, empty/final rows, wrapping, and excerpts. Verify HelixGotoLine's distinct `Shift-G` count and selection behavior separately.
- Exercise Ex command tokens `$` and `%` in the command-line parser, with and without ranges or trailing commands, and verify the cursor/selection and command-line focus result.
- Dispatch the shared Vim profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence for Vim `G`, Helix `ge`/`G`, operator/selection effects, column, count, theme, and scale.

## Source records inspected

- Action registration and keymap routes: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/assets/keymaps/vim.json`.
- Vim motion action, linewise classification, column-preserving targets, and line counts: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Helix `ge`, Select extension, and HelixGotoLine distinction: `zed-refrence/zed/crates/vim/src/helix.rs` and `zed-refrence/zed/crates/vim/src/normal.rs`.
- Ex command table entries: `zed-refrence/zed/crates/vim/src/command.rs`.
- Vim, operator, and Helix fixtures: `zed-refrence/zed/crates/vim/src/test.rs`, `zed-refrence/zed/crates/vim/src/normal.rs`, `zed-refrence/zed/crates/vim/src/normal/delete.rs`, `zed-refrence/zed/crates/vim/src/normal/change.rs`, and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Local Vim `G` and Visual/Visual Line movement: `crates/editor/src/vim.rs` and `crates/editor/src/document.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
