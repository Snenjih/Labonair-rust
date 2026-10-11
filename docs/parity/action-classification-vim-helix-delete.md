# Vim HelixDelete Action Classification

**Status:** Helix Normal/Select deletion, register/mark updates, undo selection restoration, and shared `d` route classified; count, range, EOF, platform, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixDelete`; binding `KEYBIND-VIM-HELIX-4ac06d98c5fa`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** The pinned Editor/Vim owns deletion, selections, mode transitions, registers, marks, and undo-selection state; Keymap owns the shared `d` route. Labonair's current single-cursor Vim behavior belongs to `labonair-editor`; a Helix route belongs to `labonair-keymap` and its mode value to Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixDelete` as a unit action without JSON arguments or deprecated aliases. Its shared Vim/Helix layer binds `d` under `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`.

The handler records the action and captures the current disjoint selection anchors. Before deletion, it extends each empty selection one display point to the right; nonempty ranges stay as selected. It then runs the standard visual-delete path over all ranges. That path captures the deleted text through the Vim register writer, sets the `[` and `]` marks, performs linked edits, and collapses the resulting selections. The handler restores the captured ranges as the transaction's undo selection state when a deletion transaction is returned, then switches to Helix Normal. The shared route does not itself distinguish `d` in Normal from `d` in Select; the current selection shape determines the range removed.

The handler does not directly consume a count. Empty-cursor expansion is one display point, while nonempty selections are used as-is. Count accumulation and cleanup, including whether a count affects the action through surrounding Vim state, still need runtime verification. `alt-d` is a distinct `editor::Delete` route whose keymap comment specifies deletion without yanking.

## Pinned Helix workflow

In Helix Normal, `d` deletes the character at each empty cursor by extending the range one display point to the right. At line end this can include the trailing newline and join the next line. In Helix Select, `d` deletes each selected range. It returns to Helix Normal and leaves collapsed carets at the deletion positions. When there is a transaction, Undo restores the selection/cursor anchors captured before the temporary one-character expansion.

Deletion uses Vim's ordinary register and mark path, unlike `alt-d`, which dispatches to the Editor delete action without yanking. The standard visual-delete handler applies the same operation to each selection and handles linked edits. Register ordering/content for complex selections, overlapping or adjacent ranges, line selections, and clipboard-register interaction remain acceptance questions.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixDelete](registered-actions-unbound.md) | Unit action. Extends empty cursors right one display point, deletes selected ranges through visual delete, updates Vim registers/marks, restores pre-expansion selections for Undo, and returns to Helix Normal. | Local Vim `d` starts a pending single-cursor operator; local `x` deletes forward. No Helix mode, multi-selection delete behavior, or Helix undo-selection restoration exists. |
| [KEYBIND-VIM-HELIX-4ac06d98c5fa](keymap-overlay-requirements.md) | Shared `d` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. | No local Helix Normal/Select context or route exists. |
| [ACT-MACOS-editor-Delete](action-requirements.md) | Neighboring `alt-d` route uses `editor::Delete`, documented in the Vim keymap as deleting selection without yanking. | Local editor deletion exists, but there is no Helix layer to provide the paired route distinction. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile enables the Helix Normal and Select contexts. | Labonair has a Vim-mode value but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local owner is `labonair-editor`. Its `VimMode` includes Normal, Insert, Visual, VisualLine, and Command, but no Helix Normal or Helix Select. In Normal mode, local `d` records a pending operator that needs a motion or a doubled `dd`; local `x` deletes forward from one cursor. Local Visual deletion has its own path, but neither route implements Helix `d`, multi-selection edits, the Helix register behavior, or restoration of the pre-expansion selection on Undo.

Settings currently exposes a Vim-mode value but no Helix-mode value. The pinned `SET-keymap-vim-mode` and `SET-keymap-helix-mode` settings are separate requirements; their UI, persistence, and runtime interaction remain unverified in Labonair.

## Pinned fixtures inspected

- `vim/src/normal.rs` declares the unit action and implements the empty-range extension, visual-delete call, undo-selection override, and return to Helix Normal.
- `vim/src/visual.rs::visual_delete` records visual marks, copies selected text through the Vim register path, applies linked deletions, and fixes cursor position. Helix mode keeps a cursor on a trailing newline instead of clipping it to the previous character.
- `vim/src/helix.rs::test_delete` covers deleting one selected range, then deleting one character at an empty cursor.
- `test_delete_character_end_of_line` checks deletion at a line end; `test_delete_to_end_of_line_keeps_cursor_on_newline` checks deleting a selected range through the last character while retaining the cursor on the newline.
- `test_helix_select_lines` includes `d u x`; its final state checks that Undo after `d` restores the original buffer/cursor state before a subsequent line selection. Tests were inspected, not run.
- The pinned `alt-d` binding has the explicit no-yank comment. Labonair's `VimMode`, pending-operator `d`, and forward-delete `x` paths were inspected; no local Helix route was found.

## Acceptance still required

- Dispatch `d` in Helix Normal and Helix Select on macOS, Linux, and Windows. Verify the menu guard, profile layering, focus, and direct `vim:` palette discovery/invocation.
- Verify empty cursors and forward/reversed, one-character, linewise, multiline, adjacent, disjoint, duplicate, and multi selections; selection ordering; linked edits; tabs, wide/combining graphemes, folded/wrapped text, line ends, final newline, and EOF.
- Verify counts and pending-count cleanup, register choice and contents, system clipboard behavior, mark ranges, selection/cursor placement, Undo/Redo, repeated deletion, and transaction state when the deletion is a no-op.
- Compare `d` with `alt-d` for register, clipboard, mark, and undo effects. Verify switching Helix mode off and restoring focus does not leave stale selection state.
- Capture paired native macOS Editor/Keymap evidence with mode, ranges, deleted text, resulting carets, registers/marks, Undo result, profile, focus, viewport, theme, and scale recorded.

## Source records inspected

- Action declaration, registry metadata, and handler: `zed-refrence/zed/crates/vim/src/normal.rs` and `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Shared `d` binding and context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Visual deletion, register writes, mark updates, linked deletion, and cursor fixup: `zed-refrence/zed/crates/vim/src/visual.rs` and `zed-refrence/zed/crates/vim/src/normal/yank.rs`.
- Direct delete, line-end, newline, and undo fixtures: `zed-refrence/zed/crates/vim/src/helix.rs`.
- Labonair Vim mode, pending `d` operator, and forward `x` deletion: `crates/editor/src/vim.rs`.
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
