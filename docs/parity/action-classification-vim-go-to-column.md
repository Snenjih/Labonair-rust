# Vim GoToColumn Action Classification

**Status:** Static counted column route and Labonair's missing motion classified; display-map edge behavior, platform dispatch, operator, and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-GoToColumn`; shared Vim binding `KEYBIND-VIM-HELIX-6e9c8371004a`

**Owner:** Editor owns cursor/selection coordinates and display mapping; Vim owns count and motion/operator dispatch; Keymap owns the shared Vim profile route; Settings owns Vim mode/profile values.

## Evidence boundary

The pinned Linux action registry declares `vim::GoToColumn` as a unit action with no JSON arguments or deprecated aliases. It is absent from the macOS, Linux, and Windows default keymap files. The shared Vim/Helix profile binds `|` only in `VimControl && !menu`; no Helix-specific GoToColumn row was found. Because the action has no required arguments, the generic `vim:` Command Palette action list can construct it when the Vim namespace is available. Palette discovery and direct invocation remain runtime acceptance.

The action handler dispatches `Motion::GoToColumn`. The motion uses the current buffer row's start and the pending Vim count. With no count, the motion uses one step and targets the row start. For a count `n`, it calls the rightward step `n - 1` times, so the count is one-based: `1|` targets the first position and `18|` targets the eighteenth position subject to DisplaySnapshot clipping. The rightward helper increments a `DisplayPoint` column and clips with right bias. The motion therefore uses the display map's coordinate and clipping pipeline rather than indexing the local Rust string by character count.

`GoToColumn` has an exclusive motion kind, clears the horizontal selection goal, and is not recorded in the jump list. Vim Normal moves the caret; Visual modes extend the selection; an active operator consumes the exclusive range. Forced-motion state can alter the usual exclusive/inclusive operator kind. The motion itself does not edit the buffer or save independent state.

## Pinned column workflow

In Vim Control mode, pressing `|` moves to the first position of the current logical buffer row. A numeric prefix `n` moves to the `n`th position by starting at the row beginning and applying `n - 1` clipped right steps. The current row is found with `start_of_relative_buffer_row(point, 0)`, so this is not a line jump and does not preserve the current horizontal column.

The point advancement goes through `movement::saturating_right` and `DisplaySnapshot::clip_point`. The source establishes that it advances in display-map coordinates and stops once clipping makes the point stop moving. It does not by itself settle how an `n|` target behaves inside a tab expansion, across a soft-wrapped segment, at a wide or combining grapheme, near an inlay/fold/block, or at a clipped line end. Those are acceptance cases rather than inferred behavior. `0` remains a separate start-of-line motion, and `^` remains the first-non-whitespace motion.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-GoToColumn](registered-actions-unbound.md) | Registered zero-argument motion; the Vim count selects a one-based target position on the current logical row, defaulting to the first position. | No local GoToColumn action or absolute-column motion route was found. |
| [KEYBIND-VIM-HELIX-6e9c8371004a](keymap-overlay-requirements.md) | Shared `|` binding in `VimControl && !menu`; it can move, extend a Visual selection, or supply an operator motion. | Local Vim has no `|` branch in its motion resolver or shared profile route. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode enables the shared profile and `vim:` Command Palette namespace; no GoToColumn-specific setting exists. | Labonair has a local Vim toggle but no equivalent typed action or profile row. |

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the future `|` binding. No new setting, event, notification, persistence, dependency, or control is needed for the action.

Labonair's [`vim.rs`](../../crates/editor/src/vim.rs) has no GoToColumn action or `|` case in `resolve_motion`; its resolver handles `0` and `^` as distinct line-start motions. No matching action or Keymap profile entry was found elsewhere in the local Editor, Keymap, or Workspace crates. A generic line/column picker, if used elsewhere, is not this counted Vim `|` motion.

## Pinned fixtures inspected

- `object.rs::test_vertical_bars` uses `18|` to place the caret before a nested `|...|` pair, asserts the resulting position, and then exercises the separate `c i |` and `v a |` vertical-bar text objects. This is one counted-motion integration example, not a GoToColumn test matrix.

Pinned source and fixtures were inspected, not run. No dedicated GoToColumn fixture was found for no-count movement, count boundaries, display-map edge cases, Visual selection, operators, palette invocation, or platform dispatch.

## Acceptance still required

- Dispatch `|` on macOS, Linux, and Windows under the shared Vim profile. Verify the physical key sequence for the platform layout, profile overrides, `VimControl` behavior, `!menu`, and direct `vim:` palette discovery/invocation.
- Verify no count, `1|`, counts above one, `18|`, very large counts, a leading zero, and count reset after motion. Record the exact buffer/display point for each target.
- Verify empty and short rows, trailing newline, final unterminated row, and counts past the available row. Establish whether clipping lands on the last character or an after-end point in every Editor mode.
- Verify tabs at different tab widths, wide Unicode characters, combining characters, soft-wrapped logical lines, inlays, folds, diff blocks, and multibuffer excerpts. Record when each counted step crosses a display segment or snaps to a valid text position.
- Verify Normal and Visual/Visual Line/Visual Block movement, reversed and multiple selections, and counted delete/change/yank/indent consumers in both directions. Include forced-motion state and confirm operator range endpoints, resulting document, cursor, mode, and undo unit.
- Invoke the registered action directly with no key count in Vim Normal, Visual, Insert, Replace, and Helix modes; verify its target, active selection, mode effects, focus, and any unavailable-view behavior.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, action/count, line text, target position, tab/wrap settings, mode/operator, theme, scale, and viewport recorded.

## Source records inspected

- Action registration, handler, motion kind, selection goal, jump-list behavior, DisplayPoint stepping, and counted target: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Shared `|` keymap binding: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- DisplayPoint clipping and tab/wrap map transitions: `zed-refrence/zed/crates/editor/src/movement.rs`, `zed-refrence/zed/crates/editor/src/display_map.rs`, `zed-refrence/zed/crates/editor/src/display_map/tab_map.rs`, and `zed-refrence/zed/crates/editor/src/display_map/wrap_map.rs`.
- Counted integration fixture and vertical-bar text objects: `zed-refrence/zed/crates/vim/src/object.rs::test_vertical_bars`.
- Local `|` resolver and existing `0`/`^` motions: `crates/editor/src/vim.rs`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
