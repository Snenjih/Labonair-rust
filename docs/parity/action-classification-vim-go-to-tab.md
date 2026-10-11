# Vim GoToTab Action Classification

**Status:** Uncounted next-item activation and counted absolute Pane-item selection classified; Pane edge cases, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-GoToTab`; shared profile binding `KEYBIND-VIM-HELIX-22f58bdb48cc`

**Owner:** The pinned Workspace/Panes own the active Pane item collection and activation; Vim owns count and profile semantics; Keymap owns the shared `g t` route. Labonair's related indexed tab commands are owned by `labonair-workspace`.

## Evidence boundary

The pinned Linux action registry declares `vim::GoToTab` as a unit action with no JSON arguments or deprecated aliases. The shared profile binds `g t` under `VimControl && !menu || !Editor && !Terminal`. Its sibling `g shift-t` dispatches GoToPreviousTab and has different counted semantics. The GoToTab action is registered on Workspace. Direct `vim:` palette discovery/invocation and physical key dispatch remain runtime acceptance.

The handler consumes the pending Vim count and forced-motion state. If both pre-count and post-count are set, `Vim::take_count` multiplies them. With no count, it dispatches `pane::ActivateNextItem::default()`, whose default wraps from the last item to the first. With effective count `n`, it computes `n.saturating_sub(1)` and dispatches `pane::ActivateItem` at that zero-based index. Thus `gt` advances one active-Pane item, while `Ngt` selects the absolute one-based Pane-item position `N`; it does not move `N` items relative to the current item and does not wrap. An index outside the active Pane's item list is ignored by Pane activation. A direct action invocation without a pending count uses the next-item branch.

Pane activation updates the active item index, deactivates the prior item when the index changes (or navigation-history traversal requires it), updates navigation history, toolbar, and status bar, focuses the destination, emits `Event::ActivateItem`, scrolls the tab strip to an unpinned destination, and notifies. For an empty Pane the next-item action attempts index zero, which activation rejects. A single-item Pane or selecting its current item still reaches activation side effects; their exact focus/history/event behavior is not directly covered by the inspected fixture. Count multiplication overflow remains an acceptance question. If the handler receives an effective zero count directly, `saturating_sub(1)` targets index zero; normal Vim input uses a leading `0` as a motion rather than a count prefix. The counted `gt` workflow is one-based.

## Pinned tab-selection workflow

In the shared Vim/Helix profile, `gt` without a count activates the next item in the active Pane, wrapping to its first item at the end. `Ngt` activates the item at one-based position `N` in that Pane. A valid counted position can therefore move either direction from the current item; an unavailable position is a no-op. Neither form edits document contents, but activation changes the focused item and its Pane state.

The active Pane's item list is not the same collection as Labonair's top-level Workspace tabs. Labonair exposes `Select Tab 1` through `Select Tab 9` and a `Switch Tab…` submenu for its Workspace tab surface; its indexed selection uses zero-based `Workspace::select_tab_by_index` and ignores a missing top-level slot. Those routes are related, but they do not establish `gt` parity for Pane items. Do not conflate counted `Ngt` absolute selection with counted `gT` relative previous-item traversal.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-GoToTab](registered-actions-unbound.md) | Registered unit action. No count activates the next item with wrap; count `N` selects active-Pane index `N - 1`, with invalid indices ignored. | Workspace has indexed Select Tab commands, but they target top-level Workspace tabs. There is no local Pane-item `GoToTab` action or counted Vim route. |
| [KEYBIND-VIM-HELIX-22f58bdb48cc](keymap-overlay-requirements.md) | Shared `g t` binding in Vim Control while menu is closed, or outside Editor and Terminal contexts. | No local `g t`/GoToTab Vim binding was found. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode/profile enables the `gt` route and Vim action namespace; no action-specific setting exists. | Local Vim mode exists, but the Workspace tab selectors use a separate command surface and collection. |

## Labonair owner and current route

The canonical local tab owner is `labonair-workspace`, which owns Workspace `TabStore` and top-level active tabs. `CommandId::SelectTab1` through `SelectTab9` are registered in `crates/workspace/src/command_provider.rs` and call `Workspace::select_tab_by_index`; the method selects a zero-based top-level slot and does nothing when it is absent. The command descriptors label these as “Select Tab 1” through “Select Tab 9.” `CommandId::SwitchTab` exposes the separate “Switch Tab…” submenu. `NextTab` and `PrevTab` cycle top-level tabs within the active Space.

No local `vim::GoToTab` action, `g t` profile binding, or Vim count-to-active-Pane-item route was found. The local indexed and cycle commands change the active Workspace tab, which may host an Editor, terminal, SFTP, or another tool. The pinned action changes an item within the already-active Pane. This is a related but distinct state owner and selection scope.

## Pinned fixtures inspected

- `workspace/src/pane.rs::test_activate_item_with_wrap_around` creates three Pane items and verifies next/previous activation at both ends with wrapping enabled and disabled. It tests Pane activation, not the Vim GoToTab handler or counted absolute-index calculation.
- No direct GoToTab Vim action fixture was found in the pinned Vim crate. The handler and Pane activation path were inspected, not run.
- Labonair `Workspace::select_tab_by_index` and command descriptors show the separate zero-based top-level selection route. Local tests for indexed Pane-item navigation do not exist because there is no such route.

## Acceptance still required

- Dispatch `gt` on macOS, Linux, and Windows with Vim enabled, including the `!Editor && !Terminal` context. Verify physical chord/layout, menu suppression, profile overrides, and direct `vim:` palette discovery/invocation.
- Verify no count and counts `1`, `2`, the last valid index, the first invalid index, counts above the item count, and the interaction of pre-count and post-count. Record destination and wrap behavior. Check count construction and multiplication at extreme values; normal leading-zero input and direct action invocation (including a supplied zero) are distinct cases.
- Verify empty and single-item Panes, selecting the active item, pinned and unpinned items, mixed Editor/terminal/tool items, item closure/reordering during dispatch, multiple Panes, and active-Pane changes.
- Verify activation focus, navigation history, toolbar/status updates, Pane activation events, tab-strip scrolling, Workspace titlebar state, and whether transient or unsaved item state survives switching.
- Compare active-Pane `gt` behavior with Labonair's top-level `Select Tab N`, `Switch Tab…`, and Next Tab workflows across Spaces. Record the distinct target collections and determine the required local contract.
- Capture paired native macOS Editor/Workspace/Keymap evidence with starting Workspace tab and Pane, ordered item list, count, focus, profile, platform, viewport, theme, and scale recorded.

## Source records inspected

- Action declaration, Workspace handler, count retrieval, and forced-motion consumption: `zed-refrence/zed/crates/vim/src/normal.rs`, `zed-refrence/zed/crates/vim/src/vim.rs`, and Linux action metadata in `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Shared `g t`/`g shift-t` binding and context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Pane next-item default, index bounds, activation side effects, and wrap fixture: `zed-refrence/zed/crates/workspace/src/pane.rs`, including `test_activate_item_with_wrap_around`.
- Labonair indexed tab commands and descriptors: `crates/workspace/src/command_provider.rs` and `crates/workspace/src/workspace.rs`.
- Labonair active-Space tab cycle: `crates/workspace/src/tabs.rs` and `crates/workspace/src/workspace.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
