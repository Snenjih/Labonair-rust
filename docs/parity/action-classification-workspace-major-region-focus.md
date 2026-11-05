# Workspace Major-Region Focus Action Classification

**Status:** Static interaction and ownership crosswalk for two actions; native
keyboard dispatch, focus-ring behavior, accessibility, and paired visual
acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns movement between major window
regions. Shell titlebar/statusbar and panel owners provide their focusable
surfaces and content handles.

## Evidence boundary

This packet specifies keyboard movement between major visible regions, not
cycling among panes inside one region. It records a stable cyclic order,
closed-dock filtering, wrapping, toolbar entry, and focus targets for sighted
keyboard and assistive-technology use. Static inspection does not prove native
F6 dispatch, actual focus rings, or screen-reader announcements.

The requirements are independently stated for Labonair's Workspace model.
This is same-session independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux host
without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) owns cross-region order and chooses region focus targets. The shell supplies the titlebar and statusbar; dock and editor owners supply interactive content handles. |
| Canonical user entry point | Pinned Workspace-context `FocusNextPart`: macOS `F6` or `Cmd+F6`; Linux/Windows `F6` or `Ctrl+F6`. `FocusPreviousPart`: `Shift+F6` on all three platforms. Labonair currently has split-pane focus cycling and local statusbar arrow traversal, but no matching major-region action. |
| Visible region order | Titlebar, open left dock, center pane, open right dock, open bottom dock, statusbar. Closed docks are omitted; the remaining regions keep this relative order. The sequence wraps forward and backward. |
| Focus targets | Titlebar and statusbar toolbars land on their first focusable control. Open docks target the active panel's activation focus handle, or the dock handle if no panel is active. The center targets the last active center pane, falling back to the first pane. When assistive technology is active, dock/center navigation targets the named region landmark; otherwise it targets usable content. |
| Unknown current focus | If current focus belongs to none of the listed regions, either action lands on the first region. The forward/backward distinction begins after focus is within a listed region. |
| State and persistence | Focus movement must not open docks, change the active panel or pane, select a tab, resize layout, or persist layout. Closing a dock removes it from subsequent traversal. |
| Events and registry | Workspace owns both actions and region ordering. Existing surface focus handles are consumed through Workspace composition; no new event, registry, setting, or dependency is required for this inventory slice. |
| UI-kit and accessibility | Keep visible keyboard focus and enter toolbar groups at an operable control. Landmark roles/labels and screen-reader announcements require native verification. The current GPUI role/name API limitation is recorded in the [design system](../design-system.md); no accessibility parity claim follows from static focus handles. |
| Failure behavior | The center pane remains available even when the titlebar is absent and all docks are closed. A toolbar with no focusable control retains focus on its container rather than escaping to an unrelated region. |

## Action requirements

| Requirement ID | Platform binding | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-FocusNextPart`](action-requirements.md) | macOS `F6`, `Cmd+F6`; Linux/Windows `F6`, `Ctrl+F6` | Focus the next visible major region, wrapping at the end. | No Labonair cross-region action or F6 binding. `FocusNextPane` cycles split leaves only. |
| [`ACT-MACOS-workspace-FocusPreviousPart`](action-requirements.md) | `Shift+F6` on macOS, Linux, and Windows | Focus the previous visible major region, wrapping at the beginning. | No Labonair cross-region action or Shift+F6 binding. The statusbar maps arrow keys through GPUI focus traversal; this is not a Workspace region action. |

## Required user-flow and state acceptance

- Start from each visible region and move forward and backward. Verify the
  order, reverse order, wrap at both ends, and that the active tab, center
  pane, selected dock panel, open state, and layout remain unchanged.
- With all docks closed, verify traversal cycles among the available titlebar,
  center pane, and statusbar regions. Open only one edge at a time, then side plus
  bottom docks, and verify closed edges are skipped without changing the order
  of remaining regions.
- Invoke each action from an unrelated focused control and verify both land on
  the first region as specified. Check behavior when the titlebar is absent,
  the center has multiple panes, a dock has no active panel, and a toolbar has
  no focusable child.
- Verify the dock target is the active panel's usable control for sighted
  keyboard use and that editor/dock landmark focus is announced correctly
  with a screen reader. Verify toolbar focus begins on its first control.
- Exercise F6 with the platform's function-key configuration, default and
  user-overridden keymaps, input in text fields, modal/popup focus, and each
  platform's registered Workspace context. The pinned `debugger::Pause` action
  also uses F6 under `Workspace && debugger_session`; verify context precedence
  so only the intended action runs. Check the modified F6 variants and
  Shift+F6 as well.
- Compare paired native macOS captures with the focus indicator visible in
  titlebar, center, each open dock, and statusbar. Record screen-reader state,
  theme, scale, window bounds, open docks, and focused control.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`Workspace::focus_next_pane`](../../crates/workspace/src/workspace.rs) and [`FocusNextPane` registration](../../crates/workspace/src/command_provider.rs) | Cycles the active workspace tab's split leaves and no-ops for a single leaf or non-workspace tab. | Pane-local operation; it does not visit titlebar, docks, center as a whole, or statusbar. |
| [`Titlebar` focus handle](../../crates/shell/src/titlebar.rs) | Provides a focusable shell titlebar surface containing tab and window controls. | No shared Workspace region list or F6 action currently consumes it. |
| [`StatusBar` focus handle and key handler](../../crates/workspace/src/status_bar.rs) | Provides a focusable statusbar surface; Left/Right invoke GPUI focus traversal and Escape blurs. | This key handler does not define the pinned previous/next major-region action or order. |
| [`DockPanelButtons`](../../crates/workspace/src/dock_status_item.rs) | Panel statusbar controls are keyboard-focusable and expose tooltips. | Controls do not contribute an explicit cross-region order or focus action. |

Static source contains no `FocusNextPart`/`FocusPreviousPart` Workspace action
or matching Labonair F6 binding. Existing pane and statusbar navigation are
related but distinct. The application may still participate in GPUI's generic
tab focus order; that does not establish the pinned stable cyclic region
behavior.

## Acceptance evidence still required

For each trace, record build and commit, OS, exact key, initial focus region,
visible region sequence, open docks, assistive-technology state, resulting
focused control or landmark, and whether any workspace state changed. Include
paired native macOS screenshots and screen-reader notes. A working
`FocusNextPane`, statusbar arrow key, generic Tab traversal, or visible focus
ring alone does not prove major-region action acceptance.
