# Vim HelixGotoLastModification Action Classification

**Status:** Helix `g .` mark jump and edit-anchor lifecycle classified; mode after insertion, mark lifetime, multi-cursor behavior, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixGotoLastModification`; binding `KEYBIND-VIM-HELIX-a2b47adfeb31`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** The pinned Editor/Vim owns the edit-position list, current `.` mark, and jump behavior; Keymap owns the shared Helix `g .` route. Labonair's current document transaction and Vim state belong to `labonair-editor`; a Helix profile route belongs to `labonair-keymap` and its mode value to Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixGotoLastModification` as a unit action without JSON arguments or deprecated aliases. The shared Vim/Helix layer binds `g .` under `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. The handler calls the Vim mark-jump path for `"."` with `line=false` and `should_pop_operator=false`; it does not consume a count or explicitly switch modes.

The `.` mark is refreshed on each `EditorEvent::Edited`. The Vim change-list hook captures all adjusted display selections as anchors, moves one display point left from each head when the edit event arrives in Insert mode, updates the Editor's in-memory change-position list, and sets the current Editor buffer's local Vim `"."` mark to those positions. The Editor change list can coalesce edits on the same display rows; the `"."` mark is still refreshed to the latest positions. Since `"."` is neither an uppercase nor a digit mark, it is not global: singleton-buffer marks are serialized through Vim's marks store, while multi-buffer marks are held on the multibuffer entity. Exact lifetime after rebasing, reload, and restart still needs acceptance. This mark jump is distinct from `g ;`/`g ,`, which traverse older/newer change-list entries, and from plain `.` repeat.

When the mark exists, `jump` creates navigation history. With no active operator it replaces selections with collapsed ranges at the stored anchors, removing consecutive duplicate anchors. With an active operator it passes the anchor through the jump-motion path. If no mark exists, it returns without moving or adding navigation history. The jump uses the current mode's generic behavior and the action handler itself does not switch Helix modes. Helix Select mode retention, especially after replacing a nonempty range with collapsed anchors, remains runtime acceptance.

## Pinned Helix workflow

In Helix Normal, `g .` returns the caret set to the latest recorded modification position(s), including the left-adjusted edit position recorded during Insert mode. It adds a navigation history entry when a mark is available. It changes no text and does not traverse the older/newer change list.

The same key route is present in Helix Select. The generic jump handler updates selections but does not explicitly switch from Helix Select to Helix Normal; runtime verification must establish the resulting mode and selection behavior. The direct pinned fixture exercises only the Normal route after inserting text and moving the cursor away. It includes a TODO warning that the mode may no longer be Helix after Escape; fixtures were inspected, not run.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixGotoLastModification](registered-actions-unbound.md) | Unit action jumps to the current Editor buffer's local Vim `.` mark. A successful jump adds navigation history and selects collapsed anchor ranges; absent mark is a no-op. | Labonair has document transaction history but no last-edit anchor mark, Helix action, or `g .` route. |
| [KEYBIND-VIM-HELIX-a2b47adfeb31](keymap-overlay-requirements.md) | Shared `g .` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. | No local Helix Normal/Select route exists. |
| [ACT-REGISTRY-LINUX-vim-ChangeListOlder](registered-actions-unbound.md) and [ACT-REGISTRY-LINUX-vim-ChangeListNewer](registered-actions-unbound.md) | `g ;` and `g ,` navigate indexed older/newer edit positions; they are distinct from jumping to the latest `.` mark. | Local Editor has undo/redo and Git changed-line navigation, but no edit-position list or matching Vim actions. |
| [ACT-REGISTRY-LINUX-vim-Repeat](registered-actions-unbound.md) | Plain `.` repeats the last recorded Vim edit; it is not the Helix `g .` navigation action. | Local editor has no Helix mode or corresponding repeat/jump key routes. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile enables the Normal/Select binding contexts. | Labonair has Vim-mode configuration but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local owner is `labonair-editor`. `Document` keeps buffer transactions and undo/redo snapshots, but inspection found no last-edited anchor, Vim `"."` mark, or Editor change-position list. Local Git change navigation operates on changed Git lines and is not a substitute for the most recent edit location. Local `VimMode` has no Helix Normal/Select states, and there is no `g .` route.

Settings exposes a Vim-mode value but no Helix-mode value. The pinned `SET-keymap-vim-mode` and `SET-keymap-helix-mode` settings are separate requirements; their UI, persistence, and runtime interaction remain unverified in Labonair.

## Pinned fixtures inspected

- `vim/src/helix.rs::test_goto_last_modification` enables Helix, inserts `modified ` at a cursor, moves away, sends `g .`, and expects the caret to return before the inserted trailing space. It also has a TODO warning that the mode may no longer be Helix after Escape; this is expected fixture state, not verified passing evidence.
- The action handler in `vim/src/helix.rs` delegates to `Vim::jump` with mark `"."`; `vim/src/normal/mark.rs::jump` resolves marks, creates navigation history, and selects anchors or applies a jump motion under an active operator.
- `vim/src/change_list.rs::push_to_change_list` receives `EditorEvent::Edited`, captures current selections, adjusts Insert-mode positions left by one display point, updates `Editor::change_list`, and refreshes mark `"."`. `vim/src/state.rs::MarksState` stores `"."` as a local mark, serializing singleton-buffer marks and holding multibuffer marks in memory.
- `editor/src/editor.rs::ChangeList` groups positions and provides older/newer traversal; these are separate from `g .`'s current-mark lookup.
- Labonair's `Document` transaction history and local Vim mode/routes were inspected; no matching last-edit position state was found. Tests were inspected, not run.

## Acceptance still required

- Dispatch `g .` in Helix Normal and Helix Select on macOS, Linux, and Windows. Verify menu guard, profile layering, focus, physical chord dispatch, and direct `vim:` palette discovery/invocation.
- Verify the post-Insert/Escape mode and cursor, whether Helix Select remains selected or becomes Normal, missing-mark behavior, navigation-history entries, multiple carets, repeated edits, same-row edit coalescing, and how the `"."` mark changes after Undo/Redo.
- Verify insertions, deletions, replacements, formatting, external edits, linked edits, empty buffers, EOF, line endings, folded/wrapped rows, and stale/rebased anchors. Confirm the Insert left-adjustment targets the expected character for empty and nonempty selections.
- Confirm mark scope and persistence across editor tabs, files, workspaces, buffer reloads, application restart, and mode-setting changes, including the different singleton/multibuffer stores. Keep the action distinct from change-list traversal and dot-repeat.
- Capture paired native macOS Editor/Keymap evidence with modification position, active mode, post-edit cursor, moved-away cursor, `g .` result, navigation history, profile/focus, viewport, theme, and scale recorded.

## Source records inspected

- Action schema, handler, and shared keymap route: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`, `zed-refrence/zed/crates/vim/src/helix.rs`, and `zed-refrence/zed/assets/keymaps/vim.json`.
- Mark resolution and jump/navigation behavior: `zed-refrence/zed/crates/vim/src/normal/mark.rs`.
- Edit event, change-list, local-mark scope/storage, and `"."` update: `zed-refrence/zed/crates/vim/src/vim.rs`, `zed-refrence/zed/crates/vim/src/change_list.rs`, `zed-refrence/zed/crates/vim/src/state.rs`, and `zed-refrence/zed/crates/editor/src/editor.rs`.
- Direct expected-location fixture with TODO: `zed-refrence/zed/crates/vim/src/helix.rs::test_goto_last_modification`.
- Labonair transaction history, Vim modes, and Git changed-line navigation: `crates/editor/src/document.rs`, `crates/editor/src/vim.rs`, and `crates/editor/src/git.rs`.
- Related action classifications: [Vim ChangeListOlder](action-classification-vim-change-list-older.md) and [Vim ChangeListNewer](action-classification-vim-change-list-newer.md).
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
