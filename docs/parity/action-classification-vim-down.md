# Vim Down Action Classification

**Status:** Logical/display-row motion and shared Vim/Helix routes classified; platform dispatch and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns cursor movement, selections, folds, and motion effects; Keymap owns the shared profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::Down` with an optional Boolean `display_lines` argument, defaulting to `false`. No platform-default map binds the action; the routes are in the shared Vim/Helix profile. The Vim action palette may expose the registered action, but availability, direct invocation, and focus behavior were not verified.

In the shared `VimControl && !menu` context, `j`, Down, and Ctrl-J use the default `display_lines: false`; `g j` and `g down` set it to `true`. A later Helix Normal context excludes menus and `BufferSearchBar` and overrides `j` and Down to `true`, while `g j` and `g down` use the default `false`. The shared rows also serve the other VimControl modes whose predicates match, including Vim Visual modes and Helix Select. This is static profile data; effective precedence, user overrides, and dispatch on macOS, Linux, and Windows remain runtime checks.

## Pinned motion behavior

The action passes its argument to `Motion::Down` and consumes the pending Vim count, defaulting to one. With `display_lines: false`, movement targets buffer rows and carries a horizontal goal across short rows; where a buffer row is wrapped, it attempts to retain the same wrapped-row offset and horizontal position. The fold-aware path maps the destination through folded rows. With `display_lines: true`, each step moves through rendered display rows, including wrapped segments, using the text layout geometry.

Both forms are classified as linewise Vim motions, regardless of `display_lines`. In Normal mode, a pending motion operator consumes the resulting linewise range. Without an operator, the cursor moves. Vim Visual and Visual Line extend their active selections; Visual Block uses its rectangular vertical-motion path. Helix Normal moves its cursor and Helix Select extends the active selection through its own mode route. Selection movement applies to the Editor's active selections. Motion count is clipped by available rows at document boundaries.

The `display_lines` argument changes which row geometry a vertical step traverses; it does not change the action's motion kind. Thus the shared Vim `j`/Down route advances through logical buffer rows, `g j` advances through display rows, and Helix Normal reverses those assignments. Wrapping, layout, folds, count, selection goal, and mode all affect the result. Bare motion changes transient cursor or selection state and does not edit text or registers.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim/Helix profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's local Vim resolver maps `j` and Enter to counted logical linewise motion. It has no typed `vim::Down` action, shared Vim/Helix profile, Helix mode, or display-row motion argument. In its current Vim path, movement reaches `Document::set_caret`, which clears the document's saved goal column; moving down through a short row therefore loses the earlier horizontal goal. The direct `Document::move_caret` path retains a goal column, but the local Vim resolver does not use it for `j`.

## Pinned fixtures inspected

`test_count_down` checks an explicit count and clamping at the last buffer row. `test_wrapped_lines` distinguishes ordinary `j` from `g j` across wrapped display rows. `test_wrapped_motions` checks vertical goal-column behavior with wrapped and multibyte text. `test_selection_goal` exercises retained vertical goal state; fold fixtures cover movement across collapsed content and linewise operator effects. The Helix suite contains general cursor tests but no focused count, wrapped-row, or profile-override test for `vim::Down`. These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence.

## Acceptance still required

- Verify macOS registration, palette availability and focus behavior, and the `j`, Down, Ctrl-J, `g j`, and `g down` routes in each applicable Vim mode. Verify Helix Normal's reversed row mapping, Helix Select inheritance, menu and `BufferSearchBar` guards, keymap precedence, profile loading, and user overrides.
- Verify count default, multi-digit counts, very large counts, zero/invalid count handling, top/EOF clipping, empty and final rows, and repeated movement across short then long rows. Check whether horizontal actions reset the preferred column.
- Compare buffer-row versus rendered-row motion under soft wraps, resize/reflow, tabs, wide glyphs, inlay hints, folds, and different line lengths. Check cursor geometry and goal persistence in both motion modes.
- Verify Vim Visual, Visual Line, Visual Block, Helix Select, and multiple active selections, including reversed selections, selection anchors, block widths, and wrap-boundary transitions.
- Verify pending delete/change/yank, indent/outdent, case conversion, and shell operators use linewise ranges with correct register contents, caret/mode transitions, undo grouping, and repeat behavior for counts and both `display_lines` values.
- Dispatch the shared profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, binding, wrap/fold geometry, count, starting goal column, result, theme, and scale recorded.

## Source records inspected

- Action schema and registry: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/crates/vim/src/motion.rs`.
- Shared Vim and Helix Normal routes: `zed-refrence/zed/assets/keymaps/vim.json`.
- Counted action dispatch, linewise classification, and display/buffer-row movement: `zed-refrence/zed/crates/vim/src/motion.rs`, `zed-refrence/zed/crates/vim/src/normal.rs`, and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Visual and Helix Select extension: `zed-refrence/zed/crates/vim/src/visual.rs` and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Count, wrapping, goal, fold, and selection fixtures: `zed-refrence/zed/crates/vim/src/test.rs` and `zed-refrence/zed/crates/vim/src/helix.rs`.
- Local Vim resolver and cursor movement: `crates/editor/src/vim.rs` and `crates/editor/src/document.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
