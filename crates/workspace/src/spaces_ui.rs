//! Spaces UI: the Spaces popover, space headers, the Recent Projects section,
//! the per-space "..." menu and the sidebar "New Space" dropdown (R10-002).
//!
//! Rows are ui-kit `ListItem`s (28 px, `radius.md`, accent hover, the active
//! space on the selected fill with a semibold label); there is no colour badge.
//! Surfaces open through [`crate::menu_slot::MenuSlot`], so every dismissal
//! fades out instead of vanishing.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_ui_kit::{
    caret, context_menu_with_exit, divider, icon_button_builder, list_header, popover_with_exit,
    Axis, ButtonSize, IconButtonShape, IconName, ListItem, MenuItem, Palette,
};

use crate::recent_projects;
use crate::spaces::{Space, DEFAULT_SPACE_ID};
use crate::Workspace;

/// Width of the Spaces popover and the sidebar "New Space" dropdown.
const POPOVER_WIDTH: f32 = 300.0;
/// The popover list scrolls past this height; the footer stays pinned.
const POPOVER_MAX_HEIGHT: f32 = 420.0;
/// Left indent of tab rows and recent-project rows under their header.
const NESTED_INDENT: f32 = 28.0;

impl Workspace {
    /// One space's header row: collapse chevron, name (or the inline rename
    /// field), the tab count and a hover-only "..." button. Shared by the
    /// Spaces popover and the Tabs sidebar (T20-008), so both presentations
    /// stay identical. Clicking the row switches to the space; the chevron
    /// only toggles collapse; "..." opens the space's actions menu.
    pub(crate) fn render_space_header(
        &mut self,
        space: &Space,
        tab_count: usize,
        highlight_active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let id = space.id;
        let active = self.spaces.read(cx).active_id() == id && highlight_active;
        let collapsed = self.collapsed_spaces.contains(&id);

        let name_slot: gpui::AnyElement =
            match self.space_rename.as_ref().filter(|(rid, _)| *rid == id) {
                Some((_, buf)) => div()
                    .track_focus(&self.rename_focus)
                    .key_context("SpaceRename")
                    .on_key_down(cx.listener(Self::on_space_rename_key))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, window, cx| {
                            // Explicit re-focus, not just `stop_propagation` — GPUI's
                            // automatic focus-on-click for `track_focus` and this
                            // listener share the same mouse-down dispatch pass, and
                            // `stop_propagation` aborts it before the automatic
                            // focus transfer runs, silently killing click-to-refocus.
                            window.focus(&this.rename_focus);
                            cx.stop_propagation();
                        }),
                    )
                    .flex_1()
                    .min_w_0()
                    .px_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.ring)
                    .flex()
                    .items_center()
                    .child(SharedString::from(buf.clone()))
                    .when(self.rename_blink.read(cx).visible(), |d| {
                        d.child(caret(c.fg, 14.0))
                    })
                    .into_any_element(),
                None => div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(space.name.clone())
                    .into_any_element(),
            };

        let chevron = div()
            .id(("space-collapse", id))
            .flex_none()
            .on_click(cx.listener(move |this, _, _w, cx| {
                cx.stop_propagation();
                if !this.collapsed_spaces.remove(&id) {
                    this.collapsed_spaces.insert(id);
                }
                cx.notify();
            }))
            .child(
                if collapsed {
                    IconName::ChevronRight
                } else {
                    IconName::ChevronDown
                }
                .svg(c.muted)
                .size(px(13.0)),
            );

        let more = icon_button_builder(("space-more", id), c, IconName::Ellipsis)
            .size(ButtonSize::IconXs)
            .shape(IconButtonShape::Square)
            .tooltip("Space actions")
            .render()
            .invisible()
            .group_hover("space-row", |s| s.visible())
            .on_click(cx.listener(move |this, ev: &gpui::ClickEvent, _w, cx| {
                cx.stop_propagation();
                this.space_actions_menu.open((id, ev.position()));
                cx.notify();
            }));

        let trailing = div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(tab_count.to_string()),
            )
            .child(more);

        ListItem::new(("space-header", id), c)
            .selected(active)
            .child(chevron)
            .child(name_slot)
            .trailing(trailing)
            .on_click(cx.listener(move |this, _, _w, cx| {
                if this.spaces_menu.is_open() {
                    this.close_spaces_menu(cx);
                }
                this.switch_space(id, cx);
            }))
            .extra(|row| row.group("space-row"))
    }

    /// The collapsible "Recent Projects" section shared by the Spaces
    /// popover and the Tabs sidebar footer: project folders previously
    /// opened via a Space, independent of which Spaces are open right now —
    /// so a closed Space's project is still one click away. Renders nothing
    /// when the list is empty.
    pub(crate) fn render_recent_projects_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let c = Palette::from_theme(self.theme.read(cx));
        let entries = self.recent_projects.entries().to_vec();
        if entries.is_empty() {
            return Vec::new();
        }
        let open = self.recent_projects_open;

        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        rows.push(
            div()
                .id("recent-projects-header")
                .flex()
                .items_center()
                .gap_1()
                .w_full()
                .pl_2()
                .rounded(px(c.radius.md))
                .cursor_pointer()
                .hover(|s| s.bg(c.accent))
                .on_click(cx.listener(|this, _, _w, cx| {
                    this.recent_projects_open = !this.recent_projects_open;
                    cx.notify();
                }))
                .child(
                    if open {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    }
                    .svg(c.muted)
                    .size(px(13.0)),
                )
                .child(list_header("Recent Projects", c.muted))
                .into_any_element(),
        );
        if open {
            for (idx, entry) in entries.iter().enumerate() {
                let root = entry.root.clone();
                let remove_root = entry.root.clone();
                let name = SharedString::from(entry.name.clone());
                let remove = icon_button_builder(("recent-remove", idx), c, IconName::X)
                    .size(ButtonSize::IconXs)
                    .shape(IconButtonShape::Square)
                    .tooltip("Remove from recent")
                    .render()
                    .invisible()
                    .group_hover("recent-project-row", |s| s.visible())
                    .on_click(cx.listener(move |this, _, _w, cx| {
                        cx.stop_propagation();
                        this.recent_projects.remove(&remove_root);
                        recent_projects::save(&this.recent_projects);
                        cx.notify();
                    }));
                rows.push(
                    ListItem::new(("recent-project", idx), c)
                        .icon(IconName::FolderOpen)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(name),
                        )
                        .trailing(remove)
                        .on_click(cx.listener(move |this, _, _w, cx| {
                            this.close_spaces_menu(cx);
                            this.open_or_create_project_space(root.clone(), cx);
                        }))
                        .extra(move |row| {
                            row.group("recent-project-row")
                                .pl(px(NESTED_INDENT))
                                .text_color(c.muted)
                        })
                        .into_any_element(),
                );
            }
        }
        rows
    }

    /// "New space" and "Open Local Folder" as list rows; `close` dismisses the
    /// surface the rows live in.
    fn space_footer_rows(
        &mut self,
        c: Palette,
        close: fn(&mut Workspace, &mut Context<Workspace>),
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        vec![
            ListItem::new("spaces-new-footer", c)
                .icon(IconName::PlusBold)
                .child("New space")
                .on_click(cx.listener(move |this, _, _w, cx| {
                    close(this, cx);
                    this.create_space_standalone(cx);
                }))
                .into_any_element(),
            ListItem::new("spaces-new-project-footer", c)
                .icon(IconName::FolderOpen)
                .child("Open Local Folder")
                .on_click(cx.listener(move |this, _, _w, cx| {
                    close(this, cx);
                    this.request_new_project_space(cx);
                }))
                .into_any_element(),
        ]
    }

    /// The Tabs sidebar's "New Space" dropdown: a search field over Recent
    /// Projects (reusing the fuzzy-free substring filter Spaces already
    /// keeps), with "New Space" / "Open Local Folder" as footer actions —
    /// the sidebar's dedicated space-creation surface, replacing the old
    /// three stacked "New Tab" / "New Space" / "New Space from Folder…"
    /// footer rows with a single icon trigger (`sidebar-new-space`).
    pub(crate) fn render_new_space_menu(
        &mut self,
        anchor: gpui::Point<gpui::Pixels>,
        closing: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let view = cx.entity();
        let query = self.new_space_search.clone();
        let matches = self.filtered_recent_projects();

        let mut rows: Vec<gpui::AnyElement> = Vec::new();

        rows.push(
            div()
                .id("new-space-search")
                .track_focus(&self.new_space_search_focus)
                .key_context("NewSpaceSearch")
                .on_key_down(cx.listener(Self::on_new_space_search_key))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, window, cx| {
                        // Explicit re-focus, see the space rename field.
                        window.focus(&this.new_space_search_focus);
                        cx.stop_propagation();
                    }),
                )
                .mb_1()
                .px_2()
                .h(px(28.0))
                .rounded(px(c.radius.md))
                .border_1()
                .border_color(c.border)
                .bg(c.input)
                .flex()
                .items_center()
                .text_size(px(13.0))
                .when(query.is_empty(), |d| {
                    d.text_color(c.muted).child("Search projects\u{2026}")
                })
                .when(!query.is_empty(), |d| {
                    d.text_color(c.fg).child(SharedString::from(query.clone()))
                })
                .when(self.new_space_search_blink.read(cx).visible(), |d| {
                    d.child(caret(c.fg, 14.0))
                })
                .into_any_element(),
        );

        if matches.is_empty() {
            rows.push(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(13.0))
                    .text_color(c.muted)
                    .child(if self.recent_projects.entries().is_empty() {
                        "No recent projects"
                    } else {
                        "No matches"
                    })
                    .into_any_element(),
            );
        } else {
            for (idx, entry) in matches.iter().enumerate() {
                let root = entry.root.clone();
                let name = SharedString::from(entry.name.clone());
                rows.push(
                    ListItem::new(("new-space-recent", idx), c)
                        .icon(IconName::FolderOpen)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(name),
                        )
                        .on_click(cx.listener(move |this, _, _w, cx| {
                            this.close_new_space_menu(cx);
                            this.open_or_create_project_space(root.clone(), cx);
                        }))
                        .into_any_element(),
                );
            }
        }

        let footer = self.space_footer_rows(c, Workspace::close_new_space_menu, cx);
        let content = div()
            .flex()
            .flex_col()
            .max_h(px(POPOVER_MAX_HEIGHT))
            .p_1()
            .child(
                div()
                    .id("new-space-menu-list")
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .flex_shrink()
                    .overflow_y_scroll()
                    .children(rows),
            )
            .child(divider(Axis::Horizontal, c.line_subtle).my_1())
            .children(footer)
            .into_any_element();

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.close_new_space_menu(cx);
                });
            }
        };
        popover_with_exit(anchor, px(POPOVER_WIDTH), c, closing, dismiss, content)
    }

    /// The Spaces popover (T20-008): a switcher for every open Space,
    /// collapsible, each listing its tabs — the reference Warp-style
    /// "Spaces" switcher's compact presentation of the same data the Tabs
    /// sidebar shows expanded (`render_tab_list_vertical`).
    pub(crate) fn render_spaces_popover(
        &mut self,
        anchor: gpui::Point<gpui::Pixels>,
        closing: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let spaces = self.spaces.read(cx).spaces().to_vec();
        let tabs = self.tabs.read(cx).tabs().to_vec();
        let active_tab = self.tabs.read(cx).active_id();
        let active_space = self.spaces.read(cx).active_id();
        let view = cx.entity();

        let new_space = icon_button_builder("spaces-new", c, IconName::PlusBold)
            .size(ButtonSize::IconXs)
            .shape(IconButtonShape::Square)
            .tooltip("New space")
            .render()
            .on_click(cx.listener(|this, _, _w, cx| {
                this.close_spaces_menu(cx);
                this.create_space_standalone(cx);
            }));
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .pr_1()
            .child(list_header("Spaces", c.muted))
            .child(new_space);

        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        for space in &spaces {
            let count = tabs.iter().filter(|t| t.space_id == space.id).count();
            rows.push(
                self.render_space_header(space, count, true, cx)
                    .into_any_element(),
            );
            if self.collapsed_spaces.contains(&space.id) {
                continue;
            }
            for tab in tabs.iter().filter(|t| t.space_id == space.id) {
                let tab_id = tab.id;
                let space_id = space.id;
                let current = tab_id == active_tab && space_id == active_space;
                let display = crate::tab_strip::tab_display(tab, self, cx);
                rows.push(
                    ListItem::new(("space-tab", tab_id), c)
                        .icon(display.icon)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(display.label),
                        )
                        .on_click(cx.listener(move |this, _, _w, cx| {
                            this.close_spaces_menu(cx);
                            this.switch_space(space_id, cx);
                            this.tabs.update(cx, |s, cx| s.set_active(tab_id, cx));
                        }))
                        .extra(move |row| {
                            row.pl(px(NESTED_INDENT)).text_color(if current {
                                c.fg
                            } else {
                                c.muted
                            })
                        })
                        .into_any_element(),
                );
            }
        }
        rows.extend(self.render_recent_projects_section(cx));

        let footer = self.space_footer_rows(c, Workspace::close_spaces_menu, cx);
        let content = div()
            .flex()
            .flex_col()
            .max_h(px(POPOVER_MAX_HEIGHT))
            .p_1()
            .child(header)
            .child(divider(Axis::Horizontal, c.line_subtle).my_1())
            .child(
                div()
                    .id("spaces-popover-list")
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .flex_shrink()
                    .overflow_y_scroll()
                    .children(rows),
            )
            .child(divider(Axis::Horizontal, c.line_subtle).my_1())
            .children(footer)
            .into_any_element();

        // A click inside the per-space "..." menu is outside this card; it must
        // not tear the popover down (Rename needs the inline field to stay).
        let dismiss = move |_w: &mut Window, cx: &mut App| {
            view.update(cx, |this, cx| {
                if this.space_actions_menu.mounted().is_none() {
                    this.close_spaces_menu(cx);
                }
            });
        };
        popover_with_exit(anchor, px(POPOVER_WIDTH), c, closing, dismiss, content)
    }

    /// The per-space "..." menu: new tab in the space, rename, close.
    pub(crate) fn render_space_actions_menu(
        &mut self,
        id: crate::spaces::SpaceId,
        anchor: gpui::Point<gpui::Pixels>,
        closing: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let view = cx.entity();
        let mut items = vec![
            MenuItem::new("space-new-tab", "New Tab")
                .icon(IconName::PlusBold)
                .on_click({
                    let v = view.clone();
                    move |_, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_space_actions_menu(cx);
                            this.close_spaces_menu(cx);
                            this.switch_space(id, cx);
                            this.open_new_tab_menu(anchor, cx);
                        })
                    }
                }),
            MenuItem::new("space-rename", "Rename")
                .icon(IconName::Pencil)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_space_actions_menu(cx);
                            this.begin_space_rename(id, w, cx);
                        })
                    }
                }),
        ];
        if id != DEFAULT_SPACE_ID {
            items.push(MenuItem::separator());
            items.push(MenuItem::new("space-close", "Close Space").on_click({
                let v = view.clone();
                move |_, w, cx| {
                    v.update(cx, |this, cx| {
                        this.close_space_actions_menu(cx);
                        this.close_spaces_menu(cx);
                        this.close_space(id, w, cx);
                    })
                }
            }));
        }
        let dismiss = move |_w: &mut Window, cx: &mut App| {
            view.update(cx, |this, cx| this.close_space_actions_menu(cx));
        };
        context_menu_with_exit(anchor, c, closing, dismiss, items)
    }
}
