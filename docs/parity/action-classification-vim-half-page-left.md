# Vim HalfPageLeft Action Classification

**Status:** Default and counted horizontal page-width scrolling classified; direct-left, edge/mode cases, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HalfPageLeft`; shared profile binding `KEYBIND-VIM-HELIX-842b17751544`

**Owner:** The pinned Editor owns viewport scrolling and selection visibility; Vim owns count and temporary-mode semantics; Keymap owns the shared `z shift-h` route. Labonair's Editor owns its horizontal viewport state and scrollbar interaction.

## Evidence boundary

The pinned Linux action registry declares `vim::HalfPageLeft` as a unit action with no JSON arguments or deprecated aliases. The shared Vim/Helix profile binds `z shift-h` under `VimControl && !menu`; the neighboring `z shift-l` invokes HalfPageRight. Physical dispatch and direct `vim:` palette discovery/invocation remain runtime acceptance.

The handler consumes Vim's pending count, clears forced-motion state, exits temporary Normal mode, and asks the Editor to scroll by `ScrollAmount::PageWidth`. With no count, the amount is `-0.5`; with effective count `N`, it is `-N`, so counted `N z shift-h` moves left by `N` full page widths rather than `N` half-widths. `ScrollAmount::PageWidth` converts that fraction to `(visible_column_count * amount).trunc()` columns. Under bounded soft wrap, when `preferred_line_length` is narrower than the viewport, Editor scrolling uses that preferred length as the effective width. Vim combines pre-count and post-count by multiplication; extreme count precision and conversion remain acceptance questions.

The Editor scrolls the viewport horizontally and clamps its scroll position. If the newest selection is on screen before the scroll, Vim's scroll helper adjusts its head to remain inside the resulting visible bounds; for a nonempty Visual selection it changes the head while retaining the anchor, and VisualBlock uses its separate block-motion path. Horizontal scroll clears the selection goal and disables navigation-history recording. If the newest selection is off-screen, the viewport scrolls without this cursor adjustment. Hover popovers and context menus can intercept editor scrolling; page-width pixel conversion is zero, so popup behavior needs direct verification. Single-line Editors propagate the scroll action rather than applying normal viewport scrolling.

## Pinned horizontal-scroll workflow

In the shared Vim Control context, `z shift-h` scrolls the active Editor viewport left by half its effective visible width when no count is pending. A count supplies a full-width multiple: `2 z shift-h` requests two page widths left. The command changes the view rather than buffer text. Depending on whether the newest selection began on-screen, the cursor/selection may be clipped to stay visible after the viewport moves.

This action is horizontal. It is separate from `vim::ScrollUp`/`ScrollDown` and the vertical half-page actions. It is also distinct from `z h` (`vim::ColumnLeft`), which requests a one-column scroll. The neighboring horizontal fixture covers `z shift-l` (HalfPageRight), then `z h` and `z l`; it does not exercise `z shift-h` itself.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HalfPageLeft](registered-actions-unbound.md) | Registered unit action. No count scrolls half an effective page width left; count `N` scrolls `N` full widths. On-screen selection heads may be clipped into the new visible area. | Editor has horizontal scroll state and clamps it, but no typed Vim HalfPageLeft action or keyboard page-width command. |
| [KEYBIND-VIM-HELIX-842b17751544](keymap-overlay-requirements.md) | Shared `z shift-h` binding under `VimControl && !menu`. | No local `z shift-h` profile route was found. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode/profile enables the `z shift-h` route; no action-specific setting exists. Editor soft-wrap and preferred-line-length settings can affect effective page width. | Local editor display settings affect layout, but no Vim profile route or horizontal page-width command exists. |

## Labonair owner and current route

The canonical local owner for editor viewport state is `labonair-editor`, hosted by Workspace. `crates/workspace/src/views/editor.rs` stores `scroll_left`, clips it against the maximum unwrapped line width, uses it to build the visible display snapshot, and updates it through horizontal scrollbar positioning. The editor view snapshot includes the horizontal offset for view restoration. No local `vim::HalfPageLeft` action, `z shift-h` key route, or keyboard command for scrolling by a page-width fraction was found. The Linux default map's `ctrl-b` is bound to the unrelated Sidebar Toggle command.

The local scrollbar reaches the same broad horizontal viewport state but does not provide the keyboard-first half-width/count workflow. It is partial state infrastructure, not proof of action parity.

## Pinned fixtures inspected

- `vim/src/normal/scroll.rs::test_horizontal_scroll` sets a 12-column viewport and confirms uncounted `z shift-l` shifts the cursor six columns. It then checks that `z h` leaves the cursor where it is while that position remains visible, and that `z l` shifts one column. It does not invoke `z shift-h` or exercise counts.
- No direct `HalfPageLeft` fixture was found. The action handler and Editor scroll calculation were inspected, not run.
- Labonair's scrollbar and `scroll_left` paths were inspected; no analogous action/profile fixture exists.

## Acceptance still required

- Dispatch `z shift-h` on macOS, Linux, and Windows with Vim enabled. Verify the shared context, physical key layout, menu suppression, mode/profile overrides, and direct `vim:` palette discovery/invocation.
- Capture no count and counts `1`, `2`, and large values over even and odd viewport widths. Verify the default half-width truncation, counted full-width movement, pre-count/post-count multiplication, left-edge clamping, long-line limits, count overflow/precision, and behavior when the buffer fits the viewport.
- Verify the visible-column source with unwrapped text, ordinary soft wrap, bounded soft wrap, preferred-line-length overrides, narrow windows, zoom/font changes, and resize between count collection and dispatch.
- Verify cursor and selection behavior when the newest selection is on-screen, left of the new viewport, right of it, empty, multi-cursor, Visual, VisualBlock, and when focus is in a single-line input. Record selection anchors, goals, jump/navigation history, and temporary-Normal-mode exit.
- Verify hover popovers, context menus, folds, inlays, hidden/folded lines, and editor scroll-beyond settings. Confirm whether horizontal offsets persist when switching items, closing/reopening an editor, or restoring a session.
- Capture paired native macOS Editor/Keymap evidence with viewport width, wrap settings, line content, scroll offset, caret/selections, count, focus, profile, platform, theme, and scale recorded.

## Source records inspected

- Action declaration, registration handler, count consumption, and temporary-mode exit: `zed-refrence/zed/crates/vim/src/normal/scroll.rs`.
- Shared binding and context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Page-width conversion and Editor horizontal position update/clamping: `zed-refrence/zed/crates/editor/src/scroll/scroll_amount.rs` and `zed-refrence/zed/crates/editor/src/scroll.rs`.
- Cursor/selection adjustment, hover interception, and neighboring fixture: `zed-refrence/zed/crates/vim/src/normal/scroll.rs`, `zed-refrence/zed/crates/editor/src/hover_links.rs`, and `zed-refrence/zed/crates/editor/src/editor_tests.rs`.
- Labonair horizontal viewport state and scrollbar path: `crates/workspace/src/views/editor.rs`; local default `ctrl-b` route: `crates/keymap/assets/keymaps/default-linux.json`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
