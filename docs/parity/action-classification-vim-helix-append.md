# Vim HelixAppend Action Classification

**Status:** Helix Normal/Select append and guarded no-edit Escape restoration classified; multiple-selection, edit/restore edges, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixAppend`; Helix Normal binding `KEYBIND-VIM-HELIX-94fcfa5159bd`; Helix Select binding `KEYBIND-VIM-HELIX-b981e342d5c0`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** The pinned Editor/Vim owns mode transitions, selections, insertion, and transient append state; Keymap owns the Helix Normal and Select routes. Labonair's current single-cursor Vim editor belongs to `labonair-editor`; a Helix profile route would belong to `labonair-keymap` and its mode value to Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixAppend` as a unit action with no JSON arguments or deprecated aliases. The shared profile binds `a` in two contexts: Helix Normal uses `VimControl && vim_mode == helix_normal && !menu && !BufferSearchBar`; Helix Select uses `vim_mode == helix_select`. These are distinct from Vim Normal `a` and Vim Visual `a` text-object behavior. Runtime profile precedence, direct `vim:` palette discovery/invocation, and physical key dispatch remain acceptance items.

The handler starts change recording, switches to Insert, and captures the current selection ranges. For each empty cursor it moves one display step right; for each nonempty selection it collapses to `selection.end`, the ordered right endpoint. It then captures the resulting empty cursor ranges in `HelixAppendState`. The action does not itself read a count; count interaction and any pending-count cleanup remain unverified.

On Escape, the Helix `NormalBefore` path switches back to Helix Normal. It restores the saved pre-append selections only when there was no insert transaction or current anchor and the current cursor ranges still exactly match the post-append ranges. This keeps a selection or original cursor when the user entered Insert and made no edits, while leaving the insertion outcome in place after edits or cursor movement. The direct HelixAppend fixture covers entry from Helix Select but not this no-edit restore guard, multiple selections, reversed ranges, or typed input. Escape-count behavior and the behavior when an edit transaction is started and then undone before Escape require acceptance.

## Pinned Helix append workflow

In Helix Normal, `a` enters Insert after the current cursor character by moving the empty cursor one display point to the right. In Helix Select, `a` enters Insert at the right edge of each selected range and collapses those ranges to insertion cursors. Multiple selections are processed together. Escape returns to Helix Normal; with no edits and unchanged insertion cursors it restores the original selections. If text was inserted, it keeps the post-edit insertion state.

This is separate from Vim Normal `a` and the Vim Visual `a` text-object binding, and from Vim Visual `shift-a`, which inserts after a selection. It also differs from Labonair's existing single-cursor Vim Normal `a` route: that route enters Insert after the current character but has no Helix Select mode or Helix-specific selection restoration.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixAppend](registered-actions-unbound.md) | Registered unit action. Moves empty cursors right one display step or collapses nonempty selections to their ordered end, enters Insert, and records state for conditional no-edit Escape restoration. | Local Vim Normal `a` enters Insert after the current character for one cursor; no Helix mode, multi-selection append, or guarded selection restore exists. |
| [KEYBIND-VIM-HELIX-94fcfa5159bd](keymap-overlay-requirements.md) | Helix Normal `a` under VimControl, menu, and BufferSearchBar guards. | No local Helix Normal context or binding exists. |
| [KEYBIND-VIM-HELIX-b981e342d5c0](keymap-overlay-requirements.md) | Helix Select `a` dispatches the same action. | No local Helix Select context or binding exists. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile gates Helix Normal/Select contexts; Vim mode enables the action layer. | Labonair has a Vim mode setting but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local owner is `labonair-editor`, which owns the compact `VimMode` state machine over `Document`. `VimMode` contains Normal, Insert, Visual, VisualLine, and Command, but no Helix Normal or Helix Select. In local Vim Normal, `a` calls `enter_insert` and advances the single caret by one column, clipped to the logical line end. The route has no Helix action or keymap profile; local Visual `a` semantics are outside this packet. Local `Document` selection state and its Vim adapter do not provide the pinned multi-selection model or `HelixAppendState`-style no-edit Escape restoration.

`SET-keymap-helix-mode` and `SET-keymap-vim-mode` are separate pinned settings. Their UI, saved/reloaded interaction, and the conditions under which each runtime context is active still need verification. The current Labonair Vim boolean does not close those Helix requirements.

## Pinned fixtures inspected

- `vim/src/helix.rs::test_helix_select_append` enables Helix, starts at a Helix Normal cursor, sends `v a`, and verifies that Insert mode places the cursor after the selected character. It tests one forward single-character selection and action entry only.
- `vim/src/helix.rs::test_helix_insert_before_after_helix_select` checks line insertion around forward and reversed Helix selections for the distinct `o`/`shift-o` routes, not HelixAppend's multi-selection behavior.
- The Insert `NormalBefore` handler's no-edit restoration guard was inspected; no focused fixture for restoring HelixAppend selections on Escape without edits was found. Fixtures were inspected, not run.
- Labonair's `VimMode` enum and `enter_insert('a')` path were inspected; no local Helix mode or route exists.

## Acceptance still required

- Dispatch `a` on macOS, Linux, and Windows in Helix Normal and Helix Select. Verify menu and BufferSearchBar precedence, profile layering, Vim/Helix settings interaction, direct `vim:` palette discovery/invocation, and physical key dispatch.
- Verify Helix Normal empty cursor at ordinary characters, EOL, final newline, EOF, tabs, wide/combining graphemes, folded and wrapped text; compare the one-display-step target with Helix's cursor model.
- Verify Helix Select forward/reversed ranges, one-character and linewise ranges, empty ranges, multiple/disjoint selections, duplicate endpoints, and selections spanning lines. Record exact post-collapse insertion points and selection order.
- Verify Escape with no edits, inserted text, backspace/undo returning to original text, cursor movement, popup dismissal, selection changes, interrupted focus, repeated Escape, and counts. Confirm when stored append state is discarded and how Undo/Redo and dot-repeat transactions behave.
- Verify transition to Insert and back to Helix Normal, selection/cursor orientation, register and clipboard state, jump/navigation history, language-service updates, and transient state when switching editor tabs or disabling Helix mode.
- Capture paired native macOS Editor/Keymap evidence with profile settings, starting mode/selections, content, no-edit/edit Escape paths, resulting mode/cursors, focus, viewport, theme, and scale recorded.

## Source records inspected

- Action declaration, registration, HelixAppend handler, and selection collapse: `zed-refrence/zed/crates/vim/src/helix.rs`.
- Helix Normal and Select bindings and contexts: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Append state and mode-transition cleanup: `zed-refrence/zed/crates/vim/src/vim.rs`.
- Insert Escape handling and conditional selection restoration: `zed-refrence/zed/crates/vim/src/insert.rs`.
- Direct Helix Select fixture and neighboring directional selection fixtures: `zed-refrence/zed/crates/vim/src/helix.rs`.
- Labonair local mode enum and single-cursor `a` insertion route: `crates/editor/src/vim.rs`; runtime view dispatch: `crates/workspace/src/views/editor.rs`.
- Mode settings: `docs/parity/settings-requirements.md`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
