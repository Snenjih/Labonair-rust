# Workspace Zoom Action Classification

**Status:** Static pane and panel zoom requirements; global focus dispatch,
Labonair action route, and paired macOS visual acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the global action, zoom
target selection, Workspace overlay state, and persisted dock layout. Pane and
panel owners expose their supported zoom transition through typed Workspace
contracts.

## Evidence boundary

This packet independently states the user-visible zoom behavior and current
Labonair gap. Pinned source review covers keymap entries, pane/panel controls,
layout state, and the related setting; it does not prove native key dispatch,
focus precedence, or rendered geometry. This is same-session independent
redevelopment, not separated-room clean-room work, as recorded in the
[parity README](README.md#provenance-and-source-boundary). The Linux host has
no native macOS runtime or display capture.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Workspace (`labonair-workspace`) owns the Pane tree, one active overlay target across center panes and docks, layout overlay, focus routing, and persisted dock zoom. Feature Panel owners report whether their current content can zoom and handle their content-specific transition through the shared `labonair-panel` contract. Keep native window zoom and Editor font zoom separate. |
| Keyboard entry | Pinned default binding is global `Shift+Escape` on macOS, Linux, and Windows. In the more-specific `CommitEditor > Editor` context, the same key invokes [`ACT-MACOS-git-ExpandCommitEditor`](action-requirements.md); it does not invoke Workspace ToggleZoom. Other more-specific modal and editor bindings, keymap profiles, and user overrides must be checked at runtime. |
| Mouse entry | A Pane header exposes a maximize/restore button. Supported dock panels, including Terminal, Agent, and Debugger panels, expose their own zoom controls. The button reflects current zoom state, uses an action tooltip, and operates on its owning Pane or Panel. Verify which panel types expose it. |
| Target and toggle | When dispatch reaches a zoom-capable focused Pane or Panel, ToggleZoom enters or leaves that target's zoomed state. Empty center panes and targets that refuse zoom are no-ops or defer to their eligible containing target. The focused target chosen by the global binding remains a runtime acceptance item. |
| Center-pane zoom | Zooming a center Pane removes it from its normal split-tree position and renders it over the Workspace content. Other panes and docks remain behind the overlay. Repeating ToggleZoom returns to the split layout. This differs from maximizing the active center Pane within the pane tree. |
| Dock-panel zoom | Zooming a dock Panel raises it over the Workspace and moves focus into the panel when needed. The next eligible zoom target replaces the single active overlay; zooming it out restores the Workspace beneath it. Verify close, removal, switching, and focus return while zoomed. |
| Padding setting | The Window & Layout setting [`SET-window-layout-zoomed-padding`](settings-requirements.md) defaults to true. It controls the small inset around the raised zoom view; false removes the inset. Verify its current JSON path, Settings control, live effect, and reload behavior. |
| Persistence | Dock zoom is saved with dock visibility and active-panel state and is restored when the dock returns. Center-Pane zoom is not part of serialized pane tab/split state and should return to the normal layout after restart. Verify both outcomes, including an unavailable restored panel. |
| Related actions | [`ACT-REGISTRY-LINUX-workspace-ToggleEditorZoom`](registered-actions-unbound.md), [`ACT-REGISTRY-LINUX-workspace-ZoomIn`](registered-actions-unbound.md), and [`ACT-REGISTRY-LINUX-workspace-ZoomOut`](registered-actions-unbound.md) are separate unbound actions. ToggleEditorZoom maximizes/restores the active center Pane within the split tree. ZoomIn and ZoomOut act in an active eligible Pane context; Pane-hosted panels may forward those events through their PanelEvent contract, but not every Panel handles these action IDs. None has a default key binding. Native Window Zoom (`ACT-REGISTRY-LINUX-zed-Zoom`) and image/text font-size zoom are separate workflows. |
| Events and registry contributions | Register the global action once in Workspace. Route keyboard and mouse requests through the same typed zoom transition, while each owner supplies its local affordance and capability state. Do not create a shell-wide target table or a second dock zoom state. |
| Failure and recovery | Zoom has no expected I/O. If focus changes or the Pane/Panel is removed during a transition, clear only the stale zoom target, preserve the surviving layout, and focus the documented neighboring view. An unsupported target must not leave the Workspace overlaid by an empty surface. |
| UI-kit | Reuse shared icon buttons, tooltips, toggled state, focus, and panel header components. Keep the maximize/restore appearance and accessible action label consistent across Pane and Panel owners. |

## Action requirement and current route

| Requirement ID | Pinned binding and context | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-ToggleZoom`](action-requirements.md) | `Shift+Escape`, global, macOS/Linux/Windows. Git CommitEditor context shadows it with [`ACT-MACOS-git-ExpandCommitEditor`](action-requirements.md). | Toggle the eligible focused Workspace Pane or Panel between normal and zoomed layout. Mouse controls invoke the owning Pane/Panel zoom transition. | No general Workspace ToggleZoom action or default binding. Existing dock zoom state is reachable only through a Debug command; the native Window Zoom action is separate. No center-Pane overlay route is implemented. |
| [`ACT-REGISTRY-LINUX-workspace-ToggleEditorZoom`](registered-actions-unbound.md) | Registered, no arguments, no default key binding. | Maximize or restore the active center Pane within the split tree, hiding sibling center Panes while leaving docks and Panels in place. If an overlay zoom target exists, clear it before maximizing. | No matching center-Pane maximize action or split-pane route. Universal tabs and the Debug-only dock zoom command are distinct. |
| [`ACT-REGISTRY-LINUX-workspace-ZoomIn`](registered-actions-unbound.md) | Registered, no arguments, no default key binding. | In the active Pane context, enter its zoom state only when zoom is supported and the Pane contains items; focus it first when needed. A Pane hosted inside a panel may forward its zoom event to that Panel; eligibility of other panel types is owner-specific. | Internal dock zoom events and a Debug-only dock command exist, but no matching typed Pane action route or center-Pane overlay is implemented. |
| [`ACT-REGISTRY-LINUX-workspace-ZoomOut`](registered-actions-unbound.md) | Registered, no arguments, no default key binding. | In the active Pane context, leave that Pane's zoom state when zoom is supported and it is zoomed; otherwise propagate or do nothing. A Pane hosted inside a panel may forward its zoom event to that Panel. | No matching typed Pane zoom-out route or center-Pane overlay is implemented; the Debug-only dock command remains separate. |

The three unbound actions have registered Linux catalog entries and distinct
requirements above; no default or alternate-profile binding was found.
Command-palette and user-keymap discoverability, native target dispatch, and
macOS runtime registration remain unverified. The existing [zoom padding
setting row](settings-requirements.md) applies to overlay zoom, not the
center-Pane maximize action. `ToggleZoom` selects a zoom-capable focused Pane
or Panel under one Workspace overlay target; `ZoomIn`/`ZoomOut` act in an
active Pane context, and Pane-hosted panels may forward those events through
their own PanelEvent contract. Do not assume every Panel handles them.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [Workspace dock state](../../crates/workspace/src/dock.rs) and [dock rendering](../../crates/workspace/src/workspace.rs) | Store a zoom flag per dock and let the zoomed dock use the available dock region. | No Workspace-wide overlay target, center-Pane zoom state, global ToggleZoom action, or action binding. Current render behavior is not the pinned overlay workflow. |
| [Debug dock zoom command](../../crates/workspace/src/command_provider.rs) | Toggles the primary dock's local zoom flag and persists dock layout. | Debug-only command is not the pinned global action and does not select a focused pane or panel. |
| Native [Window Zoom menu route](../../crates/shell/src/menu.rs) | Zooms the native application window. | OS window management does not zoom a Pane or dock Panel within Workspace. |
| [Workspace dock settings record](settings-requirements.md) | Records the pinned `zoomed_padding` value with a true default. | Labonair has no matching user setting or overlay geometry to consume it. |

The dock flag, debug route, and native Window Zoom do not establish parity for
`ACT-MACOS-workspace-ToggleZoom`.

## Required user-flow and state acceptance

- Invoke Shift+Escape with focus in a center Editor Pane and in every supported
  dock Panel. Record the exact target, overlay, focus change, and result when
  no eligible target exists.
- Zoom a center Pane in a single-pane workspace and in a multi-pane split.
  Verify the overlay, sibling panes behind it, visible docks, status surfaces,
  repeat-to-restore behavior, active-pane identity, and layout after restore.
- Zoom left, right, and bottom dock panels. Switch focus and panels while
  zoomed, close or remove the active panel, close its dock, and reopen it.
  Verify one active overlay, panel focus, recovery from a removed target, and
  restored dock state after restarting.
- Click every Pane and Panel maximize/restore control. Confirm the same
  transition as the keyboard action, accurate icon/toggled state, tooltip,
  accessible label, and focus behavior.
- In `CommitEditor > Editor`, press Shift+Escape and verify that
  ExpandCommitEditor wins. Check other modal/editor contexts, alternate
  keymap profiles, user overrides, and conflicts on all supported platforms.
- Toggle Zoomed Padding through Settings. Check default, saved value, live
  inset, zero-inset behavior, restart, and narrow-window geometry.
- Compare paired native macOS captures for center-pane overlays, all dock
  edges, normal/zoomed controls, occlusion, padding, focus, and restoration at
  documented window sizes, themes, and scale.

## Acceptance evidence still required

Static source inspection establishes the required targets and current route
gaps only. For each trace, record build and commit, platform, keymap and
context, focused view, zoomable/empty state, active Pane and Panel, overlay
target, focus before/after, setting value, persistence and restart results,
and recovery after closing a target. Global key dispatch and paired native
captures remain pending; no parity claim is made from the available dock
state or Debug command.
