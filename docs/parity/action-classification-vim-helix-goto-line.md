# Vim HelixGotoLine Action Classification

**Status:** Counted Helix line target, no-count behavior, and `shift-g` override classified; count/display-map edges, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixGotoLine`; binding `KEYBIND-VIM-HELIX-5d6dfd3d88fc`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** Editor owns target calculation and selection movement; Keymap owns the shared Helix `shift-g` route and its precedence over Vim's `EndOfDocument`. Labonair's current caret and line-jump operations belong to `labonair-editor`; a Helix route belongs to `labonair-keymap`, with mode values owned by Settings.

## Evidence boundary

The pinned Linux action registry declares `vim::HelixGotoLine` as a unit action without JSON arguments or deprecated aliases. The Helix layer binds `shift-g` under `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. In the broader `VimControl && !menu` context, the same chord maps to `vim::EndOfDocument`; the Helix-specific row overrides it. Static registry and profile evidence does not establish palette invocation or dispatch on macOS, Linux, or Windows.

The handler takes the Vim count and calls the Helix `StartOfDocument` motion only when a count is present. The helper interprets that count as a one-based buffer line, subtracts one with saturating arithmetic, clamps the row to the final buffer row, creates a column-zero buffer point, and maps it to a display point with left bias. Thus bare `shift-g` is a no-op, while a counted action moves to the requested line start. The movement path updates each Editor selection and clears its selection goal. Boundary behavior for empty buffers, very large counts, excerpts, folds, and wrapped display rows remains unverified at runtime.

## Pinned Helix workflow

In Helix Normal, a count followed by `shift-g` collapses the caret to column 0 of the one-based target line. A count targeting the current line still resets the caret to column 0. Without a count, the cursor is unchanged.

The same action is bound in Helix Select. The direct fixture expects bare `shift-g` to leave both the active selection and Helix Select mode unchanged. A counted `shift-g` moves to the target without extending the selection; its expected result is a collapsed caret at the target while Helix Select mode remains active. This differs from counted `gg`, which extends a Select range. These are fixture expectations from pinned source, not executed or native acceptance evidence.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixGotoLine](registered-actions-unbound.md) | Unit action consumes an optional Vim count; when present, it moves each selection to column 0 of the clamped one-based buffer line. Without a count it does not move. | Labonair has no Helix action or Helix mode. Its local Vim `G` is a distinct counted linewise motion that defaults to the final line and retains a clipped column. |
| [KEYBIND-VIM-HELIX-5d6dfd3d88fc](keymap-overlay-requirements.md) | `shift-g` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`; this overrides the shared VimControl `shift-g` route to `vim::EndOfDocument`. | No local Helix Normal/Select route or profile precedence exists. |
| [ACT-MACOS-go_to_line-Toggle](action-classification-go-to-line.md) | Separate Go to Line prompt accepts absolute line/column and relative line input, with preview and commit/cancel workflow. | Labonair has clamped line/position primitives and Outline known-line palette jumps, but no direct prompt; this does not implement counted Helix `shift-g`. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode/profile enables the Normal and Select binding contexts. | Labonair has Vim-mode configuration but no Helix mode/profile setting. |

## Labonair owner and current route

The canonical local capability owner is `labonair-editor`; `labonair-keymap` should own a future Helix profile route. Labonair's local Vim resolver maps `G` to a linewise range ending at the counted one-based row, or the final row without a count, and preserves the current column subject to target-line length. That behavior is partially present and is separately classified in [Vim EndOfDocument](action-classification-vim-end-of-document.md).

Labonair also has Editor `goto_line`/`goto_position` operations and a typed `SubmenuAction::GoToLine(usize)` used for known Outline symbol rows. It does not register the direct `go_to_line::Toggle` prompt, accept a Helix count through Keymap, or define Helix Normal/Select modes. The richer Go to Line prompt is a separate incomplete workflow, not a substitute for this counted action.

## Pinned fixtures inspected

- `vim/src/helix.rs::test_helix_goto_line` checks counted `2 shift-g` from line three to column 0 of line two, a count targeting the current line, bare `shift-g` as a no-op, unchanged Select state without a count, and a counted Select jump that collapses instead of extending the selection. The fixture was inspected, not run.
- `vim/src/helix.rs::helix_goto_line` only calls `helix_move_cursor(Motion::StartOfDocument, Some(line_number), ...)` when `Vim::take_count` returns a value.
- `vim/src/helix.rs::start_of_document` applies one-based count conversion, last-row clamping, and buffer-to-display point conversion at column 0. `helix_move_cursor` applies the target to all current selections with `SelectionGoal::None`.
- The Helix keymap entry uses `shift-g` in Normal/Select; the shared VimControl entry maps it to `vim::EndOfDocument`. The action is distinct from Helix `g e`, which is EndOfDocument, and ordinary Vim `g e`, which remains PreviousWordEnd.
- Labonair's `crates/editor/src/vim.rs` local `G` resolver and [Go to Line crosswalk](action-classification-go-to-line.md) were inspected. Tests and runtime were not run.

## Acceptance still required

- Verify macOS, Linux, and Windows registration, physical `Shift-G` dispatch, Helix mode settings, menu exclusion, focus, action-palette discovery/direct invocation, and profile precedence over Vim `EndOfDocument`.
- Verify count parsing, pre/post-count multiplication, count one, counts targeting the current/final line, very large counts, and any direct zero-count invocation. Confirm bare `shift-g` consumes no stale count and leaves cursor, selection, and mode unchanged.
- Verify single/multiple carets, forward and reversed active selections, Helix Normal and Select retention, target-line empty rows, final trailing newlines, empty buffers, and movement from before/after the target.
- Verify one-based buffer-row targets through soft wraps, folds, inlays, excerpts, multibuffer Editors, Unicode content, and line-ending variations; confirm column-zero buffer points map to the expected display location.
- Capture paired native macOS Editor/Keymap evidence with initial line/column and selection direction, mode, count, destination, resulting selection, focus/profile, viewport, theme, and scale recorded. Keep this action separate from local Vim `G` and the direct Go to Line prompt workflow.

## Source records inspected

- Action schema and documentation: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Count handler, target helper, selection motion, and direct fixture: `zed-refrence/zed/crates/vim/src/helix.rs` and `zed-refrence/zed/crates/vim/src/vim.rs`.
- Helix and Vim keymap layers: `zed-refrence/zed/assets/keymaps/vim.json` and the stable binding records in `keymap-overlay-requirements.md`.
- Labonair local Vim resolver: `crates/editor/src/vim.rs`; line-jump prompt/Outline route: `docs/parity/action-classification-go-to-line.md`, `crates/editor/src/command_provider.rs`, and `crates/workspace/src/workspace.rs`.
- Related action classifications: [Vim EndOfDocument](action-classification-vim-end-of-document.md) and [Editor Go to Line](action-classification-go-to-line.md).
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
