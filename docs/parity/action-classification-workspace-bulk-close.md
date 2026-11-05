# Workspace Bulk Item and Pane Close Action Classification

**Status:** Static intent, Workspace ownership, key routes, and current local
entry points; runtime dispatch, cancellation, focus, and paired visual
acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns selecting items across center
panes and removing panes that become empty. View owners keep their item state
and save/close behavior; Editor owns text-buffer dirty state.

## Evidence boundary

This packet describes two bulk actions over the items in center panes. It
records which items survive each action, how the optional save intent works,
and how Labonair's Space-scoped universal tabs differ from pane-local item
closing. Static inspection does not establish native key dispatch or rendered
interaction.

The requirements are independently stated for Labonair's Workspace model.
This is same-session independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux host
without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) owns selecting items across center panes, applying retain/close rules, removing empty panes, and updating layout. Each view owner controls saving and closing its content. |
| Canonical user entry points | Pinned actions use the Pane context. macOS: close all `Cmd+K Cmd+W`; close inactive `Ctrl+Alt+Cmd+W`. Linux: `Ctrl+K Ctrl+W`; `Alt+Ctrl+Shift+W`. Windows: `Ctrl+K Ctrl+W`; `Ctrl+Shift+Alt+W`. Labonair exposes `Close Others` and `Close All` from tab context menus and `Close Other Tabs` in the command palette. |
| Public typed contract | `CloseAllItemsAndPanes` closes every unpinned item in every center pane. `CloseInactiveTabsAndPanes` keeps the active item in the active center pane and closes other unpinned items in every center pane. Both actions preserve pinned items. An optional `save_intent` defaults to `Close`; the inactive action's retained-active-pane path uses the default close intent, while other panes use the supplied value. Empty panes are removed when possible, but the last center pane remains. |
| State and persistence | The actions mutate pane item sets and may remove empty panes. Workspace owns the resulting layout state. No automatic reopen history is established by these actions; verify saved layout and session restore separately. |
| Save intent | The optional value uses the existing `SaveIntent` contract. With the default `Close`, dirty items use their normal save-on-close behavior. When more than one eligible dirty item is in a pane, the group prompt offers Save all, Discard all, or Cancel. An individual save error offers Close Without Saving or Cancel. Verify each non-default save intent and its effect on all panes. |
| Cancellation and partial completion | The group Cancel result stops that pane's current close operation; the overall action is not transactional, so pane tasks already completed remain closed and other pane tasks may still close their own items. An individual item cancelled after a save error stays open while the close attempt continues for later items in that pane. No rollback is promised. Verify exact ordering, prompts, focus, and partial outcomes at runtime. |
| Events and registry contributions | Workspace remains the action owner. No shell-wide close table, new event, item registry, setting, or dependency is needed for this inventory slice. |
| Settings and notifications | No new setting. Ordinary close and user cancellation do not need a notification; save failures use the view's existing close prompt. |
| UI-kit | Reuse the shared tab, pane, menu, and dialog components. Add no local bulk-close controls in this inventory slice. |

## Action requirements

| Requirement ID | Platform binding | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-CloseAllItemsAndPanes`](action-requirements.md) | macOS `Cmd+K Cmd+W`; Linux/Windows `Ctrl+K Ctrl+W` | Close unpinned items across all center panes; remove empty panes where possible and retain the final center pane. The default save intent is `Close`. | The Pane packet classifies the separate pane-local `CloseAllItems` action. Labonair `Close All` closes tabs in one Space, not every pane item in the Workspace. |
| [`ACT-MACOS-workspace-CloseInactiveTabsAndPanes`](action-requirements.md) | macOS `Ctrl+Alt+Cmd+W`; Linux `Alt+Ctrl+Shift+W`; Windows `Ctrl+Shift+Alt+W` | Keep the active item in the active center pane, preserve pinned items everywhere, and close other unpinned items across center panes. The optional save intent defaults to `Close`; the retained-pane path uses that default. | The Pane packet classifies the separate pane-local `CloseOtherItems` action. Labonair `Close Others` and its palette command retain one universal tab in one Space; they do not retain one item while closing across center panes. |

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`Workspace::close_others` and `close_all_tabs`](../../crates/workspace/src/workspace.rs) | Close Others retains the selected tab; Close All closes tabs within that tab's Space. Both route through the Workspace close-request flow. | These are universal-tab operations scoped to a Space, not item operations over center panes. |
| [`CloseOtherTabs` command registration](../../crates/workspace/src/command_provider.rs) | The command palette exposes closing the other tabs in the active Space while keeping the active tab. | It has no pinned-item policy or cross-pane item scope. |
| [`TabStore::close_others` and `TabStore::close_by_kind`](../../crates/workspace/src/tabs.rs) | The tab store removes the selected set and chooses a fallback active tab when needed. | The local tab model has an Editor preview (`peek`) state but no pinned-item state. Preview is not equivalent to a pinned pane item. |
| Workspace pending-close flow | Dirty Editor tabs and configured running Terminal tabs can request confirmation. Cancelling clears the remaining queued tab closes; tabs closed earlier stay closed. | This tab-level sequential prompt flow is not the reference's per-pane save-intent flow. |

The separate Pane packet covers `Pane::CloseAllItems` and
`Pane::CloseOtherItems`, which act inside a single pane. Do not use those rows
to claim coverage for these Workspace actions.

## Required user-flow and state acceptance

- Invoke both actions with one and several center panes. Verify their item
  selection, active item retention, pinned-item preservation, removal of empty
  panes, and survival of the final center pane.
- For the inactive action, verify that the active item remains only in the
  active center pane. Verify that pinned items in other panes survive even
  when those panes otherwise become empty.
- Exercise clean items, one dirty item, multiple dirty items in one pane, and
  dirty items spread across panes. Record every Save all, Discard all, Cancel,
  Close Without Saving, and retry outcome, including which panes continue
  after a group cancellation and which later items continue after an
  individual cancellation.
- Exercise the default and every supported explicit `save_intent`. For the
  inactive action, confirm the retained active pane follows default `Close`
  behavior even when other panes receive an explicit intent.
- Verify focus and active-item selection as items and panes close, with focus
  initially in the retained item, an item being closed, and a different pane.
  Verify layout persistence and session restore without assuming close history.
- Verify all platform bindings, Pane-context dispatch, user keymap overrides,
  and conflicts. Keep these bulk actions distinct from `CloseWindow`, window
  quit, Pane-local close actions, and Labonair's Space-scoped tab commands.
- Compare paired native macOS captures for multiple panes, pinned items,
  prompt states, empty/final panes, and the post-close focus target. Record
  binary, build, theme, display scale, and window bounds.

## Acceptance evidence still required

For each trace, record build and commit, OS, key/action and active context,
focused pane/item, item pin and dirty state, save intent, prompt choice,
panes before and after, retained items, final active item, layout snapshot, and
whether later pane operations continued after cancellation. Include paired
macOS captures for the visible pane and prompt states. Static key rows and
Labonair's related Space-scoped tab commands do not establish acceptance.
