# Vim HelixInsert Action Classification

**Status:** Helix Normal `i` insertion point and mode entry classified; count/repeat, selection restoration, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixInsert`; binding `KEYBIND-VIM-HELIX-84adab6e6e44`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`. Related alternate routes: `KEYBIND-VIM-HELIX-a18dd5eeb9c1` and `KEYBIND-VIM-HELIX-d22a7f2c782d`.

**Owner:** Editor/Vim owns selection positioning, Insert mode, text edits, and repeat recording; Keymap owns the mode-scoped `i` routes. Labonair's current single-cursor Vim state belongs to `labonair-editor`; a Helix profile route belongs to `labonair-keymap`, with mode values owned by Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixInsert` as a unit action without JSON arguments or deprecated aliases. The shared profile binds `i` in `VimControl && vim_mode == helix_normal && !menu && !BufferSearchBar`. In Helix Select, `i` is a separate binding to `vim::InsertBefore`. The shared `insert` key also dispatches `vim::InsertBefore` for Helix Normal/Select. Vim Normal `i` remains `vim::InsertBefore`. Static registrations do not prove profile precedence, direct palette invocation, or physical dispatch on any platform.

The HelixInsert handler starts Vim change recording, then visits every Editor selection. It leaves an empty cursor at its existing point; each nonempty selection collapses to `selection.start` with no selection goal. It switches the Vim addon to Insert mode without editing document text. The action itself does not take a Vim count. The Insert Escape route (`vim::NormalBefore`) later consumes the count and can invoke the recorded-change replay path; the generic repeatable-insert matcher lists ordinary Vim insert actions but not `HelixInsert`, so counted Helix insertion needs direct acceptance rather than inference from ordinary Vim `i` behavior.

The handler does not save a HelixInsert-specific copy of the original selections. `NormalBefore` contains guarded restoration for HelixAppend's saved state, but this handler does not create that state. The direct HelixInsert fixtures establish selection collapse on entry and HelixNormal mode after a separate empty-cursor Escape; selection restoration after Escape, with or without edits, remains unverified.

## Pinned Helix workflow

In Helix Normal, `i` begins insertion before the current character. A nonempty Normal selection is collapsed to its ordered start, placing the Insert caret before the selected text. An empty cursor stays where it is. The action starts change recording and enters Insert mode; it does not edit content until the user types.

Helix Select uses `InsertBefore` for `i`, rather than `HelixInsert`. The shared `insert` key is also mapped to `InsertBefore`. These routes must be verified separately because they do not pass through the HelixInsert handler. Ordinary Vim Normal `i` is another `InsertBefore` route.

Escape from Insert uses `NormalBefore`. With Helix mode enabled, that path returns to Helix Normal for the ordinary no-count case. A focused fixture checks this mode transition from an empty cursor. The selected-range Escape outcome, edits followed by undo, count-prefixed insertion, mode setting changes, and repeat behavior remain runtime acceptance.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixInsert](registered-actions-unbound.md) | Unit action begins change recording, collapses each nonempty Helix Normal selection to its start, preserves empty cursor points, and enters Insert mode without an immediate text edit. | Labonair's local Vim `i` enters Insert at its current single-cursor position; it has no Helix mode, Helix selection model, or HelixInsert action route. |
| [KEYBIND-VIM-HELIX-84adab6e6e44](keymap-overlay-requirements.md) | Helix Normal `i` in `VimControl`, excluding menu and BufferSearchBar contexts. | No local Helix Normal context or profile binding exists. |
| [KEYBIND-VIM-HELIX-a18dd5eeb9c1](keymap-overlay-requirements.md) | Helix Select `i` dispatches `vim::InsertBefore`, not HelixInsert. | Labonair has no Helix Select mode or matching route. |
| [KEYBIND-VIM-HELIX-d22a7f2c782d](keymap-overlay-requirements.md) | Shared `insert` alias dispatches `vim::InsertBefore` in Helix Normal/Select. | No local Helix profile or alias behavior exists. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile gates the Helix Normal and Select contexts; Vim mode enables the shared layer. | Labonair exposes partial Vim-mode configuration but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local capability owner is `labonair-editor`; `labonair-keymap` should own a future Helix profile route. Labonair's `crates/editor/src/vim.rs::enter_insert` treats `i` as no cursor movement and switches its single-cursor `VimMode` to Insert. The local mode enum has no Helix Normal or Helix Select state, and the editor's Vim adapter does not implement the pinned multi-selection collapse or Helix-specific mode route.

The pinned `SET-keymap-vim-mode` and `SET-keymap-helix-mode` settings remain separate requirements. Their UI, persistence/reload, and runtime interaction are not established by this action classification.

## Pinned fixtures inspected

- `vim/src/helix.rs::test_insert_selected` enables Helix, starts with a forward nonempty selection in Helix Normal, sends `i`, and expects a collapsed caret at the selection's start in Insert mode. It covers one selection direction and action entry only.
- `vim/src/helix.rs::test_insert_mode_stickiness` enters Insert with an empty cursor, sends Escape, and checks that the mode returns from Insert to Helix Normal. It does not check selection restoration or text changes.
- `vim/src/helix.rs::helix_insert` starts change recording, collapses nonempty ranges to `selection.start`, and switches to Insert. `vim/src/insert.rs::normal_before` handles Escape and consumes the pending count. `vim/src/normal/repeat.rs::repeatable_insert` does not include HelixInsert among its listed repeatable insert entry actions.
- Keymap rows establish Helix Normal `i` → HelixInsert, Helix Select `i` → InsertBefore, and shared `insert` → InsertBefore. Labonair's local `enter_insert('i')` path preserves its current single caret and sets Insert mode.
- Fixtures and source were inspected, not run.

## Acceptance still required

- Verify macOS, Linux, and Windows registration, physical `i` dispatch, profile precedence, menu and BufferSearchBar exclusion, action-palette discovery/direct invocation, and Vim/Helix mode settings.
- Verify empty cursors and nonempty ranges at ordinary characters, line start/end, EOF, final newline, Unicode/grapheme boundaries, tabs, and wrapped/folded lines. Check forward/reversed ranges and multiple disjoint selections, their order, and exact insertion positions.
- Verify Helix Select `i` and the `insert` alias independently from Helix Normal `i`; confirm the resulting mode and selection shape for each.
- Verify Escape without edits, after inserted text, after backspace/undo restores original text, after cursor movement or focus changes, and after selection edits. Establish whether any original Helix Normal range is restored and whether Undo/Redo or dot-repeat changes that result.
- Verify count-prefixed Helix `i`, count interaction with `NormalBefore`, repeated inserted text, dot-repeat replay, Escape on count one and greater counts, and stale count cleanup. Confirm behavior separately from Vim Normal `i` and the ordinary InsertBefore route.
- Capture paired native macOS Editor/Keymap evidence with settings, focus, starting mode/ranges, cursor positions, count, typed/no-edit path, Escape result, platform, viewport, theme, and scale recorded.

## Source records inspected

- Action schema and documentation: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- HelixInsert handler, action registration, and direct fixtures: `zed-refrence/zed/crates/vim/src/helix.rs`.
- Escape/count handling and repeatable insert matcher: `zed-refrence/zed/crates/vim/src/insert.rs`, `zed-refrence/zed/crates/vim/src/vim.rs`, and `zed-refrence/zed/crates/vim/src/normal/repeat.rs`.
- Helix Normal/Select and alias routes: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Labonair single-cursor Vim insert state: `crates/editor/src/vim.rs`.
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
