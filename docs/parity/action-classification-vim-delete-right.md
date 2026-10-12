# Vim DeleteRight Action Classification

**Status:** Static Normal-mode deletion routes and local lowercase-x equivalent classified; register, platform dispatch, and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns Vim motion ranges, delete/register effects, and buffer edits; Keymap owns the shared profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::DeleteRight` as a unit action with no arguments or deprecated aliases. No platform-default map binds the action. The shared Vim profile binds it in Vim Normal for both `x` and `delete`. In Vim Visual, those keys instead invoke `vim::VisualDelete`. In Helix Normal/Select, `x` invokes `vim::HelixSelectLine`. These are distinct action routes. Static rows do not prove profile loading or key dispatch on any platform. Palette availability and direct action behavior are unverified.

## Pinned behavior

Normal `x` and `delete` record the current action, consume the pending Vim count and forced-motion flag, and call the common delete path with `Motion::Right`. A missing count means one. Right is an exclusive, characterwise motion. It uses saturating movement within the current line, so the range does not wrap into the next line. Each active cursor deletes up to the counted number of characters beginning at its current point; movement is clipped at line end, and no character to the right means no deletion.

The shared delete path expands the motion for each selection, copies deleted ranges into the Vim register as a delete, removes the ranges in an Editor transaction, and collapses each selection at the clipped motion head. It does not explicitly switch Vim modes. Existing selection behavior, multiple cursors, register selection, undo grouping, and clipboard interaction require runtime acceptance.

## Pinned fixture inspected

`test_x` runs Normal `x` at each cursor offset of a single-line word and at a cursor within the first line of a two-line buffer. It is Neovim-backed and compares behavior; it does not establish count handling, end-of-line results, register contents, multiple selections, direct palette invocation, or native Labonair dispatch. The fixture was inspected, not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's `labonair-editor` Vim parser has a lowercase `x` Normal-mode route that deletes right by count and writes the deleted text to its local Vim register. Its existing `x_deletes_char_with_count` test covers `3x`, but not end-of-line, multibyte, register, Visual selection, or multi-cursor behavior. The local path has no typed `vim::DeleteRight` action or shared Keymap profile.

## Acceptance still required

- Verify macOS action registration, palette availability and focus restoration, shared Vim profile loading, Normal `x` and `delete` dispatch, and user overrides. Verify Visual `x`/`delete` resolve to `vim::VisualDelete` and Helix Normal/Select `x` resolves to `vim::HelixSelectLine`.
- Verify default count one and explicit counts; cursor at a character, at end of line, on empty lines and buffers, and immediately before a newline. Confirm Right never deletes a line break or wraps into the next row.
- Verify multibyte character boundaries, multiple cursors, selected text, register contents and selected-register behavior, caret positions, undo grouping, repeat, save/reload, and clipboard interaction.
- Compare the local `x` implementation with the pinned action, including count, boundary, and register outcomes.
- Dispatch the shared profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, count, cursor, deleted range, register result, theme, and scale recorded.

## Source records inspected

- Action registration and bindings: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/assets/keymaps/vim.json`.
- Delete motion, range kind, Right boundary, and register update: `zed-refrence/zed/crates/vim/src/normal.rs`, `zed-refrence/zed/crates/vim/src/normal/delete.rs`, `zed-refrence/zed/crates/vim/src/motion.rs`, `zed-refrence/zed/crates/editor/src/movement.rs`, and `zed-refrence/zed/crates/vim/src/normal/yank.rs`.
- Focused pinned fixture: `zed-refrence/zed/crates/vim/src/normal.rs::test_x`.
- Local lowercase-x and count behavior: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
