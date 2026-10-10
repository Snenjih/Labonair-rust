# Pane and Item Action Classification

**Status:** Static semantic and Labonair-route crosswalk for 22 default-macOS
Pane actions; pinned runtime, native interaction, and paired visual evidence
remain pending  
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`  
**Owner:** Workspace for outer tabs/panes; Editor for editor-internal groups;
each active view owner retains its own navigation behavior

## Evidence boundary

This independently worded register describes the visible user outcome behind
each pinned Pane action, then checks the current Labonair contract and source.
The [action register](action-requirements.md) retains each action's exact
default key, context, and arguments. Pinned action catalog metadata is preserved
in the [registry evidence](evidence/zed-actions-3569541-linux-x86_64.json).
Static descriptions and source inspection do not prove that a key dispatches or
that a user sees the expected result.

All 22 records are `Source checked`; none is `Runtime observed`. No pinned
macOS action dispatch or native Labonair interaction was available to inspect
on the current Linux host, which has neither `DISPLAY` nor `WAYLAND_DISPLAY`.
There are no paired native visual artifacts for this slice. Provenance remains
the same-session independent-redevelopment record in the [parity README](README.md#provenance-and-source-boundary).

## Owner and workflow contract

| Contract field | Current Labonair record |
|---|---|
| Outer tabs and panes | Workspace — `labonair-workspace` owns the universal tab strip, Space-scoped tab lifecycle, focus, and outer pane layout. The [Workspace model](../workspace-model.md) and [tab-strip acceptance packet](titlebar-tab-strip.md) remain canonical; the current recursive pane tree is implemented for Terminal workspaces, so cross-capability pane hosting remains an open requirement. |
| Editor groups | Editor — `labonair-editor` owns editor-only split state and commands under [ADR 0004](../adr/0004-editor-internal-splits.md). This is separate from a Workspace pane split. Do not add a second global or per-pane tab strip to imitate a reference layout. |
| Search | Editor owns query and match state; Workspace hosts the current SearchOverlay. `pane::DeploySearch` is cross-referenced to the [Search action classification](action-classification-search.md). |
| Project-file reveal | Explorer owns its tree and active-file following. `explorer.autoRevealActiveFile` is currently false by default; the explicit reveal action is not registered. |
| Canonical user routes | The universal tab strip and its context menu; Command Palette / menu `Find`; existing Workspace pane commands; Editor split commands. Routes remain owner-contributed through existing command providers. |
| Public contracts | `TabStore`, `WorkspaceLayout`, Workspace command descriptors, and Editor `EditorSplitTree`/`EditorCommand` boundaries. No new contract is added by this inventory. |
| Registry coverage | Local Workspace has `CloseTab`, `CloseOtherTabs`, `NextTab`, `PrevTab`, numbered tab selection, `SplitRight`, `SplitDown`, `ClosePane`, and `FocusNextPane`. It does not register one action per pinned Pane ID. The default macOS key/context gaps remain explicit below. |
| State and persistence | Workspace owns tab/outer-layout session state and sequential dirty-close confirmation. Tabs have an Editor preview (`peek`) flag, not a general pinned flag or closed-item history. Editor-owned group state is a separate lifecycle. |
| Notifications and failure | Existing dirty-close decision and session/layout recovery paths remain with their owners. No notifications, settings, or persistence schema are added here. |
| Settings and assets | No new value is assigned to Settings. Explorer's existing auto-reveal value is a related setting, not an implementation of an explicit reveal command. |
| UI-kit and input | Existing tab, menu, split, and focus controls follow the shared UI-kit contract. Native pointer, keyboard, narrow, disabled, and focus states remain pending. |

## Required states, input, and persistence checks

The exact default macOS key, context, and action arguments are recorded on
each ID in the action register. Acceptance must exercise those keys and the
owner's pointer route, including an inactive context where the key must not
run. The following states are required; all remain pending native interaction
evidence:

- **Activation:** empty, one-item, and multi-item Pane/Space; valid and
  out-of-range numbered positions; next/previous at both ends; configured wrap
  on and off; most-recent item when that history exists.
- **Close:** zero targets, clean and dirty Editors, cancellation and recovery
  from the dirty-close decision, pinned and unpinned items, side/bulk scopes,
  and the final-item/empty layout transition. Disabled close-left/right/clean
  menu rows must follow their current target set.
- **Search deployment:** preserve the query and replacement/matching options
  supplied by the action; invalid regex, empty results, loading, filter
  validation, cancellation, and recovery follow the Search packet.
- **Navigation and reopen:** both ends of the history stack, no history, empty
  closed-item history, and restoration of a view with the correct owner state.
- **Reveal:** active file present, absent, ignored by Explorer filters, tree
  collapsed, Explorer closed, and `explorer.autoRevealActiveFile` both off and
  on. Explicit reveal must select the file and focus the intended surface.
- **Split, swap, and pin:** all four directions, single and nested layouts,
  narrow windows, active/inactive focus, spawn/layout failure, first/last
  ordering boundaries, persisted Workspace layout, and pinned items during
  each bulk-close action. Editor groups use their separate Editor lifecycle.

Workspace session persistence owns outer tab state and the implemented
Terminal pane layout. Cross-capability split restoration remains pending. The
existing `TabStore` has no general pinned flag or reopen-closed history;
Editor-internal split persistence is a separate contract. No new Settings
value, migration, or persistence behavior is introduced by this inventory.

## Per-action semantic crosswalk

`Partial` means a related user operation exists but its owner scope, default
route, state, or behavior differs. `Missing` means source inspection found no
corresponding user operation. These labels are static and do not count as
runtime acceptance. The exact pinned keys and arguments are linked per ID in
the action register.

| Requirement | Labonair owner / canonical crate | Pinned user goal, stated independently | Labonair static correspondence and gap |
|---|---|---|---|
| [`ACT-MACOS-pane-ActivateItem`](action-requirements.md) | Workspace (labonair-workspace) | Select the item at a numbered position in the active Pane; the supplied index identifies the position. | **Partial.** Workspace exposes `SelectTab1` through `SelectTab9`, but selects across the global tab list rather than within a Pane; its macOS descriptors use `Cmd+1…9`, while the pinned bindings use a different modifier. Evidence: [`command_provider.rs`](../../crates/workspace/src/command_provider.rs), [`tabs.rs`](../../crates/workspace/src/tabs.rs). |
| [`ACT-MACOS-pane-ActivateLastItem`](action-requirements.md) | Workspace (labonair-workspace) | Return to the Pane's most recently active item. | **Missing.** TabStore records the current active tab and order, but no most-recent-item history or command exists. Evidence: [`tabs.rs`](../../crates/workspace/src/tabs.rs). |
| [`ACT-MACOS-pane-ActivateNextItem`](action-requirements.md) | Workspace (labonair-workspace) | Activate the next item and wrap to the first when the list ends, unless the action argument opts out. | **Partial.** `NextTab` wraps among tabs in the active Space; it does not navigate a Pane-local item list or expose the pinned action's no-wrap option. Evidence: `adjacent_tab_navigation_wraps_inside_the_source_space` in [`tabs.rs`](../../crates/workspace/src/tabs.rs). |
| [`ACT-MACOS-pane-ActivatePreviousItem`](action-requirements.md) | Workspace (labonair-workspace) | Activate the previous item and wrap to the last when the list begins, unless the action argument opts out. | **Partial.** `PrevTab` wraps among tabs in the active Space; Pane scope and the configurable no-wrap behavior are absent. Evidence: [`command_provider.rs`](../../crates/workspace/src/command_provider.rs), [`tabs.rs`](../../crates/workspace/src/tabs.rs). |
| [`ACT-MACOS-pane-CloseActiveItem`](action-requirements.md) | Workspace (labonair-workspace) | Close the active Pane item, preserving pinned items by default and honoring an optional save intent. | **Partial.** `CloseTab` closes the active global tab and confirms dirty Editor tabs. On a split Terminal workspace, `Cmd+W` instead closes the active terminal pane. There is no pinned-tab state or per-Pane item close contract. Evidence: [`command_provider.rs`](../../crates/workspace/src/command_provider.rs), [`workspace.rs`](../../crates/workspace/src/workspace.rs), test `dirty_editor_needs_confirm` in [`tabs.rs`](../../crates/workspace/src/tabs.rs). |
| [`ACT-MACOS-pane-CloseAllItems`](action-requirements.md) | Workspace (labonair-workspace) | Close all closable items in the active Pane while preserving pinned items by default; dirty items use the close decision flow. | **Partial.** The tab context menu has `Close All`, scoped to the clicked tab's Space and sequenced through dirty-editor confirmations. It does not close a Pane-local item set or preserve pinned items; the menu action has no matching Pane command ID. Evidence: [`workspace.rs`](../../crates/workspace/src/workspace.rs), test `close_all_clears_every_tab` in [`tabs.rs`](../../crates/workspace/src/tabs.rs). |
| [`ACT-MACOS-pane-CloseCleanItems`](action-requirements.md) | Workspace (labonair-workspace) | Close only clean items in the active Pane, retaining changed items and pinned items. | **Partial.** `Close Clean` is available in the tab context menu and filters by the dirty flag across a Space; it has no Pane scope or pin state. Evidence: `close_clean_tabs` and the tab menu in [`workspace.rs`](../../crates/workspace/src/workspace.rs). |
| [`ACT-MACOS-pane-CloseItemsToTheLeft`](action-requirements.md) | Workspace (labonair-workspace) | Close preceding Pane items while keeping the current item and pinned items. | **Partial.** `Close Left` operates on the clicked tab's position in its Space and uses sequential close handling. It is not pane-local and does not preserve a pin state. Evidence: `close_tabs_to_side` and the tab menu in [`workspace.rs`](../../crates/workspace/src/workspace.rs). |
| [`ACT-MACOS-pane-CloseItemsToTheRight`](action-requirements.md) | Workspace (labonair-workspace) | Close following Pane items while keeping the current item and pinned items. | **Partial.** `Close Right` is Space-scoped in the tab context menu; pane scope and pin preservation are absent. Evidence: [`workspace.rs`](../../crates/workspace/src/workspace.rs). |
| [`ACT-MACOS-pane-CloseOtherItems`](action-requirements.md) | Workspace (labonair-workspace) | Keep the current Pane item and close other closable items, preserving pinned items by default. | **Partial.** `CloseOtherTabs` and the tab menu close other tabs in the current Space through dirty-close handling; there is no Pane-local item group or pinned-tab state. Evidence: `close_others` in [`workspace.rs`](../../crates/workspace/src/workspace.rs). |
| [`ACT-MACOS-pane-DeploySearch`](action-requirements.md) | Workspace hosts; Editor owns search state (see Search packet) | Open search with supplied query, include/exclude paths, replacement visibility, and match-mode options. | **Partial.** Workspace `Find` opens SearchOverlay, which has File/Project scope and transient match controls. It does not accept the pinned deployment configuration or provide project include/exclude filters and ignored-path control. See [`action-classification-search.md`](action-classification-search.md). |
| [`ACT-MACOS-pane-GoBack`](action-requirements.md) | Active view owner (no shared Workspace history route) | Return to the previous visited item or location in the active view's navigation history. | **Missing as a shared route.** No Workspace active-item history or `GoBack` command is registered. Any future Editor or AI history remains with that owning view; adjacent/previous-tab navigation is not the same operation. |
| [`ACT-MACOS-pane-GoForward`](action-requirements.md) | Active view owner (no shared Workspace history route) | Move forward through the active view's navigation history after returning to an earlier location. | **Missing as a shared route.** No Workspace forward-history stack or `GoForward` command is registered. `NextTab` moves by tab order, not visit history. |
| [`ACT-MACOS-pane-ReopenClosedItem`](action-requirements.md) | Workspace (labonair-workspace) | Restore the most recently closed Pane item with its prior view identity. | **Missing.** The local close lifecycle retires removed tabs and panes; it does not keep a reopenable item stack. Evidence: [`workspace.rs`](../../crates/workspace/src/workspace.rs), [`tabs.rs`](../../crates/workspace/src/tabs.rs). |
| [`ACT-MACOS-pane-RevealInProjectPanel`](action-requirements.md) | Explorer (labonair-panel-explorer) | Select and reveal the active file in the project tree. | **Partial.** Explorer can follow the active Editor file when `explorer.autoRevealActiveFile` is enabled, but the setting defaults off and there is no explicit `RevealInProjectPanel` command or focus/selection route. Evidence: [`panel_explorer.rs`](../../crates/panel-explorer/src/panel_explorer.rs), [`file_manager.rs`](../../crates/settings-content/src/file_manager.rs). |
| [`ACT-MACOS-pane-SplitDown`](action-requirements.md) | Workspace (`labonair-workspace`); Editor (`labonair-editor`) has a separate down-group route | Create a pane below the active pane and place the resulting view in focus. | **Partial.** Workspace can split a Terminal workspace and start another terminal in the same cwd; Editor separately supports a down-oriented Editor group. Neither route is a general cross-capability Pane action with the pinned key/context. Evidence: [`workspace.rs`](../../crates/workspace/src/workspace.rs), [`command_provider.rs`](../../crates/editor/src/command_provider.rs), model test `split_in_all_four_directions` in [`pane_group.rs`](../../crates/workspace/src/pane_group.rs). |
| [`ACT-MACOS-pane-SplitLeft`](action-requirements.md) | Workspace (`labonair-workspace`); no Editor left-group route | Create a pane to the left of the active pane and focus its resulting view. | **Missing as a user action.** `WorkspaceLayout` can represent a left split, but Workspace registers no `SplitLeft` command; Editor groups provide only right/down commands. The model capability alone is not a reachable workflow. Evidence: [`pane_group.rs`](../../crates/workspace/src/pane_group.rs), [`command_provider.rs`](../../crates/workspace/src/command_provider.rs). |
| [`ACT-MACOS-pane-SplitRight`](action-requirements.md) | Workspace (`labonair-workspace`); Editor (`labonair-editor`) has a separate right-group route | Create a pane to the right of the active pane and focus its resulting view. | **Partial.** Workspace exposes a right split only in Terminal context and spawns another terminal; Editor offers a distinct Editor-owned right group. There is no single context-general Pane route or matching default macOS action. Evidence: [`workspace.rs`](../../crates/workspace/src/workspace.rs), [`command_provider.rs`](../../crates/workspace/src/command_provider.rs), [`editor/command_provider.rs`](../../crates/editor/src/command_provider.rs). |
| [`ACT-MACOS-pane-SplitUp`](action-requirements.md) | Workspace (`labonair-workspace`); no Editor up-group route | Create a pane above the active pane and focus its resulting view. | **Missing as a user action.** The Workspace layout model accepts an upward direction, but no user command is registered; Editor groups have no up-oriented command. Evidence: [`pane_group.rs`](../../crates/workspace/src/pane_group.rs), [`command_provider.rs`](../../crates/workspace/src/command_provider.rs). |
| [`ACT-MACOS-pane-SwapItemLeft`](action-requirements.md) | Workspace (labonair-workspace) | Exchange the active item with its immediate left neighbor in the Pane. | **Partial.** TabStore supports reordering by drag inside a Space, but there is no keyboard swap command and no Pane-local item order. Evidence: `reorder_moves_tab` in [`tabs.rs`](../../crates/workspace/src/tabs.rs), tab pointer interaction in [`workspace.rs`](../../crates/workspace/src/workspace.rs). |
| [`ACT-MACOS-pane-SwapItemRight`](action-requirements.md) | Workspace (labonair-workspace) | Exchange the active item with its immediate right neighbor in the Pane. | **Partial.** Pointer reordering can move a tab within its Space; an adjacent keyboard swap action and Pane-local order are absent. Evidence: [`tabs.rs`](../../crates/workspace/src/tabs.rs), [`workspace.rs`](../../crates/workspace/src/workspace.rs). |
| [`ACT-MACOS-pane-TogglePinTab`](action-requirements.md) | Workspace (labonair-workspace) | Toggle a persistent pinned state on the current tab so it can stay anchored and survive bulk-close defaults. | **Missing.** Labonair has a temporary Editor `peek`/“Keep Tab Open” promotion, but no general pin flag, pinned ordering, pin toggle, or bulk-close preservation. Evidence: [`tabs.rs`](../../crates/workspace/src/tabs.rs), [`workspace.rs`](../../crates/workspace/src/workspace.rs). |

## Existing test traceability

| Related test evidence | Requirement IDs with model/contract evidence | Coverage limit |
|---|---|---|
| `adjacent_tab_navigation_wraps_inside_the_source_space` in [`tabs.rs`](../../crates/workspace/src/tabs.rs) | `ACT-MACOS-pane-ActivateNextItem`, `ACT-MACOS-pane-ActivatePreviousItem` | Proves Space-scoped tab navigation and wrap; not Pane-local context or pinned key dispatch. |
| `close_all_clears_every_tab`, `tabs_by_kind_and_close_by_kind`, and `dirty_editor_needs_confirm` in [`tabs.rs`](../../crates/workspace/src/tabs.rs) | `ACT-MACOS-pane-CloseActiveItem`, `ACT-MACOS-pane-CloseAllItems`, `ACT-MACOS-pane-CloseCleanItems`, `ACT-MACOS-pane-CloseItemsToTheLeft`, `ACT-MACOS-pane-CloseItemsToTheRight`, `ACT-MACOS-pane-CloseOtherItems` | Exercises TabStore primitives only; does not exercise Workspace's context menu, sequential dirty-close UI, Pane scope, or pinned preservation. |
| `split_in_all_four_directions` in [`pane_group.rs`](../../crates/workspace/src/pane_group.rs) | `ACT-MACOS-pane-SplitDown`, `ACT-MACOS-pane-SplitLeft`, `ACT-MACOS-pane-SplitRight`, `ACT-MACOS-pane-SplitUp` | Proves pure layout geometry, not user reachability, view creation, owner routing, or focus after split. |
| `reorder_moves_tab` in [`tabs.rs`](../../crates/workspace/src/tabs.rs) | `ACT-MACOS-pane-SwapItemLeft`, `ACT-MACOS-pane-SwapItemRight` | Proves generic tab reorder, not keyboard adjacent-swap behavior or Pane scope. |
| `workspace_provider_owns_project_lifecycle_commands` in [`command_provider.rs`](../../crates/workspace/src/command_provider.rs) | `ACT-MACOS-pane-SplitRight` | Checks that one Workspace split descriptor is present, not its dispatch, direction-specific behavior, or resulting focus/content. |

The reviewed code has no action-specific regression test or pinned-key
dispatch test for any of the 22 IDs; the tests above exercise related model or
descriptor behavior only. Action workflows for last-item history, close-menu
scope and pin preservation, search deployment arguments, back/forward history,
reopening closed items, explicit Explorer reveal, keyboard item swaps, and pin
state have no corresponding operation test. There are no native interaction or
paired visual artifacts for any action in this family.

## Next work

Continue with the remaining default-macOS action namespaces and the registered
but unbound catalog. Keep outer Workspace panes, Editor-owned groups, and the
accepted universal tab strip as distinct Labonair owners and routes. Do not
infer runtime action behavior from its name, a model test, or a layout method.
