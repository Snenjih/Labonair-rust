# Universal UI and Interaction Acceptance Checklist

**Status:** Normative checklist for R07 parity work; baseline measurements pending
**Version:** 1
**Owner:** Product architecture and each surface owner

Attach this checklist to every UI or interaction task. Record the result for
each applicable state with the exact Labonair and reference builds, platform,
window bounds, scale, theme, font settings, capture path, and date. A source
inspection or successful build does not verify rendered parity.

## Measurements for each reusable control

| Control | Record from the paired reference and Labonair captures |
|---|---|
| Button and icon button | Outer bounds, hit target, icon box, wide/square shape, label baseline, horizontal/vertical padding, gap, border, radius, tooltip, indicator, and default/hover/pressed/selected/focused/disabled states |
| Text and search input | Single-line and multiline bounds, height and growth, horizontal/vertical padding, radius, UI-font and density scaling, text baseline, leading/trailing icons, placeholder, caret, selection, focus ring, validation/disabled state, paste, IME, undo/redo, keyboard editing, and owner-specific Enter/newline behavior |
| Slider and numeric field | Track and thumb bounds, value display and units, range limits, pointer adjustment, keyboard adjustment, focus, disabled behavior, and update/commit timing |
| Select and menu item | Trigger bounds, popup bounds, row height, item padding, icon/label/shortcut columns, selected/checked/disabled/destructive states, nesting, and keyboard traversal |
| List row, tab item, and tab strip | Row/tab height, horizontal/vertical geometry, leading/trailing content, selected/inactive surfaces and text, separators, active-content seam, selected/focused/dirty/busy/peek treatment, title truncation, close hit target and hover visibility, navigation controls, arrow-key direction, context-menu actions, reorder/drop target, and narrow overflow behavior |
| Settings value and owner-action row | Label/description wrapping, control alignment, modified-source label, reset placement beside the setting title, hover copy-link action and field deep-link destination, registered owner action destination, keyboard activation, disabled explanation, search match, 16-pixel top padding, 16-pixel inner-row bottom padding, 40-pixel section-end padding, and divider only between rows in one section |
| Dialog and tooltip | Shared card geometry, anchor or center alignment, viewport margins, maximum bounds, scrim, text wrapping, focus entry/return, keyboard dismissal, and action order |
| Divider and empty/loading/error view | Stroke and spacing; icon/text alignment; message width; progress treatment; actionable recovery; and stable layout while the state changes |

Record measured values in logical points and device scale separately. Name the
element and state used for each measurement. If the reference cannot be
captured, leave the value `Pending`; do not infer it from a source constant.

## Popup and menu acceptance

- A popup opens from the visible trigger bounds and stays within the usable
  window viewport. Test all four edges, each corner, short windows, and a
  popup taller or wider than the available area.
- Record anchor edge, offset, flip/clamp result, popup bounds, scroll bounds,
  and whether focus returns to the trigger after dismissal.
- Menu rows define mouse hover, click, disabled, checked, destructive, nested,
  separator, keyboard highlight, activation, and Escape behavior.
- Nested menus remain reachable without pointer-only hover. The selected row
  and focused row are visually distinguishable.
- Dialogs and pickers preserve the active task when validation or an operation
  fails. Their failure copy and next action are part of the evidence record.

## Surface states

For each surface, mark every state as `Verified`, `Pending`, or `N/A` with a
reason:

- normal, narrow, short, resized, high-DPI, light, and dark;
- focused, keyboard-only, pointer-only, hover, selected, disabled, and
  multi-selection where applicable;
- empty, loading, long-list/large-file, success, error, offline, permission
  denied, missing data, and corrupted data where applicable;
- overlay open/closed, trigger-edge anchoring, dismissal, and focus return;
- accessible names, focus order, visible focus, text scaling, and platform
  input methods.

## Workflow review

For Settings owner actions, verify that one registered contribution supplies
the row metadata, page placement, search terms, and canonical action. Check
duplicate IDs and conflicting page metadata at registration time; the renderer
must not need a feature-specific row or search branch.

- Every visible action has an owner and a stable command identity where it is
  command-capable. Frequent actions have discoverable bindings; destructive
  actions have a clear confirmation path.
- A workflow can be completed with the keyboard and with the mouse. Dragging,
  scrolling, resizing, context actions, and touchpad behavior are checked when
  the surface supports them.
- Feedback is immediate for local actions and truthful for asynchronous work.
  Cancellation, retry, recovery, and undo are checked where applicable.
- Large projects, long lists, and large files use named fixtures and recorded
  measurements. Performance targets come from Labonair's performance contract,
  not an unverified claim from a public reference page.
- Labels use the glossary in [`glossary.md`](glossary.md). Public documentation
  is checked against the pinned runtime before it is treated as acceptance
  evidence.

## Evidence record

```text
Surface/state:
Reference build and revision:
Labonair revision and executable:
OS/platform:
Window bounds and usable viewport:
Device scale and UI scale:
Theme and fonts:
Input path:
Capture/reproduction artifact:
Measurements:
Observed result:
Date and reviewer:
Open differences and owner:
```
