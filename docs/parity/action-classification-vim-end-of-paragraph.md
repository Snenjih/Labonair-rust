# Vim EndOfParagraph Action Classification

**Status:** Counted logical-row paragraph motion classified; platform and mode-specific acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns paragraph scanning, cursor/selections, and operator ranges; Keymap owns the shared Vim/Helix profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::EndOfParagraph` as a unit action with no arguments or deprecated aliases. The shared `VimControl && !menu` profile binds `}` to this action; no platform-default map binds it. The same profile has the distinct `{` `vim::StartOfParagraph` route. No other `EndOfParagraph` binding or Helix-specific override was found. Palette availability and direct invocation remain unverified.

This `}` motion is distinct from paragraph text objects and from curly-bracket text objects, which use their own actions and contexts.

## Pinned motion behavior

The action consumes the pending count, defaulting to one, and runs `Motion::EndOfParagraph`. It starts from the current logical buffer row and scans forward for paragraph ends. A paragraph is a run of non-empty buffer rows; a row containing whitespace is non-empty and does not form a boundary. With count one, the motion stops at the first truly empty row after the current or next non-empty paragraph. Larger counts advance across subsequent paragraph ends, skipping runs of empty rows between non-empty paragraphs. If no later empty boundary exists, it targets the document's maximum point. Wrapped display rows do not define paragraphs.

The motion is exclusive for operator range calculation and does not set a horizontal selection goal. Operator handling has paragraph-specific adjustments at an empty boundary: it can convert the range to linewise or inclusive based on the start column and indentation, and it extends through the final paragraph characters when the motion reaches EOF without a blank separator. The motion contributes to the jump list.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's local Vim resolver has a `}` branch, but it is not equivalent: it treats whitespace-only rows as blank through `trim().is_empty()`, ignores the pending count, and returns column zero on the final row instead of the end-of-document point. It has no typed `vim::EndOfParagraph` action, shared Vim/Helix profile, or Helix integration.

## Pinned fixtures inspected

- `motion.rs::test_start_end_of_paragraph` checks forward/backward movement, a count crossing paragraphs, and runs of blank rows.
- `motion.rs::test_paragraph_motion_with_whitespace_lines` distinguishes a whitespace-only row from a truly empty paragraph boundary.
- `normal/delete.rs::test_delete_end_of_paragraph` covers an operator consuming the motion.

These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence. They do not establish platform dispatch, palette behavior, Helix mode behavior, all EOF/indentation operator cases, or native user-visible jump-list behavior.

## Acceptance still required

- Verify macOS action registration, palette availability/focus behavior, `}` routing in Vim Normal/Visual/Visual Line/Visual Block, pending operators, and Helix Normal/Select. Check menu behavior, keymap overrides, and user profile precedence.
- Verify physical key dispatch for `}` across keyboard layouts and platforms, counts (including counts from blank rows), consecutive blank rows, whitespace-only rows, empty files, final empty rows, final newlines, and EOF without a separating blank row.
- Verify behavior with wrapping, resizing, folds, inlay hints, excerpts, and multi-cursor selections; confirm paragraph boundaries use buffer rows and inspect jump-list creation/restoration.
- Verify inclusive/linewise operator adjustments for varying start columns and indentation; delete/change/yank/indent/conversion/shell ranges, registers, forced-motion state, Visual anchors and reversal, Helix Select, undo grouping, repeat, and recovery.
- Dispatch the shared profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, count, paragraph contents, blank-row pattern, target, result, theme, and scale recorded.

## Source records inspected

- Action schema and shared profile: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/assets/keymaps/vim.json`.
- Action registration, motion kind, count dispatch, jump-list behavior, endpoint scan, and operator adjustments: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Focused motion and whitespace fixtures: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Operator fixture: `zed-refrence/zed/crates/vim/src/normal/delete.rs`.
- Local Vim resolver: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
