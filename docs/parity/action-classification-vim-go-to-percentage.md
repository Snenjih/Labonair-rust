# Vim GoToPercentage Action Classification

**Status:** Counted percentage-row calculation and Labonair's bracket-only percent motion classified; row mapping, boundaries, operator, platform, and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-GoToPercentage`; shared Vim binding `KEYBIND-VIM-HELIX-958307561b7c`

**Owner:** Editor owns cursor/selection coordinates and document row mapping; Vim owns count, mode, motion, and operator dispatch; Keymap owns the shared Vim profile route; Settings owns Vim mode/profile values.

## Evidence boundary

The pinned Linux action registry declares `vim::GoToPercentage` as a unit action with no JSON arguments or deprecated aliases. The shared Vim profile binds `%` in `VimControl && VimCount`; the ordinary Vim Control `%` binding is a separate `vim::Matching { match_quotes: true }` action. Thus a numeric prefix followed by `%` takes the percentage route, while bare `%` remains bracket/quote matching. Direct palette discovery and invocation of the zero-argument action remain runtime acceptance; its motion defaults a missing count to one.

The action dispatches `Motion::GoToPercentage`. Its default motion kind is Linewise, its selection goal is None, and it is added to the jump list. The motion uses the pending count or one; when both an operator pre-count and a motion post-count are pending, `Vim::take_count` multiplies them. It computes `ceil(count * total_buffer_rows / 100) - 1` for the target index, constructs a `DisplayPoint` at that row and the current display column, and clips it with left bias. The formula is one-based: on a nine-row buffer, 20% selects row 2 and 100% selects row 9. The percentage is based on buffer rows, while the computed index is passed as a `DisplayRow`; soft wrapping and multibuffer behavior need direct acceptance.

The focused implementation preserves the starting display column subject to clipping. The function comment describes movement to the first nonblank in the target line, but the implementation passes through the starting column and the fixture confirms nonzero-column retention. This source-comment discrepancy is recorded rather than treating the comment as runtime behavior.

## Pinned percentage workflow

In Vim Control mode, bare `%` invokes bracket matching. After a count has started, `%` moves to the corresponding percentage position through the buffer, using the current column where the target row allows it. For an effective count `n` and `L` buffer rows, the target index is `ceil(n * L / 100) - 1`; an operator pre-count and motion post-count multiply to form `n`. The count is consumed once by the motion. The target is clipped through the display map. A direct action invocation with no pending count uses one percent.

The motion is linewise for operator dispatch and enters the jump list. In Visual mode, it extends the selection to the target; the fixture shows the visible selection endpoint. Forced-motion and operator paths can alter the usual motion kind, while exact deletion/yank/change ranges and column behavior need acceptance. The ordinary profile also has a separate `vim::Matching` route for `%`, so local bracket matching alone is not percentage parity.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-GoToPercentage](registered-actions-unbound.md) | Registered zero-argument linewise motion. Count defaults to one for direct invocation and selects a one-based percentage row, retaining the current display column subject to clipping. | Local Vim has no percentage action; its `%` resolver performs bracket matching. |
| [KEYBIND-VIM-HELIX-958307561b7c](keymap-overlay-requirements.md) | Shared `%` binding in `VimControl && VimCount`; ordinary `%` dispatches Matching separately. | Local count followed by `%` still takes the local bracket-matching resolver; there is no percentage route. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode/profile enables the counted `%` route and `vim:` Command Palette namespace; no percentage-specific setting exists. | Labonair has local Vim mode but no typed percentage action or corresponding profile route. |

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the future counted `%` binding. Labonair's [`vim.rs`](../../crates/editor/src/vim.rs) handles `%` as an inclusive bracket match and returns no percentage-based destination. Its `match_bracket_percent` fixture verifies matching parentheses in both directions. The resolver receives a count but the `%` branch does not use it. No local `GoToPercentage` action or separate keymap profile route was found. No new setting, event, notification, persistence, dependency, or control is needed for this motion.

## Pinned fixture inspected

- `motion.rs::test_go_to_percentage` uses a nine-row buffer and verifies Normal-mode counts 20, 25, and 75, plus Visual-mode counts 50 and 100. The expected endpoints retain the starting column where the target row is long enough, and 100% reaches the final row. The fixture does not cover no-count invocation, 1%, 0, values above 100, row clipping, blank/empty buffers, soft wrapping, multibuffers, linewise operators, palette dispatch, or platform layouts.

Pinned source and fixtures were inspected, not run. The local `match_bracket_percent` test was also inspected, not run.

## Acceptance still required

- Dispatch bare `%`, counted `N%`, leading/embedded zero counts, and direct `vim:` palette invocation on macOS, Linux, and Windows. Verify the physical `%` key sequence per layout and the `VimControl && VimCount` profile context.
- Verify counts 0, 1, 2, 20, 25, 50, 75, 99, 100, above 100, and extreme values; record the exact row under empty, one-row, and large buffers. Check pre-count/post-count multiplication, overflow behavior in the multiplication and count parser.
- Verify retained column for short and long target rows, indentation, tabs, wide/combining characters, and positions at or beyond line ends. Resolve the source comment's first-nonblank wording against runtime behavior.
- Verify logical rows versus display rows under wrapping, inlays, folds, excerpts, and multi-buffer documents. The implementation computes from buffer row count and constructs a `DisplayRow`; record the exact mapping.
- Verify Normal, Visual, Visual Line, Visual Block, Insert, Replace, and Helix behavior; single/multiple/reversed selections; operator delete/change/yank/indent ranges; forced motion; jump-list entries; mode transitions; cursor, undo unit, scroll, and fold effects.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, count, line count, start/target positions, selection/operator, wrapping/tab settings, theme, viewport, and scale recorded.

## Source records inspected

- Action registration, handler, motion defaults, jump-list and selection-goal policy, count dispatch, and percentage algorithm: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Counted `%` and ordinary bracket/quote `%` bindings: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Nine-row Normal/Visual percentage fixture: `zed-refrence/zed/crates/vim/src/motion.rs::test_go_to_percentage`.
- Local `%` resolver and bracket-match fixture: `crates/editor/src/vim.rs::resolve_motion` and `match_bracket_percent`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
