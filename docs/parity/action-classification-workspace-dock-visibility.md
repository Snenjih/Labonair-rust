# Workspace Dock Visibility Action Classification

**Status:** Static intent, Workspace ownership, key routes, and current local
entry points; runtime dispatch, focus transitions, and paired visual acceptance
remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns edge-dock open state, active
panel focus, remembered open positions, and layout persistence. Panel owners
contribute the panels and their edge placement.

## Evidence boundary

This packet states the expected behavior for closing the focused dock,
toggling each dock edge, and hiding/restoring all open docks. It distinguishes
edge visibility from selecting or closing an individual panel. Static source
inspection maps Labonair's current statusbar panel buttons and primary-sidebar
command; it does not establish keyboard dispatch or rendered behavior.

The requirements are independently stated for Labonair's Workspace model.
This is same-session independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux host
without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) owns open/closed state for left, right, and bottom edges, dock focus, remembered visibility, and persisted layout. Panel owners supply panel identity, enabled state, and edge placement. |
| Canonical user entry points | Pinned actions use the Workspace key context. macOS: close active dock `Cmd+W`; toggle all `Alt+Cmd+Y`; toggle left `Cmd+B`, right `Cmd+Alt+B` or `Cmd+R`, bottom `Cmd+J`. Linux: close `Ctrl+F4` or `Ctrl+W`; toggle all `Ctrl+Alt+Y`; left `Ctrl+B`, right `Ctrl+Alt+B`, bottom `Ctrl+J`. Windows: close `Ctrl+F4` or `Ctrl+W`; toggle all `Ctrl+Shift+Y`; left `Ctrl+B`, right `Ctrl+Alt+B`, bottom `Ctrl+J`. Labonair has per-panel statusbar buttons and a primary-dock `Toggle Sidebar` command/menu item. |
| Public typed contract | `CloseActiveDock` closes the open dock containing keyboard focus and propagates if none owns focus. Per-edge actions toggle only that edge. `ToggleAllDocks` closes the current open-edge set and restores the last remembered set when all edges are closed and a remembered set exists. No new event or registry is needed for this inventory slice. |
| Target and focus | Closing one dock must leave other docks open. Opening an edge activates an enabled panel if none is active and transfers focus to the dock. Closing an edge returns focus to the center only when focus was inside that dock. Closing all docks returns focus to the Workspace. Validate exact native focus behavior. |
| Remembered state | Toggling all while any edge is open records the open-edge set, then closes all open edges. Toggling all while every edge is closed restores that set if it is nonempty; otherwise it is a no-op. The remembered set is in-memory state; persistence across app reload is not established. |
| Settings and persistence | No new setting is introduced. Existing panel-edge settings determine where panels live. Pinned open/closed state is Workspace-owned. Labonair persists each dock's open state and active panel in Workspace layout data; its primary-dock `Toggle Sidebar` changes that one configured edge. |
| Notifications and failure | Closing with no focused dock propagates to the next eligible action handler; it is not an error. Toggling an empty or unavailable edge must not change unrelated dock state. Toggling all with no open or remembered edges is a no-op. No notification is expected. |
| Events and registry | Workspace registers and handles the five actions. No additional panel registry, event, setting, command, or dependency is required. `CloseAllDocks` is a separate unbound action and is outside this five-action packet. |
| UI-kit | Reuse existing dock and registered panel statusbar controls and shared menus. Add no local per-edge toolbar or close-button style. |

## Action requirements

| Requirement ID | Platform binding | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-CloseActiveDock`](action-requirements.md) | macOS `Cmd+W`; Linux/Windows `Ctrl+F4` and `Ctrl+W` | Close the open dock that currently owns focus; propagate when focus is elsewhere. | No matching Workspace action. The local `Cmd+W` command is Close Current Tab; clicking the active panel statusbar button can close its dock. |
| [`ACT-MACOS-workspace-ToggleAllDocks`](action-requirements.md) | macOS `Alt+Cmd+Y`; Linux `Ctrl+Alt+Y`; Windows `Ctrl+Shift+Y` | Close all open docks, then restore the last remembered open-edge set on the next invocation. | No all-docks keyboard action. `Toggle Sidebar` changes only the primary dock. |
| [`ACT-MACOS-workspace-ToggleLeftDock`](action-requirements.md) | macOS `Cmd+B`; Linux/Windows `Ctrl+B` | Toggle only the left dock, activating and focusing an eligible panel when opened. | No fixed-left action. Explorer's `Toggle Sidebar` command targets the configurable primary dock; statusbar buttons target individual panels. |
| [`ACT-MACOS-workspace-ToggleRightDock`](action-requirements.md) | macOS `Cmd+Alt+B` or `Cmd+R`; Linux/Windows `Ctrl+Alt+B` | Toggle only the right dock. | No fixed-right keyboard action; statusbar buttons select or toggle the panels on that edge. |
| [`ACT-MACOS-workspace-ToggleBottomDock`](action-requirements.md) | macOS `Cmd+J`; Linux/Windows `Ctrl+J` | Toggle only the bottom dock. | No fixed-bottom keyboard action; statusbar buttons select or toggle the panels on that edge. |

## Required user-flow and state acceptance

- With several docks open, focus a panel in each edge and invoke
  `CloseActiveDock`. Verify that only the focused edge closes, other edges and
  panel selections remain intact, and focus lands according to the pinned
  behavior. With focus in the center or another non-dock surface, verify the
  action propagates and does not close a dock.
- Toggle left, right, and bottom independently from open and closed states.
  Verify only the selected edge changes; an open edge retains its active panel
  and size; an edge with no active panel opens an enabled panel; and closing
  an edge does not discard its panel selection or size.
- Toggle all with one, two, and three edges open. Verify every open edge closes
  and only the just-closed edge set reopens on the next toggle. Close edges
  individually before invoking Toggle All and verify remembered-set updates.
  With no open and no remembered edges, verify the action is a no-op.
- Verify remembered positions across workspace recreation and app restart.
  Do not assume that the in-memory restore set is persisted just because
  current open states are saved in layout data.
- Exercise empty docks, disabled panels, panels moved to another edge, dock
  zoom, center focus, panel focus, simultaneous side and bottom docks, and
  narrow windows. Verify no unrelated panel, tab, split, or dock size changes.
- Verify all platform bindings, action contexts, keymap overrides, conflicts,
  and focus-dependent `Cmd+W` behavior. In Labonair, verify how Close Current
  Tab and panel statusbar toggles behave when focus is inside each dock.
- Use pointer controls and keyboard actions with the same open-state matrix.
  Compare paired native macOS captures for each edge, all-docks close/restore,
  focused panel, zoom, and narrow layout, recording build, theme, scale, and
  window bounds.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`Workspace::select_panel`, `open_panel`, and `persist_docks`](../../crates/workspace/src/workspace.rs) | Statusbar panel selection opens and activates a panel; selecting the already-active panel toggles that panel's dock closed. Workspace layout persists dock data. | This is panel-driven behavior, not fixed-edge actions or remembered all-dock restoration. |
| [`DockPanelButtons`](../../crates/workspace/src/dock_status_item.rs) | Registers left/right/bottom groups of shared icon-toggle controls in the statusbar. Clicking a button calls `select_panel`; right-click offers panel movement and button visibility. | The control selects a panel within its registered edge; it is not a dock-wide keyboard route. |
| [`ToggleSidebar` command and View menu item](../../crates/workspace/src/command_provider.rs) | Toggles the current primary dock; Explorer's descriptor provides a `Cmd+B` hint. | Does not independently address left, right, bottom, or all docks. |
| [`CloseTab` command descriptor](../../crates/workspace/src/command_provider.rs) | Local `Cmd+W` is assigned to closing the current tab. | It is not `CloseActiveDock`; behavior with focus in a dock requires native key dispatch verification. |
| [`DockData` and Workspace layout snapshot](../../crates/workspace/src/dock.rs) | Local layout persists per-edge open and active-panel state. | Does not establish the pinned transient last-open-edge restoration workflow. |

No Labonair test or runtime trace covers the five pinned actions. Existing
panel selection, primary-sidebar toggle, and persisted open state are related
routes, not proof of fixed-edge, active-dock, or all-docks keyboard behavior.

## Acceptance evidence still required

For each trace, record build and commit, OS, exact key/action, focused area,
open edges and active panels, zoom state, starting and resulting visibility,
focus target, remembered restoration set, and saved layout. Include paired
macOS captures with display scale, theme, and window bounds. The `Cmd+W`
binding, edge statusbar buttons, or a successful Toggle Sidebar action do not
establish acceptance for the distinct pinned dock actions.
