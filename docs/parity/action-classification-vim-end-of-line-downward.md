# Vim EndOfLineDownward Action Classification

**Status:** Counted logical-row last-non-whitespace motion classified; platform and mode-specific acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns target-row scanning, cursor/selections, and operator ranges; Keymap owns the shared Vim/Helix profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::EndOfLineDownward` as a unit action with no arguments or deprecated aliases. No platform-default map binds it. The shared `VimControl && !menu` profile binds `g _` to this action. The adjacent `_` binding is `vim::StartOfLineDownward`; `$` and `end` are the separate `vim::EndOfLine` action. No Helix-specific override for `g _` was found. Palette availability and direct invocation are unverified.

## Pinned motion behavior

The action consumes the pending count, defaulting to one, and runs `Motion::EndOfLineDownward`. It is an inclusive motion for Vim operators. Its helper first resolves the counted target buffer row through the logical-row EndOfLine helper, then scans backward over characters classified as whitespace until it reaches the last non-whitespace character or the row boundary. It does not have a `display_lines` argument and does not count wrapped display rows. Unlike `vim::EndOfLine`, it does not set the horizontal selection goal to infinity.

When the target row contains trailing whitespace, the cursor lands on its last non-whitespace character. If the row contains only whitespace, the static scan reaches the row start; empty and whitespace-only outcomes still need runtime acceptance. Character classification, Unicode whitespace, tabs, and row boundaries affect the scan.

The `g _` action is shared under `VimControl`, including the Vim and Helix modes whose context predicates match. The shared profile data does not prove platform dispatch. Visual, Visual Block, Helix Select, and pending-operator behavior use their mode-specific motion paths and need separate acceptance.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's local Vim resolver has no `_` or `g _` route, typed `vim::EndOfLineDownward` action, or shared profile. Its `$` motion goes to the logical line end, including trailing whitespace, so it does not provide the last-non-whitespace behavior. There is no local Helix mode or focused `g _` fixture.

## Pinned fixture inspected

`motion.rs::test_end_of_line_downward` starts on a line with leading indentation and verifies the last non-whitespace character with no trailing spaces, with trailing spaces, and with a count targeting the next buffer row. This focused pinned-source fixture was inspected, not run, and is not Labonair acceptance evidence. It does not cover whitespace-only rows, Unicode whitespace, Visual modes, Helix modes, operators, or platform dispatch.

## Acceptance still required

- Verify macOS registration, palette availability/focus behavior, `g _` routing in Vim Normal/Visual/Visual Line/Visual Block, pending operators, and Helix Normal/Select. Check menu behavior, keymap overrides, and user profile precedence.
- Verify default count, multi-digit counts, clipping beyond EOF, counted target rows, long trailing whitespace, tabs, Unicode whitespace, non-breaking spaces, empty rows, whitespace-only rows, mixed indentation, and final rows with or without a newline.
- Verify the target stays on the logical buffer row under wrapping, resizing, folds, inlay hints, excerpts, and multi-cursor selections. Check whether horizontal movement resets the selection goal and whether EOLDownward changes it.
- Verify inclusive delete/change/yank/indent/conversion/shell ranges, register contents, forced-motion state, visual anchors, selection reversal, Visual Block geometry, Helix Select extension, mode/caret transitions, undo grouping, repeat, and recovery on blank rows.
- Dispatch the shared profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, count, whitespace, target row, result, theme, and scale recorded.

## Source records inspected

- Action schema and shared key: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/assets/keymaps/vim.json`.
- Action registration, inclusive motion kind, counted dispatch, and whitespace scan: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Visual and mode-specific motion paths: `zed-refrence/zed/crates/vim/src/visual.rs` and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Focused fixture: `zed-refrence/zed/crates/vim/src/motion.rs::test_end_of_line_downward`.
- Local Vim resolver: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
