# Workspace Dock Sizing Action Classification

**Status:** Static intent, Workspace ownership, Settings, and route crosswalk
for six default-macOS actions; key dispatch, native interaction, and paired
visual acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns dock focus, dimensions,
bounds, resize/reset behavior, and layout persistence. Registered panel owners
provide panel defaults and minimum sizes.

## Evidence boundary

This packet describes six related keyboard actions for resizing the focused
dock, resizing every open dock, and resetting one or all open docks. The
default increment follows UI font size; reset behavior also depends on the
JSON-only `workspace.resize_all_panels_in_dock` setting. Static source checks
identify handlers, parameters, Settings dependencies, and current Labonair
pointer routes. They do not prove key dispatch, rendered dimensions, or
persistence after restart.

The requirements are independently stated for Labonair's Workspace and dock
model. This is same-session independent redevelopment, not separated-room
clean-room work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux
host without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace — `labonair-workspace` owns edge dock geometry, focus, size constraints, reset behavior, and persistence. Panel owners provide each panel's default and minimum sizes. |
| Composition | Workspace consumes typed resize/reset actions and mutates only eligible dock dimensions. Shell contributes no duplicate size table or resize logic. |
| Canonical user entry points | Pinned actions use the Workspace key context. macOS and Linux: active decrement `Ctrl+Alt+-`, active increment `Ctrl+Alt+=`, active reset `Ctrl+Alt+0`, all-open decrement `Ctrl+Alt+_`, all-open increment `Ctrl+Alt++`, and all-open reset `Ctrl+Alt+)`. Windows: active decrement/increment `Shift+Alt+-` / `Shift+Alt+=`, active reset `Alt+R`, all-open reset `Shift+Alt+0`; there are no default Windows all-open increment/decrement bindings. Labonair uses left/right/bottom dock resize handles: drag to resize and double-click a handle to reset that one dock. |
| Public typed contract | The six Workspace actions carry a `px` delta where applicable. A zero value resolves to `ui_font_size`; nonzero values are used as explicit pixel steps. The Workspace action handlers choose the focused dock or open docks, apply the signed delta or registered panel defaults, and clamp through normal dock sizing. |
| Focus and target | Active-dock increase/decrease/reset require focus within an open dock; without one they are no-ops. Open-dock increase/decrease affect every open dock with an active panel. Open-dock reset affects docks with a visible panel. The center split tree is not resized by these actions. |
| Delta and bounds | Increase adds the step; decrease subtracts it. A zero `px` step is the UI font size (pinned packaged default 16); an explicit nonzero `px` selects its own step. Normal Workspace and panel min/max limits still apply. Actions must not change dock visibility, active panel, tab selection, or center-pane layout. |
| Reset behavior | Reset-active restores the focused dock panel's registered default size. Reset-open restores visible dock panel sizes. `workspace.resize_all_panels_in_dock` determines whether resize/reset acts on the active panel only or on all panels in that dock that share the same sizing mode. Its pinned default is `["left"]`. |
| Registry and events | Workspace registers and handles the six actions directly. No event, provider registry, or new dependency is required. |
| Settings and persistence | `SET-appearance-ui-font-size` (`ui_font_size`, 16, USER) provides the zero-argument step. Added [`SET-settings-json-only-workspace-resize-all-panels-in-dock`](settings-requirements.md): `workspace.resize_all_panels_in_dock` defaults to `["left"]`, supports combinations of `left`, `right`, and `bottom`, has no Settings control, and has scope pending. Pinned dock panel sizes persist in Workspace layout state. Labonair has no resize-all setting; its dock stores one size per edge, so every panel in that dock shares the same dimension. |
| Notifications and failure | Missing active/open docks are no-ops. A size that would cross panel or Workspace bounds is clamped. Do not disturb persisted open state or other docks on a no-op. No notification is expected. |
| UI-kit | Reuse existing dock handles, panel surfaces, and shared Workspace layout. Add no local resize shortcuts or dock-specific controls. |

Labonair's `Dock` stores one size per edge, not a size for each registered
panel. Its current default sizes are left 260 px, right 380 px, and bottom
320 px; drag updates the edge size within panel and dock bounds, and double
click resets that edge. These values are static source records and need native
visual confirmation.

## Action requirements

| Requirement ID | Platform binding | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-DecreaseActiveDockSize`](action-requirements.md) | macOS/Linux `Ctrl+Alt+-`; Windows `Shift+Alt+-`; `px=0` | Reduce the focused open dock by UI font size or an explicit `px` argument. | No keyboard action. Drag resizes one pointer-selected dock; double-click restores its default. |
| [`ACT-MACOS-workspace-IncreaseActiveDockSize`](action-requirements.md) | macOS/Linux `Ctrl+Alt+=`; Windows `Shift+Alt+=`; `px=0` | Increase the focused open dock by UI font size or an explicit `px` argument. | No keyboard action. Drag resizes one pointer-selected dock; double-click restores its default. |
| [`ACT-MACOS-workspace-ResetActiveDockSize`](action-requirements.md) | macOS/Linux `Ctrl+Alt+0`; Windows `Alt+R` | Restore the focused dock panel's default size, using the resize-all setting when configured. | No keyboard action. Double-click restores one dock; current layout is per edge rather than per panel. |
| [`ACT-MACOS-workspace-DecreaseOpenDocksSize`](action-requirements.md) | macOS/Linux `Ctrl+Alt+_`; no Windows default; `px=0` | Reduce every open dock with an active panel by the step. | No keyboard action; local pointer drag targets one dock at a time. |
| [`ACT-MACOS-workspace-IncreaseOpenDocksSize`](action-requirements.md) | macOS/Linux `Ctrl+Alt++`; no Windows default; `px=0` | Increase every open dock with an active panel by the step. | No keyboard action; local pointer drag targets one dock at a time. |
| [`ACT-MACOS-workspace-ResetOpenDocksSize`](action-requirements.md) | macOS/Linux `Ctrl+Alt+)`; Windows `Shift+Alt+0` | Restore visible dock panel sizes, applying the resize-all setting within each dock. | No action resets all open local docks together. |

## Required user-flow and state acceptance

- With a left, right, or bottom dock focused, invoke active-dock increase and
  decrease. Verify one default `ui_font_size` step, explicit `px` values,
  direction, focus preservation, and unchanged dock visibility and panel state.
- Invoke active-dock actions with focus in the center workspace, titlebar,
  statusbar, and with all docks closed. Verify they do nothing.
- Open multiple docks and invoke open-dock increase/decrease from inside and
  outside a dock. Verify each open dock with a visible active panel changes by
  the same logical step, closed docks do not change, and orientation uses
  width for side docks and height for the bottom dock.
- Reset the focused dock and then all open docks. Verify registered panel
  default sizes, active panel preservation, and that closed dock sizes remain
  unchanged. Test `resize_all_panels_in_dock` with no edges, each edge, and
  multiple edges; include panels that use fixed and flexible sizing modes.
- Test custom and default UI font sizes. Verify `px=0` follows the current
  font-size setting and a nonzero argument is honored independently.
- Exercise minimum and maximum bounds, narrow windows, zoomed docks, empty
  docks, simultaneous side and bottom docks, and actions repeated to the
  bounds. The center split sizes and dock open/closed state must remain intact.
- Verify every platform shortcut, the absent Windows bindings for adjusting
  all open docks, user keymap overrides, and keyboard discoverability.
- Save and reload Workspace layout after resizing and resetting. Verify size,
  active panel, dock visibility, and behavior when a panel's default/minimum
  size changes.
- Compare native macOS captures for initial, increased, decreased, reset,
  clamped, side-dock, and bottom-dock states. Include font size and resize-all
  setting values with display scale and window bounds.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`set_dock_size`, `reset_dock_size`, and dock resize rendering in `workspace.rs`](../../crates/workspace/src/workspace.rs) | Pointer resizing of one edge dock and double-click reset | Changes are clamped and sent to workspace layout persistence; there are no matching keyboard command IDs or action handlers. |
| [`Dock` defaults and `DockData`](../../crates/workspace/src/dock.rs) | One stored dimension for each left/right/bottom dock | All panels on the same edge share the dock size; per-panel flexible sizing and the pinned resize-all policy are not represented locally. |
| [`WorkspaceContent`](../../crates/settings-content/src/workspace.rs) | Workspace Settings values | No matching `resizeAllPanelsInDock` value exists. The UI font setting does not drive local dock resize because there are no local keyboard size actions. |
| [`workspace.resize_all_panels_in_dock` requirement](settings-requirements.md) | Pinned default and panel-grouping policy | Newly inventoried from the pinned Settings schema; project/user scope and Settings UI rendering remain pending. |

No Labonair test or runtime trace exercises the six pinned actions. Existing
pointer resize code establishes only the local handle route and per-edge
layout persistence, not action dispatch, keyboard accessibility, all-open
operations, or paired visual behavior.

## Acceptance evidence still required

For each action trace, record build and commit, OS, exact key/action arguments,
focused area, open docks and panels, UI font size, resize-all setting, starting
and resulting dimensions, bounds, active panel, visibility, and persisted
layout. Include matched macOS captures with scale, theme, and window bounds.
Static keymap records and a working drag handle do not establish keyboard
action acceptance.
