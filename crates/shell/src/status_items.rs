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

use gpui::{
    div, px, AnyElement, App, AppContext, ClickEvent, Context, Entity, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};
use labonair_panel::{
    AnyStatusItemHandle, DockPosition, StatusItem, StatusItemRegistration, StatusSide,
};
use labonair_transfers_ui::TransfersView;
use labonair_ui_kit::IconName;
use labonair_workspace::agent_access::AgentAccessStore;

use crate::theme::ThemeStore;
use crate::updater::{UpdaterStatus, UpdaterView};
use crate::workspace::Workspace;

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
    let cwd = labonair_workspace::cwd_status_item::registration(workspace, theme, cx);
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
        cwd,
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
