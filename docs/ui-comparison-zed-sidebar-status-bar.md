# Labonair vs. Zed: Sidebar, Git Panel, and Status Bar

**Source-level UI comparison and clean-room implementation blueprint**

**Date:** 2026-09-05

**Labonair baseline:** the current working tree on `master`

**Zed baseline:** local source snapshot `zed-refrence/zed` at commit `3569541038` (2026-09-04)

## 1. Purpose and conclusion

This report compares the current native Labonair implementation with Zed's current GPUI implementation in three tightly connected areas:

1. the shared dock/sidebar shell;
2. the Explorer and Source Control panels;
3. the bottom status bar used to open, switch, move, and close panels.

The comparison is based primarily on code, not screenshots. Visual behavior is traced from the concrete render trees, state models, sizing rules, interaction handlers, settings, and list virtualization paths in both repositories. Zed's official documentation is used only to confirm user-facing behavior that is distributed across several source files.

The central finding is straightforward:

> Zed does not look substantially better because it uses a radically different color palette. It looks better because it spends less permanent chrome on controls, maps controls spatially to the dock they affect, uses purpose-built dense rows, reveals secondary actions only in context, and makes selection, focus, staging, diagnostics, and active-dock state unambiguous.

Labonair already has much of the necessary architecture: a panel registry, three docks, movable panels, persisted layout, status items, GPUI rendering, and panel-specific view models. The largest gap is the presentation layer. Labonair's current native UI deliberately preserves the frozen web reference's visual structure, while Zed's UI lets each panel own its meaningful chrome and keeps the shell almost invisible.

The recommended path is therefore not a wholesale replacement. It is a staged clean-room reimplementation of Zed's interaction patterns on Labonair's existing panel and workspace APIs:

- replace the global panel-toggle strip with one status-bar button group per dock;
- remove the generic dock title row;
- introduce purpose-built dense tree/change rows instead of stretching the generic `ListItem`;
- virtualize Explorer and Source Control rows;
- move secondary operations into contextual menus and split buttons;
- move diffs into the main workspace instead of a narrow inline panel;
- add Zed-style state, keyboard, accessibility, and feedback semantics;
- preserve Labonair-specific features such as SFTP, snippets, Git Graph, AI, and the three-dock personalization model.

## 2. Important implementation boundary: Zed's license

The relevant Zed crates declare `GPL-3.0-or-later`:

- `zed-refrence/zed/crates/workspace/Cargo.toml`
- `zed-refrence/zed/crates/project_panel/Cargo.toml`
- `zed-refrence/zed/crates/git_ui/Cargo.toml`
- `zed-refrence/zed/crates/ui/Cargo.toml`

Labonair's root `LICENSE` is Apache-2.0. Directly copying these implementations, or producing a close line-by-line translation, would create a licensing problem unless the project deliberately changes its licensing strategy. This report therefore distinguishes three categories:

| Category | Meaning | Safe default |
|---|---|---|
| **Behavioral pattern** | Observable interaction, layout rule, information hierarchy, or accessibility outcome | Reimplement independently |
| **Architectural idea** | Per-dock button groups, virtualized lists, panel-owned chrome, contextual actions | Recreate on Labonair APIs |
| **Zed expression/code** | Function bodies, struct layouts, exact algorithms, comments, or source-specific organization | Do not copy into the Apache-2.0 codebase |

This is an engineering boundary, not legal advice. The implementation plan below is intentionally phrased as a clean-room behavioral specification.

## 3. Sources inspected

### 3.1 Labonair

The highest-value Labonair sources are:

- [`docs/architecture.md`](architecture.md): normative shell, dock, titlebar, and status-bar contract.
- [`crates/workspace/src/status_bar.rs`](../crates/workspace/src/status_bar.rs): status-bar registry, layout, grouping, and item placement.
- [`crates/shell/src/status_items.rs`](../crates/shell/src/status_items.rs): the aggregate panel-toggle strip and built-in status items.
- [`crates/workspace/src/dock.rs`](../crates/workspace/src/dock.rs): dock state, size constraints, activation, movement, and persistence.
- [`crates/workspace/src/workspace.rs`](../crates/workspace/src/workspace.rs): shared dock chrome and resize-handle rendering.
- [`crates/ui-kit/src/list.rs`](../crates/ui-kit/src/list.rs): generic list-row primitive used by both panels.
- [`crates/ui-kit/src/toggle.rs`](../crates/ui-kit/src/toggle.rs): 20 px status-bar toggle primitive.
- [`crates/panel-explorer/src/panel_explorer.rs`](../crates/panel-explorer/src/panel_explorer.rs): Explorer model, toolbar, search, tree rows, selection, drag/drop, and context menu.
- [`crates/panel-scm/src/panel_scm.rs`](../crates/panel-scm/src/panel_scm.rs): Source Control toolbar, sections, file rows, diff, branch controls, stashes, and commit form.
- [`reference-src/src/modules/statusbar/StatusBar.tsx`](../reference-src/src/modules/statusbar/StatusBar.tsx), [`renderBarItem.tsx`](../reference-src/src/modules/statusbar/lib/renderBarItem.tsx), [`FileExplorer.tsx`](../reference-src/src/modules/explorer/FileExplorer.tsx), [`FileTreeNode.tsx`](../reference-src/src/modules/explorer/FileTreeNode.tsx), and [`VirtualizedTreeList.tsx`](../reference-src/src/modules/explorer/components/VirtualizedTreeList.tsx): the frozen web UI that explains many current presentation choices.

### 3.2 Zed

The highest-value Zed sources are:

- [`workspace/src/status_bar.rs`](../zed-refrence/zed/crates/workspace/src/status_bar.rs): status-bar layout, keyboard focus, accessibility, and hideable items.
- [`workspace/src/dock.rs`](../zed-refrence/zed/crates/workspace/src/dock.rs): dock renderer and per-dock `PanelButtons`.
- [`project_panel/src/project_panel.rs`](../zed-refrence/zed/crates/project_panel/src/project_panel.rs): virtualized tree, row states, decorations, sticky ancestors, and interaction behavior.
- [`project_panel/src/project_panel_settings.rs`](../zed-refrence/zed/crates/project_panel/src/project_panel_settings.rs): user-adjustable density, indentation, icons, Git status, diagnostics, sticky scroll, reveal, folding, and sorting.
- [`git_ui/src/git_panel.rs`](../zed-refrence/zed/crates/git_ui/src/git_panel.rs): Changes/History navigation, action hierarchy, change list, diff entry point, repository footer, and commit editor.
- [`git_ui/src/git_panel_settings.rs`](../zed-refrence/zed/crates/git_ui/src/git_panel_settings.rs): Git panel view and behavior settings.
- [`ui/src/components/list/list_item.rs`](../zed-refrence/zed/crates/ui/src/components/list/list_item.rs): density-aware semantic list rows.
- [`ui/src/components/button/icon_button.rs`](../zed-refrence/zed/crates/ui/src/components/button/icon_button.rs), [`button_like.rs`](../zed-refrence/zed/crates/ui/src/components/button/button_like.rs), and [`styles/spacing.rs`](../zed-refrence/zed/crates/ui/src/styles/spacing.rs): the shared sizing, state, spacing, and accessibility foundations.

User-facing behavior is corroborated by Zed's official [Project Panel](https://zed.dev/docs/project-panel), [Git](https://zed.dev/docs/git), [All Settings](https://zed.dev/docs/reference/all-settings), [Visual Customization](https://zed.dev/docs/visual-customization), [panel-system article](https://zed.dev/blog/new-panel-system), and [workspace glossary](https://zed.dev/docs/development/glossary).

### 3.3 Exact code comparison map

These anchors are the shortest route from a report finding to the two implementations:

| Concern | Labonair anchor | Zed anchor |
|---|---|---|
| Status-bar root | [`StatusBar::render`, line 260](../crates/workspace/src/status_bar.rs#L260) | [`StatusBar::render`, line 115](../zed-refrence/zed/crates/workspace/src/status_bar.rs#L115) |
| Panel-button group | [`PanelTogglesStatusItem`, line 213](../crates/shell/src/status_items.rs#L213) | [`PanelButtons`, line 1390](../zed-refrence/zed/crates/workspace/src/dock.rs#L1390) |
| Dock surface/resize | [`Workspace::render_dock`, line 4702](../crates/workspace/src/workspace.rs#L4702) | [`Dock::render`, line 1270](../zed-refrence/zed/crates/workspace/src/dock.rs#L1270) |
| General list row | [`ListItem::into_element`, line 135](../crates/ui-kit/src/list.rs#L135) | [`ListItem`, line 32](../zed-refrence/zed/crates/ui/src/components/list/list_item.rs#L32) |
| Explorer/Project Panel root | [`ExplorerView::render`, line 1004](../crates/panel-explorer/src/panel_explorer.rs#L1004) | [`ProjectPanel::render`, line 7120](../zed-refrence/zed/crates/project_panel/src/project_panel.rs#L7120) |
| Explorer/Project entry | [`ExplorerView::render_row`, line 1408](../crates/panel-explorer/src/panel_explorer.rs#L1408) | [`ProjectPanel::render_entry`, line 5741](../zed-refrence/zed/crates/project_panel/src/project_panel.rs#L5741) |
| Explorer virtualization | no virtual list in the current Explorer root | [`uniform_list`, line 7315](../zed-refrence/zed/crates/project_panel/src/project_panel.rs#L7315) |
| Explorer settings | scattered current settings and fixed values | [`ProjectPanelSettings`, line 13](../zed-refrence/zed/crates/project_panel/src/project_panel_settings.rs#L13) |
| Git panel root | [`GitPanelView::render`, line 2941](../crates/panel-scm/src/panel_scm.rs#L2941) | [`GitPanel::render`, line 8928](../zed-refrence/zed/crates/git_ui/src/git_panel.rs#L8928) |
| Git header | action bar at [`GitPanelView::render`, line 2977](../crates/panel-scm/src/panel_scm.rs#L2977) | [`render_changes_header`, line 6390](../zed-refrence/zed/crates/git_ui/src/git_panel.rs#L6390) |
| Git change rows | [`render_section`, line 1533](../crates/panel-scm/src/panel_scm.rs#L1533) | virtualized entries around [`uniform_list`, line 7760](../zed-refrence/zed/crates/git_ui/src/git_panel.rs#L7760) |
| Git diff | [`render_diff`, line 1701](../crates/panel-scm/src/panel_scm.rs#L1701) | Project Diff flow documented and dispatched from Git UI rather than embedded in the panel |
| Git repository footer | [`render_branch_bar`, line 1852](../crates/panel-scm/src/panel_scm.rs#L1852) | [`render_footer`, line 6485](../zed-refrence/zed/crates/git_ui/src/git_panel.rs#L6485) |
| Commit composer/action | [`render_commit_form`, line 1962](../crates/panel-scm/src/panel_scm.rs#L1962) | [`render_commit_button`, line 6667](../zed-refrence/zed/crates/git_ui/src/git_panel.rs#L6667) |
| Git settings | current behavior is mostly fixed in the panel | [`GitPanelSettings`, line 18](../zed-refrence/zed/crates/git_ui/src/git_panel_settings.rs#L18) |

## 4. Structural anatomy

The most consequential difference is visible before either panel's domain-specific content is considered.

### 4.1 Current Labonair anatomy

```text
┌───────────────────────────────────────────────────────────────┐
│ Titlebar: tabs                                      menu      │
├──────────────┬────────────────────────────────────────────────┤
│ EXPLORER   ⇄ │                                                │ ← generic dock header
├──────────────┤                                                │
│ root   ⌕ + … │                                                │ ← Explorer toolbar
├──────────────┤                   workspace                    │
│ search box   │                                                │
│ file rows    │                                                │
│              │                                                │
├──────────────┴────────────────────────────────────────────────┤
│ [Exp][SCM][Graph][Snip][AI]      branch · agent · notifications│ ← one global group
└───────────────────────────────────────────────────────────────┘
```

The shell adds a generic uppercase title row and move glyph to every open dock. Explorer then adds another root/action toolbar immediately below it. Source Control adds its own permanent action toolbar. The result is stacked chrome before the user reaches content.

### 4.2 Zed anatomy

```text
┌───────────────────────────────────────────────────────────────┐
│ Titlebar / tabs                                               │
├──────────────┬────────────────────────────────────────────────┤
│ file rows    │                                                │ ← panel begins with content
│ sticky path  │                   workspace                    │
│ file rows    │                                                │
│              │                                                │
├──────────────┴────────────────────────────────────────────────┤
│ [left-dock buttons]  status items    [bottom][right buttons]  │ ← spatially mapped groups
└───────────────────────────────────────────────────────────────┘
```

Zed's `Dock::render` supplies background, a one-pixel boundary, focus context, and an overlaid resize hit target. It does not inject a generic panel title bar. The active panel owns all meaningful local navigation and controls. The status bar renders a separate `PanelButtons` view for each dock, so buttons visually remain attached to the side they control.

### 4.3 Why this matters

This is not merely a few pixels of vertical space. The two structures express different mental models:

- Labonair says: “These are globally available application modules; choose one from a central launcher, then manage the dock separately.”
- Zed says: “This group is the left dock, that group is the right or bottom dock; click the thing exactly where its result will appear.”

Zed therefore reduces the cognitive step between control and consequence. Its panel buttons act like direct manipulation of the workspace edges rather than navigation through a global feature list.

## 5. Executive comparison matrix

| Area | Current Labonair | Zed | Design consequence | Priority |
|---|---|---|---|---|
| Panel toggles | One fixed-left aggregate group for all registered panels | One button group per dock, positioned near that dock's edge | Zed communicates panel destination before click | P0 |
| Active panel click | Closes the active dock through shared toggle semantics | Explicitly dispatches the dock's close action | Similar outcome; Zed's ownership is clearer | P1 |
| Panel movement | Context menu plus a generic `⇄`/`↑` dock-header glyph | Context menu on the exact panel button | Zed removes permanent, ambiguous chrome | P0 |
| Dock chrome | Generic uppercase title row above every panel | No generic title row | Zed gives more room to content and avoids duplicated titles | P0 |
| Resize handle | 6 px sibling consuming layout space, visible border, accent hover fill | 6 px absolute hit target over a 1 px boundary | Zed feels structurally lighter; hit area remains usable | P1 |
| Status-bar height | Fixed 32 px; 20 px controls; 12 px horizontal padding | Approximately 30 px at default density: 22 px controls plus 4 px outer padding | Height is not the main difference | P2 |
| Status-bar spacing | Fixed GPUI utility spacing | Density-aware `DynamicSpacing` | Zed scales coherently with UI density | P2 |
| Accessibility | No toolbar role, tab group, arrow-key focus loop, or button ARIA labels in the shared bar | Toolbar semantics, tab group, left/right key navigation, labelled buttons | Zed remains understandable without a mouse | P0 |
| Explorer top chrome | Dock header + root toolbar + optional search strip | Tree begins immediately; actions are command/context driven | Zed's content-to-chrome ratio is higher | P0 |
| Explorer row primitive | Generic rounded list item, 8 px gap, 8 px horizontal and 6 px vertical padding | Purpose-built flat tree row with dense spacing and explicit focus/mark states | Zed reads as a continuous hierarchy, not a stack of cards | P0 |
| Explorer rendering | Materializes and renders every visible expanded row | `uniform_list` virtualization | Zed remains stable on very large projects | P0 |
| Explorer context | Manual expansion and simple loaded-row filtering | Sticky ancestors, indent guides, auto-fold chains, auto-reveal | Zed preserves location in deep trees | P1 |
| Explorer metadata | File/folder icon and selected/drop state | Git status, diagnostic badge/count, file icon, folder icon, marked/selected/focused states | Zed conveys more without extra columns | P1 |
| Explorer selection | Multi-select implemented; preview/open behavior is more limited | Marked selection, preview/permanent open, compare two files, keyboard-first range handling | Zed separates navigation from committed opening | P1 |
| Git top chrome | Permanent text actions: Refresh, Stage/Unstage All, Discard, Clean | Changes/History tabs; View Diff; view options; adaptive stage split button | Zed establishes one primary task and hides hazards | P0 |
| Git change row | Status letter, shortened path, permanent discard and stage controls | Checkbox staging, file/folder identity, semantic status, tree/flat grouping, contextual actions | Zed makes staging a direct state, not a command hunt | P0 |
| Git list rendering | All section rows rendered in scroll container | Virtualized list | Better scale and steadier interaction | P0 |
| Diff | Narrow inline viewer capped at 280 px | Opens a full Project Diff multibuffer in the main workspace | Zed uses the correct surface for code review | P0 |
| Commit form | 48 px hand-rolled field plus full-width 32 px button | Integrated editor/footer, validation, expand/modal, AI/co-author actions, adaptive split commit button | Zed is compact yet more capable | P1 |
| Branch/remotes | Bottom strip with branch, counts, and Fetch/Pull/Push/Force text buttons | Repository footer and menus, state-aware operations | Zed lowers permanent command noise | P1 |
| Feature exposure | Most capabilities are visible simultaneously | Progressive disclosure by frequency, safety, and current state | This is the largest perceived-polish difference | P0 |

## 6. Status bar and panel switching

### 6.1 Current Labonair implementation

`crates/workspace/src/status_bar.rs` fixes the row to 32 px, adds 12 px horizontal padding (`px_3`), uses 11 px text, and separates left and right item registries. This is a sound base. The registry also supports ordering, logical group dividers, persisted side placement, and hideable status items.

The weakness is the special case around panel buttons:

- `"panel-toggles"` is explicitly fixed to the left and cannot be moved or hidden as a unit.
- `PanelTogglesStatusItem` iterates over every registered panel, independent of its current dock.
- each button is a 20 × 20 px rounded toggle with a 16 px icon;
- active state uses `muted_bg`, while inactive state uses muted foreground;
- right-clicking a button offers dock movement and visibility;
- only Explorer and AI currently expose a dedicated shortcut in their tooltip; Source Control, Git Graph, and Snippets only expose their title.

The dock state already has the correct click semantics: clicking the open active panel closes its dock; clicking another panel activates it and opens the dock. The issue is not behavior but representation.

### 6.2 Zed implementation

Zed's status bar is a toolbar and tab group. Unmodified Left and Right Arrow keys move focus across items. Icon buttons have tab indices and accessible labels. At default density, the root uses 4 px outer padding and a density-aware 8 px separation between the left and right regions. The default icon button is 22 px tall, while the panel icon is 14 px.

The decisive component is `PanelButtons` in `workspace/src/dock.rs`:

1. It is constructed with one specific `Dock` entity.
2. It only renders panels belonging to that dock.
3. It reads that dock's open state, active index, and position.
4. It derives the active button from both active index and dock visibility.
5. It reverses button order for the right dock, keeping the active edge visually nearest the corresponding workspace boundary.
6. It inserts a divider on the interior side of the group: after a left-dock group, before right/bottom groups.
7. It can show a numeric count badge on an inactive panel icon.
8. Its context menu includes only positions the panel actually supports, plus flexible/fixed sizing and button visibility where applicable.
9. Clicking the active button dispatches “Close [side] Dock”; clicking an inactive button dispatches the panel's own toggle action.

This is a compact visual grammar. Group position, order, divider direction, toggled state, tooltip, and count badge all communicate workspace state without a textual title row.

### 6.3 Exact design differences

#### Spatial correspondence

Labonair's five adjacent icons may control three different physical destinations. A Source Control icon can remain in the fixed-left strip even after Source Control has been moved to the right dock. The user must remember configuration state.

Zed's button physically moves with its dock group. The UI shows the relationship continuously. This follows the proximity principle: controls that operate on the same surface are grouped with that surface.

#### Active-state meaning

In Labonair the pressed fill identifies an active panel, but the cluster itself does not encode which dock is open. In Zed the toggled button exists inside a dock-specific group, so one visual state communicates both active panel and open destination.

#### Dividers as structure

Labonair inserts dividers between status-item registry groups but treats all panel toggles as one group. Zed uses dividers to mark dock boundaries. The divider therefore describes the workspace model instead of merely decorating unrelated status items.

#### Density system

Labonair's 32/20/16/11 px status-bar scale is coherent internally, but fixed. Zed resolves padding and control sizing from UI density and rem scale. This makes compact/default/comfortable modes feel deliberately designed rather than globally zoomed.

#### Accessibility and keyboard focus

Zed's status bar is a named toolbar and tab group; its buttons are labelled and keyboard reachable. Labonair currently relies on click handlers and hover tooltips. The visual result also benefits sighted users: a true focus state forces the component system to distinguish hover, active press, selected/toggled, and keyboard focus rather than collapsing them into one background fill.

### 6.4 Recommended clean-room target

Introduce a Labonair-owned `DockPanelButtons` status item with one instance for each dock. It should consume the existing `Dock` state and panel registry rather than duplicating it.

Required behavior:

- left-dock group at the left status-bar edge;
- bottom- and right-dock groups at the right edge, separated from informational items;
- reverse the right-dock button sequence so the visual edge remains stable;
- render only panels currently assigned to that dock;
- active means `dock.open && dock.active_name == panel.name`;
- click active: close that dock;
- click inactive: activate panel and open its dock;
- right-click: valid dock positions, button visibility, and any supported sizing mode;
- optional count badge through a new panel metadata callback;
- tooltip for every button, including resolved shortcut;
- toolbar role, button role, accessible label, toggled state, tab index, and left/right focus movement;
- a one-pixel divider on the group's workspace-facing edge.

The current aggregate `PanelTogglesStatusItem` should then be removed. The status-item registry remains valuable for non-panel data such as branch, host, agent, updates, and notifications.

## 7. Shared dock chrome

### 7.1 Current Labonair implementation

`Workspace::render_dock` adds a shell-owned header containing:

- the active panel title, converted to uppercase;
- a `⇄` glyph for side docks or `↑` for the bottom dock;
- a click action that moves the panel to the next dock;
- horizontal 12 px and vertical 8 px padding;
- extra muted text and hover behavior.

It then inserts a separate resize-handle element beside the panel. The handle is 6 px wide or tall and contains the visible one-pixel border. Because it is a flex sibling, the interaction gutter consumes layout space.

### 7.2 Zed implementation

Zed's dock renderer is intentionally minimal:

- panel background;
- one-pixel edge border;
- clipping, focus context, and correct axis sizing;
- active panel body;
- a 6 px absolute resize target centered over the boundary;
- double-click on the resize target resets panel sizes.

There is no universal panel title and no permanent move glyph. Panel movement belongs to the corresponding status-bar button's context menu. This reduces chrome without removing functionality.

### 7.3 Why Labonair's current header hurts perceived quality

The header has three distinct problems:

1. **Duplication:** Explorer immediately repeats location and actions in its own toolbar; Source Control immediately follows with a second action row.
2. **Weak semantics:** `⇄` does not name the destination and `↑` describes direction rather than the resulting dock. A menu labelled “Dock Left/Right/Bottom” is more explicit.
3. **Permanent cost for an infrequent action:** moving a panel is personalization, not the primary task. It should not occupy every open panel at all times.

### 7.4 Recommended clean-room target

- Remove the shared title/move row from `Workspace::render_dock`.
- Make the panel body fill the entire dock.
- Render a one-pixel structural border on the dock container.
- Position a transparent 6 px resize hit target absolutely over that border.
- Preserve drag resizing and add double-click reset.
- Expose panel movement only through its status-bar button menu and personalization settings.
- Allow individual panels to render a title or tab bar only where that title distinguishes meaningful modes, such as Git “Changes” versus “History”.

## 8. Explorer / Project Panel

### 8.1 Current Labonair Explorer anatomy

The Explorer render tree is:

```text
generic dock title
└── Explorer root/action toolbar
    ├── root folder icon and root name
    └── search, new file, new folder, refresh, hidden-files buttons
optional bordered search strip
scroll container
└── every expanded/filtered row rendered as a GPUI element
```

The panel already supports a substantial behavior set:

- local and remote roots;
- lazy directory loading;
- expansion and collapse;
- command/shift multi-selection;
- keyboard navigation;
- preview/open behavior;
- internal and external drag/drop;
- cut/copy/paste;
- create, rename, delete, duplicate, bookmark;
- open in terminal and Finder;
- hidden-file filtering;
- context menus and confirmation flows.

The shortcomings are concentrated in rendering, hierarchy, and contextual intelligence rather than basic file operations.

### 8.2 Explorer row geometry

Explorer uses the shared `ui-kit::ListItem`. That primitive applies:

- 8 px horizontal padding;
- 6 px vertical padding;
- an 8 px child gap (`gap_2`);
- 12 px text;
- rounded corners;
- the same selected fill for both selection and unselected hover.

Explorer overrides indentation, right padding, and some content styling, but inherits the generic row's vertical padding and rounded-card treatment. This is appropriate for command results and settings rows, but not for a dense hierarchical tree.

The visual effect is subtle but important:

- each file reads as an isolated clickable control;
- vertical rhythm becomes loose;
- rounded hover/selection islands break the continuous tree silhouette;
- icon-to-label distance is larger than necessary;
- depth is expressed mainly by left padding, without persistent structural guides;
- selection, keyboard focus, multi-selection marks, and drop target have too few independent visual channels.

### 8.3 Zed Project Panel anatomy

Zed's Project Panel begins with the tree itself. Its row implementation is panel-specific and composes shared primitives without accepting generic-list defaults blindly.

Key characteristics:

- flat full-width rows (`rounded_none`);
- approximately 24 px label-row height;
- density-adjustable entry spacing;
- configurable indent width and optional indent guides;
- distinct states for hovered, selected, marked, keyboard focused, dragged, and drop target;
- a narrow focus/selection accent at the row edge in addition to background state;
- file and folder icons controlled independently;
- Git status tint or icon decoration;
- diagnostic severity icons and counts;
- compact disclosure chevrons aligned to hierarchy;
- `uniform_list` virtualization;
- sticky ancestor rows with a subtle gradient/shadow boundary;
- optional auto-folding of single-child directory chains;
- optional auto-reveal of the active editor file;
- natural/path-aware sorting and configurable directory grouping.

Official behavior also includes single-click/Space preview, double-click or middle-click permanent opening, range selection, comparing two marked files, drag-with-modifier copy, trash versus permanent deletion, and undo/redo of file operations.

### 8.4 Detailed difference table

| Concern | Labonair | Zed | Perceived result |
|---|---|---|---|
| First visible row | Generic dock title, then root toolbar | File hierarchy | Zed reaches user content immediately |
| Actions | Five permanent toolbar icons | Commands, context actions, and situational controls | Zed stays calm when no action is needed |
| Search | Dedicated bordered strip; filters currently loaded rows | Project search/command flows and rich tree navigation | Labonair search looks heavier while being narrower in scope |
| Row surface | Rounded generic list item | Flat continuous tree row | Zed feels like one hierarchy |
| Row density | Generic 6 px vertical padding around 12 px text | Purpose-built dense row, configurable spacing | Zed displays more context without appearing cramped |
| Indentation | Fixed depth padding | Configurable indentation and guides | Zed makes deep ancestry easier to scan |
| Deep navigation | Scroll position plus expansion state | Sticky ancestors and auto-folded chains | Zed reduces “where am I?” moments |
| Active file | No equivalent fully integrated auto-reveal path | Optional auto-reveal | Workspace and tree stay synchronized |
| Metadata | Core file/folder identity | Git and diagnostic status layered onto identity | Zed avoids separate panels for basic awareness |
| Large trees | Renders all current rows | Virtualized | Zed keeps frame and input cost bounded |
| Selection model | Selected rows and drop highlights | Selected, marked, focused, previewed, active editor, drag/drop states | Zed gives each interaction a legible state |
| Open model | Open/preview logic exists but is less explicit in the surface | Preview versus permanent open is a core interaction contract | Zed supports fast browsing without tab clutter |
| Settings | Hidden toggle and core behavior | Density, icons, Git status, diagnostics, indentation, guides, sticky scroll, sorting, reveal/fold, scrollbar, root/hidden visibility | Zed lets density and information load fit the user |

### 8.5 What specifically makes Zed's Explorer look better

#### Content-first composition

Removing two stacked headers does more than reclaim height. It stops the user's eye at the actual project hierarchy instead of a sequence of containers and controls.

#### Continuous rather than card-like rows

A project tree is a spatial document. Full-width flat rows preserve continuity. Rounded row islands visually overstate every file as an independent button, which creates noise at scale.

#### Multiple low-amplitude state channels

Zed avoids making one strong fill carry every meaning. Background shade, foreground tint, icon decoration, a narrow edge indicator, disclosure state, and badge placement can each communicate one thing. The result contains more information but feels quieter.

#### Structural memory

Sticky ancestors, indentation guides, auto-folding, and auto-reveal reduce the user's memory burden. Perceived polish often comes from not needing to reconstruct context after scrolling or switching editors.

#### Bounded rendering cost

Virtualization is visible as design quality: scrolling remains smooth, hover feedback remains immediate, and selection does not lag in large repositories. Performance is part of the interface, not merely an implementation metric.

### 8.6 Recommended Labonair Explorer target

Create a dedicated `TreeRow`/`ProjectEntryRow` in the Explorer crate or a narrowly scoped tree component in `ui-kit`. Do not globally change `ListItem`, because command palettes, settings, and other list surfaces may depend on its current card-like spacing.

The target row should provide:

- fixed 24 px standard height, with compact and comfortable variants derived from theme density;
- flat full-width surface and no per-row corner radius;
- 4 px internal icon/label gaps;
- 12–14 px disclosure and file icons;
- configurable indent step, initially preserving the current reference value until settings are added;
- independent visual inputs for selected, marked, focused, active-file, cut, drag source, and drop target;
- right-edge focus/active indicator no wider than 2 px;
- optional Git and diagnostic decorations;
- trailing actions hidden until hover/focus unless persistent status is essential;
- ellipsis based on available width, with the full path in the tooltip/accessibility label.

Then restructure the panel:

1. remove the generic dock header;
2. collapse the Explorer's permanent root toolbar into either a compact root row or an overflow/context menu;
3. keep search available through a shortcut and a transient in-panel filter row;
4. retain one discoverable search button and one overflow button if needed, rather than five simultaneous icons;
5. replace the eager row `.map(render_row)` path with a GPUI uniform/virtualized list owned by Labonair;
6. cache flattened visible entries and update only affected subtrees;
7. add sticky ancestor presentation over the virtual list;
8. add optional indent guides;
9. add active-editor auto-reveal and single-child-chain folding as settings;
10. make search traverse the model/filesystem rather than only the subset already loaded by lazy expansion.

Labonair-specific local/remote behavior must remain explicit. Remote/SFTP loading, latency, errors, and unavailable metadata should use the same row geometry but may show a small state indicator or skeleton; they must not block the GPUI thread.

## 9. Source Control / Git panel

### 9.1 Current Labonair Source Control anatomy

The current panel stacks many capabilities vertically:

```text
generic dock title
└── 28 px action bar
    ├── Refresh
    ├── Stage All / Unstage All
    ├── Discard
    └── Clean
poll error, if present
stash panel, if open
inline diff, if selected (up to 280 px)
scrolling change sections
branch picker, if open
28 px branch/remotes bar
└── branch · ahead/behind · Fetch · Pull · Push · Force
commit form
└── 48 px message field + 32 px full-width Commit button
```

Change sections use 22 px uppercase headers with counts. File rows are also forced to 22 px and show a status letter, a shortened path, and permanent trailing discard/stage controls. The panel supports a broad feature set: staging, unstaging, discard, clean, diff modes, branches, fetch/pull/push/force-push, stashes, tags, and commit creation.

The problem is not missing functionality. It is that frequency, danger, and context are not reflected in the visual hierarchy. Refresh, routine staging, destructive discard/clean, and force push all receive similar permanent text-button treatment.

### 9.2 Zed Git Panel anatomy

Zed structures the panel around modes and the current task:

```text
Changes | History tabs
contextual Changes header
├── View Diff + optional aggregate stats
├── view/filter options
└── Stage All / Unstage All split action + overflow
virtualized tree/flat change list
└── checkbox-driven stage state and contextual file actions
repository footer
commit editor
└── validation, expand/modal, optional AI/co-authors, adaptive commit action
```

The Project Diff opens in the main workspace as an editable multibuffer rather than being constrained to the narrow side panel. Hunk staging belongs to that diff surface. The Git panel remains an overview and control surface.

### 9.3 Detailed difference table

| Concern | Labonair | Zed | Design effect |
|---|---|---|---|
| Top-level modes | One long surface | Changes and History tabs | Zed gives the panel an explicit information architecture |
| Primary action | Several equally weighted text actions | “View Diff” plus context-sensitive stage split action | Zed directs attention to review and current stage state |
| Secondary actions | Permanently visible | View/options/overflow menus | Zed reduces command noise |
| Destructive actions | Discard and Clean in the primary bar | Contextual/overflow placement with validation | Zed better matches visual weight to risk |
| Staging model | Small plus/minus action per row | Checkbox/state control at file and section level | Zed makes stage state directly readable and editable |
| File identity | Status letter plus last path segments | File/folder icons, tree or flat mode, semantic status style | Zed is faster to scan |
| Organization | Fixed status sections | Configurable status/path sorting and grouping; tree/flat view | Zed adapts to repository size and user workflow |
| Aggregate change data | Limited | Optional diff statistics in the header and rows | Zed communicates review size before opening a diff |
| Diff surface | Inline, max 280 px high inside a side panel | Full Project Diff in workspace | Zed aligns code review with editor geometry |
| List cost | Eager row rendering | Virtualized list | Zed scales to large change sets |
| Branch/remotes | Dense permanent bottom command row | Repository footer and contextual operations | Zed preserves focus on changes |
| Commit message | Hand-rolled compact input | Editor-backed composer with expansion/modal path | Zed supports real commit-message editing without always taking space |
| Commit action | Fixed “Commit” | Commit, Commit Tracked, Amend, disabled reason, and split options | Zed explains what will happen before click |
| Empty state | Functional but command-oriented | State-aware guidance and disabled reasons | Zed feels intentional when nothing is actionable |

### 9.4 What specifically makes Zed's Git UI look better

#### A task hierarchy instead of a command inventory

Zed visually prioritizes the normal loop: inspect changes, stage the intended scope, write a message, commit. Rare, dangerous, or view-configuration actions move to menus. Labonair currently exposes its command surface almost as an API list.

#### State controls instead of action controls

A checkbox communicates both current staging state and the action available on click. Separate plus/minus icons require the user to infer state from section membership, locate a small trailing target, and translate the symbol into an operation.

#### Correct surface for the content

Diffs are editor content. A 280 px-tall inline block inside a 300 px-wide panel creates nested scrolling, cramped code, and competing focus. Zed's Project Diff occupies the workspace, so normal editor navigation, selection, and hunk actions remain coherent.

#### Adaptive action labels

“Commit,” “Commit Tracked,” and “Amend” communicate scope and mode. Disabled tooltips explain why an action is unavailable. The UI prevents uncertainty rather than merely disabling a generic button.

#### Progressive disclosure with retained discoverability

Zed does not remove advanced capabilities. It groups them behind view controls, split-button menus, context menus, and expandable editors. The common path stays visually short, while advanced paths remain adjacent to the object they affect.

### 9.5 Recommended Labonair Git target

#### Panel navigation

- Remove the generic dock header.
- Add a compact panel-owned tab bar with `Changes` and `History`.
- Keep the tab bar only because it separates two meaningful information modes; do not use it as decoration.

#### Changes header

- Primary left action: `View Diff`, optionally followed by aggregate additions/deletions.
- Secondary view button: grouping, sorting, tree/flat mode, status presentation, collapse behavior.
- Primary right action: adaptive `Stage All` or `Unstage All` split button.
- Overflow: Refresh, Discard All, Clean, stash operations, and other repository-wide actions.
- Destructive actions must be separated, explicitly named, and confirmed where data loss is possible.

#### Change rows

- Replace status letters as the primary staging affordance with tri-state checkboxes at repository, section/folder, and file levels.
- Preserve semantic status through a compact icon/tint/label independent of the checkbox.
- Add tree and flat presentations without duplicating the underlying change model.
- Use a dedicated dense `GitChangeRow`, not the generic `ListItem`.
- Hide discard/revert and other secondary file actions until hover/focus or expose them in the context menu.
- Preserve the complete path through tree structure or secondary muted text; never reduce identity to ambiguous last-two-segment strings without a full-path fallback.
- Virtualize the flattened presentation.

#### Diff flow

- Replace the inline 280 px viewer with a workspace-level Project Diff item.
- Let the Source Control panel select/focus files in that diff.
- Support unified/split view and hunk stage/unstage in the diff item.
- Keep only a small selected-file summary in the panel when useful.

#### Repository footer and commit composer

- Put branch, ahead/behind, remote state, and a compact repository menu into one footer row.
- Move Fetch/Pull/Push/Force Push into a state-aware menu or split action; Force Push must never have the same visual weight as ordinary Push.
- Replace the hand-rolled 48 px input with an editor-backed commit composer.
- Keep the default composer compact but allow expansion or a workspace/modal editor.
- Show commit-title length/warnings without turning the whole footer into an error state.
- Use an adaptive commit label derived from repository/staging state.
- Explain disabled state in a tooltip and accessibility description.
- Preserve stashes and tags, but expose their workflows through tabs, pickers, or menus rather than another permanently open panel segment.

## 10. Visual-system differences below the feature level

### 10.1 Color is not the main gap

Labonair's dark theme already uses a restrained hierarchy: sidebar background `#181818`, status bar `#1f1f1f`, border `#2b2b2b`, foreground `#ededed`, and muted foreground `#9d9d9d`. These values are not inherently less polished than Zed's theme values. Replacing them would not solve the structural issues.

The palette should remain recognizably Labonair. The gold primary token can remain a brand signal, but it should be reserved for high-value focus, selection, or primary-action states rather than broad active fills.

### 10.2 Semantic states

Labonair's low-level primitives often model `selected` and `disabled`, then derive hover from the selected fill. Zed's UI foundation differentiates:

- enabled versus disabled;
- hover;
- mouse-down/active;
- selected/toggled;
- keyboard focus-visible;
- semantic tint such as warning/error/success;
- element layer/elevation.

This richer state vocabulary is a major contributor to polish. Components react with the smallest necessary visual change instead of jumping between transparent and a single generic fill.

### 10.3 Density-aware spacing

Zed's `DynamicSpacing` is not simply a set of constants. Each base spacing resolves differently for compact, default, and comfortable UI density. Controls and gaps therefore preserve their relationship across density modes.

Labonair currently centralizes many values through `Palette::space`, but individual feature renderers still encode fixed heights and padding. The target should introduce semantic density tokens for:

- status-bar outer padding;
- control height;
- tree row height;
- section-header height;
- row inner gap;
- panel tab/header height;
- dock boundary and resize hit target.

Do not scale every dimension uniformly. Borders and focus indicators should usually remain one or two physical/logical pixels while touch targets, rows, and gaps change with density.

### 10.4 Purpose-built primitives

The current generic `ListItem` is being asked to represent command results, settings items, Explorer entries, and Git changes. These surfaces have different geometry and state requirements.

The clean solution is a small family rather than one infinitely configurable abstraction:

- `ListItem`: current general-purpose/card-like selectable row;
- `TreeRow`: continuous hierarchical entry with indentation and disclosure;
- `ChangeRow`: dense version-control entry with staging state and diff metadata;
- `PanelTabBar`: meaningful local mode navigation;
- `DockPanelButtons`: dock-specific status-bar controls.

This avoids both code duplication and an over-configurable universal row.

### 10.5 Motion and feedback

Neither codebase relies on decorative animation for these panels. Zed's responsiveness comes mainly from immediate state updates and bounded rendering. Labonair should prioritize:

- immediate local toggled/selection feedback;
- optimistic feedback only where rollback is reliable;
- visible async progress for remote Explorer and Git operations;
- stable row geometry while metadata arrives;
- toast/error routing through the existing overlay layers;
- no long color or position animations in high-frequency tree/list interactions.

## 11. Feature-by-feature port map

The table below maps each desired Zed behavior to the most appropriate Labonair ownership point. “Reimplement” means implement the behavior independently; it does not mean copy Zed source.

| Desired behavior | Zed reference area | Labonair target | Approach |
|---|---|---|---|
| Per-dock status buttons | `workspace::dock::PanelButtons` | `crates/workspace` + `crates/shell` | Reimplement over existing three `Dock` values |
| Active button closes dock | dock toggle action | existing `Dock::toggle_panel` | Retain current semantics, change presentation |
| Valid move destinations | `Panel::position_is_valid` | existing panel metadata | Reuse Labonair validity checks in context menu |
| Count badge | panel `icon_label` | panel trait/metadata | Add optional nonnegative count label |
| Status toolbar semantics | `workspace::status_bar` | `crates/workspace/src/status_bar.rs` | Add role, labels, tab group, arrow focus |
| Overlaid resize handle | `workspace::Dock::render` | `Workspace::render_dock` | Reimplement absolute hit target; preserve persistence |
| Density-aware spacing | `ui::DynamicSpacing` | theme/ui-kit | Add a small semantic density layer, not a copy of Zed enum |
| Flat tree rows | `project_panel::render_entry` | `panel-explorer` or focused ui-kit primitive | New `TreeRow` |
| Virtual tree | Project Panel `uniform_list` | `panel-explorer` | Flatten visible model and virtualize |
| Sticky ancestors | Project Panel sticky entries | `panel-explorer` | Independent ancestry calculation over flattened model |
| Indent guides | Project Panel guide render | `TreeRow`/Explorer overlay | Independent guide geometry |
| Active-file reveal | Project Panel setting/observer | workspace–Explorer event | Add active path event and reveal policy |
| Auto-fold chains | Project Panel settings/model | Explorer flattening | Present compressed path without changing filesystem model |
| Git/diagnostic decorations | Project Panel row metadata | Explorer data providers | Merge optional metadata at presentation boundary |
| Changes/History modes | `git_panel` tabs | `panel-scm` | Add panel-owned mode state |
| Adaptive Git header | Git changes header | `panel-scm` | Derive primary action from staged/unstaged state |
| Checkbox staging | Git entries | `GitChangeRow` | Tri-state state model plus async action dispatch |
| Tree/flat changes | Git panel settings | `panel-scm` presenter | Two flatteners over one change model |
| Virtual change list | Git panel uniform list | `panel-scm` | Virtualize flattened presentation |
| Project Diff | Git diff multibuffer | workspace item + SCM events | New native workspace item using existing editor/diff capabilities |
| Commit editor | Git panel footer | `panel-scm` + editor primitive | Compact editor-backed composer and expandable view |
| Adaptive commit action | Git panel commit button | `panel-scm` derived state | Explicit operation enum and disabled reason |
| Repo footer | Git repository footer | `panel-scm` | Consolidate branch/remote/status controls |

## 12. Concrete Labonair code plan

This section translates the visual recommendations into Labonair-owned code boundaries. The names are proposals, not a requirement to mirror Zed's types.

### 12.1 Status-bar ownership

The simplest robust design is to stop treating panel buttons as an ordinary movable `StatusItem`. The current implementation already admits that they are structural by special-casing `"panel-toggles"` as non-movable. Make that distinction explicit:

```text
StatusBar
├── left_edge: DockPanelButtons(Left)
├── left_status_items: user-placeable status items
├── flexible spacer
├── right_status_items: user-placeable information/actions
├── bottom_edge: DockPanelButtons(Bottom)
└── right_edge: DockPanelButtons(Right)
```

Recommended module changes:

- `crates/shell/src/status_items.rs`
  - remove `PanelTogglesStatusItem` and its global `panel_toggle_*` iteration;
  - retain the icon-name mapping in a neutral panel-button helper if `PanelIcon` remains the public metadata type;
  - register only true status/information items through `register_builtin_status_items`.
- `crates/workspace/src/status_bar.rs`
  - own or render three `DockPanelButtons` views outside the movable status-item registry;
  - add the toolbar/tab-group keyboard contract;
  - keep user placement and grouping for non-panel items;
  - prevent right-side informational items from pushing the edge button groups out of view.
- `crates/workspace/src/dock_panel_buttons.rs` or a small private module beside `status_bar.rs`
  - receive `Entity<Workspace>`, `DockPosition`, and `Entity<ThemeStore>`;
  - derive its buttons from `Workspace::dock(position).panels()` on every notified render;
  - read active/open state from the same dock snapshot;
  - call existing `select_panel`, `move_panel_persist`, and visibility persistence methods;
  - filter the movement menu with `PanelHandle::position_is_valid` before presenting a destination.

The current movement menu displays every other dock and relies on `Workspace::move_panel` to reject an invalid destination. The redesigned menu should omit invalid destinations so the presentation and executable state cannot disagree.

The existing plain `Dock` does not have to become an `Entity<Dock>` for this phase. `DockPanelButtons` can observe the workspace entity, just as the aggregate toggle view does today. Converting docks to entities would be a larger architectural change with no immediate UI benefit.

### 12.2 Minimal panel metadata extension

The current `Panel`/`PanelHandle` surface already supplies persistent name, title, icon, position, valid positions, sizing, focus, and rendering. Only optional button metadata is missing. A minimal Labonair-owned extension is sufficient:

```rust
// Behavioral sketch; naming can be adjusted during implementation.
fn status_badge(&self, cx: &App) -> Option<u32> {
    None
}

fn button_is_visible(&self, cx: &App) -> bool {
    true
}
```

Visibility may remain in the existing settings map instead of becoming a trait method. A badge method is only worthwhile when at least one real panel has a meaningful count; it should not be added speculatively in the first shell task.

After the generic dock header is removed, update the `Panel::title` documentation: the title becomes the accessible name, tooltip/menu label, and any command-palette label, not a promise that a dock title row exists.

### 12.3 Dock renderer changes

`Workspace::render_dock` can be simplified without changing dock state:

```text
read active panel + size + zoom
build relative dock container
apply one structural edge border
render active panel at full size
overlay transparent 6 px drag target on the inner edge
attach double-click size reset
```

Remove the `header` value and the `title` read entirely. Keep `sidebar_bg`/`sidebar_fg`, but no longer use `accent` for a six-pixel hover slab. A subtle focused border color is enough while dragging or hovering the resize boundary.

The resize handle must be absolute/overlaid so its six-pixel hit area does not become six visible pixels or reduce the editor/dock width. It must still stop pointer propagation, preserve drag behavior, clamp through the current size rules, and persist the final size.

### 12.4 Explorer presentation model

Do not virtualize by putting the current recursive render loop inside a `uniform_list` closure. First separate model flattening from element construction:

```text
Explorer model
  └── flatten_visible_entries(query, expansion, folding)
        └── Vec<ExplorerRowData>
              ├── Entry(path, depth, metadata, visual_state)
              ├── Loading(parent, depth)
              ├── Error(parent, depth, message)
              └── LoadMore(parent, depth)

virtual list receives only [start..end]
  └── TreeRow::render(ExplorerRowData)
```

`ExplorerRowData` should be cheap to clone and contain presentation-ready facts. It should not perform filesystem access while rendering. The model remains responsible for asynchronous local/remote loading and sends a notification when a subtree changes.

Recommended state separation:

```text
identity: path, display name, is_directory, icon
structure: depth, expanded, has_children, compressed ancestors
selection: selected, marked, focused, active_editor_entry
operation: cut, drag_source, drop_target, loading, unavailable
metadata: git_status, diagnostic_severity, diagnostic_count
```

This avoids deriving several unrelated visual meanings from `is_selected`. The renderer can assign one small visual channel to each state while preserving row geometry.

`panel-git-graph` already uses GPUI's `uniform_list` in this repository, so it is the best local API example. Reuse the locally proven GPUI calling pattern, not Zed's project-specific list state and algorithms.

### 12.5 Source Control presentation model

Build a flattened presentation list independent of the current four nested section render loops:

```text
Vec<GitListEntry>
├── SectionHeader { kind, count, stage_state, collapsed }
├── Directory { path, depth, stage_state, aggregate_status }
├── File { path, depth, stage_state, file_status, diff_stats }
├── Loading
├── Error
└── EmptyState
```

The model should expose explicit derived operation state rather than making the view infer behavior from which section owns a path:

```text
StageState = Unstaged | Staged | PartiallyStaged | Conflicted
CommitMode = CommitStaged | CommitTracked | Amend
RepoOperation = Idle | Fetching | Pulling | Pushing | Mutating
DisabledReason = NoMessage | NoChanges | Conflict | OperationInProgress | …
```

The existing backend methods remain the execution layer. The UI refactor should initially wrap them, not rewrite Git plumbing. Each async operation must carry enough identity to disable or show progress on the affected aggregate/file action without freezing the entire panel.

### 12.6 Project Diff integration boundary

The Source Control panel should not own a full code renderer after the redesign. Define a workspace-facing request containing repository/session identity, ordered changed paths, initial selection, and diff mode. The workspace then opens or focuses one Project Diff item. Subsequent SCM selection updates can focus the corresponding file inside that item.

This preserves crate direction:

```text
panel-scm emits intent/data
        ↓
workspace owns tab/item lifecycle
        ↓
editor/diff layer renders code and hunks
        ↓
backend Git operations stage/unstage selected hunks
```

Avoid a direct dependency from the generic workspace crate back into concrete Source Control view types. Use an existing workspace item/event abstraction or a small neutral request type in the lowest suitable crate.

### 12.7 Tests that should be written before visual replacement

The following state tests make the port safe and independent of pixel snapshots:

- dock-button membership follows panel movement;
- only one active button exists per open dock;
- a closed dock has no active/toggled button even though it retains an active panel index;
- clicking the toggled button closes the dock;
- invalid positions never appear in the movement menu;
- hidden panel-button settings survive movement and restart;
- right-dock visual order is deterministic;
- Explorer flattening preserves depth, expansion, loading, and selection across viewport ranges;
- Explorer search is not limited to previously rendered rows;
- Git flattening preserves aggregate tri-state staging;
- commit mode and disabled reason are correct for staged-only, tracked-only, amend, conflict, and empty repositories;
- a Project Diff request is idempotent: repeated “View Diff” focuses the existing item instead of opening duplicates.

## 13. Proposed implementation sequence

The order matters because later panel work should be built on the final shell geometry and row primitives.

### Phase 1 — shell geometry and status-bar ownership

**Goal:** make panel location and controls spatially coherent without changing Explorer or Git behavior.

1. Add one `DockPanelButtons` entity per dock.
2. Place left buttons in the left status region and bottom/right buttons in the right region.
3. Preserve all current panel movement and visibility persistence.
4. Add full tooltips, shortcuts, focus-visible state, ARIA labels, and arrow navigation.
5. Remove the aggregate global toggle strip.
6. Remove the generic dock title/move header.
7. Convert the resize gutter into an overlaid hit target with double-click reset.

**Verification:** every panel opens, closes, switches, moves, hides, restores, and persists correctly in all valid docks; status buttons always appear in the group matching their dock.

### Phase 2 — dedicated dense rows and virtualization

**Goal:** improve visual rhythm and large-repository behavior before adding features.

1. Introduce `TreeRow` and `GitChangeRow` with explicit state inputs.
2. Add density tokens and focus-visible styling.
3. Convert Explorer's flattened rows to a virtual list.
4. Convert Source Control sections/rows to a virtual list.
5. Preserve all current selection, context menu, drag/drop, and async operations.

**Verification:** row counts no longer cause proportional GPUI element creation; selection and drag/drop tests pass; compact/default/comfortable snapshots or render tests show stable alignment.

### Phase 3 — Explorer context and information design

**Goal:** make the tree content-first and context-preserving.

1. Replace the five-action permanent toolbar with a compact root/overflow treatment.
2. Make search transient and project-wide rather than loaded-row-only.
3. Add sticky ancestors and optional indent guides.
4. Add active-file auto-reveal.
5. Add optional single-child chain folding.
6. Add Git and diagnostic decorations through optional data providers.
7. Clarify preview versus permanent-open state.

**Verification:** deep trees remain understandable during scroll; the active file can be revealed; remote roots degrade gracefully; all former file operations remain reachable.

### Phase 4 — Git information architecture

**Goal:** center the routine review–stage–commit workflow.

1. Add Changes/History tabs.
2. Replace the permanent action bar with View Diff, view options, adaptive stage action, and overflow.
3. Add checkbox-driven staging and tree/flat views.
4. Move inline diff to a workspace-level Project Diff.
5. Consolidate repository/remote actions in a footer/menu.
6. Replace the commit field with a compact expandable editor.
7. Add adaptive commit labels and disabled reasons.
8. Integrate stashes and tags through contextual workflows.

**Verification:** the common path needs fewer visible controls and fewer pointer movements; dangerous actions are never primary; staging state is readable without interpreting section position.

### Phase 5 — polish, accessibility, and performance evidence

**Goal:** prove the design behaves as well as it looks.

1. Audit keyboard traversal for status bar, tree, change list, tabs, menus, and commit editor.
2. Audit roles, names, toggled/expanded state, and focus restoration.
3. Measure rendering and input latency on large local and remote projects.
4. Validate theme contrast across built-in light/dark and custom themes.
5. Validate all density modes and narrow dock widths.
6. Add render/snapshot tests where the GPUI test environment permits.

## 14. Acceptance specification

The redesign should not be accepted based on visual resemblance alone. The following outcomes are measurable.

### Status bar and docks

- No generic dock header is rendered.
- Every visible panel button belongs to exactly one dock-specific status-bar group.
- Moving a panel immediately moves its button to the destination group.
- The active button communicates both selected panel and open dock.
- The active button closes its dock; an inactive button opens and focuses its panel.
- All panel buttons expose a readable label and shortcut where configured.
- Keyboard users can enter the status bar and traverse controls with Left/Right Arrow.
- The resize target is at least 6 px while the visible boundary remains 1 px.
- Double-clicking a resize boundary restores the configured/default panel size.

### Explorer

- Standard rows occupy a consistent dense height and do not use rounded card islands.
- Selected, focused, marked, active-file, cut, and drop-target states remain distinguishable.
- Rendering cost is bounded by viewport size rather than the number of expanded entries.
- Deep ancestry remains visible through sticky ancestors and/or guides.
- Active-file reveal is optional and predictable.
- Search can find entries that were not previously expanded and loaded into the visible row list.
- Git and diagnostic decorations do not change row height.
- All existing local, remote, selection, file-operation, drag/drop, bookmark, and context-menu behavior remains available.

### Source Control

- Changes and History are distinct panel modes.
- Review, stage, and commit are visually primary; refresh/configuration/destructive operations are secondary.
- Staging state is visible and controllable at aggregate and file levels.
- Large change sets use bounded/virtualized rendering.
- Full diffs open in the workspace and support file/hunk navigation.
- Force push, clean, discard-all, and similar destructive actions are not permanent primary buttons.
- The commit action states exactly what scope/mode it will commit and explains disabled state.
- Branch, remote, ahead/behind, stash, tag, and force operations remain reachable.

### Quality gates

- `cargo fmt --check` passes.
- `cargo check` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- `cargo test` passes.
- Large-project profiling shows no row-count-linear render regression.
- No filesystem, Git, SFTP, or other I/O is introduced on the GPUI main thread.

## 15. What should not be copied from Zed unchanged

Zed is the stronger reference for these interaction patterns, but Labonair should not become a visual clone.

1. **Do not replace the Labonair theme identity.** Keep its neutral surfaces and restrained gold accent; improve semantic use rather than importing Zed colors.
2. **Do not discard the three-dock personalization contract.** Labonair has Explorer, Source Control, Git Graph, Snippets, and AI; the per-dock button model is more valuable here than in a smaller panel set.
3. **Do not hide remote state.** Zed's primarily local Project Panel assumptions do not fully cover SFTP latency, reconnect, permissions, or partial metadata.
4. **Do not remove Git Graph, stash, tag, or remote operations.** Reorganize them through progressive disclosure.
5. **Do not force one global row primitive.** Tree and change rows need different semantics.
6. **Do not copy source organization or algorithms.** Reproduce specified outcomes on Labonair's own models and traits.
7. **Do not assume official documentation and source always agree.** For example, current Zed source-level `position_is_valid` behavior should be checked when deciding valid Project/Git dock positions; Labonair's own architecture remains normative.

## 16. Final design diagnosis

The difference can be summarized as six interacting qualities:

1. **Zed minimizes permanent chrome.** Labonair currently stacks a shell header, a panel toolbar, and sometimes another local mode or search strip.
2. **Zed gives controls spatial ownership.** Dock buttons live next to the dock they operate; Labonair's buttons live in a global launcher.
3. **Zed uses progressive disclosure.** Frequent safe actions stay visible; rare, configurational, and destructive actions move into contextual surfaces.
4. **Zed uses domain-specific density.** A tree row is designed as a tree row and a Git change as a Git change; Labonair stretches a general list row across both.
5. **Zed maintains context.** Sticky ancestry, auto-reveal, diagnostics, Git state, Project Diff, adaptive labels, and disabled reasons reduce memory and uncertainty.
6. **Zed treats performance and accessibility as visual quality.** Virtualized lists, immediate state, keyboard traversal, and explicit semantics make the interface feel precise.

The highest-leverage Labonair change is therefore the shell/status-bar restructuring, followed immediately by dedicated virtualized rows. Those two steps remove most of the visual weight and provide the foundation for the richer Explorer and Git interactions. The remaining feature work then becomes additive instead of another layer of permanent controls.

## 17. Recommended first implementation task

The first implementation task should be narrowly scoped to **per-dock status-bar buttons plus removal of the generic dock header**. It should not simultaneously redesign Explorer or Git.

That task has a clear success boundary:

- no domain behavior changes;
- no new panel features;
- existing panel movement/persistence reused;
- one button group per dock;
- button movement mirrors panel movement;
- accessible focus and complete tooltips;
- panel body gains the reclaimed vertical space;
- all existing tests plus new dock/button state tests pass.

This produces an immediately visible improvement, validates the spatial model, and keeps the later Explorer and Git work surgical.
