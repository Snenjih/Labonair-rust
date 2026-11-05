# Vim DeleteToEndOfLine Action Classification

**Status:** Static count-aware Vim/Helix Normal route and local D gaps classified; forced-motion, platform, and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns EndOfLine motion ranges, Vim register effects, and buffer edits; Keymap owns the shared profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::DeleteToEndOfLine` as a unit action with no arguments or deprecated aliases. No platform-default map binds the action. The shared profile binds `shift-d` in `(vim_mode == normal || vim_mode == helix_normal) && !menu`. In Vim Visual, `shift-d` instead invokes `vim::VisualDeleteLine`. The shared rows are static requirements; they do not prove key dispatch on any platform. Palette availability and direct invocation are unverified.

## Pinned behavior

The `shift-d` action records the current action, consumes the pending Vim count and forced-motion state, and calls the common delete path with `Motion::EndOfLine { display_lines: false }`. A missing count defaults to one. The motion uses buffer rows rather than soft-wrapped display rows. For a count greater than one, it targets the corresponding later buffer row; this target clips at the final row. The usual inclusive motion deletes from the cursor through the last character of the target row, including intervening line breaks when the count spans rows and leaving the newline after the target row outside the range. Forced-motion context changes the motion kind and requires separate acceptance.

The shared delete path copies each expanded range into the Vim register as a delete, removes the ranges in an Editor transaction, and collapses each selection at the motion head. The action does not explicitly switch modes. The pinned `test_delete_to_end_of_line` fixture covers a mid-line cursor and a cursor on a blank row; it was inspected, not run. Exact blank-row, EOF, cursor, register, forced-motion, and counted-range outcomes remain runtime acceptance points.

## Pinned fixtures inspected

`test_delete_to_end_of_line` exercises Normal `shift-d` from within a line and on a blank line in a multiline buffer. `test_delete_end_of_line` is a separate `d $` operator test, not the direct action route. `test_forced_motion_delete_to_end_of_line` exercises the related `d v $` forced-motion operator route and confirms that forced-motion state is consumed; it is not a direct `shift-d` fixture. These pinned-source fixtures were inspected without running.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim/Helix profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's local Vim parser has an uppercase `D` route that deletes from the cursor to the current row's end and writes the deleted text to its local Vim register. It does not consume the pending count, so it cannot cover the pinned multi-row count behavior. The local path also has no typed `vim::DeleteToEndOfLine` action or shared Keymap profile. For a blank row after another row, its zero-width range reaches `range_delete`, whose empty-caret backspace path can remove the preceding line break; there is no local D fixture for this case. This is a static code-path risk to verify, not a runtime result.

## Acceptance still required

- Verify macOS registration, palette availability and focus restoration, `shift-d` dispatch in Vim Normal and Helix Normal, menu exclusion, profile loading, and user overrides. Verify Visual `shift-d` invokes `vim::VisualDeleteLine` instead.
- Verify count one, count greater than one, counts beyond EOF, soft wrapping, final rows with and without a trailing newline, blank rows, empty buffers, and multiple selections. Confirm whether the range includes intermediate line breaks and excludes the target row's trailing newline.
- Verify forced-motion state and exact endpoint/cursor behavior, characterwise register contents and selected-register behavior, undo grouping, repeat, save/reload, and no-op behavior at the line end.
- Compare local `D` with the pinned route for count, blank rows, line breaks, register results, cursor placement, and undo. Confirm or rule out the zero-width backspace behavior on a blank row after existing content.
- Dispatch the shared profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, count, range, register result, theme, and scale recorded.

## Source records inspected

- Action registration and shared binding: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`, `zed-refrence/zed/crates/vim/src/normal.rs`, and `zed-refrence/zed/assets/keymaps/vim.json`.
- EndOfLine range/count logic and delete/register effects: `zed-refrence/zed/crates/vim/src/motion.rs`, `zed-refrence/zed/crates/vim/src/normal/delete.rs`, and `zed-refrence/zed/crates/vim/src/normal/yank.rs`.
- Pinned direct, operator, and forced-motion fixtures: `zed-refrence/zed/crates/vim/src/normal.rs::test_delete_to_end_of_line`, `zed-refrence/zed/crates/vim/src/normal/delete.rs::test_delete_end_of_line`, and `zed-refrence/zed/crates/vim/src/motion.rs::test_forced_motion_delete_to_end_of_line`.
- Local uppercase D, operator register, and empty-range backspace: `crates/editor/src/vim.rs` and `crates/editor/src/document.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
