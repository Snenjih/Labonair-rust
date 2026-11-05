# R10-002 Manual Visual QA Checklist

**Status:** Evidence checklist (not normative)
**Scope:** R10-002 UI overhaul (universal panes, floating tabs, Spaces, Settings, themes, motion)

Scenarios that agents could not verify with screenshots. Run the native app
with `cargo run -p labonair` (never `open -a Labonair`, see
[`../visual-verification.md`](../visual-verification.md)). Tick each box only
after looking at the running app.

## Appearance matrix

Repeat the smoke list below for each combination.

- [ ] Dark + Classic + Comfortable
- [ ] Dark + Classic + Compact
- [ ] Dark + Glassy + Comfortable
- [ ] Dark + Glassy + Compact
- [ ] Light + Classic + Comfortable
- [ ] Light + Classic + Compact
- [ ] Light + Glassy + Comfortable
- [ ] Light + Glassy + Compact

Smoke list: titlebar and tabs, one pane, two panes, Settings window, a menu, a
dialog. No clipped corners, no double borders, text contrast readable.

## Panes

- [ ] Single terminal is full-bleed with no pane bar
- [ ] Two terminals: bars appear, active bar sunken and semibold, inactive bar shows actions on hover only
- [ ] Terminal + editor side by side: both render, focus follows clicks
- [ ] Four panes (2x2): dividers align, no overlap at corners
- [ ] Zoom a pane (bar button, double-click bar, command) and restore; other panes return unchanged
- [ ] Resize by dragging dividers; minimum sizes hold; compact and comfortable both
- [ ] Drag a pane by its bar onto each drop zone (left, right, top, bottom, centre swap); the drop marker matches the result
- [ ] Drag onto itself and onto an invalid target: nothing breaks, no stuck marker
- [ ] Close a pane with the bar button; neighbours reclaim the space
- [ ] Hover list (tab/pane group label) shows the right panes and icons
- [ ] Rounded frame: square content (terminal cells, editor gutter) stays inside the corner in the comfortable layout
- [ ] Restart the app: pane layout, zoom state and focus restore (session v3); an old v2 session still opens

## Spaces and tabs

- [ ] Spaces popover opens, fades in, fades out on Esc, outside click and item pick
- [ ] Space rows: active row sunken + semibold, tab count muted, no colour badge
- [ ] Per-Space "..." menu: New Tab, Rename (inline field), Close Space; the Default Space cannot be closed
- [ ] Rename commits on popover dismiss, Esc cancels only the rename
- [ ] Recent Projects section collapses; remove button shows on hover only
- [ ] Floating tabs strip: selected, hover, dirty dot, busy marker, pinned group gap, close on hover
- [ ] Long title truncates with a tooltip; the strip scrolls and keeps the selected tab in view
- [ ] Drag a tab: insertion marker on the correct side; rejected drops (pinned onto unpinned, other Space) show none
- [ ] Tab context menu and new-tab menu anchor below the strip and fade out
- [ ] `tabsLocation = sidebar`: vertical floating tabs in the Tabs panel, same states as the strip
- [ ] Docks open and close with the reveal animation; startup does not animate

## Settings

Check in Light and Dark, in a narrow window, and with search.

- [ ] General
- [ ] Appearance (colour scheme, interface style, tabs, panes, density, typography, motion, Zen mode)
- [ ] Themes (colour theme grid, icon theme, background)
- [ ] Editor
- [ ] Languages & Tools
- [ ] Search & Files
- [ ] Terminal (and Terminal > Advanced)
- [ ] Keymap (opens the Keymap tab)
- [ ] Sidebar chips, selected row, back/forward buttons and `cmd-[` / `cmd-]`
- [ ] Preview tiles: selection ring, arrow keys, Enter/Space
- [ ] Project scope: whitelisted fields editable, others visibly inert
- [ ] Reset button appears for modified values and clears the override
- [ ] Search finds fields and inline owner sections and navigates to them

## Themes

- [ ] Colour theme grid shows light and dark halves; selection applies live
- [ ] Hover preview and cancel restore the previous theme
- [ ] A theme family missing a light or dark variant is rejected with a notification
- [ ] Icon theme picker applies to Explorer and tabs
- [ ] Inline background: pick an image, opacity and blur sliders, tint; remove the image
- [ ] With an image: Classic and Glassy both let it show through chrome, terminal and sidebars; menus stay readable
- [ ] Without an image: the page background is opaque in Classic, blurred only in Glassy

## Animations

Set `appearance.animations` to each value and relaunch or re-apply.

- [ ] Full: popover/menu rise and fade, dialog fade, switch thumb slide, tab selection fade, dock reveal
- [ ] Reduced: fades only, no slides or size reveals
- [ ] Off: everything instant, no flicker
- [ ] Esc spam on a menu does not restart the exit fade; a closing menu swallows clicks
- [ ] Component Gallery (debug build, command palette) Motion section agrees with the above

## Notifications

- [ ] A notification with an action shows its button; clicking it runs the owner's handler
- [ ] The dropdown fades out

## Known limitations (expected, do not file as bugs)

- Editor mirror panes show plain text until clicked
- `cmd-alt-up` / `cmd-alt-down` and `cmd-[` / `cmd-]` are shadowed while an editor has focus
- Splitting an SSH or SFTP pane opens a new connection
- `animations = system` currently behaves like Full (no OS Reduce-motion lookup)
- GPUI 0.2.2 has no hover transitions: hover is instant by design
