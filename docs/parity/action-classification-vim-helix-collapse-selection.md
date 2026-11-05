# Vim HelixCollapseSelection Action Classification

**Status:** Helix selection collapse and both shared `;` routes classified; selection edge cases, route precedence, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixCollapseSelection`; Helix Select binding `KEYBIND-VIM-HELIX-e0b158c1db46`; shared Helix binding `KEYBIND-VIM-HELIX-b76cb2779bad`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** The pinned Editor/Vim owns mode and selection behavior; Keymap owns both `;` routes. Labonair's current single-cursor selection and Vim state belong to `labonair-editor`; a Helix profile route belongs to `labonair-keymap` and its mode value to Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixCollapseSelection` as a unit action without JSON arguments or deprecated aliases. Its handler applies `SelectionEffects::no_scroll()` to every Editor selection. An empty selection collapses to its active head. A nonempty forward selection collapses one display point left from its active head, placing the cursor on the final selected display point. A reversed selection collapses directly to its active head. The existing selection goal is retained. The action does not edit text, consume a count, or change mode.

The Vim/Helix keymap contains two active `;` entries for the same action. The Helix Select layer binds `;` in `vim_mode == helix_select`, without an explicit `!menu` condition. A later selection-manipulation block binds `;` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. These contexts overlap in Helix Select when the menu is closed. Static records establish that both entries name the same action, but do not establish runtime precedence or menu behavior.

## Pinned Helix workflow

In Helix Select, `;` removes the active selection and leaves one caret per range. Forward ranges place the caret on the last selected display point by moving left from the exclusive head; reversed ranges use their active head directly. Empty cursors remain at their current head. Every range is handled independently and the selection update requests no scrolling.

The duplicated route also makes the action available in Helix Normal through the shared selection-manipulation row. Since Helix Normal selections are normally empty, this route leaves each cursor where it is. The action is distinct from `vim::HelixKeepNewestSelection` on `,`, which removes all but the newest selection, and from Labonair's existing Vim `;` repeat-find behavior.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixCollapseSelection](registered-actions-unbound.md) | Registered unit action. Collapses each selection without scrolling; forward nonempty ranges move one display point left from their head, while empty or reversed selections collapse to their head. | Local Editor selection primitives exist, but no Helix mode or action implementing this per-selection rule is present. |
| [KEYBIND-VIM-HELIX-e0b158c1db46](keymap-overlay-requirements.md) | `;` in `vim_mode == helix_select`, with no explicit menu exclusion in this row. | No local Helix Select context or `;` collapse route exists. |
| [KEYBIND-VIM-HELIX-b76cb2779bad](keymap-overlay-requirements.md) | `;` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`; overlaps the Helix Select row when the menu is closed. | No local Helix Normal/Select route exists; route precedence and menu behavior need runtime verification. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile enables the Helix Normal and Select keymap contexts. | Labonair has Vim mode configuration but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local owner is `labonair-editor`. Its `VimMode` includes Normal, Insert, Visual, VisualLine, and Command, but no Helix Normal or Helix Select. The local Vim `;` path repeats the prior character-find motion, as covered by `find_char_motion`; it does not collapse a selection. Local `Document` selection state is single-cursor and is not connected to a Helix action or profile route.

The local Settings schema exposes an Editor Vim-mode value, not a Helix-mode value. The pinned `SET-keymap-vim-mode` and `SET-keymap-helix-mode` settings are separate requirements; their UI, persistence, and runtime interaction remain unverified in Labonair.

## Pinned fixtures inspected

- `vim/src/normal.rs` declares the unit action and implements the no-scroll per-selection collapse rule.
- `assets/keymaps/vim.json` contains the Helix Select-specific and shared selection-manipulation `;` rows. Their conditions overlap in Helix Select when `!menu` holds.
- `vim/src/test.rs::find_char_motion` uses `;` to repeat a preceding `f` motion; it is not a HelixCollapseSelection fixture.
- No focused HelixCollapseSelection fixture was found in the inspected Vim source. Fixtures were inspected, not run.
- Labonair's `VimMode`, append insertion path, and semicolon repeat-find fixture were inspected; no Helix mode, multi-selection collapse action, or corresponding route was found.

## Acceptance still required

- Dispatch both rows in Helix Normal and Helix Select on macOS, Linux, and Windows. Verify the overlapping-row precedence, menu/open-overlay behavior, profile layering, focus, and direct `vim:` palette discovery/invocation.
- Verify empty, forward, reversed, one-character, linewise, multiline, and multiple selections; duplicate endpoints; selection ordering; and caret placement at line starts, line ends, EOF, tabs, wide/combining graphemes, folded text, and wrapped rows.
- Confirm selection-goal retention, no-scroll viewport behavior, and interaction with active Vim/Helix mode settings and selection-changing actions.
- Capture paired native macOS Editor/Keymap evidence with initial ranges and heads, resulting carets, mode/profile, menu and focus state, viewport, theme, and scale recorded.

## Source records inspected

- Action declaration, registry metadata, and handler: `zed-refrence/zed/crates/vim/src/normal.rs` and `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Both default `;` rows and their conditions: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Neighboring semicolon repeat-find fixture: `zed-refrence/zed/crates/vim/src/test.rs`.
- Labonair `VimMode`, semicolon repeat-find, and current single-cursor selection route: `crates/editor/src/vim.rs`.
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
