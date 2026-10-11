# Vim GoToPreviousTab Action Classification

**Status:** Counted active-Pane previous-item navigation and Labonair's different Workspace tab cycle classified; count edges, Pane scope, platform, and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-GoToPreviousTab`; shared profile binding `KEYBIND-VIM-HELIX-1ba39d12bbb8`

**Owner:** The pinned Workspace/Panes own the active Pane item collection and activation; Vim owns count and profile semantics; Keymap owns the shared `g shift-t` route. Labonair's related tab cycle is owned by `labonair-workspace`.

## Evidence boundary

The pinned Linux action registry declares `vim::GoToPreviousTab` as a unit action with no JSON arguments or deprecated aliases. The shared profile binds `g shift-t` with the context `VimControl && !menu || !Editor && !Terminal`. The neighboring `g t` binding dispatches the distinct GoToTab action, whose counted form selects an absolute one-based item index; it is not the same count behavior. The `GoToPreviousTab` action is declared in the Vim action registry and registered on Workspace. Direct `vim:` palette discovery/invocation and physical dispatch remain runtime acceptance.

The handler consumes the pending count and forced-motion state. If both Vim pre-count and post-count are set, `Vim::take_count` multiplies them. With no count, the handler dispatches `pane::ActivatePreviousItem::default()`, whose default wraps from the first item to the last. With effective count `n`, it reads the active Pane's item count `N`, gets the zero-based active index `i`, and dispatches `pane::ActivateItem((i - n).rem_euclid(N))` when `N > 0`. A count of one moves to the previous item; counts beyond `N` wrap by the Pane item count. The counted route returns without dispatch when there are zero items.

Pane activation updates its active item index, deactivates the prior item when needed, updates navigation history, toolbar, and status bar, focuses the destination item, emits `Event::ActivateItem`, scrolls the tab strip to an unpinned destination, and notifies. A count that resolves to the current index still dispatches `ActivateItem`; exact repeated focus/history effects are untested. The handler casts `usize` count to `isize` before `rem_euclid`, so extreme counts need acceptance.

## Pinned previous-item workflow

In a Vim-enabled Editor, `gT` without a count activates the prior tab item in the active Pane and wraps around. With count `n`, it moves backward `n` Pane items modulo the active Pane's item count. This means `2gT` moves to the second previous Pane item, rather than treating two as an absolute tab number. The action switches the active item and its focus; it does not manipulate document text.

Pinned `Pane::ActivatePreviousItem` and `Pane::ActivateItem` operate on the active Pane's items. This is a narrower collection than Labonair's top-level Workspace `TabStore`. Do not treat the existing local Previous Tab command as direct proof of Vim `gT` parity. The exact Pane-versus-workspace user expectation, item types, activation history, empty/single-item behavior, count edge cases, and dispatch in non-Editor contexts remain acceptance questions.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-GoToPreviousTab](registered-actions-unbound.md) | Registered unit action. Without a count, dispatches wrapped previous-item activation; with count, computes a backward modulo index in the active Pane. | Workspace has `CommandId::PrevTab`, but it cycles top-level Workspace tabs in the active Space and has no count or Vim action. |
| [KEYBIND-VIM-HELIX-1ba39d12bbb8](keymap-overlay-requirements.md) | Shared `g shift-t` binding in Vim Control while menu is closed, or outside Editor and Terminal contexts. | No local `gT`/GoToPreviousTab binding was found; local Previous Tab is bound as `ctrl-shift-tab`. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode/profile enables the `gT` route and Vim action namespace; no action-specific setting exists. | Local Vim mode exists, but its Workspace tab cycle is a separate command and scope. |

## Labonair owner and current route

The canonical owner for local tab navigation is `labonair-workspace`, which owns Workspace `TabStore` and its active tab. `CommandId::PrevTab` is registered by `crates/workspace/src/command_provider.rs` and calls `Workspace::cycle(false)`. The command descriptor labels it “Previous Tab” and binds `ctrl-shift-tab`. `TabStore::cycle` uses `adjacent_in_space`, wraps among tabs in the active Space, and does nothing if there is no adjacent tab (including a one-tab Space). Workspace then focuses the active tab.

Labonair has no local typed `vim::GoToPreviousTab` action, no `gT` profile binding, and no route for count-based previous-item activation within a Pane. The existing `PrevTab` command changes which top-level editor/terminal/SFTP or other Workspace tab is active; the pinned route changes an item in the active Pane. It is related functionality with a different owning state and selection scope, so it does not close the migration edge by itself.

## Pinned fixtures inspected

- `workspace/src/pane.rs::test_activate_item_with_wrap_around` creates three Pane items and verifies previous/next activation with wrapping disabled and enabled. It tests the Pane action behavior but not Vim `gT` or its counted index calculation.
- No direct `GoToPreviousTab` Vim action fixture was found in the pinned Vim crate. The action handler was inspected, not run.
- Labonair `tabs.rs::adjacent_tab_navigation_wraps_inside_the_source_space` checks previous wrap, multiple spaces, and unknown IDs. The one-tab case is represented by `adjacent_in_space` returning `None` when a Space has fewer than two tabs; tests were inspected, not run. They cover Workspace TabStore behavior, not Pane-item `gT` parity.

## Acceptance still required

- Dispatch `gT` on macOS, Linux, and Windows with Vim enabled, plus the `!Editor && !Terminal` keymap contexts. Verify the physical chord for each layout, menu suppression, profile overrides, and direct `vim:` palette discovery/invocation.
- Verify no count, `1gT`, operator pre-count plus motion post-count, counts greater than the Pane item count, count equal to item count, zero, and extreme counts. Record active index, item count, destination, whether it refocuses the same item, and the `usize` to `isize` conversion behavior.
- Verify empty and single-item Panes, pinned and unpinned items, mixed Editor/terminal/tool items, item closure/reordering during dispatch, multiple panes, and changes to which Pane is active.
- Verify activation focus, navigation history, toolbar/status updates, Pane activation events, tab-strip scrolling, workspace titlebar state, and any persisted selection effects. Confirm the previous document/item's unsaved and transient state survives switching.
- Compare `gT` inside the active Editor Pane with the local top-level `Previous Tab` command across multiple active Spaces. Confirm whether both workflows are required and record the distinct result collections.
- Capture paired native macOS Editor/Workspace/Keymap evidence with starting Pane and Workspace tab, item list/order, count, active Space, focus, profile, platform, viewport, theme, and scale recorded.

## Source records inspected

- Action declaration and Workspace handler, including no-count and counted branches: `zed-refrence/zed/crates/vim/src/normal.rs` and `zed-refrence/zed/crates/vim/src/vim.rs`.
- Shared `g t`/`g shift-t` binding and context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Pane previous-item wrap and item activation side effects: `zed-refrence/zed/crates/workspace/src/pane.rs`.
- Pane wrap fixture: `zed-refrence/zed/crates/workspace/src/pane.rs::test_activate_item_with_wrap_around`.
- Local Previous Tab command and default binding: `crates/workspace/src/command_provider.rs`.
- Local active-Space tab cycling and focus: `crates/workspace/src/tabs.rs` and `crates/workspace/src/workspace.rs`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
