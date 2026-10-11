# Vim HelixDuplicateAbove Action Classification

**Status:** Counted Helix selection duplication above and shared route classified; count edges, display-map cases, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixDuplicateAbove`; binding `KEYBIND-VIM-HELIX-58875e17dc79`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** The pinned Editor/Vim owns selection duplication and display/buffer coordinate conversion; Keymap owns the shared `alt-shift-c` route. Labonair's current Editor selections and duplicate-line edit belong to `labonair-editor`; a Helix profile route belongs to `labonair-keymap` and its mode value to Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixDuplicateAbove` as a unit action without JSON arguments or deprecated aliases. The shared Vim/Helix layer binds `alt-shift-c` under `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. The handler consumes the pending Vim count and delegates to the same duplication helper as `vim::HelixDuplicateBelow`.

The helper defaults an absent count to one additional copy per original selection. It reads every display selection, preserves its oriented tail/head range, and appends eligible duplicate ranges to the selection set without editing text. For Above, it reverses the original selection iteration order before building the new range list. For each selection, it searches one display row at a time above the previous accepted range until both endpoints fit the mapped buffer rows or the top of the display map is reached. Endpoint columns are converted through UTF-16 buffer columns; a too-short row is skipped. It then maps the accepted display range back to buffer offsets using left/right bias.

The `shift-c` neighbor uses the same helper below the selection. The direct fixtures exercise `alt-C` above after selections have been duplicated below and also with a backward-oriented selection. The explicit count fixture covers `C` below, not `alt-C` above; the shared helper implements the same count loop for both directions. Count zero/extremes, destination collisions, selection ordering at complex multi-cursor arrangements, and profile/physical-key dispatch remain acceptance items.

## Pinned Helix workflow

In Helix Normal or Helix Select, `alt-shift-c` keeps every original selection and adds up to the requested number of copies above it. The default adds one copy per selection. Each copy retains the range's endpoint columns and orientation when a destination display row can fit both endpoints. Rows that are too short are skipped while searching farther above; the search ends at the top of the display map. If fewer than the requested number of destinations fit, fewer copies are added.

This is selection replication: the action does not edit buffer text, write Vim registers, or create a text transaction. New ranges join the existing selection set, with Above traversing the input selections in reverse order before assembling that set. `shift-c` is the corresponding below action. HelixDelete and Vim/Visual `Shift-C` are separate edit actions.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixDuplicateAbove](registered-actions-unbound.md) | Unit action. Consumes the pending count (default one), retains each original selection, and adds up to that many copies on eligible display rows above. It does not edit text. | Local `Duplicate Lines` is a text edit that duplicates selected physical lines below their last selected line; it does not add corresponding selections above. |
| [KEYBIND-VIM-HELIX-58875e17dc79](keymap-overlay-requirements.md) | `alt-shift-c` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. | No local Helix Normal/Select context or profile route exists. |
| [ACT-REGISTRY-LINUX-vim-HelixDuplicateBelow](registered-actions-unbound.md) | Neighboring `shift-c` calls the same directional helper with `Below`. | No local selection-only above or below duplication action exists. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile enables the shared Normal/Select route. | Labonair has Vim-mode configuration but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local owner is `labonair-editor`. Its `Duplicate Lines` edit collects physical line numbers covered by the current selection ranges, deduplicates those rows, and inserts each selected line after its existing line end. It edits document content and does not create a set of matching caret/range selections. No local `HelixDuplicateAbove` action, `alt-shift-c` profile route, or Helix mode exists. The local `VimMode` remains single-cursor and has no Helix Normal/Select states.

Settings exposes a Vim-mode value but no Helix-mode value. The pinned `SET-keymap-vim-mode` and `SET-keymap-helix-mode` settings are separate requirements; their UI, persistence, and runtime interaction remain unverified in Labonair.

## Pinned fixtures inspected

- `vim/src/helix.rs` declares both actions and routes their consumed count to the common directional helper in `vim/src/helix/duplicate.rs`.
- `helix/duplicate.rs::test_selection_duplication` exercises `C` below, then `alt-C` above, and then newest-selection retention.
- `test_selection_duplication_backwards` exercises Above after two Below copies with a backward-oriented range and checks the resulting oriented ranges.
- `test_selection_duplication_count` checks a large count for Below and shows that copies stop when no further rows remain before the document boundary.
- The multiline/multibyte and inlay-hint duplication fixtures exercise Below; they demonstrate UTF-16 column mapping and display-map handling but are not direct Above tests.
- Labonair's `crates/editor/src/editing.rs::duplicate_lines` and `EditorDuplicateLines` command route edit text below selected physical lines; they do not reproduce selections. Fixtures were inspected, not run.

## Acceptance still required

- Dispatch `alt-shift-c` in Helix Normal and Helix Select on macOS, Linux, and Windows. Verify menu guard, Vim/Helix profile layering, physical Option/Alt key mapping, focus, and direct `vim:` palette discovery/invocation.
- Verify default and counted Above behavior, count zero and extremes, document-top exhaustion, skipped short rows, multiple/overlapping/adjacent selections, duplicate destinations, selection order, and forward/reversed orientations.
- Verify physical lines versus soft-wrapped display rows, folds, inlays, tabs, wide/combining graphemes, UTF-16 columns, multiline selections, and endpoint clamping at line ends.
- Confirm no buffer edits, register writes, undo transactions, or stale selection goals occur; verify viewport and cursor visibility after adding selections.
- Capture paired native macOS Editor/Keymap evidence with initial ranges/order, count, skipped rows, resulting ranges/order, mode/profile, focus, viewport/wrap/inlay state, theme, and scale recorded.

## Source records inspected

- Action metadata and handlers: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`, `zed-refrence/zed/crates/vim/src/helix.rs`, and `zed-refrence/zed/crates/vim/src/helix/duplicate.rs`.
- Shared `alt-shift-c` binding and context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Direct Above/Below, backward-selection, count, UTF-16, and inlay fixtures: `zed-refrence/zed/crates/vim/src/helix/duplicate.rs`.
- Labonair duplicate-lines edit and command: `crates/editor/src/editing.rs` and `crates/editor/src/command_provider.rs`.
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
