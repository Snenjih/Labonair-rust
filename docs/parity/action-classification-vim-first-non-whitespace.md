# Vim FirstNonWhitespace Action Classification

**Status:** Static motion semantics and Labonair's partial `^` route classified; wrapped-line, Helix, operator, platform, and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-FirstNonWhitespace`; Vim `^` binding `KEYBIND-VIM-HELIX-9643ef26df23`; Vim `g ^` binding `KEYBIND-VIM-HELIX-f2de7cc67532`; Helix `g s` binding `KEYBIND-VIM-HELIX-cb1d5049795d`

**Owner:** Editor owns cursor and selection movement; Vim owns motion and operator dispatch; Keymap owns the shared Vim/Helix profile; Settings owns Vim/Helix mode selection and editor wrapping behavior.

## Evidence boundary

The pinned Linux action registry declares `vim::FirstNonWhitespace` with optional `display_lines: boolean` defaulting to `false`, no extra fields, and no deprecated aliases. The action is absent from the macOS, Linux, and Windows default keymap files. The shared Vim/Helix profile has three active rows:

- `^` in `VimControl && !menu`, using the default logical-line behavior;
- `g ^` in `VimControl && !menu`, passing `display_lines: true`;
- `g s` in Helix Normal and Select contexts, using the default logical-line behavior.

The action handler creates `Motion::FirstNonWhitespace` and sends it through the active Vim mode's motion path. It is an exclusive motion with no horizontal selection goal and is not added to the jump list. The motion implementation does not use the count passed to `move_point`; a count does not change which current line or display row is searched. When an operator is active, the same exclusive motion range is consumed by that operator, subject to the usual forced-motion kind change.

The action can be default-constructed because its only argument has a default. With Vim enabled, it is therefore a static candidate for the registered `vim:` Command Palette action list; action discovery and invocation remain runtime acceptance. `vim::InsertFirstNonWhitespace` is a separate `Shift-I` action that enters Insert mode, while `0` and `_` are separate start-of-line motions.

## Pinned motion behavior

With `display_lines: false`, the motion starts from column zero of the current buffer row. With `display_lines: true`, it starts from column zero of the current displayed row, so `g ^` addresses the visible segment of a soft-wrapped logical line. From that start it scans buffer characters and uses the language-scope character classifier to skip whitespace until the first non-whitespace character.

If the scan reaches a newline before finding a non-whitespace character, it returns the original cursor point. Thus a blank or whitespace-only line ending in a newline does not move the cursor. If a final line contains only whitespace and has no newline, the scan reaches the end of that whitespace and returns its last offset. Whitespace is recognized by `char::is_whitespace()` after the classifier's word-character checks. The action is a motion rather than an edit and does not itself change Vim mode.

In Vim Visual modes the motion extends the active selection. Under `d`, `c`, `y`, and other motion-consuming operators, the exclusive target range is used. In Helix Normal, `g s` moves within the logical buffer row. In Helix Select, the motion extends the selection, and the Helix selection path adjusts its head so the current character remains included. Multi-selection and exact reversed-selection behavior need direct acceptance.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-FirstNonWhitespace](registered-actions-unbound.md) | Registered optional-argument motion; `display_lines` defaults false, and the shared profile uses both logical-row and displayed-row forms. | Labonair's Vim `^` has a logical-row first-nonblank route. Its blank-line fallback differs; no display-row or Helix action route was found. |
| [KEYBIND-VIM-HELIX-9643ef26df23](keymap-overlay-requirements.md) | Vim `^` moves to the first non-whitespace character of the current logical line and also works as a motion for operators. | Local `resolve_motion('^')` uses `first_non_blank`; keyboard, selection, and operator behavior remain unaccepted. |
| [KEYBIND-VIM-HELIX-f2de7cc67532](keymap-overlay-requirements.md) | Vim `g ^` passes `display_lines: true` and targets the start of the current soft-wrapped display row. | Labonair has no display-row parameter or `g ^` route in its local Vim parser. |
| [KEYBIND-VIM-HELIX-cb1d5049795d](keymap-overlay-requirements.md) | Helix Normal/Select `g s` uses the logical-row motion. | Labonair has no Helix mode or `g s` route. |
| [KEYBIND-VIM-HELIX-a86b2d217038](keymap-overlay-requirements.md) | Separate `Shift-I` invokes `vim::InsertFirstNonWhitespace`, which enters Insert mode at the first non-whitespace position. | Labonair's `I` uses the local `first_non_blank` helper, but it is an insert command rather than this movement action. |

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns any future shared profile bindings. No new setting, event, notification, persistence, dependency, or UI control is proposed for the motion.

Labonair's [`vim.rs`](../../crates/editor/src/vim.rs) already routes `^` through `resolve_motion` to `first_non_blank`, which scans the current logical line using Rust's `char::is_whitespace()`. On a nonblank line it reaches the first non-whitespace character and the local motion is exclusive, broadly matching Vim's logical-line route. If the line contains only whitespace, local `first_non_blank` falls back to column zero; the pinned motion returns the original point when a newline ends the blank line. The local Vim layer has no `display_lines` motion option, and its `g` prefix currently handles only `gg` and `ge`, so it has neither `g ^` nor Helix `g s`. Its existing `I` route uses the same helper but enters Insert mode and does not supply those missing movement paths.

The local movement is tied to document logical lines and a single document cursor. It does not route through display-row geometry, a Helix mode/selection engine, the typed `vim::FirstNonWhitespace` action, or the shared profile layer. `0` remains the separate logical line-start route, and `_` is not interchangeable with this exclusive motion.

## Pinned fixtures inspected

- `normal.rs::test_jump_to_first_non_whitespace` checks `^` on a non-indented line, an indented line, an empty line, a first-row and later-row position, and a whitespace-only line followed by a newline.
- macOS-only `test.rs::test_wrapped_lines` uses a 12-column wrap to check `g ^` at the start of the current display row and `^` at the start of the logical row.
- `visual.rs::test_visual_block_mode` applies `^` across a three-row Visual Block selection and checks each selection head; `normal/paste.rs::test_special_registers` uses `d ^` and checks the resulting `the ` register text.
- `normal.rs::test_insert_first_non_whitespace` covers the separate `Shift-I` Insert-mode action.

The pinned fixtures were inspected, not run. No focused fixture was found for the Helix `g s` alias, count/forced-motion behavior, nonzero-cursor whitespace-only lines, or the final whitespace-only line without a newline. Existing `^`, display-row, Visual Block, and `d ^` fixtures do not cover the full operator, selection-direction, multi-cursor, or platform matrix.

## Acceptance still required

- Dispatch `^`, `g ^`, and Helix `g s` on macOS, Linux, and Windows with the relevant Vim/Helix profiles and menu context. Check physical key layout, profile overrides, action discovery, and direct `vim:` palette invocation.
- Verify cursor positions before, at, and after the first non-whitespace character; leading spaces and tabs; Unicode whitespace; empty and whitespace-only lines; trailing whitespace; newline-terminated and final unterminated rows; and multiple cursors. Record the cursor for every boundary case.
- Verify `g ^` at each segment of soft-wrapped lines as viewport width and wrapping change, including wrapped rows that begin with whitespace or have only trailing whitespace.
- Verify `g s` in Helix Normal and Select, including selection direction, current-character inclusion, count behavior, mode changes, and multiple selections.
- Verify Normal and Visual movement plus operator ranges for delete, change, yank, indent/outdent, and other consumers. Include movement in both directions, forced-motion behavior, count prefixes before/after operators, blank rows, and wrapped display rows.
- Compare local `^` on a whitespace-only line with the pinned no-op behavior; verify whether the local fallback should preserve the cursor or intentionally move to column zero.
- Capture paired native macOS Editor and Keymap evidence with profile/mode, caret/selection, operator, soft-wrap and viewport width, theme, scale, and platform recorded.

## Source records inspected

- Action schema and Linux registration: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Shared `^`, `g ^`, Helix `g s`, and separate `Shift-I` profile rows: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Action registration, `Motion::FirstNonWhitespace`, motion kind, display/logical line start, whitespace scan, and jump-list behavior: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Normal operator/Visual path and Helix Normal/Select behavior: `zed-refrence/zed/crates/vim/src/normal.rs`, `zed-refrence/zed/crates/vim/src/visual.rs`, and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Whitespace classification: `zed-refrence/zed/crates/language/src/buffer.rs`.
- Focused jump and insert fixtures: `zed-refrence/zed/crates/vim/src/normal.rs`.
- Wrapped logical/display-row routes: `zed-refrence/zed/crates/vim/src/test.rs::test_wrapped_lines`.
- Visual Block and `d ^` examples: `zed-refrence/zed/crates/vim/src/visual.rs::test_visual_block_mode` and `zed-refrence/zed/crates/vim/src/normal/paste.rs::test_special_registers`.
- Existing local `^`, `I`, and `g` key handling: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
