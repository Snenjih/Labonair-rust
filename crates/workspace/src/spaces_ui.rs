//! Spaces UI: the Spaces popover, space headers, the Recent Projects section
//! and the sidebar "New Space" dropdown (R10-002).

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_ui_kit::{caret, IconName, Palette};

use crate::recent_projects;
use crate::spaces::{Space, DEFAULT_SPACE_ID};
use crate::Workspace;

impl Workspace {
    /// One space's header row (colored initial badge, name, tab count,
    /// collapse chevron) — shared by the Spaces popover and the Tabs sidebar
    /// (T20-008), so both presentations stay identical. Clicking the row
    /// body switches to it; clicking the chevron only toggles collapse.
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

        let badge = div()
            .flex_none()
            .size(px(20.0))
            .rounded_sm()
            .bg(space.color.hsla())
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.0))
            .text_color(gpui::white())
            .child(
                space
                    .name
                    .chars()
                    .next()
                    .map(|ch| ch.to_uppercase().to_string())
                    .unwrap_or_default(),
            );

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
                    .text_sm()
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
                    .text_sm()
                    .child(space.name.clone())
                    .into_any_element(),
            };

        div()
            .id(("space-header", id))
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .px_2()
            .py_1p5()
            .rounded_md()
            .cursor_pointer()
            .when(active, |d| d.bg(c.accent))
            .when(!active, |d| d.hover(|s| s.bg(c.muted_bg)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _w, cx| {
                    this.spaces_menu = None;
                    this.switch_space(id, cx);
                }),
            )
            .child(
                div()
                    .id(("space-collapse", id))
                    .flex_none()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _w, cx| {
                            cx.stop_propagation();
                            if this.collapsed_spaces.contains(&id) {
                                this.collapsed_spaces.remove(&id);
                            } else {
                                this.collapsed_spaces.insert(id);
                            }
                            cx.notify();
                        }),
                    )
                    .child(
                        if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        }
                        .svg(c.muted)
                        .size(px(13.0)),
                    ),
            )
            .child(badge)
            .child(name_slot)
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(tab_count.to_string()),
            )
            .child(
                div()
                    .id(("space-new-tab-btn", id))
                    .flex_none()
                    .invisible()
                    .group_hover("space-row", |s| s.visible())
                    .px_1()
                    .py_1()
                    .rounded_sm()
                    .hover(|s| s.bg(c.muted_bg))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                            cx.stop_propagation();
                            this.spaces_menu = None;
                            this.switch_space(id, cx);
                            this.new_tab_menu = Some(ev.position);
                            this.new_tab_submenu = None;
                            this.context_menu = None;
                            cx.notify();
                        }),
                    )
                    .child(IconName::PlusBold.svg(c.muted).size(px(12.0))),
            )
            .child(
                div()
                    .id(("space-rename-btn", id))
                    .flex_none()
                    .invisible()
                    .group_hover("space-row", |s| s.visible())
                    .px_1()
                    .py_1()
                    .rounded_sm()
                    .hover(|s| s.bg(c.muted_bg))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, w, cx| {
                            cx.stop_propagation();
                            this.begin_space_rename(id, w, cx);
                        }),
                    )
                    .child(IconName::Pencil.svg(c.muted).size(px(12.0))),
            )
            .when(id != DEFAULT_SPACE_ID, |d| {
                d.child(
                    div()
                        .id(("space-close-btn", id))
                        .flex_none()
                        .invisible()
                        .group_hover("space-row", |s| s.visible())
                        .px_1()
                        .py_1()
                        .rounded_sm()
                        .text_color(c.muted)
                        .hover(|s| s.bg(c.muted_bg).text_color(c.fg))
                        .child("\u{2715}")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseDownEvent, w, cx| {
                                cx.stop_propagation();
                                this.spaces_menu = None;
                                this.close_space(id, w, cx);
                            }),
                        ),
                )
            })
            .group("space-row")
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
        let view = cx.entity();

        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        rows.push(
            div()
                .id("recent-projects-header")
                .flex()
                .items_center()
                .gap_2()
                .w_full()
                .px_2()
                .py_1p5()
                .rounded_md()
                .cursor_pointer()
                .hover(|s| s.bg(c.muted_bg))
                .on_mouse_down(MouseButton::Left, {
                    let v = view.clone();
                    move |_: &MouseDownEvent, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.recent_projects_open = !this.recent_projects_open;
                            cx.notify();
                        });
                    }
                })
                .child(
                    if open {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    }
                    .svg(c.muted)
                    .size(px(13.0)),
                )
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(c.muted)
                        .child("Recent Projects"),
                )
                .into_any_element(),
        );
        if open {
            for (idx, entry) in entries.iter().enumerate() {
                let root = entry.root.clone();
                let name = SharedString::from(entry.name.clone());
                rows.push(
                    div()
                        .id(("recent-project", idx))
                        .flex()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .pl_6()
                        .pr_2()
                        .py_1()
                        .rounded_md()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(c.muted)
                        .hover(|s| s.bg(c.muted_bg).text_color(c.fg))
                        .child(IconName::FolderOpen.svg(c.muted).size(px(13.0)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(name.clone()),
                        )
                        .child(
                            div()
                                .id(("recent-project-remove", idx))
                                .flex_none()
                                .invisible()
                                .group_hover("recent-project-row", |s| s.visible())
                                .px_1()
                                .rounded_sm()
                                .hover(|s| s.bg(c.border))
                                .child("\u{2715}")
                                .on_mouse_down(MouseButton::Left, {
                                    let v = view.clone();
                                    let root = root.clone();
                                    move |_: &MouseDownEvent, _w, cx| {
                                        cx.stop_propagation();
                                        v.update(cx, |this, cx| {
                                            this.recent_projects.remove(&root);
                                            recent_projects::save(&this.recent_projects);
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .on_mouse_down(MouseButton::Left, {
                            let v = view.clone();
                            let root = root.clone();
                            move |_: &MouseDownEvent, _w, cx| {
                                v.update(cx, |this, cx| {
                                    this.spaces_menu = None;
                                    this.open_or_create_project_space(root.clone(), cx);
                                });
                            }
                        })
                        .group("recent-project-row")
                        .into_any_element(),
                );
            }
        }
        rows
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
                        // Explicit re-focus, not just `stop_propagation` — GPUI's
                        // automatic focus-on-click for `track_focus` and this
                        // listener share the same mouse-down dispatch pass, and
                        // `stop_propagation` aborts it before the automatic
                        // focus transfer runs, silently killing click-to-refocus.
                        window.focus(&this.new_space_search_focus);
                        cx.stop_propagation();
                    }),
                )
                .mb_1()
                .px_2()
                .h(px(28.0))
                .rounded_md()
                .border_1()
                .border_color(c.border)
                .bg(c.input)
                .flex()
                .items_center()
                .text_sm()
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
                    .text_sm()
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
                    div()
                        .id(("new-space-recent", idx))
                        .flex()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(c.fg)
                        .hover(|s| s.bg(c.muted_bg))
                        .child(IconName::FolderOpen.svg(c.muted).size(px(13.0)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(name),
                        )
                        .on_mouse_down(MouseButton::Left, {
                            let v = view.clone();
                            let root = root.clone();
                            move |_: &MouseDownEvent, _w, cx| {
                                v.update(cx, |this, cx| {
                                    this.close_new_space_menu(cx);
                                    this.open_or_create_project_space(root.clone(), cx);
                                });
                            }
                        })
                        .into_any_element(),
                );
            }
        }

        rows.push(
            div()
                .mt_1()
                .pt_1()
                .border_t_1()
                .border_color(c.border)
                .into_any_element(),
        );
        rows.push(
            div()
                .id("new-space-create")
                .flex()
                .items_center()
                .gap_1p5()
                .w_full()
                .px_2()
                .py_1p5()
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(c.muted)
                .hover(|s| s.bg(c.muted_bg).text_color(c.fg))
                .child(IconName::PlusBold.svg(c.muted).size(px(13.0)))
                .child("New Space")
                .on_mouse_down(MouseButton::Left, {
                    let v = view.clone();
                    move |_: &MouseDownEvent, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_new_space_menu(cx);
                            this.create_space_standalone(cx);
                        })
                    }
                })
                .into_any_element(),
        );
        rows.push(
            div()
                .id("new-space-open-folder")
                .flex()
                .items_center()
                .gap_1p5()
                .w_full()
                .px_2()
                .py_1p5()
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(c.muted)
                .hover(|s| s.bg(c.muted_bg).text_color(c.fg))
                .child(IconName::FolderOpen.svg(c.muted).size(px(13.0)))
                .child("Open Local Folder")
                .on_mouse_down(MouseButton::Left, {
                    let v = view.clone();
                    move |_: &MouseDownEvent, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.close_new_space_menu(cx);
                            this.request_new_project_space(cx);
                        })
                    }
                })
                .into_any_element(),
        );

        let content = div()
            .id("new-space-menu-list")
            .flex()
            .flex_col()
            .max_h(px(420.0))
            .overflow_y_scroll()
            .children(rows)
            .into_any_element();

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.close_new_space_menu(cx);
                });
            }
        };
        labonair_ui_kit::popover(anchor, px(280.0), c, dismiss, content)
    }

    /// The Spaces popover (T20-008): a switcher for every open Space,
    /// collapsible, each listing its tabs — the reference Warp-style
    /// "Spaces" switcher's compact presentation of the same data the Tabs
    /// sidebar shows expanded (`render_tab_list_vertical`).
    pub(crate) fn render_spaces_popover(
        &mut self,
        anchor: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let spaces = self.spaces.read(cx).spaces().to_vec();
        let tabs = self.tabs.read(cx).tabs().to_vec();
        let view = cx.entity();

        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        rows.push(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .py_1()
                .child(div().text_sm().text_color(c.muted).child("Spaces"))
                .child(
                    div()
                        .id("spaces-new")
                        .cursor_pointer()
                        .rounded_sm()
                        .p_0p5()
                        .hover(|s| s.bg(c.muted_bg))
                        .child(IconName::PlusBold.svg(c.muted).size(px(14.0)))
                        .on_mouse_down(MouseButton::Left, {
                            let v = view.clone();
                            move |_: &MouseDownEvent, _w, cx| {
                                v.update(cx, |this, cx| {
                                    this.spaces_menu = None;
                                    this.create_space_standalone(cx);
                                })
                            }
                        }),
                )
                .into_any_element(),
        );
        rows.push(div().my_1().h(px(1.0)).bg(c.border).into_any_element());

        for space in &spaces {
            let count = tabs.iter().filter(|t| t.space_id == space.id).count();
            rows.push(
                self.render_space_header(space, count, true, cx)
                    .into_any_element(),
            );
            if !self.collapsed_spaces.contains(&space.id) {
                for tab in tabs.iter().filter(|t| t.space_id == space.id) {
                    let tab_id = tab.id;
                    let space_id = space.id;
                    let label = SharedString::from(tab.label());
                    let icon = tab.kind.indicator();
                    rows.push(
                        div()
                            .id(("space-tab", tab_id))
                            .flex()
                            .items_center()
                            .gap_2()
                            .w_full()
                            .pl_6()
                            .pr_2()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .text_sm()
                            .text_color(c.muted)
                            .hover(|s| s.bg(c.muted_bg).text_color(c.fg))
                            .child(icon.svg(c.muted).size(px(13.0)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(label),
                            )
                            .on_mouse_down(MouseButton::Left, {
                                let v = view.clone();
                                move |_: &MouseDownEvent, _w, cx| {
                                    v.update(cx, |this, cx| {
                                        this.spaces_menu = None;
                                        this.switch_space(space_id, cx);
                                        this.tabs.update(cx, |s, cx| s.set_active(tab_id, cx));
                                    })
                                }
                            })
                            .into_any_element(),
                    );
                }
            }
        }

        rows.extend(self.render_recent_projects_section(cx));

        rows.push(
            div()
                .mt_1()
                .pt_1()
                .border_t_1()
                .border_color(c.border)
                .into_any_element(),
        );
        rows.push(
            div()
                .id("spaces-new-footer")
                .flex()
                .items_center()
                .gap_1p5()
                .px_2()
                .py_1p5()
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(c.muted)
                .hover(|s| s.bg(c.muted_bg).text_color(c.fg))
                .child(IconName::PlusBold.svg(c.muted).size(px(13.0)))
                .child("New space")
                .on_mouse_down(MouseButton::Left, {
                    let v = view.clone();
                    move |_: &MouseDownEvent, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.spaces_menu = None;
                            this.create_space_standalone(cx);
                        })
                    }
                })
                .into_any_element(),
        );
        rows.push(
            div()
                .id("spaces-new-project-footer")
                .flex()
                .items_center()
                .gap_1p5()
                .px_2()
                .py_1p5()
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(c.muted)
                .hover(|s| s.bg(c.muted_bg).text_color(c.fg))
                .child(IconName::FolderOpen.svg(c.muted).size(px(13.0)))
                .child("Open Local Folder")
                .on_mouse_down(MouseButton::Left, {
                    let v = view.clone();
                    move |_: &MouseDownEvent, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.spaces_menu = None;
                            this.request_new_project_space(cx);
                        })
                    }
                })
                .into_any_element(),
        );

        let content = div()
            .id("spaces-popover-list")
            .flex()
            .flex_col()
            .max_h(px(420.0))
            .overflow_y_scroll()
            .children(rows)
            .into_any_element();

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.spaces_menu = None;
                    cx.notify();
                });
            }
        };
        labonair_ui_kit::popover(anchor, px(320.0), c, dismiss, content)
    }
}
