# Titlebar Tab Strip Acceptance Packet

**Status:** Implementation packet for R07-000
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`
**Reference artifact:** User-provided editor tab-bar image; its display conditions are not recorded.

This packet describes observable requirements for the Labonair implementation.
It is written independently of the reference implementation. The associated
source-level comparison remains in
[`../reports/zed-titlebar-tabbar-analysis-2026-10-10.md`](../reports/zed-titlebar-tabbar-analysis-2026-10-10.md).

## Ownership and entry points

```text
Request type: Clean-room UI/layout parity slice
Owner module: Workspace tab state and behavior; Shell titlebar placement; UI kit reusable tab/button/menu chrome
Canonical capability crate: labonair-workspace
UI crate: labonair-ui-kit
Canonical user entry point: Native window titlebar tab strip
Public typed contract: Existing Workspace, TabStore, TabKind, TabData, ActiveTabChanged, TitlebarEvent
State owner: Workspace and TabStore; SpaceStore selects which tabs are visible
Persistence owner: Existing Workspace/Space session persistence; no new persisted values
Commands: Existing Previous Tab, Next Tab, New Editor/Terminal/Preview Tab, Close Tab, Close Other Tabs
Events and registries: No new event or registry contribution
Settings: Existing theme, UI font/density, zen-mode header visibility, and tabs-location values
Notifications: None; close decisions remain local to the active operation
UI-kit components: Palette, horizontal/vertical tab item, icon button, shared context menu and menu items
Allowed dependency changes: None
Security/performance: No new boundary or I/O
Compatibility removal: N/A
```

## Required appearance

- The titlebar tab row uses a flat, contiguous visual treatment. Tabs size to
  their contents, cap long titles, and scroll horizontally without shrinking
  neighboring tabs.
- The row has a fixed 32-pixel base height, follows the existing UI font and
  density scales, and uses the existing toolbar, background, foreground,
  muted-foreground, border, and accent theme roles. Views add no literal color
  tokens.
- Inactive labels use muted foreground on the toolbar surface. The active label
  uses foreground on the application background surface. Thin vertical strokes
  separate tabs; the active tab visually joins the workspace content below.
- Editor tabs do not show the generic kind glyph in the titlebar. Other Labonair
  tab kinds retain their existing identifying glyphs. Preview editor labels
  remain italic and dirty editors keep their changed-state marker.
- A close glyph is hidden at rest and appears on the hovered tab. It has a
  tooltip, shared hit target, and the existing close-confirmation behavior.
- Previous/next controls occupy a compact, separated leading group. They stay
  visible and disabled when fewer than two tabs exist in the active Space. When
  enabled they use the Workspace's registered previous/next tab behavior.
- The Space selector, New Tab menu, global menu, macOS traffic-light inset,
  window drag, and double-click zoom retain their existing owners and actions.
  Zen Mode and `tabs_location == sidebar` continue to control placement.

## Input and lifecycle matrix

| State/input | Required result |
|---|---|
| One tab | Active treatment; previous/next controls visible but disabled; close action remains available |
| Multiple tabs | One active tab; inactive tabs remain muted; previous/next controls activate adjacent tabs in the active Space |
| Pointer over inactive tab | Label stays readable; close glyph appears without moving the label or changing tab width |
| Pointer over active tab | Active surface remains stable; close glyph appears; close action does not activate twice |
| Pointer click | Selects the tab and focuses its existing owner view |
| Double-click | Promotes an Editor preview tab and opens the Workspace tab rename field |
| Middle click | Requests close through Workspace, including dirty-editor confirmation |
| Right click | Opens the clicked tab's menu beneath the row and clears competing Workspace menus |
| Menu: Close | Requests only the clicked tab's close policy |
| Menu: Close Others | Preserves the clicked tab; closes only other tabs in its Space |
| Menu: Close Left/Right | Closes only tabs on that side in its Space; disabled when that side is empty |
| Menu: Close Clean | Closes only clean tabs in its Space; dirty tabs remain open |
| Menu: Close All / by kind | Applies only to the clicked tab's Space and preserves sequential dirty-tab confirmation |
| Menu: Keep Tab Open | Promotes an Editor preview tab to a persistent tab |
| Menu: Rename / Duplicate | Preserves existing Workspace owner behavior |
| Drag/reorder | Reorders tabs in the visible Space without changing their owner state |
| Keyboard focus | Enter/Space activates a focused tab; left/right moves among horizontal tabs; close button uses its own keyboard activation |
| Narrow width | Tabs keep their content width and the strip scrolls; spaces, new-tab, global-menu controls retain hit targets |
| Zen/header/sidebar mode | Existing titlebar visibility and sidebar placement rules remain effective |

## Acceptance evidence

1. Capture a paired Zed and Labonair titlebar at the same macOS window bounds,
   UI font, theme, scale, and pointer state. Include one inactive and one active
   preview Editor tab plus a dirty Editor tab in an additional capture.
2. Capture tab hover, close hover, disabled/enabled navigation, context menu,
   narrow overflow, Zen Mode, and Tabs-sidebar placement.
3. Exercise click, double-click preview promotion/rename, middle click, right
   click, drag reorder, left/right tab movement, each close-menu action,
   Escape/cancel during dirty-close, and keyboard activation.
4. Use the exact native Labonair executable PID for screenshots. If the native
   window cannot be inspected, leave visual cells Pending and state why.

The titlebar surface remains `Partial` until the paired native evidence is
captured and reviewed.
