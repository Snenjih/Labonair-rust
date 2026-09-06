# Vergleichsbericht — Subagent 1: App-Shell / Window-Chrome / "Unibar"

Scope: window/titlebar chrome, the unified titlebar+statusbar ("unibar") bar-item
system, tabs (tab bar + "+" dropdown + tab context menus), sidebar docking model,
and the native macOS menu bar / Dock menu.

Reference (read-only): `reference-src/`. Rust port: `crates/`.
All paths below are absolute-relative to the repo root
`/Users/niklas/Developer/active/Labonair/Labonair-rust`.

---

## 0. Executive summary

The Rust port never ported the reference's **unified bar-item ("unibar")
architecture**. In the reference, one registry (`barItems.ts`) + one renderer
(`renderBarItem.tsx` / `buildBarBucket`) drives BOTH the titlebar (`Header.tsx`)
and the statusbar (`StatusBar.tsx`); every item (badges, panel toggles, cwd
breadcrumb, cursor position, preview chip, AI cluster) is individually placeable
into `{bar: titlebar|statusbar, side: left|right, hidden}` via a shared
right-click menu (`BarItemContextMenu.tsx`) and persisted in the
`barItemPlacements` preference.

The Rust `AppShell` (`crates/ui/src/app_shell.rs`) instead hardcodes:
* a header with a hamburger, a **static "Labonair" text label**, a spacer, an
  inline search box, the agent-access badge, and a **dead `⋯` button**;
* a statusbar with a non-interactive cwd breadcrumb + an "N panes" counter.

Neither bar renders tabs. Tabs live in a **third** location —
`Workspace::render_tab_bar` (`crates/ui/src/workspace.rs:2654`) — as a 36px-tall
strip with its own bottom border, above the workspace content.

Combined with `main.rs` opening the window with a **non-transparent native
titlebar** (`appears_transparent: false`, `crates/app/src/main.rs:70-74`), this
produces exactly the three symptoms the user reports:

| Symptom | Root cause |
|---|---|
| "Two titlebars: top one with a stray button; second one says 'Labonair' where tabs + traffic lights belong" | Native opaque titlebar (OS-drawn, shows the `"Labonair"` window title + traffic lights) **plus** the custom `render_header` below it (hamburger = sidebar toggle, `.child(div().text_xs()…child("Labonair"))` at `app_shell.rs:859`). The reference uses ONE transparent overlay titlebar with the tab bar and traffic lights in it. |
| "Unibar not functional / not modular" | The entire `barItems` registry + `buildBarBucket` + `BarItemContextMenu` system is unported on the UI side. Header/statusbar contents are hardcoded. |
| "Phantom vertical bar on the LEFT" | `render_sidebar` invents a 44px VS-Code-style activity **rail** with `border_r_1` (`app_shell.rs:1070-1097`, RAIL_W = `44.0` at line 60) that the reference does not have, plus a 6px solid-colour resize `handle` (`app_shell.rs:1117-1125`) where the reference uses a thin `ResizableHandle`. |
| "'+' tab button should open a dropdown" | `Workspace::render_tab_bar` wires the `+` straight to `open_terminal_tab` with no menu (`workspace.rs:2696-2699`). Reference `TabBar.tsx:286-308` opens a `DropdownMenu` with `NewTabDropdownItems`. |
| "Native OS menu bar correctness" | Mostly correct; several deviations listed in §6 (extra App-menu items, extra Window-menu item, a duplicated `Cmd+I`, missing accelerators for zoom, `Cmd+K`, etc.). |

Note: the backend *data* layer for placements **was** ported —
`crates/backend/src/modules/settings/mod.rs:37` `settings_set_bar_item_placement`
and the `barItemPlacements` settings key exist — but nothing in the UI ever calls
it, and there is no `BarItemId` enum, no defaults table, and no consumer. It is
currently dead code.

---

## 1. Window chrome / titlebar

### 1a. Two titlebars (native + custom)

**(a) Reference.** `reference-src/src-tauri/tauri.conf.json:23-24`:
```json
"titleBarStyle": "Overlay",
"hiddenTitle": true,
```
On macOS this yields a single **transparent overlay** titlebar: no OS-drawn title
text, traffic lights floating over the app's own `Header`. `Header.tsx:233-241`:
```tsx
<div data-tauri-drag-region className={cn(
  "flex h-10 shrink-0 items-center gap-2 border-b border-border/60 bg-toolbar select-none",
  IS_MAC ? "pr-2 pl-20" : "pr-0 pl-2")}>
```
`pl-20` (80px) reserves space for the traffic lights inside the same 40px bar
that also holds the tab bar. `WindowControls.tsx:7` returns `null` on macOS
(`USE_CUSTOM_WINDOW_CONTROLS = !IS_MAC`), i.e. the traffic lights stay native but
positioned into the custom bar.

**(b) Rust port.** `crates/app/src/main.rs:70-74`:
```rust
titlebar: Some(TitlebarOptions {
    title: Some("Labonair".into()),
    appears_transparent: false,
    traffic_light_position: None,
}),
```
`appears_transparent: false` ⇒ macOS draws its **own opaque titlebar** with the
`"Labonair"` string and the traffic lights. Then `AppShell::render_header`
(`app_shell.rs:818-877`) draws a *second* 40px bar directly below it, itself
containing `.pl(px(TRAFFIC_LIGHT_INSET))` = 78px of empty left padding
(`app_shell.rs:58`) for traffic lights that are not there, a hamburger
sidebar-toggle, and a literal `"Labonair"` text node (`app_shell.rs:859`).
Result: the user sees the OS titlebar ("top one"), then the custom header
("second one showing 'Labonair'"), and the tabs are nowhere near either.

**(c) Fix.**
1. `main.rs`: `appears_transparent: true`, `title: None` (or keep title — it
   won't render when transparent), and set
   `traffic_light_position: Some(point(px(19.0), px((40.0 - 14.0) / 2.0)))` so
   the lights vertically-centre in the 40px header. GPUI's `TitlebarOptions`
   supports exactly these three fields; confirm the `point`/`px` import.
2. Make `render_header` the drag region for the window
   (`.on_mouse_down`/window-drag — GPUI: `.window_control_area(…)` or an empty
   `div` with `cx.listener` calling `window.start_window_move()`; verify the
   0.2.x API name against the Zed source before use).
3. Delete the static `"Labonair"` text node (`app_shell.rs:859`).
4. Keep `TRAFFIC_LIGHT_INSET` (~78px, reference uses 80) as the header's left
   pad; drop it everywhere else.
5. Move the tab bar into the header (see §3).

### 1b. Header drag region / height / styling

* Reference header: `h-10` (40px), `bg-toolbar`, `border-b border-border/60`,
  `gap-2`, `select-none`, `data-tauri-drag-region` on the root and on the
  central flex filler (`Header.tsx:253,273`).
* Rust header: `h(px(HEADER_H))` = 40 ✓, `bg(toolbar)` ✓, `border_b_1` ✓,
  `gap_2` ✓ — but **no drag region**, so the window can't be moved by dragging
  the header. Add it.
* Reference has an `IS_MAC` divider after the traffic-light area
  (`Header.tsx:244`: `<span className="mr-1 h-full w-px shrink-0 bg-border" />`).
  Missing in Rust — minor, add when porting the bucket layout.

### 1c. The `⋯` "app menu" button is dead

**Reference** (`Header.tsx:199-231`, `sideButtons`): a `DropdownMenu` triggered
by a `Menu01Icon` button (`size-7`, `rounded-md`, `text-muted-foreground
hover:bg-accent hover:text-foreground`, `title="Menu"`), rendered **after** the
right bucket on macOS (`Header.tsx:283`), before on other platforms
(`Header.tsx:242`). Menu content (`align="end"`, `w-52`):

| Item | Icon | Shortcut hint | Action |
|---|---|---|---|
| Settings | `Settings01Icon` | — | `onOpenSettings` |
| Keyboard Shortcuts | `KeyboardIcon` | `⌘?` | `onOpenShortcuts` |
| Keymap | `Key01Icon` | — | `onOpenKeybindings` |
| Themes... | `EyeIcon` | — | `onOpenThemes` |

**Rust** (`app_shell.rs:865-876`): a `div` with child `"\u{22EF}"` (`⋯`), correct
size/hover styling, but **no `on_click`, no dropdown** — a decorative dead
control.

**Fix:** give it a popover (same pattern as `render_agent_badge` at
`app_shell.rs:883-1018`) with the four items above, dispatching
`menu::OpenSettings`, `menu::OpenShortcuts`, a new keymap action (settings →
shortcuts page), and `CommandId::…`/theme page open. Use `Menu01Icon`
equivalent glyph, not `⋯`.

---

## 2. The unified bar-item ("unibar") system — architecture

This is the single biggest gap. The reference model, file by file:

### 2a. Registry — `reference-src/src/modules/settings/lib/barItems.ts`

* `BarItemId` (lines 25-45): `updater`, `notifications`, `jumpHosts`,
  `agentAccess`, `transfers`, `bookmarks`, `explorerPanel`, `snippetsPanel`,
  `sourceControlPanel`, `tabsPanel`, `cwdBreadcrumb`, `cursorPosition`,
  `previewUrl`, `aiMini`, `aiPanel` — 15 items.
* `BarItemPlacement` (lines 7-23): `{ itemId, bar: "titlebar"|"statusbar",
  side: "left"|"right", hidden: bool, extra? }`.
* `BAR_ITEM_CATEGORY` (lines 52-68): each item → `"badge"|"panel"|"info"|"ai"`;
  drives divider placement.
* `BAR_ITEM_ORDER` (lines 71-87): stable iteration order (NOT object-key order).
* `DEFAULT_BAR_ITEM_PLACEMENTS` (lines 119-135): fresh-install layout — all
  badges → `titlebar/right`; panel toggles + `cwdBreadcrumb` → `statusbar/left`;
  `cursorPosition`, `previewUrl`, `aiMini`, `aiPanel` → `statusbar/right`.
* `visibleItemsFor(placements, bar, side)` (lines 187-196): the bucket query.
* `PANEL_ITEM_TO_PANEL` / `PANEL_TO_ITEM_ID` (lines 89-106): panel-toggle item ↔
  `SidebarPanel` mapping; a panel toggle's own `side` also chooses which dock
  slot the panel content opens into.

### 2b. Renderer — `reference-src/src/modules/statusbar/lib/renderBarItem.tsx`

* `RenderBarItemCtx` (lines 36-71): everything an item might need.
* `renderBarItem(id, ctx)` (lines 136-245): a `switch` returning the element for
  each id, **sized by that item's current placement** (`compact = placement.bar
  === "statusbar"`; titlebar renders items one notch larger).
* `buildBarBucket(bar, side, ctx, dividerClassName)` (lines 253-287): the SINGLE
  shared entry point `Header` and `StatusBar` both call. It:
  1. `visibleItemsFor(…)` → ids in order,
  2. `renderBarItem` each (drop `null` results — items that self-hide),
  3. wrap each non-breadcrumb cluster in a `ContextMenu` + `BarItemContextMenu`,
  4. `withDividers(clusters, dividerClassName)`.

### 2c. Divider rule — `reference-src/src/modules/settings/lib/barItemLayout.tsx`

`withDividers` (lines 20-30): insert a divider **only between two adjacent
clusters of different `category`**; never leading/trailing, never between two
same-category items. Titlebar divider class:
`"mx-0.5 h-5 w-px shrink-0 bg-border/60"` (`Header.tsx:196-197`); statusbar:
`"mx-1 h-3.5 w-px shrink-0 bg-border/60"` (`StatusBar.tsx:132,135`).

### 2d. Per-item right-click menu — `reference-src/src/modules/settings/components/BarItemContextMenu.tsx`

Content (lines 34-64), width `w-44` (breadcrumb overrides to `w-56`):
* radio group **Side**: `Left` / `Right` → `setBarItemPlacement(itemId,{side})`
* separator
* radio group **Bar**: `Titlebar` / `Statusbar` → `setBarItemPlacement(itemId,{bar})`
* optional `extra(placement)` block (used by `cwdBreadcrumb` for its path
  actions — see §5, and by the AI cluster for a Panel/Mini surface radio)
* separator
* **Hide** → `setBarItemPlacement(itemId,{hidden:true})` (feature/shortcut/
  command-palette entry stay live; only the button is hidden).

### 2e. Host bars

* `Header.tsx:196-281`: `titlebarLeft = buildBarBucket("titlebar","left",…)`,
  `titlebarRight = buildBarBucket("titlebar","right",…)`, rendered on either side
  of the central `TabBar` (rendered only when `tabsLocation === "titlebar"`).
* `StatusBar.tsx:129-138`: `footer` `h-8` `bg-status-bar` `px-3` `text-[11px]`,
  left bucket `buildBarBucket("statusbar","left",…)`, right bucket
  `("statusbar","right",…)`.
* `AppShell.tsx:227-261,337-352`: `<Header>` gated on `zenModeShowHeader`,
  `<StatusBar>` on `zenModeShowStatusbar`.

### 2f. What the Rust port has instead

`AppShell::render_header` (`app_shell.rs:832-877`) — fixed children:
hamburger toggle, `"Labonair"` text, `flex_1` spacer, optional search,
agent-access badge (only when `bridge_enabled && !entries.is_empty()`), dead `⋯`.

`AppShell::render_statusbar` (`app_shell.rs:1172-1257`) — fixed children:
a hand-rolled non-interactive breadcrumb (or the tab label when no cwd), and a
right group that only ever shows `"{panes} panes"` when `panes > 1`. Comment at
`app_shell.rs:1251-1252` even says *"Connection / jump-host / AI badge slots stay
empty until their phases…"* — they were never filled.

There is **no** `BarItemId`, no `BarItemPlacement`, no `buildBarBucket`, no
`withDividers`, no `BarItemContextMenu`, no `barItemPlacements` consumption, no
`tabsLocation`, no `badgesAlwaysVisible`, no dual bucket per bar.

### 2g. Fix recommendation (architecture)

Create `crates/ui/src/bar_items.rs`:

1. `enum BarItemId` (15 variants) + `enum BarCategory { Badge, Panel, Info, Ai }`
   + `const BAR_ITEM_ORDER: [BarItemId; 15]` + `fn category(id) -> BarCategory`.
2. `struct BarItemPlacement { bar: BarLoc, side: BarSide, hidden: bool }` with
   `enum BarLoc { Titlebar, Statusbar }`, `enum BarSide { Left, Right }`.
3. `fn default_placements() -> HashMap<BarItemId, BarItemPlacement>` reproducing
   `DEFAULT_BAR_ITEM_PLACEMENTS` exactly.
4. Load/merge from the `barItemPlacements` settings blob via a new
   `PreferencesStore` accessor; persist changes through the already-existing
   `settings_set_bar_item_placement` backend fn
   (`crates/backend/src/modules/settings/mod.rs:37`).
5. `fn visible_items_for(&placements, bar, side) -> Vec<BarItemId>`.
6. `struct BarCtx<'a>` (the Rust analogue of `RenderBarItemCtx`) carrying
   `&ThemeStore`, `Entity<Workspace>`, agent-access store, updater view,
   notifications, transfers store, bookmarks, AI store, cursor pos, preview URL,
   panel-toggle callbacks, etc.
7. `fn render_bar_item(id, &BarCtx, compact: bool, cx) -> Option<AnyElement>`.
8. `fn build_bar_bucket(bar, side, &BarCtx, divider_style, cx) -> Vec<AnyElement>`
   implementing the same divider rule as `withDividers`.
9. `fn bar_item_context_menu(id, cx)` — a small popover with the Side/Bar radios
   + Hide, calling the persist fn. Reuse the existing ad-hoc context-menu
   pattern from `workspace.rs:2948`.

Then rewrite `render_header` / `render_statusbar` to call `build_bar_bucket`
twice each and drop all the hardcoded children. Gate on the existing
`zen_mode_show_header` / `zen_mode_show_statusbar` prefs (already wired,
`app_shell.rs:1277-1283`).

---

## 3. Tabs

### 3a. Tab bar location

* Reference: tab bar renders **inside `Header`** when `tabsLocation ===
  "titlebar"` (`Header.tsx:253-274`), or in the left sidebar as
  `SidebarTabList` when `tabsLocation === "sidebar"` (via the `tabsPanel` bar
  item, `renderBarItem.tsx:154-170`). Default is `titlebar`.
* Rust: tab bar is a separate `Workspace::render_tab_bar`
  (`workspace.rs:2654-2701`, `h(px(36.0))`, `border_b_1`) rendered as the first
  child of the `Workspace` view (`workspace.rs:3133,3155`). It is neither in the
  header nor in the sidebar — a third location, and the reason the user says the
  tabs "should be" in the titlebar.

**Fix:** render the tab strip inside `render_header` between the left and right
buckets (mirroring `Header.tsx:253-274`), height `h-7` (28px) not 36, no
bottom border. Remove `render_tab_bar` from `Workspace`. Longer term add a
`tabs_location` pref + `SidebarTabList` equivalent, but titlebar-only is
acceptable for first parity.

### 3b. Tab visual spec (reference `TabBar.tsx` + `tabUtils.tsx`)

| Aspect | Reference | Rust (`workspace.rs:2541-2652`) |
|---|---|---|
| Height | `h-7` (28px) | `h(px(28.0))` ✓ |
| Active indicator | sliding pill behind triggers, `bg-foreground/[0.07]`, `ring-1 ring-inset`, `shadow-row`, animated `transform/width` with `--ease-premium`/`--dur-base` (`TabBar.tsx:121-135`) | solid `bg(accent)` fill, no pill, no slide animation. Acceptable simplification; note divergence from Critical Rule #3. |
| Icon | per-kind `HugeiconsIcon` / file-type `img` (`TabIconFor`, `tabUtils.tsx:238-291`) — terminal, ssh-terminal, globe, git-compare (warning/modified/info tinted), home, cloud-server, git-branch | single text glyph via `TabKind::indicator()` (`tabs.rs:47-59`): `⌂ ▸ ✎ ◈ ✦ ⇅ ⎇ ±`. No SSH-vs-local distinction, no file-type icons, no dirty/peek tint on icon. |
| Dirty dot | `size-1.5 rounded-full bg-foreground/70`; **remote-sync-failed** shows `bg-destructive` instead (`TabBar.tsx:216-227`) | `size(px(6.0)) rounded_full bg(fg) opacity(0.7)` for `dirty` only; **no remote-sync-failed state** (`workspace.rs:2610-2612`). |
| Peek (italic) | `italic` on label when `t.kind==="editor" && t.peek`; double-click promotes to permanent (`TabBar.tsx:184-190,213`) | `.italic()` on label ✓ (`workspace.rs:2607`); **no double-click-to-promote**. |
| Close button | shows only when `tabs.length > 1 && kind !== "home"`; `opacity-0 group-hover:opacity-60 hover:opacity-100`; `Cancel01Icon` size 11 | `closable = total > 1 && kind != Home` ✓; always visible (no hover-reveal), glyph `✕` (`workspace.rs:2575-2585`). |
| Middle-click close | `shouldMiddleClickClose` (`TabBar.tsx:174-183`) | `on_mouse_down(Middle, …)` ✓ (`workspace.rs:2617-2624`). |
| Drag reorder | `@dnd-kit` sortable, horizontal strategy, drag preview, `opacity-50` while dragging | `on_drag`/`drag_over` (`border_l_2`) / `on_drop` → `s.reorder` ✓ (`workspace.rs:2632-2646`). Reasonable parity. |
| Wheel-scroll horizontally, scroll-active-into-view | `TabBar.tsx:90-109` | `overflow_x_scroll` only; no wheel handler, no scroll-into-view. |
| Rename in place | inline `<input>` on `startEditing` (double-click / context menu) with commit-on-Enter, freeze `customTitle` (`TabBar.tsx:145-166`, `TabRenameInput`) | **not implemented** — no rename UI at all (see §3d). |
| Entrance animation | `labonair-tab-in` keyframe: fade + `scale(0.86→1)` | opacity-only fade (`workspace.rs:2647-2651`) — documented GPUI limitation, acceptable. |

### 3c. The "+" new-tab dropdown

**Reference** (`TabBar.tsx:286-308` → `NewTabDropdownItems`,
`tabUtils.tsx:305-393`), `DropdownMenuContent align="start" min-w-44`:

| Item | Icon | Shortcut | Notes |
|---|---|---|---|
| Terminal | `TerminalIcon` | `⌘T` | `onNew()` |
| Editor | `PencilEdit02Icon` | `⌘E` | `onNewEditor()` |
| Preview | `Globe02Icon` | `⌘P` | `onNewPreview()` |
| Git Graph | `GitBranchIcon` | — | only if `onNewGitGraph` provided |
| — separator — | | | |
| SSH ▶ | `ComputerTerminal02Icon` | | submenu: up to 5 recent hosts (name + `host_address`, sorted by `last_connected_at`), or "No hosts yet"; separator; "All hosts..." → `onOpenHostManager` |
| SFTP ▶ | `CloudServerIcon` | | same submenu shape, `onNewSftp` |

Trigger button: `Button variant="ghost" size="icon" className="size-7 shrink-0
rounded-md text-muted-foreground hover:bg-accent hover:text-foreground"`,
`title="New tab"`, `PlusSignIcon` size 14 strokeWidth 2.

**Rust** (`workspace.rs:2685-2700`): a 24px `div` with child `"+"`, `on_click` →
`this.open_terminal_tab(window, cx)`. **No dropdown, no submenus, no
editor/preview/git-graph/ssh/sftp entries.**

**Fix:** replace the `+` `on_click` with a popover (pattern: `workspace.rs:2948`
context menu, or the bookmarks popover). Items dispatch existing actions:
`menu::NewTerminalTab`, `menu::NewEditorTab`, `menu::NewPreviewTab`, git-graph →
`CommandId::OpenGitGraph`/`w.open_git_graph_tab`. SSH/SFTP submenus need the
hosts list — `HostManagerView`/hosts store already exists (`hosts.rs`); pull the
5 most-recent by `last_connected_at` and call the SSH/SFTP tab openers that
`workspace.rs` already has (`open_sftp_tab` is referenced at
`app_shell.rs:701`). Size the trigger `size(px(28.0))`, glyph a proper `+`
(strokeWidth-2 plus), `title = "New tab"`.

### 3d. Tab context menus

**Reference — workspace tabs** (`WorkspaceTabContextMenuContent.tsx:56-107`),
`min-w-36`:
1. **Rename** (`PencilEdit02Icon`) → inline edit
2. **Duplicate** (`Copy01Icon`)
3. *(if ssh/local session and `agentBridgeEnabled`)* separator +
   **Grant AI Agent Access** checkbox (disabled + tooltip when
   `host.block_agent_access`)
4. *(if `tabsLength > 1`)* separator + **Close Others** (`Cancel02Icon`) +
   **Close All {pluralLabelFor(kind)}** (`Layers01Icon`) + separator +
   **Close** (`Cancel01Icon`)

**Reference — non-workspace tabs** (`NonWorkspaceTabContextMenuContent.tsx:35-77`):
1. *(if `editor && peek`)* **Keep Tab Open** (`PinIcon`) + separator
2. *(if not `home`)* **Duplicate Tab** (`Copy01Icon`) + separator
3. **Close Others** (`Cancel02Icon`)
4. **Close All** (`CancelSquareIcon`)
5. **Close All {pluralLabelFor(kind)}** (`Layers01Icon`)
6. *(if not `home`)* separator + **Close Tab** (`Cancel01Icon`)

`pluralLabelFor` (`tabUtils.tsx:213-234`): `Terminals`, `Editors`, `Previews`,
`AI Diffs`, `Home Tabs`, `SFTP Tabs`, `Git Graphs`, `Git Diffs`, `Commit Diffs`.

**Rust** (`Workspace::render_context_menu`, `workspace.rs:2948-3053`),
`min_w(px(160.0))`, hand-rolled absolute overlay:
1. **Close**
2. **Close Others**
3. **Close All Of This Type** *(literal string, not pluralised per kind)*
4. *(if grant target)* **Grant AI Agent Access** (prefixed `✓ ` when granted —
   reference uses a real checkbox item)

Missing entirely: **Rename**, **Duplicate / Duplicate Tab**, **Keep Tab Open**
(peek promote), **Close All** (all tabs), the per-kind pluralised label, the
`host.block_agent_access` disabled state, separators/icons, and the
workspace-vs-non-workspace split (Rust uses one menu for every kind).

**Fix:** split into two builders mirroring the two reference components; add
`pluralLabelFor` (`crates/ui/src/tabs.rs` already has `TabKind` — add a
`plural_label()` method); wire Rename to a new inline-edit state on the tab strip
(store already supports it: `TabStore::set_custom_title`, `tabs.rs:366`);
Duplicate to `w.duplicate_active_tab` (exists, `app_shell.rs:739`); "Keep Tab
Open" to `TabStore::set_peek(id, false)` (`tabs.rs:382`).

### 3e. Tab close confirmation

Reference: `closeConfirmation.ts` + `CloseDialogs.tsx` — 3 alert dialogs (save
untitled / close dirty editor / close running terminal). Rust has
`render_confirm` (`workspace.rs:3135`) for dirty editors
(`Tab::needs_close_confirm`, `tabs.rs:138`). Roughly covered; verify the
"running terminal" and "save untitled" variants exist — out of this subagent's
core scope, flag for the editor/terminal subagents.

---

## 4. Sidebar / docking model

### 4a. Reference: dual-dock, no activity rail

`AppShell.tsx:263-313`: a horizontal `ResizablePanelGroup` with **`SidebarContent
side="left"`**, a `ResizableHandle withHandle`, the `WorkspacePanel`
(`defaultSize="78%"`), another handle, and **`SidebarContent side="right"`**.
Each `SidebarContent` renders whichever of `FileExplorer` / `SidebarTabList` /
`SnippetsPanel` / source-control its `activePanel` is (or nothing / collapsed).

There is **no persistent icon rail**. Panels are switched by the four
**panel-toggle bar items** (`explorerPanel`, `snippetsPanel`,
`sourceControlPanel`, `tabsPanel`) which live in the statusbar by default
(`barItems.ts:126-129`); each item's `side` decides which dock (left/right) it
opens into (`renderBarItem.tsx:154-170`, `useSidebar.ts` +
`sidebarSlotLogic.ts`). `resolveToggle` (`sidebarSlotLogic.ts:34-49`): clicking
the active panel collapses it; clicking another switches; clicking when
collapsed re-expands. Widths per slot persisted
(`setSidebarWidth`/`setSidebarRightWidth`), debounced 300ms.

Icons for the panel toggles (`renderBarItem.tsx:73-85`): `FolderTreeIcon`,
`FlashIcon`, `GitBranchIcon`, `LayoutTopIcon`; titles `"Explorer (Cmd+B)"`,
`"Snippets"`, `"Source Control"`, `"Tabs"`. Active state:
`bg-primary/20 text-foreground dark:text-primary`.

### 4b. Rust: single left dock + invented 44px activity rail

`AppShell::render_sidebar` (`app_shell.rs:1059-1135`):
* `SidebarPanel` enum (`app_shell.rs:73-79`): `Explorer, Snippets, SourceControl,
  GitGraph, Ai` — note `GitGraph` and `Ai` are **panels** here, whereas the
  reference opens Git Graph as a **tab** and AI as a bottom panel / mini-window
  driven by the `aiPanel`/`aiMini` bar items.
* A 44px `rail` (`RAIL_W`, `app_shell.rs:60`) with **`border_r_1`**
  (`app_shell.rs:1079-1080`) and one `30px` button per panel, glyphs
  `📁 ✂ ⌥ ⛓ ✨` (`app_shell.rs:102-110`) — placeholder emoji, not the
  reference icons.
* The panel body (`app_shell.rs:1099-1115`) with an uppercased text title
  header.
* A **6px solid `bg(sidebar_border)` resize `handle`** (`app_shell.rs:1117-1125`)
  — a visible vertical bar, vs the reference's thin `ResizableHandle`.
* Only ever a **left** dock; no right dock.
* `select_panel` (`app_shell.rs:451-459`) implements collapse-on-reclick but not
  the per-side / re-expand-from-collapsed nuances.

**This activity rail + its `border_r` + the thick handle are the "phantom
vertical bar on the LEFT" the user reports.** The reference simply has no such
element.

### 4c. Fix

1. Remove the `rail` entirely (`app_shell.rs:1070-1097`) and `RAIL_W`.
2. Replace the 6px `handle` with a 1px border + a wider invisible hit area, or a
   GPUI resizable-panel equivalent (check `gpui-component` for a resizable
   panel; else keep a 1px visual + ~6px transparent grab zone).
3. Drive panel selection from the new `explorerPanel`/`snippetsPanel`/
   `sourceControlPanel`/`tabsPanel` bar items (§2), not a rail.
4. Add a right dock: a second sidebar slot on the other side of the workspace,
   each bar item's `side` choosing which slot it targets. If a full dual-dock is
   too large for one pass, at minimum honour `side` so an item set to `right`
   docks right.
5. Move Git Graph out of the sidebar to a tab (there's already
   `TabKind::GitGraph`); keep AI as `aiPanel` (docked) + `aiMini` per the
   reference, not a sidebar panel.
6. Use real icons + the reference titles/active-styling for the toggles.

---

## 5. CwdBreadcrumb (statusbar `info` item)

**Reference** (`reference-src/src/modules/statusbar/CwdBreadcrumb.tsx`): a rich
component:
* **File mode** vs **dir mode** (`filePath` present ⇒ dir segments navigate,
  filename is a non-clickable leaf) (lines 72-118).
* Each parent segment is a **clickable `Badge`** → `onCd(seg.fullPath)`; home
  segment shows a `Home03Icon` and label "Home" (lines 257-295).
* Middle segments **collapse** into a `⋯` dropdown on narrow widths
  (`CollapsedSegments`, lines 396-456).
* The **current** segment is a `DropdownMenu` that lazily `readDir`s and lists
  subfolders to `cd` into (`CurrentSegmentDropdown`, lines 327-394), with an
  `ArrowDown01Icon` affordance.
* Right-click on any segment opens **`BarItemContextMenu` with an `extra` slot**
  (`SegmentExtraActions`, lines 195-255): segment label, "Copy absolute path",
  "Copy relative path", "Open in current terminal" (`onCd`), "Open in new
  terminal" (`onCdInNewTab`), "Bookmark this path" / "Remove bookmark" (when
  `bookmarksEnabled`), "Reference in AI chat". Note it's the bar-item menu — so
  the same right-click also lets you move/hide the breadcrumb.
* Remote-aware via `remoteTarget` (`{hostId, sessionId}`) — browses through the
  same SSH session the explorer tree uses.
* When no cwd: `"no directory"` muted text, still right-clickable to
  `BarItemContextMenu itemId="cwdBreadcrumb"`.

**Rust** (`app_shell.rs:1186-1220`): splits the display path on `/`, renders
plain non-interactive `div`s per segment (last = `fg`, rest = `muted`) with `/`
separators; `~` home substitution via `display_path` (`app_shell.rs:1359-1369`).
**No clicks, no cd, no subfolder dropdown, no collapse, no context menu, no
file/dir mode, no remote support, no bookmark integration.**

**Fix:** port as a dedicated `crates/ui/src/cwd_breadcrumb.rs` rendering
clickable segments (`on_click` → `workspace.send_cd(path)`), a current-segment
subfolder popover (`fs` listing already available via the explorer's fs
provider), the `⋯` collapse for overflow, and the `extra`-slot right-click menu
(wire into the new `bar_item_context_menu`). `send_cd`/`cd_in_new_tab` already
exist on `Workspace` (referenced in `app_shell.rs`).

---

## 6. Native macOS menu bar + Dock menu

Source of truth: `reference-src/src-tauri/src/lib.rs` `build_menu` (lines
223-333) and `reference-src/src-tauri/src/modules/dock_menu.rs`.
Rust port: `crates/ui/src/menu.rs`.

### 6a. Menu bar — structure

Order matches: `Labonair, File, Edit, View, Terminal, Connections, AI, Window`
(`menu.rs:213-317`, test at `menu.rs:353-368`) ✓ vs reference
(`lib.rs:238-333`).

### 6b. Menu bar — per-item discrepancies

**App menu ("Labonair")**
* Reference (`lib.rs:238-244`): `About Labonair`, `Settings...`, ─, `Hide`,
  `Hide Others`, `Show All`, ─, `Quit`. (No separator between About and
  Settings; **no Services submenu; no Check for Updates**.)
* Rust (`menu.rs:215-232`): `About Labonair`, ─, `Settings…`, ─,
  **`Check for Updates…`**, ─, **`Services` (os_submenu)**, ─, `Hide Labonair`,
  `Hide Others`, `Show All`, ─, `Quit Labonair`.
* Deltas: extra separators; **extra `Check for Updates…`** (added by T15-005 —
  an intentional feature addition, but not reference parity); **extra `Services`
  submenu**; item labels `Hide Labonair` / `Quit Labonair` vs reference's
  predefined `Hide` / `Quit`.
* Recommendation: acceptable to keep `Check for Updates…` (real feature), but
  drop the extra separators and match reference layout otherwise; `Services` is
  a reasonable macOS-native addition — flag as deliberate.

**File menu** — `menu.rs:233-247` vs `lib.rs:254-258`: items + order + separator
match (`New Terminal Tab, New SSH Tab, New SFTP Tab, New Preview Tab, New Editor
Tab, ─, Save, ─, Close Tab, Close Pane`). Reference has no separator before
`Save` group vs Rust — actually reference: `…New Editor Tab, ─, close_tab,
close_pane` (one separator). Rust adds `─` after `New Editor Tab` **and** `─`
after `Save`. Minor: reference has `Save` inside the same block? Re-check —
reference `file_menu` (`lib.rs:254-258`) = `[new_terminal, new_ssh_tab,
new_sftp_tab, new_preview, new_editor, separator, close_tab, close_pane]` —
**there is no `Save` item in the reference File menu at all.** Rust **adds
`Save`** (`menu.rs:242`). Deviation — reference has no menu entry for Save (only
the `Cmd+S` accelerator, bound elsewhere). Recommendation: remove `Save` from
the File menu, or accept as a deliberate improvement and note it.

**Edit menu** — both use OS predefined Undo/Redo/Cut/Copy/Paste/Select All ✓.

**View menu** — `menu.rs:260-272` vs `lib.rs:278-284`: `Toggle Sidebar`,
`Toggle AI Panel`, ─, `Zoom In`, `Zoom Out`, `Reset Zoom`, ─, `Toggle Full
Screen`. Match ✓. Reference `fullscreen` is the predefined item (gets the
standard `Ctrl+Cmd+F`); Rust binds `ctrl-cmd-f` manually (`menu.rs:171`) ✓.

**Terminal menu** — `menu.rs:273-281` vs `lib.rs:290-294`: `Split Pane Right`,
`Split Pane Down`, ─, `Find…`. Match ✓.

**Connections menu** — `menu.rs:282-290` vs `lib.rs:300-304`: `Open Host
Manager`, ─, `New SSH Connection…`, `New Quick SSH…`. Match ✓ (Rust `NewQuickSsh`
has no handler yet — renders disabled, per the stub convention).

**AI menu** — `menu.rs:291-302` vs `lib.rs:312-318`: `Toggle AI Panel`, `New AI
Session`, `Ask about Selection`, ─, `Clear Current Chat`, ─, `AI Settings…`.
Match ✓.

**Window menu**
* Reference (`lib.rs:327-333`): `Minimize`, `Zoom`, ─, `Keyboard Shortcuts`,
  `Settings`, ─, `Next Tab`, `Previous Tab`.
* Rust (`menu.rs:303-316`): `Minimize`, `Zoom`, ─, **`Command Palette…`**,
  `Keyboard Shortcuts`, `Settings`, ─, `Next Tab`, `Previous Tab`.
* Delta: Rust **adds `Command Palette…`**. Reference exposes the palette only via
  `Cmd+K`-style shortcut, not a menu item. Recommendation: drop it from the menu
  (keep the shortcut) for parity, or flag as deliberate.

### 6c. Accelerators

Reference accelerators (`lib.rs`): `New Terminal Tab` `Cmd+T`, `New Preview Tab`
`Cmd+Shift+P`, `New Editor Tab` `Cmd+E`, `Close Tab` `Cmd+W`, `Close Pane`
`Cmd+Shift+W`, `Toggle Sidebar` `Cmd+B`, `Toggle AI Panel` `Cmd+I`, `Zoom In`
`Cmd+Plus`, `Zoom Out` `Cmd+-`, `Reset Zoom` `Cmd+0`, `Split Pane Right` `Cmd+D`,
`Split Pane Down` `Cmd+Shift+D`, `Find…` `Cmd+F`, `New SSH Connection…`
`Cmd+Shift+N`, `Ask about Selection` `Cmd+L`, `Keyboard Shortcuts` `Cmd+K`,
`Settings` `Cmd+,`, `Next Tab` `Ctrl+Tab`, `Previous Tab` `Ctrl+Shift+Tab`,
`new_preview` also `Cmd+Shift+P`.

Rust (`menu.rs:156-209`): fixed set (`cmd-s`, `cmd-shift-n`, `ctrl-cmd-f`,
`cmd-,`, `cmd-m`, `cmd-q`, `cmd-h`) + rebindable via `ShortcutId` defaults. Points
to verify against `command_palette::effective_binding` defaults:
* `ZoomIn/ZoomOut/ResetZoom` — bound only through `ShortcutId::ViewZoomIn/Out/
  Reset`; confirm their defaults are `cmd-+` / `cmd--` / `cmd-0`. If those
  `ShortcutId`s have empty defaults, the zoom items show no accelerator and the
  keys don't work.
* `OpenShortcuts` via `ShortcutId::ShortcutsOpen` — confirm default `cmd-k`
  (reference) not `cmd-?`.
* `Next Tab` / `Prev Tab` via `ShortcutId::TabNext/TabPrev` — confirm
  `ctrl-tab` / `ctrl-shift-tab`.
* `Toggle AI Panel` `Cmd+I` appears **twice** in the reference (View + AI menus,
  `toggle_ai` and `toggle_ai_2`, both `CmdOrCtrl+I`); Rust reuses the single
  `ToggleAiPanel` action in both menus (`menu.rs:264,294`) — cleaner, fine.
* Rust `NewSshConnection` bound `cmd-shift-n` fixed ✓.

Recommendation: audit `command_palette.rs` `ShortcutId` default table against
the reference accelerator list above; fix any mismatches so menu hints + keys
match the reference exactly.

### 6d. Dock menu

Reference (`dock_menu.rs:32-36`): `New Terminal Tab`, `New SSH Connection…`, ─,
`Open Host Manager` — **title-only items, no action wired** (the Tauri dock menu
just shows them). Rust (`menu.rs:322-329`): same 4 entries **with actions
wired** (`NewTerminalTab`, `NewSshConnection`, `OpenHostManager`). Functional
improvement; structurally matches. ✓ (test `menu.rs:374-377`).

### 6e. Menu-item enable/disable

Reference: `menu_sync.rs` only syncs *accelerators*; enable/disable is frontend
`setEnabled` calls. Rust: relies on GPUI `validate_menu_item` →
`is_action_available` against the focus dispatch tree; `AppShell` conditionally
registers `SplitPaneRight/Down` + `ClosePane` only when the active tab is a
splittable/split workspace (`app_shell.rs:1325-1331`). Sound approach. Items with
no handler (`NewSshTab`, `NewSftpTab`, `NewQuickSsh`, `ClearChat`, `NewAiSession`
if unhandled, zoom if unbound) render disabled — matches the "stub now, wire
later" note, but several of these features **do** exist now (SSH/SFTP tabs,
AI) and should be wired:
* `NewSshTab` / `NewSftpTab` — `Workspace` has `open_sftp_tab`
  (`app_shell.rs:701`) and SSH tab creation; wire handlers.
* `NewAiSession` / `ClearChat` — `AiChatView` exists (`ai_chat.rs`); wire.
* `OpenHostManager` — `HostManagerView` exists; wire (currently only a
  `CommandId` dispatch path).

---

## 7. Miscellaneous / smaller items

* **`ToggleAiPanel` action** (`menu.rs`) → in `run_palette_command` it maps to
  `select_panel(SidebarPanel::Ai)` (`app_shell.rs:747`) but there is **no
  `on_action` handler for `menu::ToggleAiPanel` on the AppShell root**
  (`app_shell.rs:1298-1324` list) — so the `Cmd+I` menu item / shortcut does
  nothing. Add `.on_action(cx.listener(Self::act_toggle_ai_panel))`.
* **`Find` accelerator** — `act_find` (`app_shell.rs:532`) first tries the
  editor's own find, else opens header search. Reference `Cmd+F` → terminal/
  editor search. OK.
* **`OpenShortcuts` / keymap** — reference App-menu `⋯` "Keymap" opens settings
  to the shortcuts page (`openSettingsWindow("shortcuts")`,
  `AppShell.tsx:112`). Rust has `menu::OpenShortcuts` → `ShortcutsDialog`
  equivalent; ensure the `⋯` dropdown (§1c) distinguishes "Keyboard Shortcuts"
  (cheatsheet) from "Keymap" (editable bindings).
* **`badgesAlwaysVisible` preference** — every reference badge self-hides when
  empty unless this pref is on (`Header.tsx:100-122`). Not ported; needed by the
  new bar-item renderer.
* **`AgentStatusPill` / `aiMini`** (`AiTools.tsx`, `renderBarItem.tsx:213-229`) —
  status pill + conversation toggle in the statusbar; Rust has none.
* **`previewUrl` chip** (`renderBarItem.tsx:191-211`) — "Open preview" chip with
  detected dev-server host; Rust has `detected_preview_url` plumbing in
  `AppShell` props? Not in statusbar. Needs the `previewUrl` bar item.
* **`cursorPosition`** (`renderBarItem.tsx:184-189`): `Ln {line}, Col {col}` when
  a file is open. Rust statusbar has nothing. Editor cursor store exists
  (`editor.rs`); expose and render as the `cursorPosition` bar item.
* **Statusbar height / typography** — reference `h-8` (32px) `text-[11px]`
  `px-3`; Rust `STATUS_H = 32.0` ✓, `px_3` ✓, text size via theme default (not
  pinned to 11px) — pin it.
* **`zen_mode_show_header` default** — `toggle_zen_pref` (`app_shell.rs:654-664`)
  defaults missing keys to `true`; reference `AppShellPrefs` defaults both to
  `true` ✓.

---

## 8. Prioritised fix list

**P0 — the visible breakage (symptoms 1 & 3)**
1. `crates/app/src/main.rs:70-74` — `appears_transparent: true`, drop the window
   title, set `traffic_light_position` to vertically-centre in the 40px header.
   Make `render_header` a window-drag region.
2. `crates/ui/src/app_shell.rs:859` — remove the static `"Labonair"` text node.
3. `crates/ui/src/app_shell.rs:1070-1097` + `:60` — delete the 44px activity
   `rail` and its `border_r` (the phantom left bar); replace the 6px solid
   resize `handle` (`:1117-1125`) with a 1px border + transparent grab zone.
4. `crates/ui/src/workspace.rs:2654-2701, 3133, 3155` — remove `render_tab_bar`
   from `Workspace`; render the tab strip inside `render_header` between the two
   bar buckets, `h-7`, no bottom border.

**P1 — the "+" dropdown & tab context menus (symptoms 4)**
5. `workspace.rs:2685-2700` — replace the bare `+` with the `NewTabDropdownItems`
   popover (Terminal ⌘T / Editor ⌘E / Preview ⌘P / Git Graph / ─ / SSH▶ / SFTP▶
   with recent-hosts submenus + "All hosts...").
6. `workspace.rs:2948-3053` — split into workspace vs non-workspace tab menus
   matching `WorkspaceTabContextMenuContent` / `NonWorkspaceTabContextMenuContent`
   (add Rename, Duplicate, Keep Tab Open, Close All, per-kind plural labels,
   `block_agent_access` disabled state, icons, separators).
7. Add inline tab rename (state on the tab strip; `TabStore::set_custom_title`).

**P2 — the unibar architecture (symptom 2)**
8. New `crates/ui/src/bar_items.rs`: `BarItemId` (15), `BarCategory`,
   `BarItemPlacement`, `default_placements()`, `visible_items_for`,
   `render_bar_item`, `build_bar_bucket` (+ divider rule), `bar_item_context_menu`
   (Side/Bar radios + Hide). Persist via the existing
   `settings_set_bar_item_placement` backend fn; add a `PreferencesStore`
   accessor + `badges_always_visible` / `tabs_location` prefs.
9. Rewrite `render_header` / `render_statusbar` to two buckets each; delete all
   hardcoded children. Port each item: `updater`, `notifications`, `jumpHosts`,
   `agentAccess` (already exists — fold in), `transfers`, `bookmarks`,
   `explorerPanel`/`snippetsPanel`/`sourceControlPanel`/`tabsPanel`,
   `cwdBreadcrumb` (§5), `cursorPosition`, `previewUrl`, `aiMini`, `aiPanel`.
10. Give the `⋯` header button its 4-item dropdown (Settings / Keyboard
    Shortcuts ⌘? / Keymap / Themes…), `Menu01Icon` glyph.

**P3 — dual-dock sidebar**
11. Honour each panel item's `side`; add a right dock slot beside the workspace.
12. Move Git Graph from a sidebar panel to a `TabKind::GitGraph` tab; keep AI as
    `aiPanel`(docked) + `aiMini`, not a sidebar panel.
13. Real icons + reference titles/active styling for the panel toggles;
    per-slot width persistence.

**P4 — native menu parity**
14. `menu.rs:215-232` — drop extra App-menu separators; decide on `Check for
    Updates…` / `Services` (flag as deliberate additions or remove).
15. `menu.rs:242` — remove `Save` from the File menu (reference has no such
    item) or flag as deliberate.
16. `menu.rs:309` — remove `Command Palette…` from the Window menu (keep the
    shortcut).
17. Audit `command_palette.rs` `ShortcutId` default bindings against §6c
    (zoom = `cmd-+`/`cmd--`/`cmd-0`, shortcuts = `cmd-k`, tab nav =
    `ctrl-tab`/`ctrl-shift-tab`, etc.).
18. `app_shell.rs` — add `on_action` for `menu::ToggleAiPanel` (currently
    unhandled); wire handlers for `NewSshTab`, `NewSftpTab`, `NewAiSession`,
    `ClearChat`, `OpenHostManager` so those menu items stop rendering disabled.

**P5 — polish**
19. Tab visuals: SSH-vs-local icon, file-type icons, remote-sync-failed dot,
    double-click peek promotion, wheel-scroll + scroll-active-into-view.
20. Pin statusbar text to 11px; add the `IS_MAC` post-traffic-light divider in
    the header; `badges_always_visible` behaviour.
