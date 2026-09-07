//! Remaining shell-owned [`StatusItem`]s + the composition hook (T17-003).
//!
//! Feature modules own their status-bar contributions and expose typed
//! registration functions. This module retains only the workspace-scoped
//! shell surface that has not yet moved to its owner; the registration hook
//! below composes those contributions into the shared registry.
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
    IntoElement, ParentElement, Pixels, Point, Render, SharedString, StatefulInteractiveElement,
    Styled, Window,
};
use labonair_panel::{
    AnyStatusItemHandle, DockPosition, StatusItem, StatusItemRegistration, StatusMenuEntry,
    StatusSide,
};
use labonair_transfers_ui::TransfersView;
use labonair_ui_kit::{icon_toggle_button, IconName, Palette};
use labonair_workspace::agent_access::AgentAccessStore;

use crate::theme::ThemeStore;
use crate::updater::{UpdaterStatus, UpdaterView};
use crate::workspace::Workspace;
use labonair_workspace::cwd_breadcrumb as bc;

// ─────────────────────────────────────────────────────────────────────────────
// CWD breadcrumb (its own state: expanded, segment menu, subdir dropdown).
// ─────────────────────────────────────────────────────────────────────────────

pub struct CwdStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    expanded: bool,
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
            subdir_menu: None,
        }
    }

    fn home_dir() -> Option<String> {
        dirs::home_dir().map(|p| p.to_string_lossy().into_owned())
    }

    fn open_subdir_menu(&mut self, dir: String, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.subdir_menu = Some((dir.clone(), pos, None));
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
                .spawn(async move { labonair_filesystem::tree::read_dir_page(&d, 0, 200, false) })
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
                            .filter(|e| matches!(e.kind, labonair_filesystem::tree::EntryKind::Dir))
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
        let (fg, muted) = {
            let theme = self.theme.read(cx);
            (theme.foreground(), theme.muted_foreground())
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
                            icon_toggle_button(
                                "crumb-collapse",
                                Palette::from_theme(self.theme.read(cx)),
                                IconName::Ellipsis,
                                false,
                            )
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _w, cx| {
                                    this.expanded = true;
                                    cx.notify();
                                },
                            )),
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
        // T20-003: a `rounded_full` breadcrumb-segment pill with an optional
        // leading home icon and trailing chevron, plus a right-click menu on
        // the same element — no `ui-kit` primitive matches this shape
        // (`ListItem` is a full-width row, `button`/`icon_toggle_button` are
        // square), documented exception (same shape as `hosts.rs`'s
        // `render_group_chips`).
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
        Some(context_menu(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        ))
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

    /// Path actions for the active directory, merged into the status bar's one
    /// right-click menu above "Move left / Move right / Hide" — the CWD widget
    /// used to open its own competing context menu here.
    fn status_menu_entries(&mut self, cx: &mut Context<Self>) -> Vec<StatusMenuEntry> {
        let Some(cwd) = self.workspace.read(cx).active_cwd(cx) else {
            return Vec::new();
        };
        let ws = self.workspace.clone();
        let copy = cwd.clone();
        let cd = cwd.clone();
        let cd_new = cwd.clone();
        vec![
            StatusMenuEntry::action("cwd-copy-path", "Copy path", move |_w, cx| {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy.clone()));
            }),
            StatusMenuEntry::action("cwd-open-terminal", "Open in current terminal", {
                let ws = ws.clone();
                move |_w, cx| {
                    ws.update(cx, |w, cx| w.send_cd(&cd, cx));
                }
            }),
            StatusMenuEntry::action(
                "cwd-open-new-terminal",
                "Open in new terminal",
                move |window, cx| {
                    ws.update(cx, |w, cx| w.cd_in_new_tab(cd_new.clone(), window, cx));
                },
            ),
        ]
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let row = self.render_breadcrumb(cx);
        let subdir_menu = self.render_subdir_menu(cx);
        div()
            .flex()
            .items_center()
            .min_w_0()
            .child(row)
            .children(subdir_menu)
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
// ─────────────────────────────────────────────────────────────────────────────
// Registration — the composition boundary for status-item contributions.
// ─────────────────────────────────────────────────────────────────────────────

/// Register the built-in status-bar items on the workspace's
/// [`StatusItemRegistry`](labonair_panel::StatusItemRegistry).
///
/// Feature modules build their own contributions; the shell only composes the
/// resulting registrations and preserves the application-level ordering.
#[allow(clippy::too_many_arguments)]
pub fn register_builtin_status_items(
    workspace: &Entity<Workspace>,
    theme: &Entity<ThemeStore>,
    notifications: &Entity<labonair_notifications::NotificationCenter>,
    updater: &Entity<UpdaterView>,
    agent_access: &Entity<AgentAccessStore>,
    transfers_view: &Entity<TransfersView>,
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

    let dock_btn_left = labonair_workspace::dock_status_item::registration(
        workspace,
        theme,
        DockPosition::Left,
        cx,
    );
    let dock_btn_right = labonair_workspace::dock_status_item::registration(
        workspace,
        theme,
        DockPosition::Right,
        cx,
    );
    let dock_btn_bottom = labonair_workspace::dock_status_item::registration(
        workspace,
        theme,
        DockPosition::Bottom,
        cx,
    );
    let notifications_item = cx.new(|cx| {
        labonair_notifications::NotificationsStatusItem::new(
            notifications.clone(),
            theme.clone(),
            cx,
        )
    });
    let cwd = cx.new(|cx| CwdStatusItem::new(workspace.clone(), theme.clone(), cx));
    let cursor =
        labonair_workspace::status_items::cursor_position_registration(workspace, theme, cx);
    let preview = labonair_workspace::status_items::preview_url_registration(workspace, theme, cx);
    let updater_item = cx.new(|cx| UpdaterStatusItem::new(updater.clone(), theme.clone(), cx));
    let transfers =
        labonair_transfers_ui::status_item_registration(workspace, transfers_view, theme, cx);
    let agent = labonair_workspace::status_items::status_item_registration(
        agent_access,
        workspace,
        theme,
        cx,
    );
    // Default right-cluster order (T18-004 point 1), each item's `order()`:
    //   cwd(10)/cursor(11)/preview(12)  — group 0, active-tab-derived text,
    //     widest first so it can collapse before anything else has to move.
    //   transfers(20)/agent(30)/updater(40) —
    //     group 1, the "action" items in the order the task file lists them.
    //   notifications(100) — group 2, always visible, pinned rightmost.
    // `StatusBar::cluster` draws a divider between groups, never within one.
    let registrations = [
        dock_btn_left,
        dock_btn_right,
        dock_btn_bottom,
        reg(&notifications_item, cx),
        reg(&cwd, cx),
        cursor,
        preview,
        reg(&updater_item, cx),
        transfers,
        agent,
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
