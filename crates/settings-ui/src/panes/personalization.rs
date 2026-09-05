//! The "Personalization" pane (T18-007): bundles the statusbar layout editor
//! (move items left/right/hide) and per-panel toggle-bar visibility switches
//! into one overviewable page, replacing the scattered right-click menus as
//! the *discoverable* entry point. Both sections write through the exact same
//! [`labonair_workspace::Workspace`] methods the in-app right-click menus
//! use (`set_status_bar_placement` / `set_panel_toggle_visible`) — there is
//! no second write path.
//!
//! Hand-built like the Theme/Shortcuts panes (`SettingsPageItem::Custom`);
//! the generic field-renderer infrastructure lands in T19-004.

use labonair_panel::{PanelIcon, StatusSide};
use labonair_ui_kit::IconName;

use crate::view::*;

/// Human-readable title for a placeable [`StatusItem`](labonair_panel::StatusItem)
/// id. Mirrors `labonair_shell::status_items::status_item_label` — duplicated
/// here (read-only, 9 entries) because `labonair-settings-ui` cannot depend on
/// `labonair-shell` (the dependency runs the other way).
fn status_item_title(id: &str) -> &'static str {
    match id {
        "notifications" => "Notifications",
        "cwd" => "CWD Breadcrumb",
        "cursor-position" => "Cursor Position",
        "preview-url" => "Preview URL",
        "updater" => "Updater",
        "transfers" => "Transfers",
        "agent-access" => "Agent Access",
        "jump-hosts" => "Jump Hosts",
        "bookmarks" => "Bookmarks",
        _ => "Status Bar Item",
    }
}

/// Icon for a placeable status-item id, matching the glyph the item itself
/// renders in the status bar (`crates/shell/src/status_items.rs`).
fn status_item_icon(id: &str) -> IconName {
    match id {
        "notifications" => IconName::Bell,
        "cwd" => IconName::Home,
        "cursor-position" => IconName::Type,
        "preview-url" => IconName::Globe,
        "updater" => IconName::Download,
        "transfers" => IconName::ArrowDownUp,
        "agent-access" => IconName::Shield,
        "jump-hosts" => IconName::Server,
        "bookmarks" => IconName::Bookmark,
        _ => IconName::Square,
    }
}

/// Human-readable title for a registered panel's persistent name. Mirrors
/// `labonair_shell::status_items::panel_toggle_title` — same duplication
/// rationale as [`status_item_title`].
fn panel_title(persistent_name: &str) -> &'static str {
    match persistent_name {
        "explorer" => "Explorer",
        "source-control" => "Source Control",
        "git-graph" => "Git Graph",
        "snippets" => "Snippets",
        "ai" => "AI",
        _ => "Panel",
    }
}

fn panel_icon(icon: PanelIcon) -> IconName {
    match icon {
        PanelIcon::Explorer => IconName::FolderTree,
        PanelIcon::SourceControl => IconName::GitBranch,
        PanelIcon::GitGraph => IconName::GitCompare,
        PanelIcon::Snippets => IconName::Zap,
        PanelIcon::Ai => IconName::MessageSquare,
    }
}

/// One placed statusbar item, resolved for rendering.
struct PlacedItem {
    id: &'static str,
    side: StatusSide,
    hidden: bool,
}

/// A small text-glyph action button on a statusbar-layout chip ("←" / "→") —
/// the shared `button()` primitive (`Ghost`/`IconXs`).
fn statusbar_glyph_btn(
    item_id: &'static str,
    glyph_id: &'static str,
    glyph: &'static str,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, &mut Context<SettingsView>) + 'static,
) -> impl IntoElement {
    button(
        SharedString::from(format!("pers-{item_id}-{glyph_id}")),
        *c,
        ButtonVariant::Ghost,
        ButtonSize::IconXs,
    )
    .child(glyph)
    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| f(this, cx)))
}

/// A small icon action button on a statusbar-layout chip (hide / show) — the
/// shared `button()` primitive (`Ghost`/`IconXs`).
fn statusbar_icon_btn(
    item_id: &'static str,
    glyph_id: &'static str,
    icon: IconName,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, &mut Context<SettingsView>) + 'static,
) -> impl IntoElement {
    button(
        SharedString::from(format!("pers-{item_id}-{glyph_id}")),
        *c,
        ButtonVariant::Ghost,
        ButtonSize::IconXs,
    )
    .child(icon.svg(c.muted))
    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| f(this, cx)))
}

impl SettingsView {
    pub(crate) fn render_personalization(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // T19-004: the 7 legacy per-button status-bar toggles are plain
        // `bool` `SettingsContent` fields (`PERSONALIZATION_GROUPS`) — fold
        // the generic grid in above the bespoke layout editor/visibility
        // grid, which own the two `BTreeMap` fields directly (rule 4).
        let generic_grid =
            self.render_field_groups(PERSONALIZATION_GROUPS, "personalization", c, cx);
        div()
            .flex()
            .flex_col()
            .child(generic_grid)
            .child(list_header("Statusbar Layout", c.muted))
            .child(div().text_size(px(11.0)).text_color(c.muted).pb_2().child(
                "Move status bar items between the left and right cluster, or hide them. \
                         The per-dock panel buttons sit next to the dock they control and \
                         aren't listed here.",
            ))
            .child(self.render_statusbar_layout_editor(c, cx))
            .child(list_header("Panel Visibility", c.muted))
            .child(div().text_size(px(11.0)).text_color(c.muted).pb_2().child(
                "Panels hidden here stay reachable from the command palette — this only \
                         controls whether they get a toggle button in the status bar.",
            ))
            .child(self.render_panel_visibility(c, cx))
            .into_any_element()
    }

    fn render_statusbar_layout_editor(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let items: Vec<PlacedItem> = {
            let ws = self.workspace.read(cx);
            let registry = ws.status_item_registry();
            registry
                .iter()
                .filter(|reg| {
                    !matches!(
                        reg.id,
                        "dock-buttons-left" | "dock-buttons-right" | "dock-buttons-bottom"
                    )
                })
                .map(|reg| PlacedItem {
                    id: reg.id,
                    side: registry.resolve_side(reg.id),
                    hidden: registry.is_hidden(reg.id),
                })
                .collect()
        };

        let left: Vec<&'static str> = items
            .iter()
            .filter(|i| !i.hidden && i.side == StatusSide::Left)
            .map(|i| i.id)
            .collect();
        let right: Vec<&'static str> = items
            .iter()
            .filter(|i| !i.hidden && i.side == StatusSide::Right)
            .map(|i| i.id)
            .collect();
        let hidden: Vec<&'static str> = items.iter().filter(|i| i.hidden).map(|i| i.id).collect();

        let columns = div()
            .flex()
            .gap_3()
            .child(self.render_column("Left", left, c, cx))
            .child(self.render_column("Right", right, c, cx))
            .child(self.render_column("Hidden", hidden, c, cx));

        div()
            .flex()
            .flex_col()
            .gap_2()
            .pb_3()
            .child(columns)
            .child(
                // T20-003: shared `button()` primitive (`Outline`/`Xs`).
                button(
                    "personalization-reset",
                    *c,
                    ButtonVariant::Outline,
                    ButtonSize::Xs,
                )
                .mt_1()
                .child("Reset to default")
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.workspace
                        .update(cx, |w, cx| w.reset_status_bar_placements(cx));
                })),
            )
            .into_any_element()
    }

    fn render_column(
        &self,
        title: &'static str,
        ids: Vec<&'static str>,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let is_hidden_col = title == "Hidden";
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(10.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c.muted)
                    .child(title),
            )
            .when(ids.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.muted.opacity(0.7))
                        .child("\u{2014}"),
                )
            })
            .children(
                ids.into_iter()
                    .map(|id| self.render_statusbar_chip(id, is_hidden_col, c, cx)),
            )
            .into_any_element()
    }

    fn render_statusbar_chip(
        &self,
        id: &'static str,
        is_hidden_col: bool,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py(px(4.0))
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(c.bg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .min_w_0()
                    .child(status_item_icon(id).svg(c.muted))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.fg)
                            .truncate()
                            .child(status_item_title(id)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .when(!is_hidden_col, |row| {
                        row.child(statusbar_glyph_btn(
                            id,
                            "left",
                            "\u{2190}",
                            c,
                            cx,
                            move |this, cx| {
                                this.workspace.update(cx, |w, cx| {
                                    w.set_status_bar_placement(
                                        id,
                                        Some(StatusSide::Left),
                                        None,
                                        cx,
                                    );
                                });
                            },
                        ))
                        .child(statusbar_glyph_btn(
                            id,
                            "right",
                            "\u{2192}",
                            c,
                            cx,
                            move |this, cx| {
                                this.workspace.update(cx, |w, cx| {
                                    w.set_status_bar_placement(
                                        id,
                                        Some(StatusSide::Right),
                                        None,
                                        cx,
                                    );
                                });
                            },
                        ))
                        .child(statusbar_icon_btn(
                            id,
                            "hide",
                            IconName::EyeOff,
                            c,
                            cx,
                            move |this, cx| {
                                this.workspace.update(cx, |w, cx| {
                                    w.set_status_bar_placement(id, None, Some(true), cx);
                                });
                            },
                        ))
                    })
                    .when(is_hidden_col, |row| {
                        row.child(statusbar_icon_btn(
                            id,
                            "show",
                            IconName::Eye,
                            c,
                            cx,
                            move |this, cx| {
                                this.workspace.update(cx, |w, cx| {
                                    w.set_status_bar_placement(id, None, Some(false), cx);
                                });
                            },
                        ))
                    }),
            )
            .into_any_element()
    }

    fn render_panel_visibility(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let panels: Vec<(&'static str, PanelIcon)> = {
            let ws = self.workspace.read(cx);
            ws.panel_registry()
                .iter()
                .map(|reg| (reg.persistent_name, reg.icon))
                .collect()
        };

        div()
            .flex()
            .flex_col()
            .children(panels.into_iter().map(|(name, icon)| {
                let visible = labonair_workspace::Workspace::panel_toggle_visible(name);
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .py_2()
                    .border_b_1()
                    .border_color(c.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(panel_icon(icon).svg(c.muted))
                            .child(div().text_color(c.fg).child(panel_title(name))),
                    )
                    .child(
                        // T20-003: the shared `gpui-component` `Switch`.
                        Switch::new(SharedString::from(format!("pers-panel-{name}")))
                            .checked(visible)
                            .on_click(cx.listener(move |this, _: &bool, _w, cx| {
                                this.workspace.update(cx, |w, cx| {
                                    w.set_panel_toggle_visible(name.to_string(), !visible, cx);
                                });
                            })),
                    )
            }))
            .into_any_element()
    }
}
