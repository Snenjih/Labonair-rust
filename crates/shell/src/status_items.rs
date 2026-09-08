//! Statusbar composition hook (T17-003).
//!
//! Feature modules own their status-bar contributions and expose typed
//! registration functions. This module only composes those contributions into
//! the shared workspace registry; it does not own a feature status item.
//!
//! The transitional `bar_items` placement blob (`BarLoc`, the
//! `barItemPlacements` → `statusBarItemPlacements` migrator, the right-click
//! "move to titlebar / hide" affordances) is deliberately *not* ported here —
//! that is T18-005 / T18-006. Items expose `default_side` + `order`; the
//! `StatusItemRegistry` resolves the rest.

use gpui::{App, Entity};
use labonair_panel::DockPosition;
use labonair_transfers_ui::TransfersView;
use labonair_workspace::agent_access::AgentAccessStore;

use crate::theme::ThemeStore;
use crate::updater::UpdaterView;
use crate::workspace::Workspace;

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
    let notifications_item = labonair_notifications::registration(notifications, theme, cx);
    let cwd = labonair_workspace::cwd_status_item::registration(workspace, theme, cx);
    let cursor =
        labonair_workspace::status_items::cursor_position_registration(workspace, theme, cx);
    let preview = labonair_workspace::status_items::preview_url_registration(workspace, theme, cx);
    let updater_item = labonair_updater_ui::status_item::registration(updater, theme, cx);
    let transfers = labonair_transfers_ui::status_item_registration(transfers_view, theme, cx);
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
        notifications_item,
        cwd,
        cursor,
        preview,
        updater_item,
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
