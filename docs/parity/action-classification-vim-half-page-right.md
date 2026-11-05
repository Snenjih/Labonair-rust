# Vim HalfPageRight Action Classification

**Status:** Default and counted horizontal page-width scrolling classified, including the left-edge gutter correction; bounded-wrap, edge/mode cases, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HalfPageRight`; shared profile binding `KEYBIND-VIM-HELIX-848d02f12a49`

**Owner:** The pinned Editor owns viewport scrolling and selection visibility; Vim owns count and temporary-mode semantics; Keymap owns the shared `z shift-l` route. Labonair's Editor owns its horizontal viewport state and scrollbar interaction.

## Evidence boundary

The pinned Linux action registry declares `vim::HalfPageRight` as a unit action with no JSON arguments or deprecated aliases. The shared Vim/Helix profile binds `z shift-l` under `VimControl && !menu`; the neighboring `z shift-h` invokes HalfPageLeft. Physical dispatch and direct `vim:` palette discovery/invocation remain runtime acceptance.

The handler consumes Vim's pending count, clears forced-motion state, exits temporary Normal mode, and asks the Editor to scroll by `ScrollAmount::PageWidth`. With no count, the amount is `0.5`; with effective count `N`, it is `N`, so counted `N z shift-l` moves right by `N` full page widths rather than `N` half-widths. `ScrollAmount::PageWidth` converts that fraction to `(visible_column_count * amount).trunc()` columns. Under bounded soft wrap, when `preferred_line_length` is narrower than the viewport, Editor scrolling uses that preferred length as the effective width. Vim combines pre-count and post-count by multiplication; extreme count precision and conversion remain acceptance questions.

When the current horizontal scroll position is zero and the requested amount is positive, Editor adds the gutter margin converted to columns before applying the page-width movement. The resulting horizontal position is clamped by the Editor scroll manager. If the newest selection is on-screen before the scroll, Vim's scroll helper adjusts its head to stay inside the visible bounds; for a nonempty Visual selection it moves the head while retaining the anchor, and VisualBlock uses its separate block-motion path. Horizontal scroll clears the selection goal and disables navigation-history recording. If the newest selection is off-screen, the viewport scrolls without this cursor adjustment. Hover popovers and context menus can intercept editor scrolling; page-width pixel conversion is zero, so popup behavior needs direct verification. Single-line Editors propagate the scroll action rather than applying normal viewport scrolling.

## Pinned horizontal-scroll workflow

In the shared Vim Control context, `z shift-l` scrolls the active Editor viewport right by half its effective visible width when no count is pending. A count supplies a full-width multiple: `2 z shift-l` requests two page widths right. The command changes the view rather than buffer text. Starting at the leftmost offset also includes the gutter correction. Depending on whether the newest selection began on-screen, the cursor/selection may be clipped to stay visible after the viewport moves.

This action is horizontal. It is separate from `vim::ScrollUp`/`ScrollDown` and the vertical half-page actions. It is also distinct from `z l` (`vim::ColumnRight`), which requests a one-column scroll. The pinned `test_horizontal_scroll` fixture exercises `z shift-l`, then `z h` and `z l`; it verifies an uncounted six-column movement in a 12-column viewport.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HalfPageRight](registered-actions-unbound.md) | Registered unit action. No count scrolls half an effective page width right; count `N` scrolls `N` full widths. At the left edge, the gutter margin is added before movement. On-screen selection heads may be clipped into the new visible area. | Editor has horizontal scroll state and clamps it, but no typed Vim HalfPageRight action or keyboard page-width command. |
| [KEYBIND-VIM-HELIX-848d02f12a49](keymap-overlay-requirements.md) | Shared `z shift-l` binding under `VimControl && !menu`. | No local `z shift-l` profile route was found. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode/profile enables the `z shift-l` route; no action-specific setting exists. Editor soft-wrap and preferred-line-length settings can affect effective page width. | Local editor display settings affect layout, but no Vim profile route or horizontal page-width command exists. |

## Labonair owner and current route

The canonical local owner for editor viewport state is `labonair-editor`, hosted by Workspace. `crates/workspace/src/views/editor.rs` stores `scroll_left`, clips it against the maximum unwrapped line width, uses it to build the visible display snapshot, and updates it through horizontal scrollbar positioning. The editor view snapshot includes the horizontal offset for view restoration. No local `vim::HalfPageRight` action, `z shift-l` key route, or keyboard command for scrolling by a page-width fraction was found. The Linux default map's `ctrl-b` is bound to the unrelated Sidebar Toggle command.

The local scrollbar reaches the same broad horizontal viewport state but does not provide the keyboard-first half-width/count workflow. It is partial state infrastructure, not proof of action parity.

## Pinned fixtures inspected

- `vim/src/normal/scroll.rs::test_horizontal_scroll` sets a 12-column viewport and confirms uncounted `z shift-l` shifts the cursor six columns. It then checks `z h` does not move a cursor that remains visible after the scroll, and `z l` shifts one column. The fixture does not exercise counts, edge clamping, or bounded-wrap width changes.
- This is a direct HalfPageRight key-sequence fixture. The test was inspected, not run.
- Labonair's scrollbar and `scroll_left` paths were inspected; no analogous action/profile fixture exists.

## Acceptance still required

- Dispatch `z shift-l` on macOS, Linux, and Windows with Vim enabled. Verify the shared context, physical key layout, menu suppression, mode/profile overrides, and direct `vim:` palette discovery/invocation.
- Capture no count and counts `1`, `2`, and large values over even and odd viewport widths. Verify default half-width truncation, counted full-width movement, pre-count/post-count multiplication, left-edge gutter adjustment, right-edge clamping, long-line limits, count overflow/precision, and behavior when the buffer fits the viewport.
- Verify the visible-column source with unwrapped text, ordinary soft wrap, bounded soft wrap, preferred-line-length overrides, narrow windows, zoom/font changes, and resize between count collection and dispatch.
- Verify cursor and selection behavior when the newest selection is on-screen, left of the viewport, right of it, empty, multi-cursor, Visual, VisualBlock, and when focus is in a single-line input. Record selection anchors, goals, jump/navigation history, and temporary-Normal-mode exit.
- Verify hover popovers, context menus, folds, inlays, hidden/folded lines, gutter-width/theme changes, and editor scroll-beyond settings. Confirm whether horizontal offsets persist when switching items, closing/reopening an editor, or restoring a session.
- Capture paired native macOS Editor/Keymap evidence with viewport width, wrap settings, line content, initial scroll offset, gutter, caret/selections, count, focus, profile, platform, theme, and scale recorded.

## Source records inspected

- Action declaration, registration handler, count consumption, and temporary-mode exit: `zed-refrence/zed/crates/vim/src/normal/scroll.rs`.
- Shared binding and context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Page-width conversion and left-edge gutter/horizontal position update: `zed-refrence/zed/crates/editor/src/scroll/scroll_amount.rs` and `zed-refrence/zed/crates/editor/src/scroll.rs`.
- Cursor/selection adjustment and hover interception: `zed-refrence/zed/crates/vim/src/normal/scroll.rs` and `zed-refrence/zed/crates/editor/src/hover_links.rs`.
- Direct horizontal-scroll fixture: `zed-refrence/zed/crates/vim/src/normal/scroll.rs::test_horizontal_scroll`.
- Labonair horizontal viewport state and scrollbar path: `crates/workspace/src/views/editor.rs`; local default `ctrl-b` route: `crates/keymap/assets/keymaps/default-linux.json`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
