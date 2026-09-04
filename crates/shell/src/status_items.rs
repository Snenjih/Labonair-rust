//! Concrete [`StatusItem`]s + the single registration hook (T17-003).
//!
//! Each type here is a small self-describing status-bar view ported 1:1 from
//! the former `AppShell::render_*_item` methods. `labonair-shell` is the only
//! crate that names these concrete types;
//! [`register_builtin_status_items`] is the only place that lists them.
//!
//! The transitional `bar_items` placement blob (`BarLoc`, the
//! `barItemPlacements` → `statusBarItemPlacements` migrator, the right-click
//! "move to titlebar / hide" affordances) is deliberately *not* ported here —
//! that is T18-005 / T18-006. Items expose `default_side` + `order`; the
//! `StatusItemRegistry` resolves the rest.

use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, AppContext, ClickEvent, Context, Entity, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_panel::{
    AnyStatusItemHandle, PanelIcon, StatusItem, StatusItemRegistration, StatusSide,
};
use labonair_panel_explorer::BookmarksView;
use labonair_ui_kit::IconName;
use labonair_workspace::agent_access::{AgentAccessEntry, AgentAccessStore};

use crate::cwd_breadcrumb as bc;
use crate::theme::ThemeStore;
use crate::updater::{UpdaterStatus, UpdaterView};
use crate::workspace::Workspace;

/// A small icon-only status-bar button with the reference toggle styling.
#[allow(clippy::too_many_arguments)]
fn simple_bar_button<T: 'static>(
    key: &'static str,
    icon: IconName,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    cx: &mut Context<T>,
    on_click: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
) -> AnyElement {
    div()
        .id(key)
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(muted)
        .hover(|s| s.bg(border).text_color(fg))
        .child(icon.svg(muted))
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            on_click(this, window, cx);
        }))
        .into_any_element()
}

fn panel_toggle_icon(icon: PanelIcon) -> IconName {
    match icon {
        PanelIcon::Explorer => IconName::FolderTree,
        PanelIcon::SourceControl => IconName::GitBranch,
        PanelIcon::GitGraph => IconName::GitCompare,
        PanelIcon::Snippets => IconName::Zap,
        PanelIcon::Ai => IconName::MessageSquare,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel toggles (aggregate) — one toggle per registered panel.
// ─────────────────────────────────────────────────────────────────────────────

/// Panel title + rebindable shortcut, for the toggle's tooltip. Only
/// "explorer" (`SidebarToggle`) and "ai" (`AiToggle`) currently have a
/// dedicated shortcut (`crates/command-palette/src/keybind.rs`); the others
/// show the title alone, per the task's "keybind if set" wording.
fn panel_toggle_shortcut(persistent_name: &str) -> Option<labonair_command_palette::ShortcutId> {
    use labonair_command_palette::ShortcutId;
    match persistent_name {
        "explorer" => Some(ShortcutId::SidebarToggle),
        "ai" => Some(ShortcutId::AiToggle),
        _ => None,
    }
}

fn panel_toggle_title(persistent_name: &str) -> &'static str {
    match persistent_name {
        "explorer" => "Explorer",
        "source-control" => "Source Control",
        "git-graph" => "Git Graph",
        "snippets" => "Snippets",
        "ai" => "AI",
        _ => "Panel",
    }
}

pub struct PanelTogglesStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    /// `(panel name, anchor)` of an open dock/hide context menu, or `None`.
    dock_menu: Option<(SharedString, Point<Pixels>)>,
    /// Panels hidden from this toggle strip, mirrored from the persisted
    /// `panelToggleVisibility` blob (T18-007). Reloaded whenever
    /// `StatusBarLayoutTick` bumps — either this window's own write below, the
    /// Personalization settings pane, or another window.
    hidden: std::collections::HashSet<SharedString>,
}

impl PanelTogglesStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe_global::<labonair_workspace::status_placements::StatusBarLayoutTick>(
            |this, cx| {
                this.reload_hidden();
                cx.notify();
            },
        )
        .detach();
        let mut this = Self {
            workspace,
            theme,
            dock_menu: None,
            hidden: Default::default(),
        };
        this.reload_hidden();
        this
    }

    /// Re-reads the persisted `panelToggleVisibility` blob (T18-007).
    fn reload_hidden(&mut self) {
        self.hidden = labonair_backend::modules::settings::panel_toggle_visibility_load()
            .into_iter()
            .filter(|(_, v)| !v.as_bool().unwrap_or(true))
            .map(|(k, _)| SharedString::from(k))
            .collect();
    }

    fn open_dock_menu(&mut self, name: SharedString, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.dock_menu = Some((name, pos));
        cx.notify();
    }

    fn render_dock_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        use labonair_panel::DockPosition;
        use labonair_ui_kit::{context_menu, MenuItem};

        let (name, pos) = self.dock_menu.clone()?;
        let current = self.workspace.read(cx).dock_for_panel(name.as_ref(), cx);
        let view = cx.entity();
        let close = {
            let v = view.clone();
            move |cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.dock_menu = None;
                    cx.notify();
                })
            }
        };

        let dest_label = |d: DockPosition| match d {
            DockPosition::Left => "Dock left",
            DockPosition::Right => "Dock right",
            DockPosition::Bottom => "Dock bottom",
        };

        let mut items: Vec<MenuItem> = Vec::new();
        for d in DockPosition::ALL {
            if d == current {
                continue;
            }
            let move_name = name.clone();
            let ws = self.workspace.clone();
            let close = close.clone();
            items.push(
                MenuItem::new(
                    SharedString::from(format!("dock-move-{}", dest_label(d))),
                    dest_label(d),
                )
                .on_click(move |_, _w, cx| {
                    let move_name = move_name.clone();
                    ws.update(cx, |w, cx| {
                        if w.move_panel(move_name.as_ref(), d, cx) {
                            w.persist_docks(cx);
                            cx.notify();
                        }
                    });
                    close(cx);
                }),
            );
        }
        items.push(MenuItem::separator());
        let hide_name = name.clone();
        let ws_hide = self.workspace.clone();
        let close_hide = close.clone();
        items.push(MenuItem::new("dock-hide", "Hide from toggle bar").on_click(
            move |_, _w, cx| {
                let hide_name = hide_name.to_string();
                ws_hide.update(cx, |w, cx| {
                    w.set_panel_toggle_visible(hide_name, false, cx);
                });
                close_hide(cx);
            },
        ));

        let dismiss = move |_w: &mut Window, cx: &mut App| close(cx);
        Some(context_menu(pos, self.theme.read(cx), dismiss, items))
    }
}

impl Render for PanelTogglesStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for PanelTogglesStatusItem {
    fn id(&self) -> &'static str {
        "panel-toggles"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Left
    }
    fn order(&self) -> i32 {
        0
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (fg, muted, accent, border) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.muted_foreground(), t.accent(), t.border())
        };
        let keybind_overrides = cx
            .try_global::<labonair_command_palette::KeybindDisplay>()
            .map(|g| g.0.clone())
            .unwrap_or_default();
        let panels: Vec<(SharedString, IconName, bool)> = {
            let ws = self.workspace.read(cx);
            ws.panel_registry()
                .iter()
                .filter(|r| !self.hidden.contains(r.persistent_name))
                .map(|r| {
                    (
                        SharedString::from(r.persistent_name),
                        panel_toggle_icon(r.icon),
                        ws.panel_is_active(r.persistent_name),
                    )
                })
                .collect()
        };

        let dock_menu = self.render_dock_menu(cx);

        div()
            .relative()
            .flex()
            .items_center()
            .gap_0p5()
            .children(panels.into_iter().map(|(name, icon, active)| {
                let click_name = name.clone();
                let rmb_name = name.clone();
                let title = panel_toggle_title(name.as_ref());
                let keys = panel_toggle_shortcut(name.as_ref())
                    .map(|id| labonair_command_palette::effective_keys(id, &keybind_overrides))
                    .unwrap_or_default();
                let tooltip_text: SharedString = if keys.is_empty() {
                    SharedString::from(title)
                } else {
                    SharedString::from(format!("{title} ({})", keys.join("")))
                };
                div()
                    .id(SharedString::from(format!("bar-toggle-{name}")))
                    .size(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .when(active, |d| d.bg(accent.opacity(0.2)).text_color(fg))
                    .when(!active, |d| {
                        d.text_color(muted).hover(|s| s.bg(border).text_color(fg))
                    })
                    .child(icon.svg(if active { fg } else { muted }))
                    .tooltip(move |window, cx| {
                        labonair_ui_kit::Tooltip::new(tooltip_text.clone()).build(window, cx)
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        let name = click_name.clone();
                        this.workspace
                            .update(cx, |w, cx| w.select_panel(name.as_ref(), cx));
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                            this.open_dock_menu(rmb_name.clone(), ev.position, cx);
                        }),
                    )
            }))
            .children(dock_menu)
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Notifications badge + dropdown.
// ─────────────────────────────────────────────────────────────────────────────

pub struct NotificationsStatusItem {
    center: Entity<labonair_notifications::NotificationCenter>,
    theme: Entity<ThemeStore>,
    open: Option<Point<Pixels>>,
    focus: gpui::FocusHandle,
}

impl NotificationsStatusItem {
    pub fn new(
        center: Entity<labonair_notifications::NotificationCenter>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&center, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            center,
            theme,
            open: None,
            focus: cx.focus_handle(),
        }
    }
}

impl gpui::Focusable for NotificationsStatusItem {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl Render for NotificationsStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for NotificationsStatusItem {
    fn id(&self) -> &'static str {
        "notifications"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    // Rightmost of the right cluster — always visible, own group (T18-004
    // default order: … → Bookmarks → Notifications).
    fn order(&self) -> i32 {
        100
    }
    fn group(&self) -> u32 {
        2
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // Bell stays visible even at 0 (T18-004 point 1: notifications is
        // always shown, rightmost); only the badge disappears (point 3).
        let count = self.center.read(cx).len();
        let (fg, muted, accent, border) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.muted_foreground(), t.accent(), t.border())
        };

        let bell = div()
            .id("bar-notifications")
            .track_focus(&self.focus)
            .key_context("StatusPopover")
            .relative()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(muted)
            .hover(|s| s.bg(border).text_color(fg))
            .child(IconName::Bell.svg(muted))
            .when(count > 0, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(-2.0))
                        .right(px(-2.0))
                        .min_w(px(13.0))
                        .h(px(13.0))
                        .px(px(2.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(accent)
                        .text_color(fg)
                        .text_size(px(8.0))
                        .child(SharedString::from(count.to_string())),
                )
            })
            .on_click(cx.listener(|this, ev: &ClickEvent, w, cx| {
                if this.open.is_some() {
                    this.open = None;
                } else {
                    this.open = Some(ev.position());
                    w.focus(&this.focus);
                }
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _w, cx| {
                if this.open.is_some() && ev.keystroke.key == "escape" {
                    this.open = None;
                    cx.notify();
                    cx.stop_propagation();
                }
            }));

        let Some(anchor) = self.open else {
            return bell.into_any_element();
        };

        let (fg2, muted2, border2) = (fg, muted, border);
        let snapshots = self.center.read(cx).snapshots();
        let view = cx.entity();
        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.open = None;
                    cx.notify();
                })
            }
        };
        let content = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border2)
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(fg2)
                    .child("Notifications")
                    .child(
                        div()
                            .id("bar-notif-clear")
                            .text_xs()
                            .text_color(muted2)
                            .hover(|s| s.text_color(fg2))
                            .child("Clear all")
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.center.update(cx, |n, cx| n.clear_all(cx));
                                this.open = None;
                                cx.notify();
                            })),
                    ),
            )
            .children(snapshots.into_iter().take(6).map(|s| {
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .px_3()
                    .py_1p5()
                    .border_b_1()
                    .border_color(border2)
                    .child(
                        div()
                            .text_xs()
                            .text_color(fg2)
                            .child(SharedString::from(s.title.to_string())),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(muted2)
                            .child(SharedString::from(s.body.to_string())),
                    )
            }))
            .into_any_element();

        div()
            .relative()
            .flex_shrink_0()
            .child(bell)
            .child(labonair_ui_kit::popover(
                anchor,
                px(300.0),
                self.theme.read(cx),
                dismiss,
                content,
            ))
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CWD breadcrumb (its own state: expanded, segment menu, subdir dropdown).
// ─────────────────────────────────────────────────────────────────────────────

pub struct CwdStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    expanded: bool,
    crumb_menu: Option<(bc::Segment, Point<Pixels>)>,
    subdir_menu: Option<(String, Point<Pixels>, Option<Vec<String>>)>,
}

impl CwdStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            workspace,
            theme,
            expanded: false,
            crumb_menu: None,
            subdir_menu: None,
        }
    }

    fn home_dir() -> Option<String> {
        dirs::home_dir().map(|p| p.to_string_lossy().into_owned())
    }

    fn open_crumb_menu(&mut self, seg: bc::Segment, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.crumb_menu = Some((seg, pos));
        self.subdir_menu = None;
        cx.notify();
    }

    fn open_subdir_menu(&mut self, dir: String, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.subdir_menu = Some((dir.clone(), pos, None));
        self.crumb_menu = None;
        cx.notify();

        // Only local listing is wired; remote SSH browsing is deferred.
        if self.workspace.read(cx).active_remote_target(cx).is_some() {
            self.subdir_menu = Some((dir, pos, Some(Vec::new())));
            return;
        }
        cx.spawn(async move |view, cx| {
            let d = dir.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    labonair_backend::modules::fs::tree::read_dir_page(&d, 0, 200, false)
                })
                .await;
            let _ = view.update(cx, |this, cx| {
                let Some((cur, _, entries)) = this.subdir_menu.as_mut() else {
                    return;
                };
                if *cur != dir {
                    return;
                }
                let names = result
                    .map(|page| {
                        page.entries
                            .into_iter()
                            .filter(|e| {
                                matches!(
                                    e.kind,
                                    labonair_backend::modules::fs::tree::EntryKind::Dir
                                )
                            })
                            .map(|e| e.name)
                            .collect()
                    })
                    .unwrap_or_default();
                *entries = Some(names);
                cx.notify();
            });
        })
        .detach();
    }

    fn render_breadcrumb(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let cwd = self.workspace.read(cx).active_cwd(cx);
        let file_path = self.workspace.read(cx).active_file_path(cx);
        let home = Self::home_dir();
        let (fg, muted, border) = {
            let theme = self.theme.read(cx);
            (theme.foreground(), theme.muted_foreground(), theme.border())
        };
        let text_px = 11.0_f32;

        let (dir, leaf) = match &file_path {
            Some(fp) => (bc::dirname(fp), Some(bc::basename(fp).to_string())),
            None => match &cwd {
                Some(c) => (c.clone(), None),
                None => {
                    return div()
                        .id("crumb-empty")
                        .text_size(px(text_px))
                        .text_color(muted.opacity(0.7))
                        .child("no directory")
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, ev: &MouseDownEvent, _w, cx| {
                                this.open_crumb_menu(
                                    bc::Segment {
                                        label: String::new(),
                                        full_path: String::new(),
                                        is_home: false,
                                    },
                                    ev.position,
                                    cx,
                                );
                            }),
                        )
                        .into_any_element();
                }
            },
        };

        let segments = bc::segments_from_cwd(&dir, home.as_deref());
        let last_idx = segments.len().saturating_sub(1);
        let current_is_dropdown = leaf.is_none();
        let parent_count = if current_is_dropdown {
            last_idx
        } else {
            segments.len()
        };
        let collapse = parent_count > 4 && !self.expanded;

        let mut row = div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .overflow_hidden();

        for (i, seg) in segments.iter().enumerate() {
            let is_current = current_is_dropdown && i == last_idx;
            if collapse && i > 0 && i < parent_count - 1 {
                if i == 1 {
                    row = row
                        .child(
                            div()
                                .id("crumb-collapse")
                                .px(px(6.0))
                                .text_size(px(text_px))
                                .text_color(muted)
                                .rounded_sm()
                                .hover(|s| s.bg(border).text_color(fg))
                                .child(IconName::Ellipsis.svg(muted))
                                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.expanded = true;
                                    cx.notify();
                                })),
                        )
                        .child(div().text_color(muted).text_size(px(text_px)).child("/"));
                }
                continue;
            }
            row = row.child(self.render_crumb_segment(
                seg.clone(),
                is_current,
                current_is_dropdown,
                text_px,
                cx,
            ));
            if i != last_idx || leaf.is_some() {
                row = row.child(div().text_color(muted).text_size(px(text_px)).child("/"));
            }
        }

        if let Some(name) = leaf {
            row = row.child(
                div()
                    .text_size(px(text_px))
                    .text_color(fg)
                    .child(SharedString::from(name)),
            );
        }

        row.into_any_element()
    }

    fn render_crumb_segment(
        &self,
        seg: bc::Segment,
        is_current: bool,
        current_is_dropdown: bool,
        text_px: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (fg, muted, border) = {
            let theme = self.theme.read(cx);
            (theme.foreground(), theme.muted_foreground(), theme.border())
        };
        let label = if seg.is_home {
            "~".to_string()
        } else {
            seg.label.clone()
        };
        let show_chevron = is_current && current_is_dropdown;
        let seg_click = seg.clone();
        let seg_menu = seg.clone();
        div()
            .id(SharedString::from(format!("crumb-{}", seg.full_path)))
            .flex()
            .items_center()
            .gap_1()
            .px(px(6.0))
            .py(px(1.0))
            .rounded_full()
            .border_1()
            .border_color(border)
            .text_size(px(text_px))
            .text_color(if is_current { fg } else { muted })
            .hover(|s| s.text_color(fg))
            .when(seg.is_home, |d| d.child(IconName::Home.svg(muted)))
            .child(SharedString::from(label))
            .when(show_chevron, |d| d.child(IconName::ChevronDown.svg(muted)))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                if show_chevron {
                    this.open_subdir_menu(seg_click.full_path.clone(), ev.position(), cx);
                } else {
                    let p = seg_click.full_path.clone();
                    this.workspace.update(cx, |w, cx| w.send_cd(&p, cx));
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                    this.open_crumb_menu(seg_menu.clone(), ev.position, cx);
                }),
            )
    }

    fn render_crumb_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        use labonair_ui_kit::{context_menu, MenuItem};
        let (seg, pos) = self.crumb_menu.clone()?;
        let cwd = self.workspace.read(cx).active_cwd(cx);
        let has_path = !seg.full_path.is_empty();
        let rel = cwd
            .as_deref()
            .map(|c| bc::relative_path(c, &seg.full_path))
            .unwrap_or_else(|| seg.full_path.clone());
        let abs = seg.full_path.clone();
        let view = cx.entity();
        let close = {
            let v = view.clone();
            move |cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.crumb_menu = None;
                    cx.notify();
                })
            }
        };

        let mut items: Vec<MenuItem> = Vec::new();
        if has_path {
            items.push(MenuItem::label(if seg.is_home {
                "Home".to_string()
            } else {
                seg.label.clone()
            }));
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("cm-copy-abs", "Copy absolute path").on_click({
                    let close = close.clone();
                    let abs = abs.clone();
                    move |_, _w, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(abs.clone()));
                        close(cx);
                    }
                }),
            );
            items.push(
                MenuItem::new("cm-copy-rel", "Copy relative path").on_click({
                    let close = close.clone();
                    let rel = rel.clone();
                    move |_, _w, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(rel.clone()));
                        close(cx);
                    }
                }),
            );
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("cm-cd", "Open in current terminal").on_click({
                    let v = view.clone();
                    let abs = abs.clone();
                    move |_, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.workspace.update(cx, |w, cx| w.send_cd(&abs, cx));
                            this.crumb_menu = None;
                            cx.notify();
                        });
                    }
                }),
            );
            items.push(
                MenuItem::new("cm-cd-new", "Open in new terminal").on_click({
                    let v = view.clone();
                    let abs = abs.clone();
                    move |_, window, cx| {
                        v.update(cx, |this, cx| {
                            this.workspace
                                .update(cx, |w, cx| w.cd_in_new_tab(abs.clone(), window, cx));
                            this.crumb_menu = None;
                            cx.notify();
                        });
                    }
                }),
            );
        }

        // The right-click "move to titlebar / hide" personalization entries
        // are T18-005 (they used the removed `move_bar_item`).
        if items.is_empty() {
            return None;
        }

        let dismiss = move |_w: &mut Window, cx: &mut App| close(cx);
        Some(context_menu(pos, self.theme.read(cx), dismiss, items))
    }

    fn render_subdir_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        use labonair_ui_kit::{context_menu, MenuItem};
        let (dir, pos, entries) = self.subdir_menu.clone()?;
        let view = cx.entity();
        let close = {
            let v = view.clone();
            move |cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.subdir_menu = None;
                    cx.notify();
                })
            }
        };

        let items: Vec<MenuItem> = match &entries {
            None => vec![MenuItem::label("Loading\u{2026}")],
            Some(list) if list.is_empty() => vec![MenuItem::label("No subfolders")],
            Some(list) => list
                .iter()
                .take(50)
                .map(|name| {
                    let full = if dir == "/" {
                        format!("/{name}")
                    } else {
                        format!("{dir}/{name}")
                    };
                    let v = view.clone();
                    MenuItem::new(SharedString::from(format!("subdir-{name}")), name.clone())
                        .on_click(move |_, _w, cx| {
                            let full = full.clone();
                            v.update(cx, |this, cx| {
                                this.workspace.update(cx, |w, cx| w.send_cd(&full, cx));
                                this.subdir_menu = None;
                                cx.notify();
                            });
                        })
                })
                .collect(),
        };

        let dismiss = move |_w: &mut Window, cx: &mut App| close(cx);
        Some(context_menu(pos, self.theme.read(cx), dismiss, items))
    }
}

impl Render for CwdStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for CwdStatusItem {
    fn id(&self) -> &'static str {
        "cwd"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    // Leftmost of the right cluster — widest item (T18-004 default order).
    fn order(&self) -> i32 {
        10
    }
    fn group(&self) -> u32 {
        0
    }

    fn on_active_tab_changed(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let row = self.render_breadcrumb(cx);
        let crumb_menu = self.render_crumb_menu(cx);
        let subdir_menu = self.render_subdir_menu(cx);
        div()
            .flex()
            .items_center()
            .min_w_0()
            .child(row)
            .children(crumb_menu)
            .children(subdir_menu)
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor cursor position.
// ─────────────────────────────────────────────────────────────────────────────

pub struct CursorPositionStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
}

impl CursorPositionStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { workspace, theme }
    }
}

impl Render for CursorPositionStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for CursorPositionStatusItem {
    fn id(&self) -> &'static str {
        "cursor-position"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    // Same breadcrumb group as `cwd`/`preview-url` — all three are
    // active-tab-derived text, not a standalone action item.
    fn order(&self) -> i32 {
        11
    }
    fn group(&self) -> u32 {
        0
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some((line, col)) = self.workspace.read(cx).active_editor_cursor(cx) else {
            return div().into_any_element();
        };
        let muted = self.theme.read(cx).muted_foreground();
        div()
            .text_size(px(11.0))
            .text_color(muted)
            .child(SharedString::from(format!("Ln {line}, Col {col}")))
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Preview URL (native markdown preview tab).
// ─────────────────────────────────────────────────────────────────────────────

pub struct PreviewUrlStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
}

impl PreviewUrlStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { workspace, theme }
    }
}

impl Render for PreviewUrlStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for PreviewUrlStatusItem {
    fn id(&self) -> &'static str {
        "preview-url"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    fn order(&self) -> i32 {
        12
    }
    fn group(&self) -> u32 {
        0
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(url) = self.workspace.read(cx).active_preview_url(cx) else {
            return div().into_any_element();
        };
        let (muted, fg) = {
            let t = self.theme.read(cx);
            (t.muted_foreground(), t.foreground())
        };
        let open = url.clone();
        div()
            .id("bar-preview-url")
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(11.0))
            .text_color(muted)
            .hover(|s| s.text_color(fg))
            .child(IconName::Globe.svg(muted).size(px(11.0)))
            .child(SharedString::from(
                url.strip_prefix("http://").unwrap_or(&url).to_string(),
            ))
            .on_click(cx.listener(move |_, _: &ClickEvent, _w, cx| {
                cx.open_url(&open);
            }))
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Auto-updater.
// ─────────────────────────────────────────────────────────────────────────────

pub struct UpdaterStatusItem {
    updater: Entity<UpdaterView>,
    theme: Entity<ThemeStore>,
}

impl UpdaterStatusItem {
    pub fn new(
        updater: Entity<UpdaterView>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&updater, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { updater, theme }
    }
}

impl Render for UpdaterStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for UpdaterStatusItem {
    fn id(&self) -> &'static str {
        "updater"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    fn order(&self) -> i32 {
        40
    }
    fn group(&self) -> u32 {
        1
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let ready = matches!(
            self.updater.read(cx).status(),
            UpdaterStatus::Available(_) | UpdaterStatus::Downloading { .. } | UpdaterStatus::Ready
        );
        if !ready {
            return div().into_any_element();
        }
        let (fg, accent, border) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.accent(), t.border())
        };
        div()
            .id("bar-updater")
            .relative()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(fg)
            .hover(|s| s.bg(border))
            .child(IconName::Download.svg(fg))
            .child(
                div()
                    .absolute()
                    .top(px(-1.0))
                    .right(px(-1.0))
                    .size(px(6.0))
                    .rounded_full()
                    .bg(accent),
            )
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                this.updater.update(cx, |u, cx| u.open_dialog(cx));
            }))
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SFTP transfers.
// ─────────────────────────────────────────────────────────────────────────────

pub struct TransfersStatusItem {
    workspace: Entity<Workspace>,
    transfers: Entity<labonair_workspace::transfers::TransfersView>,
    theme: Entity<ThemeStore>,
}

impl TransfersStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        let transfers = workspace.read(cx).transfers_entity();
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&transfers, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            workspace,
            transfers,
            theme,
        }
    }
}

impl Render for TransfersStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for TransfersStatusItem {
    fn id(&self) -> &'static str {
        "transfers"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    fn order(&self) -> i32 {
        20
    }
    fn group(&self) -> u32 {
        1
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // Only shown while a transfer is queued/running (T18-004 point 5) —
        // the reference "conditional status item" rule, same as `updater`.
        if self.transfers.read(cx).active_count() == 0 {
            return div().into_any_element();
        }
        let (fg, muted, border) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.muted_foreground(), t.border())
        };
        simple_bar_button(
            "bar-transfers",
            IconName::ArrowDownUp,
            fg,
            muted,
            border,
            cx,
            |this, _window, cx| {
                this.workspace.update(cx, |w, cx| w.reveal_transfers(cx));
            },
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AI-agent access badge (MCP grants).
// ─────────────────────────────────────────────────────────────────────────────

pub struct AgentAccessStatusItem {
    store: Entity<AgentAccessStore>,
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    open: Option<Point<Pixels>>,
    focus: gpui::FocusHandle,
}

impl AgentAccessStatusItem {
    pub fn new(
        store: Entity<AgentAccessStore>,
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            store,
            workspace,
            theme,
            open: None,
            focus: cx.focus_handle(),
        }
    }

    fn render_badge(
        &mut self,
        entries: Vec<AgentAccessEntry>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (fg, muted, border, accent) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.muted_foreground(), t.border(), t.accent())
        };
        let count = entries.len();

        let badge = div()
            .id("agent-access-badge")
            .track_focus(&self.focus)
            .key_context("StatusPopover")
            .relative()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(muted)
            .hover(|s| s.bg(border).text_color(fg))
            .child(IconName::Shield.svg(muted))
            .child(
                div()
                    .absolute()
                    .top(px(-2.0))
                    .right(px(-2.0))
                    .min_w(px(13.0))
                    .h(px(13.0))
                    .px(px(2.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(accent)
                    .text_color(fg)
                    .text_size(px(8.0))
                    .child(SharedString::from(count.to_string())),
            )
            .on_click(cx.listener(|this, ev: &ClickEvent, w, cx| {
                if this.open.is_some() {
                    this.open = None;
                } else {
                    this.open = Some(ev.position());
                    w.focus(&this.focus);
                }
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _w, cx| {
                if this.open.is_some() && ev.keystroke.key == "escape" {
                    this.open = None;
                    cx.notify();
                    cx.stop_propagation();
                }
            }));

        let Some(anchor) = self.open else {
            return div()
                .relative()
                .flex_shrink_0()
                .child(badge)
                .into_any_element();
        };

        let view = cx.entity();
        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.open = None;
                    cx.notify();
                })
            }
        };
        let content = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border)
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child("AI Agent Access"),
            )
            .children(entries.into_iter().map(|entry| {
                let tab_id = entry.tab_id;
                let session_id = entry.session_id.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .hover(|s| s.bg(border))
                    .child(
                        div()
                            .id(SharedString::from(format!("agent-jump-{tab_id}")))
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_color(fg)
                            .truncate()
                            .child(SharedString::from(entry.label.clone()))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.open = None;
                                this.workspace
                                    .update(cx, |w, cx| w.reveal_tab(tab_id, window, cx));
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("agent-revoke-{tab_id}")))
                            .px_1()
                            .rounded_sm()
                            .text_xs()
                            .text_color(muted)
                            .hover(|s| s.text_color(fg))
                            .child("\u{2715}")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                let session_id = session_id.clone();
                                this.store.update(cx, |s, cx| {
                                    s.set_grant(
                                        tab_id,
                                        session_id,
                                        false,
                                        String::new(),
                                        labonair_backend::modules::mcp::SessionKind::Ssh,
                                        None,
                                        None,
                                        cx,
                                    );
                                });
                                cx.notify();
                            })),
                    )
            }))
            .into_any_element();

        div()
            .relative()
            .flex_shrink_0()
            .child(badge)
            .child(labonair_ui_kit::popover(
                anchor,
                px(300.0),
                self.theme.read(cx),
                dismiss,
                content,
            ))
            .into_any_element()
    }
}

impl Render for AgentAccessStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for AgentAccessStatusItem {
    fn id(&self) -> &'static str {
        "agent-access"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    fn order(&self) -> i32 {
        30
    }
    fn group(&self) -> u32 {
        1
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (enabled, entries) = {
            let aa = self.store.read(cx);
            (aa.bridge_enabled(), aa.entries())
        };
        if !enabled || entries.is_empty() {
            return div().into_any_element();
        }
        self.render_badge(entries, cx)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Jump hosts — opens the host manager (rewiring to Palette::Hosts is T17-009 /
// T19-010, per the task; keep the current call 1:1 here).
// ─────────────────────────────────────────────────────────────────────────────

pub struct JumpHostsStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
}

impl JumpHostsStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { workspace, theme }
    }
}

impl Render for JumpHostsStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for JumpHostsStatusItem {
    fn id(&self) -> &'static str {
        "jump-hosts"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    fn order(&self) -> i32 {
        50
    }
    fn group(&self) -> u32 {
        1
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (fg, muted, border) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.muted_foreground(), t.border())
        };
        simple_bar_button(
            "bar-jump-hosts",
            IconName::Server,
            fg,
            muted,
            border,
            cx,
            |this, _window, cx| {
                this.workspace.update(cx, |w, cx| w.open_host_settings(cx));
            },
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Path bookmarks.
// ─────────────────────────────────────────────────────────────────────────────

pub struct BookmarksStatusItem {
    bookmarks: Entity<BookmarksView>,
    theme: Entity<ThemeStore>,
}

impl BookmarksStatusItem {
    pub fn new(
        bookmarks: Entity<BookmarksView>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { bookmarks, theme }
    }
}

impl Render for BookmarksStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for BookmarksStatusItem {
    fn id(&self) -> &'static str {
        "bookmarks"
    }
    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }
    fn order(&self) -> i32 {
        60
    }
    fn group(&self) -> u32 {
        1
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (fg, muted, border) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.muted_foreground(), t.border())
        };
        simple_bar_button(
            "bar-bookmarks",
            IconName::Bookmark,
            fg,
            muted,
            border,
            cx,
            |this, window, cx| {
                this.bookmarks.update(cx, |b, cx| b.toggle(window, cx));
            },
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Registration — the single place that names concrete status-item types.
// ─────────────────────────────────────────────────────────────────────────────

/// Human-readable label for a [`StatusItem::id`] (T18-005) — used by the
/// command palette's "Statusbar: Show Hidden Item…" page and could grow into
/// the personalization page's row titles (T18-007).
pub fn status_item_label(id: &str) -> &'static str {
    match id {
        "panel-toggles" => "Panel Toggles",
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

/// Register the built-in status-bar items on the workspace's
/// [`StatusItemRegistry`](labonair_panel::StatusItemRegistry).
///
/// Mirrors [`register_builtin_panels`](crate::app_shell): the shell builds each
/// item entity once and the registry constructor hands back a clone (an
/// `Entity` refcount bump), so the shell can keep observing its dependencies.
/// Adding a status item = a new type here + one array entry.
#[allow(clippy::too_many_arguments)]
pub fn register_builtin_status_items(
    workspace: &Entity<Workspace>,
    theme: &Entity<ThemeStore>,
    notifications: &Entity<labonair_notifications::NotificationCenter>,
    updater: &Entity<UpdaterView>,
    agent_access: &Entity<AgentAccessStore>,
    bookmarks: &Entity<BookmarksView>,
    cx: &mut App,
) {
    fn reg<T: StatusItem + 'static>(view: &Entity<T>, cx: &App) -> StatusItemRegistration {
        let handle = view.clone();
        StatusItemRegistration {
            id: view.read(cx).id(),
            default_side: view.read(cx).default_side(),
            order: view.read(cx).order(),
            group: view.read(cx).group(),
            build: Arc::new(move |_window, _cx| Arc::new(handle.clone()) as AnyStatusItemHandle),
        }
    }

    let panel_toggles =
        cx.new(|cx| PanelTogglesStatusItem::new(workspace.clone(), theme.clone(), cx));
    let notifications_item =
        cx.new(|cx| NotificationsStatusItem::new(notifications.clone(), theme.clone(), cx));
    let cwd = cx.new(|cx| CwdStatusItem::new(workspace.clone(), theme.clone(), cx));
    let cursor = cx.new(|cx| CursorPositionStatusItem::new(workspace.clone(), theme.clone(), cx));
    let preview = cx.new(|cx| PreviewUrlStatusItem::new(workspace.clone(), theme.clone(), cx));
    let updater_item = cx.new(|cx| UpdaterStatusItem::new(updater.clone(), theme.clone(), cx));
    let transfers = cx.new(|cx| TransfersStatusItem::new(workspace.clone(), theme.clone(), cx));
    let agent = cx.new(|cx| {
        AgentAccessStatusItem::new(agent_access.clone(), workspace.clone(), theme.clone(), cx)
    });
    let jump_hosts = cx.new(|cx| JumpHostsStatusItem::new(workspace.clone(), theme.clone(), cx));
    let bookmarks_item =
        cx.new(|cx| BookmarksStatusItem::new(bookmarks.clone(), theme.clone(), cx));

    // Default right-cluster order (T18-004 point 1), each item's `order()`:
    //   cwd(10)/cursor(11)/preview(12)  — group 0, active-tab-derived text,
    //     widest first so it can collapse before anything else has to move.
    //   transfers(20)/agent(30)/updater(40)/jump-hosts(50)/bookmarks(60) —
    //     group 1, the "action" items in the order the task file lists them.
    //   notifications(100) — group 2, always visible, pinned rightmost.
    // `StatusBar::cluster` draws a divider between groups, never within one.
    let registrations = [
        reg(&panel_toggles, cx),
        reg(&notifications_item, cx),
        reg(&cwd, cx),
        reg(&cursor, cx),
        reg(&preview, cx),
        reg(&updater_item, cx),
        reg(&transfers, cx),
        reg(&agent, cx),
        reg(&jump_hosts, cx),
        reg(&bookmarks_item, cx),
    ];
    workspace.update(cx, |w, _| {
        let registry = w.status_item_registry_mut();
        for registration in registrations {
            registry.register(registration);
        }
        // T18-005: apply any persisted per-item side/hidden overrides now
        // that every id is registered.
        w.reload_status_bar_placements();
    });
}
