# Vim HelixDuplicateBelow Action Classification

**Status:** Counted Helix selection duplication below and shared route classified; count edges, display-map cases, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixDuplicateBelow`; binding `KEYBIND-VIM-HELIX-ebbe6f18c16f`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** The pinned Editor/Vim owns selection duplication and display/buffer coordinate conversion; Keymap owns the shared `shift-c` route. Labonair's current Editor selections and duplicate-line edit belong to `labonair-editor`; a Helix profile route belongs to `labonair-keymap` and its mode value to Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixDuplicateBelow` as a unit action without JSON arguments or deprecated aliases. The shared Vim/Helix layer binds `shift-c` under `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. In Vim Normal, `shift-c` is a separate `vim::ChangeToEndOfLine` action; the Helix layer route is not the Vim Normal command.

The handler consumes the pending Vim count and delegates to the common selection-duplication helper. The helper defaults an absent count to one additional copy per original selection. It reads all display selections in their existing order, preserves each oriented tail/head range, and adds eligible duplicate ranges to the selection set without editing text. For Below, it searches one display row at a time after the previous accepted range until both endpoints fit the mapped buffer rows or the bottom of the display map is reached. Endpoint columns are converted through UTF-16 buffer columns; a too-short row is skipped. Accepted display ranges are mapped back to buffer offsets using left/right bias.

For count `N`, the helper attempts up to `N` successful copies per original selection. If fewer eligible destinations remain, it adds fewer copies. Above uses the same algorithm in the opposite direction and reverses the initial selection iteration order; Below keeps the input order. The explicit count fixture covers Below. Count zero/extremes, destination collisions, selection order in complex multi-cursor arrangements, and profile/physical-key dispatch remain acceptance items.

## Pinned Helix workflow

In Helix Normal or Helix Select, `shift-c` keeps every original selection and adds up to the requested number of copies below it. The default adds one copy per selection. Each copy retains the range's endpoint columns and orientation when a destination display row can fit both endpoints. Rows that are too short are skipped while searching farther below; the search ends at the bottom of the display map. A count larger than the remaining eligible destinations cannot create additional copies past that boundary.

This is selection replication: the action does not edit buffer text, write Vim registers, or create a text transaction. New ranges join the existing selection set in input order. `alt-shift-c` is the corresponding Above action. In Vim Normal, `shift-c` changes text from the cursor to end of line and remains a different action.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixDuplicateBelow](registered-actions-unbound.md) | Unit action. Consumes the pending count (default one), retains each original selection, and adds up to that many copies on eligible display rows below. It does not edit text. | Local `Duplicate Lines` is a text edit that duplicates selected physical lines below their last selected line; it does not add corresponding selections. |
| [KEYBIND-VIM-HELIX-ebbe6f18c16f](keymap-overlay-requirements.md) | `shift-c` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`; Vim Normal `shift-c` is separately bound to ChangeToEndOfLine. | No local Helix Normal/Select context or profile route exists. |
| [ACT-REGISTRY-LINUX-vim-HelixDuplicateAbove](registered-actions-unbound.md) | Neighboring `alt-shift-c` calls the same directional helper with `Above`. | No local selection-only above or below duplication action exists. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile enables the shared Normal/Select route. | Labonair has Vim-mode configuration but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local owner is `labonair-editor`. Its `Duplicate Lines` edit collects physical line numbers covered by the current selection ranges, deduplicates those rows, and inserts each selected line after its existing line end. It edits document content and does not create a set of matching caret/range selections. No local `HelixDuplicateBelow` action, Helix `shift-c` profile route, or Helix mode exists. Local Vim Normal `Shift-C` currently has its own change-to-end-of-line route and does not provide selection replication.

Settings exposes a Vim-mode value but no Helix-mode value. The pinned `SET-keymap-vim-mode` and `SET-keymap-helix-mode` settings are separate requirements; their UI, persistence, and runtime interaction remain unverified in Labonair.

## Pinned fixtures inspected

- `vim/src/helix.rs` declares both duplicate actions and routes their consumed count to the common directional helper in `vim/src/helix/duplicate.rs`.
- `helix/duplicate.rs::test_selection_duplication` exercises `C` below, including skipping the too-short `over the` row for the selected range, stopping at the document bottom on a second `C`, and a later Above action.
- `test_selection_duplication_backwards` exercises two Below copies with a backward-oriented range and checks the resulting oriented ranges.
- `test_selection_duplication_count` checks a large Below count and shows that copies stop when no further rows remain before the document boundary.
- `test_selection_duplication_multiline_multibyte`, `test_selection_duplication_multibyte`, and `test_selection_duplication_with_inlay_hints` exercise Below column preservation and display-map conversion. These are not count or platform tests.
- Labonair's `crates/editor/src/editing.rs::duplicate_lines` and `EditorDuplicateLines` command route edit text below selected physical lines; they do not reproduce selections. Fixtures were inspected, not run.

## Acceptance still required

- Dispatch `shift-c` in Helix Normal and Helix Select on macOS, Linux, and Windows. Verify menu guard, Vim/Helix profile layering, physical Shift-C mapping, focus, and direct `vim:` palette discovery/invocation.
- Verify default and counted Below behavior, count zero/extremes, document-bottom exhaustion, skipped short rows, multiple/overlapping/adjacent selections, duplicate destinations, selection order, and forward/reversed orientations.
- Verify physical lines versus soft-wrapped display rows, folds, inlays, tabs, wide/combining graphemes, UTF-16 columns, multiline selections, and endpoint clamping at line ends.
- Confirm no buffer edits, register writes, undo transactions, or stale selection goals occur; verify viewport and cursor visibility after adding selections.
- Capture paired native macOS Editor/Keymap evidence with initial ranges/order, count, skipped rows, resulting ranges/order, mode/profile, focus, viewport/wrap/inlay state, theme, and scale recorded.

## Source records inspected

- Action metadata and handlers: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`, `zed-refrence/zed/crates/vim/src/helix.rs`, and `zed-refrence/zed/crates/vim/src/helix/duplicate.rs`.
- Shared `shift-c` binding and context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Direct Below/Above, short-row, count, backward-selection, UTF-16, multiline, and inlay fixtures: `zed-refrence/zed/crates/vim/src/helix/duplicate.rs`.
- Labonair duplicate-lines edit and command: `crates/editor/src/editing.rs` and `crates/editor/src/command_provider.rs`.
- Vim Normal ChangeToEndOfLine: `zed-refrence/zed/crates/vim/src/normal.rs` and `docs/parity/action-classification-vim-change-to-end-of-line.md`.
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
