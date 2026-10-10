# Zed Editor Tab Bar and Labonair Titlebar Analysis

**Date:** 2026-10-10
**Reference:** `zed-refrence/zed` at `3569541038dd51524b03998ba4d38d253cb54f80`
**User visual:** `/Users/niklas/.codex/attachments/fd57a3d2-c018-4795-a0a4-105e641d4dc3/image-1.png`

## Evidence and clean-room record

The reference checkout is clean at the pinned commit. The image supplied by
the user shows a cropped dark editor-tab row: two navigation arrows precede
four file tabs; `lib.rs` is active and italic; inactive labels are muted; the
tab boundaries are thin vertical strokes. No file icons or close glyphs are
visible in this pointer-neutral capture. The screenshot does not record its
window bounds, native scale, theme name, or capture pointer coordinates.

Source inspection was used to discover visible behaviors in these pinned
files:

- `crates/ui/src/components/tab_bar.rs` — row background, height, start/end
  slots, borders, and horizontal scrolling;
- `crates/ui/src/components/tab.rs` — selected/inactive surfaces, separators,
  content height, and tab slot geometry;
- `crates/workspace/src/pane.rs` — navigation buttons, tab activation,
  preview/rename handling, close affordances, drag/drop, overflow, and the
  per-tab context menu.

The same contributor inspected the reference and implements the Labonair
changes. This is an independently written adaptation, not a separated-room
process. No Zed source, comments, assets, or file/type structure are carried
into Labonair. This record is not a legal-clearance claim.

## Reference behavior and appearance

### Row and tab geometry

- The row spans the pane width and follows the active UI font and density. Its
  height is the 32-unit tab-bar token; tab content is one logical pixel shorter
  so the selected surface joins the editor below it.
- The bar has three horizontal regions: optional start controls, a flexible
  tab viewport, and optional end controls. The tab viewport scrolls
  horizontally; it does not shrink every tab to fit.
- The navigation group and optional end group have a vertical separator and
  compact internal spacing. The screenshot shows the navigation group and no
  visible end controls.
- Tabs use content-driven widths with bounded title truncation, rather than
  equal-width columns. Each tab has a compact leading slot and trailing slot;
  item icons and actions may occupy them when enabled.
- The active tab uses the active-tab surface and primary foreground. Inactive
  tabs use the tab-strip surface and muted foreground. Thin side and lower
  borders define inactive tabs; the active tab has no visible lower seam into
  its editor. Text and separator colors follow the selected theme.
- The screenshot has no visible file glyphs. File icons are an optional tab
  presentation in the reference, so their absence is a valid editor-tab state.
- `lib.rs` is italic in the screenshot, matching the reference preview-tab
  treatment. A dirty editor uses its item indicator; it is not represented by
  changing the whole tab surface.

### Controls and state changes

- Back and forward controls call the focused pane's navigation-history actions
  and are disabled independently when the corresponding history stack is empty.
  Their icons remain in the row when disabled.
- A tab click activates that item and its pane. Double-click also commits a
  preview tab and invokes the item's rename action when the item supplies one.
  A middle click closes an unpinned tab. A tab can be dragged to reorder it or
  dropped into a pane/split target.
- The close affordance follows the user's close-button setting. In the hover
  setting, its compact close icon appears on the hovered tab and carries a
  “Close Tab” tooltip. Pinned tabs show a pin action instead.
- Right-click opens a tab-specific menu. Common entries include Close, Close
  Others, Close Left, Close Right, Close Clean, and Close All; Close Left/Right
  and other inapplicable operations are disabled or omitted. Pinned tabs add
  Pin/Unpin. File-backed items can contribute path, reveal, terminal, permalink,
  and other owner actions. Read-only items can toggle editability. These
  context actions are conditional on the tab and project capabilities.
- New/split controls are optional pane controls. The supplied screenshot does
  not show them. The reference can keep them at the row's end without forcing
  them into every image or tab configuration.

### Labonair comparison before this change

The shell owns a 40-pixel titlebar with a macOS traffic-light inset, drag-to-
move area, double-click zoom, and one global-menu button. Workspace renders the
visible Space's tabs in that titlebar, with a Space picker and New Tab menu.
Those shell and Workspace ownership boundaries are consistent with Labonair's
product contract and remain in place.

Before the port, horizontal tabs were 28 pixels high, separated by small gaps,
rounded individually, and filled with the generic primary-tinted selection
color. Every tab used a kind icon and a permanently visible close button. The
top strip had no history buttons. The tab context menu had Rename, Duplicate,
Keep Tab Open for editor previews, Close, Close Others, Close All, and
Close All by kind. Batch close behavior was not scoped to the active Space and
did not sequence dirty-tab confirmation.

The vertical Tabs panel already uses the shared UI-kit tab item and has its own
full-row geometry. Its presentation must remain independent from the horizontal
titlebar treatment.

## Crosswalk

| Concern | Reference evidence | Labonair owner/state | Port decision |
|---|---|---|---|
| Placement | Pane-local row above editor content | Shell places titlebar; Workspace renders active Space tabs | Keep permanent shell zone and existing active-Space boundary |
| Horizontal tab style | Flat full-height tabs, content widths, thin separators | `labonair-ui-kit::tab_item` | Add a horizontal chrome variant using toolbar/background/border theme roles; preserve vertical row styling |
| Editor labels | Muted inactive, foreground active, preview italic, dirty marker | Workspace `Tab`/`TabStore` snapshot | Preserve preview and dirty state; omit generic kind glyph from horizontal Editor tabs |
| Navigation | Back/forward history buttons, independently disabled | Workspace owns active tabs; existing Previous/Next Tab commands are registered | Add compact controls routed through current-space tab traversal; no new command or persisted state |
| Closing | Hover close, middle click, context operations | Workspace close policy and dirty-editor confirmation | Reveal close glyph on tab hover; keep close confirmation and sequence batch operations |
| Context menu | Close, close others/left/right/clean/all, then conditional item actions | Workspace owns tab collection and menu | Add positional/clean actions, scope menu operations to the clicked tab's Space, preserve Labonair rename/duplicate/grant actions |
| Overflow | Horizontal scroll; content widths remain stable | Workspace strip and UI-kit tabs | Preserve overflow and prevent tab compression in narrow windows |
| Global titlebar | Not shown in this cropped pane-tab screenshot | Shell titlebar owns traffic-light inset, global menu, window drag/zoom | Keep Labonair shell behavior and align the tab row to the pinned row height |
| Visual proof | Supplied crop plus pinned source | Native visual evidence catalog | Keep native paired captures Pending until an exact Labonair window can be captured |

## Limitations

The pinned Zed executable could not be opened through this session's computer
control runtime (`cua.getApp` is unavailable), and the current inventory reports
no native apps. The user screenshot and pinned source establish the target
specification but do not prove the rendered Labonair result. Native, matching
macOS captures and interaction review remain required.
