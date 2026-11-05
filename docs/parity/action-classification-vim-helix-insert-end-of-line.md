# Vim HelixInsertEndOfLine Action Classification

**Status:** Helix Normal `shift-a` line targeting and generic override classified; reversed/multiple selections, count/Escape behavior, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixInsertEndOfLine`; binding `KEYBIND-VIM-HELIX-5af32121bd5d`; related Vim route `KEYBIND-VIM-HELIX-b4f30cc642ac`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** Editor/Vim owns selection-to-line mapping, Insert mode, text edits, and repeat recording; Keymap owns the Helix Normal override of shared `shift-a`. Labonair's current single-cursor text and caret state belongs to `labonair-editor`; a Helix route belongs to `labonair-keymap`, with mode values owned by Settings.

## Evidence boundary

The pinned Linux registry declares `vim::HelixInsertEndOfLine` as a unit action without JSON arguments or deprecated aliases. The Helix Normal profile binds `shift-a` under `VimControl && vim_mode == helix_normal && !menu && !BufferSearchBar`. A broader `(vim_mode == normal || vim_mode == helix_normal) && !menu` row maps `shift-a` to `vim::InsertEndOfLine`; the more specific Helix Normal row overrides it. Helix Select has no `shift-a` entry. Vim Visual `shift-a` maps to `vim::InsertAfter`, while `g shift-a` maps to the separate `vim::VisualInsertEndOfLine` action.

The handler starts Vim change recording, enters Insert mode, then maps every selection to an end-of-line insertion point. For a nonempty forward selection it moves one display point left from the selection head before calculating the line end; other selections use their head directly. `motion::next_line_end(map, cursor, 1)` maps to the logical buffer-row end (not the rendered display-row end), and the handler collapses the selection there with no selection goal. The left adjustment prevents a full-line selection ending at column 0 of the following row from placing the insertion point on that following row.

The action does not take a count or save an action-specific copy of the prior selections. Insert Escape goes through `vim::NormalBefore`, which consumes any pending count and can invoke repeat handling. The repeatable-insert matcher does not list HelixInsertEndOfLine; count-prefixed behavior and selection results after Escape therefore need direct acceptance. Static handler and fixture evidence do not establish platform dispatch or rendered caret behavior.

## Pinned Helix workflow

In Helix Normal, `shift-a` enters Insert at the end of the current logical buffer row. An empty cursor stays on its row. When the current selection is a forward full-line range ending at column 0 of the following line, the action targets the selected line's end instead of the next line's end. A selected range whose head is partway through a later row targets that row's end.

This action is distinct from the shared Vim Normal/Helix Normal `InsertEndOfLine` route, which lacks the Helix-specific exclusive-selection-end correction, and from Helix `g l`, which is an EndOfLine motion that stays in the current mode. Helix Select has no `shift-a` route in the inspected profile; Vim Visual `shift-a` and `g shift-a` invoke different actions.

The direct fixture checks the two forward-selection cases and verifies entry into Insert mode. It does not cover escape back to Helix Normal, selection restoration, empty cursors, reversed/multiple selections, or count behavior. Those results remain runtime acceptance.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixInsertEndOfLine](registered-actions-unbound.md) | Unit action starts change recording, enters Insert, maps each selection head to the corresponding logical row end, and collapses there. A forward nonempty selection first moves left from its exclusive head. | Labonair's local Vim `A` moves its single caret to the logical row end and enters Insert; there is no Helix mode or multi-selection correction. |
| [KEYBIND-VIM-HELIX-5af32121bd5d](keymap-overlay-requirements.md) | Helix Normal `shift-a` under VimControl, menu, and BufferSearchBar guards. | No local Helix Normal context or binding exists. |
| [KEYBIND-VIM-HELIX-b4f30cc642ac](keymap-overlay-requirements.md) | Shared Normal/Helix Normal `shift-a` dispatches InsertEndOfLine; Helix Normal is overridden by HelixInsertEndOfLine. | Labonair has a partial single-cursor `A` route, with no typed action or profile override. |
| [KEYBIND-VIM-HELIX-5af32121bd5d](keymap-overlay-requirements.md) and [Vim EndOfLine](action-classification-vim-end-of-line.md) | Helix `shift-a` enters Insert at line end; Helix `g l` is a separate EndOfLine motion. | No local Helix mode or corresponding split of insertion and movement routes exists. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile enables the Helix Normal context. | Labonair has Vim-mode configuration but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local owner is `labonair-editor`; `labonair-keymap` should own a future Helix profile route. Local `crates/editor/src/vim.rs::enter_insert` maps uppercase `A` to the current line length and then enters Insert mode. This is a single-caret path. It does not account for an exclusive Helix selection ending at the next row's column 0, nor for a set of selections.

The pinned shared `shift-a` key is overridden only in Helix Normal. Helix Select has no binding in the inspected profile; ordinary Vim Visual `shift-a` is `InsertAfter`, and Vim Visual `g shift-a` is `VisualInsertEndOfLine`. These routes are not substitutes for HelixInsertEndOfLine.

Settings exposes distinct Vim Mode and Helix Mode requirements. Their UI, persistence/reload, and runtime interaction remain unverified in Labonair.

## Pinned fixtures inspected

- `vim/src/helix.rs::test_helix_insert_end_of_line` first uses `x` to create a full-line selection ending at column 0 of the next row; `shift-a` is expected to place the Insert caret at the end of the selected line. It then uses a forward selection crossing into the next row and expects the caret at the end of the row containing the selection head. Both assertions check Insert mode.
- `vim/src/helix.rs::helix_insert_end_of_line` performs the forward-range adjustment, invokes `motion::next_line_end`, and collapses each range. `vim/src/motion.rs::next_line_end` delegates to the logical buffer-row `end_of_line` path.
- The keymap records both the shared InsertEndOfLine row and Helix Normal-specific override. The local `enter_insert('A')` path moves to `line_len` and sets local `VimMode::Insert`.
- Fixtures and source were inspected, not run.

## Acceptance still required

- Verify macOS, Linux, and Windows action registration, physical `shift-a` dispatch, Helix Normal profile precedence, menu and BufferSearchBar exclusion, Vim/Helix mode settings, and palette discovery/direct invocation.
- Verify empty and nonempty selections, forward and reversed direction, multiple/disjoint selections, all-cursor ordering, selected lines ending at the next row's column 0, and ranges whose head is on later rows. Confirm line end on empty rows and final trailing newlines.
- Verify logical row end under soft wrapping, resize/reflow, tabs, wide/combining graphemes, folds, inlay hints, excerpts, and multibuffer Editors. Confirm the resulting insertion point and caret visibility.
- Verify count-prefixed `shift-a`, count cleanup, Insert Escape, cursor movement, inserted text, backspace/undo before Escape, selection restoration, dot-repeat, and focus/tab changes. Keep Helix Select unbound behavior distinct from Vim Visual routes.
- Capture paired native macOS Editor/Keymap evidence with settings/profile, focus, initial selection direction/range, logical/display row geometry, target line/column, Insert/Escape result, count, platform, viewport, theme, and scale recorded.

## Source records inspected

- Action schema and documentation: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- HelixInsertEndOfLine handler, registration, and fixture: `zed-refrence/zed/crates/vim/src/helix.rs`.
- Logical line-end helper: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Insert Escape/count and repeat handling: `zed-refrence/zed/crates/vim/src/insert.rs`, `zed-refrence/zed/crates/vim/src/vim.rs`, and `zed-refrence/zed/crates/vim/src/normal/repeat.rs`.
- Normal/Helix Normal and Visual keymap routes: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Labonair local Vim `A` route: `crates/editor/src/vim.rs`.
- Related action classifications: [Vim EndOfLine](action-classification-vim-end-of-line.md) and [Vim HelixInsert](action-classification-vim-helix-insert.md).
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
