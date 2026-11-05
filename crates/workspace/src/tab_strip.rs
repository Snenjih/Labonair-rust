//! Tab strip rendering: the titlebar strip, the vertical sidebar list and the
//! per-tab block (R10-002).
//!
//! Tabs render as `TabStyle::Floating` blocks. Everything a block shows (label,
//! icon, status markers) comes from [`tab_display`], the one seam tab rendering
//! reads through, so pane-group names and hover lists extend it in one place.

use std::path::Path;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, point, px, App, AppContext, ClipboardItem, Context, DragMoveEvent, Hsla,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_command_palette_core::CommandId;
use labonair_settings::{Settings as _, WorkspaceSettings};
use labonair_ui_kit::{
    caret, context_menu_with_exit, divider, floating_tab_strip, icon_button_builder,
    tab_drop_marker, tab_group_gap, tab_item, Axis, ButtonSize, Density, IconButtonShape, IconName,
    MenuItem, MotionExt, Palette, SubmenuHoverSource, TabDropSide, TabLayout, TabStyle, Tooltip,
};

use crate::pane_drop::tab_join_highlight;
use crate::pane_group_naming::group_label;
use crate::pane_view::PaneDragChip;
use crate::tabs::{Tab, TabDiagnosticStatus, TabGitStatus, TabKind};
use crate::{relative_path_under_root, DraggedPane, DraggedTab, NewTabSubmenu, Workspace};

/// Everything one tab block shows. The seam for tab rendering: both the
/// horizontal strip and the sidebar list read a tab only through this.
#[derive(Clone, Debug)]
pub(crate) struct TabDisplay {
    /// Title shown on the block (custom title, process title, cwd basename...).
    pub label: SharedString,
    /// Full text for the tooltip of a (possibly truncated) title.
    pub tooltip: SharedString,
    /// Kind icon, shown on non-editor tabs and in the sidebar.
    pub icon: IconName,
    /// Git status marker of an editor tab, when enabled in settings.
    pub status_color: Option<Hsla>,
    /// Diagnostics marker of an editor tab, when enabled in settings.
    pub diagnostic_color: Option<Hsla>,
    /// Number of panes in the tab (`1` for a plain tab). From two on the label
    /// and icon describe the group and the hover list is available.
    pub pane_count: usize,
    /// Unsaved-changes marker: an editor tab, or a group holding a dirty editor.
    pub dirty: bool,
}

/// Resolve how `tab` is displayed. Reads settings and the theme, never mutates.
pub(crate) fn tab_display(tab: &Tab, workspace: &Workspace, cx: &App) -> TabDisplay {
    let c = Palette::from_theme(workspace.theme.read(cx));
    // A pane group names itself after its panes (R10-002 "Tab-Name/Icon einer
    // Gruppe"); a single pane keeps the tab's own label and kind icon.
    let panes = workspace.tab_pane_ids(tab.id);
    let group = (panes.len() > 1).then(|| {
        let active = workspace.layouts.get(&tab.id).and_then(|l| l.active);
        group_label(
            &panes,
            &workspace.pane_meta,
            active,
            tab.custom_title.as_deref(),
        )
    });
    let pane_count = panes.len();
    let label = SharedString::from(match &group {
        Some(g) => g.title.clone(),
        None => tab.label(),
    });
    let editor = tab.kind == TabKind::Editor && group.is_none();
    let status_color = (editor
        && WorkspaceSettings::try_get(cx)
            .is_some_and(|settings| settings.show_editor_tab_git_status()))
    .then_some(tab.git_status)
    .flatten()
    .map(|status| match status {
        TabGitStatus::Added => c.success,
        TabGitStatus::Modified => c.warning,
        TabGitStatus::Deleted => c.error,
    });
    let diagnostic_color = (editor
        && WorkspaceSettings::try_get(cx)
            .is_some_and(|settings| settings.show_editor_tab_diagnostics()))
    .then_some(tab.diagnostic_status)
    .flatten()
    .map(|status| match status {
        TabDiagnosticStatus::Error => c.error,
        TabDiagnosticStatus::Warning => c.warning,
    });
    TabDisplay {
        tooltip: label.clone(),
        label,
        icon: group.map_or_else(|| tab.kind.indicator(), |g| g.icon),
        status_color,
        diagnostic_color,
        pane_count,
        dirty: tab.dirty && (tab.kind == TabKind::Editor || pane_count > 1),
    }
}

/// Strip-local view state: scroll handles and the drag-over marker.
#[derive(Default)]
pub(crate) struct TabStripState {
    /// Horizontal titlebar strip.
    pub scroll_h: ScrollHandle,
    /// Vertical sidebar list.
    pub scroll_v: ScrollHandle,
    /// The tab last scrolled into view per presentation (`[horizontal,
    /// vertical]`), so manual scrolling is not fought on every render.
    scrolled_to: [Option<u64>; 2],
    /// The block a tab drag hovers and the side an insertion would land on.
    /// Only meaningful while a drag is active.
    pub drop_target: Option<(u64, TabDropSide)>,
}

/// Which side of `target` a reorder of `dragged` lands on, or `None` when the
/// drop would be rejected (same tab, different Space, pinned/unpinned mix).
/// `tabs` is `(id, space id, pinned)` in store order. Mirrors
/// `TabStore::reorder`, which moves the dragged tab into the target's slot.
pub(crate) fn drop_side(
    tabs: &[(u64, u64, bool)],
    dragged: u64,
    target: u64,
) -> Option<TabDropSide> {
    let from = tabs.iter().position(|t| t.0 == dragged)?;
    let to = tabs.iter().position(|t| t.0 == target)?;
    let (a, b) = (tabs[from], tabs[to]);
    if from == to || a.1 != b.1 || a.2 != b.2 {
        return None;
    }
    Some(if from < to {
        TabDropSide::After
    } else {
        TabDropSide::Before
    })
}

impl Workspace {
    /// Height of `AppShell`'s titlebar (density scaled). The tab strip lives
    /// inside it, so strip-opened menus drop from the bar's bottom edge.
    fn titlebar_offset(c: &Palette) -> gpui::Pixels {
        Density::from_palette(c).titlebar_height()
    }

    /// Render one tab block. `sidebar == true` lays it out for the vertical
    /// Tabs sidebar panel (full width, click-anchored menus, top-edge drop
    /// marker); `false` is the horizontal titlebar strip.
    pub(crate) fn render_tab(
        &self,
        tab: &Tab,
        sidebar: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let fg = theme.foreground();
        let c = Palette::from_theme(theme);
        let titlebar_offset = Self::titlebar_offset(&c);
        let id = tab.id;
        let active = self.tabs.read(cx).active_id() == id;
        let display = tab_display(tab, self, cx);
        let layout = if sidebar {
            TabLayout::Vertical
        } else {
            TabLayout::Horizontal
        };
        let label = display.label.clone();

        let label_content = match self.rename_tab.as_ref().filter(|(rid, _)| *rid == id) {
            Some((_, buf)) => div()
                .track_focus(&self.rename_focus)
                .key_context("TabRename")
                .on_key_down(cx.listener(Self::on_rename_key))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, window, cx| {
                        window.focus(&this.rename_focus);
                        cx.stop_propagation();
                    }),
                )
                .min_w(px(80.0))
                .max_w(px(180.0))
                .px_1()
                .rounded_sm()
                .border_1()
                .border_color(c.primary)
                .flex()
                .items_center()
                .child(SharedString::from(buf.clone()))
                .when(self.rename_blink.read(cx).visible(), |d| {
                    d.child(caret(fg, 12.0))
                })
                .into_any_element(),
            None => div().child(label.clone()).into_any_element(),
        };

        let view = cx.entity();
        let mut tab_item = tab_item(format!("workspace-tab-{id}"), c, label.clone());
        if sidebar || tab.kind != TabKind::Editor || display.pane_count > 1 {
            tab_item = tab_item.leading(display.icon);
        }
        tab_item = tab_item
            .style(TabStyle::Floating)
            .layout(layout)
            .selected(active)
            .dirty(display.dirty)
            .peek(tab.kind == TabKind::Editor && tab.peek)
            .pinned(tab.pinned)
            .status_color(display.status_color)
            .status_marker(display.diagnostic_color)
            .label_content(label_content)
            .on_activate({
                let view = view.clone();
                move |window, app| {
                    view.update(app, |this, cx| this.select_tab(id, window, cx));
                }
            })
            .on_navigate({
                let view = view.clone();
                move |forward, window, app| {
                    view.update(app, |this, cx| this.cycle_from_tab(id, forward, window, cx));
                }
            })
            .on_double_click({
                let view = view.clone();
                move |window, app| {
                    view.update(app, |this, cx| {
                        if this.tabs.read(cx).get(id).is_some_and(|tab| tab.peek) {
                            this.tabs
                                .update(cx, |store, cx| store.set_peek(id, false, cx));
                        } else {
                            this.begin_tab_rename(id, window, cx);
                        }
                    });
                }
            });
        if !tab.pinned {
            tab_item = tab_item.on_close(move |window, app| {
                view.update(app, |this, cx| this.request_close(id, window, cx));
            });
        }
        let renaming = self.rename_tab.as_ref().is_some_and(|(rid, _)| *rid == id);
        let (join, strip_end) = self.tab_drop_hints(id, cx);
        let marker = self
            .tab_strip
            .drop_target
            .filter(|(target, _)| *target == id && cx.has_active_drag())
            .map(|(_, side)| tab_drop_marker(c, layout, side))
            .or_else(|| strip_end.then(|| tab_drop_marker(c, layout, TabDropSide::After)));
        tab_item
            .render()
            .when(!renaming && display.pane_count < 2, |row| {
                let tooltip = display.tooltip.clone();
                row.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            })
            .children(marker)
            .when(join, |row| row.child(tab_join_highlight(c)))
            .on_hover(cx.listener(move |this, hovered: &bool, window, cx| {
                let pos = window.mouse_position();
                let anchor = if sidebar {
                    point(pos.x, pos.y + px(14.0))
                } else {
                    point(pos.x - px(12.0), titlebar_offset)
                };
                this.tab_hover_changed(id, *hovered, anchor, cx);
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    let y = if sidebar {
                        ev.position.y
                    } else {
                        titlebar_offset
                    };
                    this.open_tab_menu(id, point(ev.position.x, y), cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    if !this.tabs.read(cx).get(id).is_some_and(|tab| tab.pinned) {
                        this.request_close(id, window, cx);
                    }
                }),
            )
            .on_drag(
                DraggedTab {
                    id,
                    label: label.clone(),
                    icon: display.icon,
                },
                move |dragged, grab, _, cx| {
                    cx.new(|_| PaneDragChip::new(dragged.label.clone(), dragged.icon, c, grab))
                },
            )
            .on_drag_move(
                cx.listener(move |this, ev: &DragMoveEvent<DraggedPane>, _window, cx| {
                    let hovered = ev.bounds.contains(&ev.event.position);
                    let drag = ev.drag(cx).clone();
                    this.tab_pane_drag_over(id, hovered, &drag, cx);
                }),
            )
            .on_drop(cx.listener(move |this, dragged: &DraggedPane, window, cx| {
                this.drop_pane_on_tab(dragged, id, window, cx);
            }))
            .on_drag_move(
                cx.listener(move |this, ev: &DragMoveEvent<DraggedTab>, _window, cx| {
                    let hovered = ev.bounds.contains(&ev.event.position);
                    let current = this.tab_strip.drop_target;
                    let next = if hovered {
                        let dragged = ev.drag(cx).id;
                        let tabs: Vec<_> = this
                            .tabs
                            .read(cx)
                            .tabs()
                            .iter()
                            .map(|t| (t.id, t.space_id, t.pinned))
                            .collect();
                        drop_side(&tabs, dragged, id).map(|side| (id, side))
                    } else if current.is_some_and(|(target, _)| target == id) {
                        None
                    } else {
                        return;
                    };
                    if next != current {
                        this.tab_strip.drop_target = next;
                        cx.notify();
                    }
                }),
            )
            .on_drop(cx.listener(move |this, dragged: &DraggedTab, _window, cx| {
                this.clear_pane_dnd(cx);
                this.tabs.update(cx, |s, cx| s.reorder(dragged.id, id, cx));
            }))
            .motion_fade_in(("tab-in", id), c)
    }

    /// The tab strip. Rendered inside `AppShell`'s single overlay titlebar
    /// (the header) between the left/right bar-item buckets — mirrors the
    /// reference `Header.tsx` where the `TabBar` lives in the titlebar.
    pub fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (muted, fg, border) = (theme.muted_foreground(), theme.foreground(), theme.border());
        let c = Palette::from_theme(theme);
        let titlebar_offset = Self::titlebar_offset(&c);
        let active_space_id = self.spaces.read(cx).active_id();
        // Only the active Space's tabs show in the titlebar strip — every
        // other Space's tabs stay alive (T20-008) but out of sight until
        // switched to. The sidebar's `render_tab_list_vertical` intentionally
        // shows every Space's tabs at once, grouped into sections instead.
        let tabs: Vec<Tab> = self
            .tabs
            .read(cx)
            .tabs_in_space(active_space_id)
            .into_iter()
            .cloned()
            .collect();
        let pinned_tabs: Vec<Tab> = tabs.iter().filter(|tab| tab.pinned).cloned().collect();
        let unpinned_tabs: Vec<Tab> = tabs.iter().filter(|tab| !tab.pinned).cloned().collect();

        // Keep the selected tab in view: once per selection change, so manual
        // scrolling is not undone on every render. The group gap is a child of
        // the strip too, so it shifts the index of the unpinned tabs.
        let active_id = self.tabs.read(cx).active_id();
        let has_gap = !pinned_tabs.is_empty() && !unpinned_tabs.is_empty();
        if self.tab_strip.scrolled_to[0] != Some(active_id) {
            if let Some(ix) = pinned_tabs
                .iter()
                .position(|t| t.id == active_id)
                .or_else(|| {
                    unpinned_tabs
                        .iter()
                        .position(|t| t.id == active_id)
                        .map(|ix| ix + pinned_tabs.len() + usize::from(has_gap))
                })
            {
                self.tab_strip.scroll_h.scroll_to_item(ix);
                self.tab_strip.scrolled_to[0] = Some(active_id);
            }
        }

        let active_space = self.spaces.read(cx).active().cloned();
        let (show_nav_history_buttons, show_tab_bar_buttons) = WorkspaceSettings::try_get(cx)
            .map(|settings| {
                (
                    settings.show_nav_history_buttons(),
                    settings.show_tab_bar_buttons(),
                )
            })
            .unwrap_or((true, true));
        let editor_history = self.active_editor_history_state(cx);
        let can_cycle_tabs = tabs.len() > 1;
        let (can_go_back, can_go_forward) =
            editor_history.unwrap_or((can_cycle_tabs, can_cycle_tabs));
        let history_buttons_active = editor_history.is_some();
        let workspace = cx.entity();
        let previous = icon_button_builder("tab-previous", c, IconName::ArrowLeft)
            .size(ButtonSize::IconLg)
            .shape(IconButtonShape::Square)
            .disabled(!can_go_back)
            .tooltip(if history_buttons_active {
                "Back"
            } else {
                "Previous Tab"
            })
            .render()
            .when(can_go_back, |button| {
                button.on_click({
                    let workspace = workspace.clone();
                    move |_, window, app| {
                        workspace.update(app, |this, cx| this.navigate_tab_bar(false, window, cx));
                    }
                })
            });
        let next = icon_button_builder("tab-next", c, IconName::ArrowRight)
            .size(ButtonSize::IconLg)
            .shape(IconButtonShape::Square)
            .disabled(!can_go_forward)
            .tooltip(if history_buttons_active {
                "Forward"
            } else {
                "Next Tab"
            })
            .render()
            .when(can_go_forward, |button| {
                button.on_click({
                    let workspace = workspace.clone();
                    move |_, window, app| {
                        workspace.update(app, |this, cx| this.navigate_tab_bar(true, window, cx));
                    }
                })
            });
        let new_tab = icon_button_builder("tab-new", c, IconName::PlusBold)
            .size(ButtonSize::IconSm)
            .shape(IconButtonShape::Square)
            .tooltip("New Tab")
            .render()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    this.open_new_tab_menu(point(ev.position.x, titlebar_offset), cx);
                }),
            );

        let spaces_trigger = div()
            .id("spaces-trigger")
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_1()
            .h(px(24.0))
            .px_1p5()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(border))
            .child(IconName::Box.svg(muted).size(px(13.0)))
            .when_some(active_space, |d, space| {
                d.child(
                    div()
                        .max_w(px(96.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_xs()
                        .text_color(fg)
                        .child(space.name),
                )
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    this.open_spaces_menu(point(ev.position.x, titlebar_offset), cx);
                }),
            );

        div()
            .flex()
            .items_center()
            .gap_1()
            .h(titlebar_offset)
            .w_full()
            .flex_shrink_0()
            .on_drag_move(
                cx.listener(|this, ev: &DragMoveEvent<DraggedPane>, _window, cx| {
                    let hovered = ev.bounds.contains(&ev.event.position);
                    let drag = ev.drag(cx).clone();
                    this.strip_pane_drag_over(hovered, &drag, cx);
                }),
            )
            .on_drop(cx.listener(|this, dragged: &DraggedPane, window, cx| {
                this.drop_pane_on_strip(dragged, window, cx);
            }))
            .when(show_nav_history_buttons, |bar| {
                bar.child(
                    div()
                        .id("tab-navigation")
                        .flex()
                        .items_center()
                        .h_full()
                        .child(previous)
                        .child(next),
                )
            })
            .when(show_tab_bar_buttons, |bar| bar.child(spaces_trigger))
            .child(
                floating_tab_strip("tab-strip", c, TabLayout::Horizontal)
                    // Shrinkable and scrollable, unlike the kit default.
                    .flex_shrink()
                    .min_w_0()
                    .overflow_x_scroll()
                    .track_scroll(&self.tab_strip.scroll_h)
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<DraggedTab>, _window, _cx| {
                            this.auto_scroll_strip(false, ev.bounds, ev.event.position);
                        },
                    ))
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<DraggedPane>, _window, _cx| {
                            this.auto_scroll_strip(false, ev.bounds, ev.event.position);
                        },
                    ))
                    .children(pinned_tabs.iter().map(|t| self.render_tab(t, false, cx)))
                    .when(has_gap, |strip| {
                        strip.child(tab_group_gap(c, TabLayout::Horizontal))
                    })
                    .children(unpinned_tabs.iter().map(|t| self.render_tab(t, false, cx))),
            )
            .when(show_tab_bar_buttons, |bar| bar.child(new_tab))
    }

    /// Open the tab context menu for `id` at `anchor`.
    pub(crate) fn open_tab_menu(
        &mut self,
        id: u64,
        anchor: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.context_menu.open((id, anchor));
        self.close_new_tab_menu(cx);
        self.close_spaces_menu(cx);
    }

    /// Open the "+" new-tab dropdown at `anchor`.
    pub(crate) fn open_new_tab_menu(
        &mut self,
        anchor: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.new_tab_menu.open(anchor);
        self.new_tab_submenu = None;
        self.close_tab_menu(cx);
        self.close_spaces_menu(cx);
    }

    /// Open the Spaces popover at `anchor`.
    pub(crate) fn open_spaces_menu(
        &mut self,
        anchor: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.spaces_menu.open(anchor);
        self.close_new_tab_menu(cx);
        self.close_tab_menu(cx);
    }

    /// The vertical tab list shown in the Tabs sidebar panel when
    /// `tabsLocation == "sidebar"`. Same behaviour as [`Workspace::render_tab_bar`]
    /// (select / close / reorder / rename / context menu), stacked in a column
    /// with a "New Space" footer. The new-tab / context menus themselves are
    /// rendered by [`Workspace::render`], anchored at the click position.
    pub fn render_tab_list_vertical(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let (muted, fg, border) = (c.muted, c.fg, c.line_subtle);
        let spaces = self.spaces.read(cx).spaces().to_vec();
        let tabs = self.tabs.read(cx).tabs().to_vec();
        let active_id = self.tabs.read(cx).active_id();

        // Grouped into a collapsible section per Space (T20-008) — the same
        // data and row component the Spaces popover uses, just expanded
        // inline instead of in a floating card. `scroll_ix` counts the direct
        // children of the scroll container so the active tab can be revealed.
        let mut groups: Vec<gpui::AnyElement> = Vec::with_capacity(spaces.len() * 2);
        let mut scroll_ix = None;
        let last_space_idx = spaces.len().saturating_sub(1);
        for (idx, space) in spaces.iter().enumerate() {
            let space_tabs: Vec<&Tab> = tabs.iter().filter(|t| t.space_id == space.id).collect();
            groups.push(
                self.render_space_header(space, space_tabs.len(), false, cx)
                    .into_any_element(),
            );
            if !self.collapsed_spaces.contains(&space.id) {
                let has_gap =
                    space_tabs.iter().any(|t| t.pinned) && space_tabs.iter().any(|t| !t.pinned);
                let mut gap_done = false;
                for tab in space_tabs {
                    if has_gap && !tab.pinned && !gap_done {
                        gap_done = true;
                        groups.push(tab_group_gap(c, TabLayout::Vertical).into_any_element());
                    }
                    if tab.id == active_id {
                        scroll_ix = Some(groups.len());
                    }
                    groups.push(self.render_tab(tab, true, cx).into_any_element());
                }
            }
            if idx != last_space_idx {
                groups.push(divider(Axis::Horizontal, border).my_1().into_any_element());
            }
        }
        if self.tab_strip.scrolled_to[1] != Some(active_id) {
            if let Some(ix) = scroll_ix {
                self.tab_strip.scroll_v.scroll_to_item(ix);
                self.tab_strip.scrolled_to[1] = Some(active_id);
            }
        }
        // Recent Projects lives in the "New Space" dropdown (`sidebar-new-space`)
        // here, not inline — the titlebar Spaces popover still shows it inline
        // since it has no equivalent creation dropdown of its own.

        div()
            .flex()
            .flex_col()
            .min_h_0()
            .h_full()
            .w_full()
            .on_drag_move(
                cx.listener(|this, ev: &DragMoveEvent<DraggedPane>, _window, cx| {
                    let hovered = ev.bounds.contains(&ev.event.position);
                    let drag = ev.drag(cx).clone();
                    this.strip_pane_drag_over(hovered, &drag, cx);
                }),
            )
            .on_drop(cx.listener(|this, dragged: &DraggedPane, window, cx| {
                this.drop_pane_on_strip(dragged, window, cx);
            }))
            .child(
                floating_tab_strip("sidebar-tab-list", c, TabLayout::Vertical)
                    .flex_1()
                    .flex_shrink()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.tab_strip.scroll_v)
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<DraggedTab>, _window, _cx| {
                            this.auto_scroll_strip(true, ev.bounds, ev.event.position);
                        },
                    ))
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<DraggedPane>, _window, _cx| {
                            this.auto_scroll_strip(true, ev.bounds, ev.event.position);
                        },
                    ))
                    .children(groups),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(border)
                    .p_1p5()
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .id("sidebar-new-space")
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .text_color(muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(c.accent).text_color(fg))
                            .child(IconName::FolderAdd.svg(muted).size(px(15.0)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                                    this.new_space_menu.open(ev.position);
                                    this.new_space_search.clear();
                                    window.focus(&this.new_space_search_focus);
                                    cx.notify();
                                }),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// The "+" new-tab dropdown (port of `NewTabDropdownItems`): Terminal /
    /// Editor / Preview / Git Graph, then `SSH ▸` / `SFTP ▸` submenus listing
    /// the recent hosts (name + address) with an "All hosts…" entry each.
    pub(crate) fn render_new_tab_menu(
        &mut self,
        pos: gpui::Point<gpui::Pixels>,
        closing: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let recent = self.recent_host_picker_rows(cx, 5);
        let view = cx.entity();
        let open_sub = self.new_tab_submenu;

        // Hover wiring for a controlled `SSH ▸` / `SFTP ▸` submenu: entering
        // the trigger or its flyout opens that one (which closes the other,
        // since only one id is held); leaving the flyout closes it. Leaving
        // the trigger is deliberately ignored — the pointer is usually on its
        // way into the flyout.
        let submenu_cb = |which: NewTabSubmenu| {
            let v = view.clone();
            move |src: SubmenuHoverSource, hovered: bool, _w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    let changed = if hovered {
                        let was = this.new_tab_submenu;
                        this.new_tab_submenu = Some(which);
                        was != Some(which)
                    } else if src == SubmenuHoverSource::Flyout
                        && this.new_tab_submenu == Some(which)
                    {
                        this.new_tab_submenu = None;
                        true
                    } else {
                        false
                    };
                    if changed {
                        cx.notify();
                    }
                });
            }
        };

        // Recent-host rows for one protocol submenu (`ssh == false` → SFTP),
        // followed by a separator and the protocol's "All hosts…" entry.
        let host_items = |ssh: bool| -> Vec<MenuItem> {
            let proto = if ssh { "ssh" } else { "sftp" };
            let mut sub: Vec<MenuItem> = Vec::new();
            if recent.is_empty() {
                sub.push(MenuItem::new(format!("nt-{proto}-empty"), "No hosts yet").disabled(true));
            } else {
                for row in &recent {
                    let (id, name, subtitle) =
                        (row.id.clone(), row.name.clone(), row.subtitle.clone());
                    sub.push(
                        MenuItem::new(SharedString::from(format!("nt-{proto}-{id}")), name)
                            .detail(subtitle)
                            .on_click({
                                let v = view.clone();
                                move |_, w, cx| {
                                    let id = id.clone();
                                    v.update(cx, |this, cx| {
                                        this.close_new_tab_menu(cx);
                                        if ssh {
                                            this.open_ssh_tab(id, w, cx)
                                        } else {
                                            this.open_sftp_tab(id, w, cx)
                                        }
                                    })
                                }
                            }),
                    );
                }
            }
            sub.push(MenuItem::separator());
            sub.push(
                MenuItem::new(format!("nt-{proto}-all"), "All hosts\u{2026}")
                    .icon(IconName::Server)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            v.update(cx, |this, cx| {
                                this.close_new_tab_menu(cx);
                                this.open_hosts_tab(cx)
                            })
                        }
                    }),
            );
            sub
        };

        let items: Vec<MenuItem> = vec![
            MenuItem::new("nt-term", "Terminal")
                .icon(IconName::ToolTerminal)
                .keybind(["\u{2318}", "T"])
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_new_tab_menu(cx);
                            this.new_terminal_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-editor", "Editor")
                .icon(IconName::Pencil)
                .keybind(["\u{2318}", "E"])
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_new_tab_menu(cx);
                            this.new_editor_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-preview", "Preview")
                .icon(IconName::Globe)
                .keybind(["\u{2318}", "\u{21E7}", "P"])
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_new_tab_menu(cx);
                            this.new_preview_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-gitgraph", "Git Graph")
                .icon(IconName::GitBranch)
                .on_click({
                    let v = view.clone();
                    move |_, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_new_tab_menu(cx);
                            this.open_git_graph_tab(cx)
                        })
                    }
                }),
            MenuItem::separator(),
            MenuItem::submenu("nt-ssh", "SSH", host_items(true))
                .icon(IconName::Server)
                .submenu_control(
                    open_sub == Some(NewTabSubmenu::Ssh),
                    submenu_cb(NewTabSubmenu::Ssh),
                ),
            MenuItem::submenu("nt-sftp", "SFTP", host_items(false))
                .icon(IconName::Cloud)
                .submenu_control(
                    open_sub == Some(NewTabSubmenu::Sftp),
                    submenu_cb(NewTabSubmenu::Sftp),
                ),
        ];

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.close_new_tab_menu(cx);
                    cx.notify()
                })
            }
        };
        context_menu_with_exit(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            closing,
            dismiss,
            items,
        )
    }

    pub(crate) fn render_context_menu(
        &mut self,
        id: u64,
        pos: gpui::Point<gpui::Pixels>,
        closing: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab = self.tabs.read(cx).get(id).cloned();
        let kind = tab.as_ref().map(|t| t.kind);
        let is_peek = tab.as_ref().map(|t| t.peek).unwrap_or(false);
        let is_pinned = tab.as_ref().is_some_and(|tab| tab.pinned);
        let plural = kind.map(|k| k.plural_label().to_string());
        let grant_target = self.mcp_grant_target(id, cx);
        let is_granted = self.agent_access.read(cx).is_granted(id);
        let space_id = tab
            .as_ref()
            .map(|tab| tab.space_id)
            .unwrap_or_else(|| self.spaces.read(cx).active_id());
        let file_path = tab
            .as_ref()
            .filter(|tab| tab.kind == TabKind::Editor && tab.data.host_id.is_none())
            .and_then(|tab| tab.data.path.as_deref())
            .filter(|path| Path::new(path).is_absolute())
            .map(str::to_string);
        let editor_editability = self
            .editors
            .get(&id)
            .and_then(|editor| editor.read(cx).editability());
        let project_root = self
            .spaces
            .read(cx)
            .get(space_id)
            .and_then(|space| space.identity.project_root())
            .map(Path::to_path_buf);
        let relative_file_path = file_path
            .as_deref()
            .and_then(|path| relative_path_under_root(Path::new(path), project_root.as_deref()?));
        let space_tabs: Vec<Tab> = self
            .tabs
            .read(cx)
            .tabs_in_space(space_id)
            .into_iter()
            .cloned()
            .collect();
        let tab_index = space_tabs.iter().position(|tab| tab.id == id).unwrap_or(0);
        let multi = space_tabs.len() > 1;
        let can_close_left = tab_index > 0;
        let can_close_right = tab_index + 1 < space_tabs.len();
        let has_clean_tabs = space_tabs.iter().any(|tab| !tab.dirty);
        let view = cx.entity();

        let mut items: Vec<MenuItem> = Vec::new();
        if let (Some(TabKind::Editor), Some(editability)) = (kind, editor_editability) {
            let (label, checked, disabled) = match editability {
                labonair_editor::EditorEditability::Editable => {
                    ("Make Tab Read-Only", false, false)
                }
                labonair_editor::EditorEditability::TemporarilyReadOnly => {
                    ("Make Tab Editable", true, false)
                }
                labonair_editor::EditorEditability::ReadOnly => ("Read-Only", true, true),
            };
            items.push(
                MenuItem::new("toggle-tab-read-only", label)
                    .icon(IconName::Lock)
                    .checked(checked)
                    .disabled(disabled)
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                this.close_tab_menu(cx);
                                if let Some(editor) =
                                    this.editors.get(&this.editor_slot_for_tab(id)).cloned()
                                {
                                    editor.update(cx, |editor, cx| {
                                        let _ = editor.dispatch_registered_command(
                                            CommandId::EditorToggleReadOnly,
                                            cx,
                                        );
                                    });
                                }
                                cx.notify();
                            });
                        }
                    }),
            );
        }
        if let Some(path) = file_path.as_deref() {
            items.push(
                MenuItem::new("copy-file-path", "Copy Path")
                    .icon(IconName::Copy)
                    .on_click({
                        let view = view.clone();
                        let path = path.to_string();
                        move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
                            view.update(cx, |this, cx| {
                                this.close_tab_menu(cx);
                                cx.notify();
                            });
                        }
                    }),
            );
            if let Some(relative_path) = relative_file_path {
                items.push(
                    MenuItem::new("copy-relative-file-path", "Copy Relative Path").on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(relative_path.clone()));
                            view.update(cx, |this, cx| {
                                this.close_tab_menu(cx);
                                cx.notify();
                            });
                        }
                    }),
                );
            }
            items.push(
                MenuItem::new("reveal-file", "Reveal in File Manager")
                    .icon(IconName::FolderOpen)
                    .on_click({
                        let view = view.clone();
                        let path = path.to_string();
                        move |_, _, cx| {
                            cx.reveal_path(Path::new(&path));
                            view.update(cx, |this, cx| {
                                this.close_tab_menu(cx);
                                cx.notify();
                            });
                        }
                    }),
            );
            if let Some(directory) = Path::new(path).parent().map(|path| path.to_path_buf()) {
                items.push(
                    MenuItem::new("open-file-terminal", "Open Terminal in File Directory")
                        .icon(IconName::Terminal)
                        .on_click({
                            let view = view.clone();
                            let directory = directory.to_string_lossy().into_owned();
                            move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.close_tab_menu(cx);
                                    this.new_terminal_tab_in(Some(directory.clone()), window, cx);
                                });
                            }
                        }),
                );
            }
            items.push(
                MenuItem::new("open-file-history", "File History")
                    .icon(IconName::GitCompare)
                    .on_click({
                        let view = view.clone();
                        let path = path.to_string();
                        move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                this.close_tab_menu(cx);
                                this.open_file_history_tab(path.clone(), cx);
                                cx.notify();
                            });
                        }
                    }),
            );
            items.push(MenuItem::separator());
        }
        if kind == Some(TabKind::Editor) && is_peek {
            items.push(
                MenuItem::new("keep", "Keep Tab Open")
                    .icon(IconName::Eye)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            v.update(cx, |this, cx| {
                                this.close_tab_menu(cx);
                                this.tabs.update(cx, |s, cx| s.set_peek(id, false, cx));
                            })
                        }
                    }),
            );
        }
        items.push(
            MenuItem::new(
                "toggle-pin-tab",
                if is_pinned { "Unpin Tab" } else { "Pin Tab" },
            )
            .icon(IconName::Pin)
            .on_click({
                let view = view.clone();
                move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.close_tab_menu(cx);
                        this.tabs
                            .update(cx, |store, cx| store.set_pinned(id, !is_pinned, cx));
                    });
                }
            }),
        );
        items.push(
            MenuItem::new("rename", "Rename")
                .icon(IconName::Pencil)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| v.update(cx, |this, cx| this.begin_tab_rename(id, w, cx))
                }),
        );
        items.push(
            MenuItem::new("dup", "Duplicate")
                .icon(IconName::Copy)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.tabs.update(cx, |s, cx| s.set_active(id, cx));
                            this.duplicate_active_tab(w, cx);
                        })
                    }
                }),
        );
        items.push(MenuItem::separator());
        items.extend(self.pane_menu_items(id, &view, cx));
        items.push(MenuItem::separator());
        // T20-001: `MenuItem::keybind` renders the item's live binding as
        // `Kbd` chips, resolved through the user's `keymap.json` overrides.
        let close_keys = cx
            .try_global::<labonair_command_palette::KeybindDisplay>()
            .map(|g| g.keys_for(labonair_command_palette::CommandId::CloseTab, Some("cmd-w")))
            .unwrap_or_default();
        items.push(
            MenuItem::new("close", "Close")
                .keybind(close_keys)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.request_close(id, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("others", "Close Others")
                .disabled(!multi)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.close_others(id, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("left", "Close Left")
                .disabled(!can_close_left)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.close_tabs_to_side(id, false, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("right", "Close Right")
                .disabled(!can_close_right)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.close_tabs_to_side(id, true, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("clean", "Close Clean")
                .disabled(!has_clean_tabs)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.close_clean_tabs(space_id, w, cx)
                        })
                    }
                }),
        );
        items.push(MenuItem::new("all", "Close All").on_click({
            let v = view.clone();
            move |_, w, cx| {
                v.update(cx, |this, cx| {
                    this.close_tab_menu(cx);
                    this.close_all_tabs(space_id, w, cx)
                })
            }
        }));
        if multi {
            if let (Some(k), Some(pl)) = (kind, plural) {
                items.push(MenuItem::new("kind", format!("Close All {pl}")).on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.close_by_kind(space_id, k, w, cx)
                        })
                    }
                }));
            }
        }
        if let Some((session_id, label, gkind, host_id)) = grant_target {
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("mcp-grant", "Grant AI Agent Access")
                    .icon(IconName::Shield)
                    .checked(is_granted)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            let (session_id, label, host_id) =
                                (session_id.clone(), label.clone(), host_id.clone());
                            v.update(cx, |this, cx| {
                                this.close_tab_menu(cx);
                                this.agent_access.update(cx, |s, cx| {
                                    s.set_grant(
                                        id,
                                        session_id,
                                        !is_granted,
                                        label,
                                        gkind,
                                        host_id,
                                        cx,
                                    );
                                });
                                cx.notify();
                            })
                        }
                    }),
            );
        }

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.close_tab_menu(cx);
                    cx.notify()
                })
            }
        };
        context_menu_with_exit(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            closing,
            dismiss,
            items,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (id, space id, pinned) in store order.
    const TABS: [(u64, u64, bool); 5] = [
        (1, 1, true),
        (2, 1, true),
        (3, 1, false),
        (4, 1, false),
        (5, 2, false),
    ];

    #[test]
    fn drop_marker_follows_the_reorder_direction() {
        assert_eq!(drop_side(&TABS, 3, 4), Some(TabDropSide::After));
        assert_eq!(drop_side(&TABS, 4, 3), Some(TabDropSide::Before));
        assert_eq!(drop_side(&TABS, 1, 2), Some(TabDropSide::After));
    }

    #[test]
    fn drop_marker_hides_for_rejected_drops() {
        // Onto itself.
        assert_eq!(drop_side(&TABS, 3, 3), None);
        // Pinned onto unpinned and back.
        assert_eq!(drop_side(&TABS, 1, 3), None);
        assert_eq!(drop_side(&TABS, 4, 2), None);
        // Across spaces.
        assert_eq!(drop_side(&TABS, 4, 5), None);
        // Unknown ids.
        assert_eq!(drop_side(&TABS, 9, 3), None);
        assert_eq!(drop_side(&TABS, 3, 9), None);
    }
}
