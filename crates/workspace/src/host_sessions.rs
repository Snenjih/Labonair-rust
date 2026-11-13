//! Workspace side of the detached session host (ADR 0009) outside of pane
//! creation and restore: recovered ("orphaned") sessions and quitting with
//! every session stopped.

use gpui::{App, Context, Window};
use labonair_settings::{GeneralSettings, Settings as _};
use labonair_notifications::{notification_center, Notification};
use labonair_terminal::{BackgroundSession, DaemonHost};

use crate::host_restore;
use crate::pane::WorkspaceLayout;
use crate::pane_kind::PaneKind;
use crate::{terminal_settings, PaneEntry, Workspace};

/// Notification action that opens the found sessions as tabs.
pub(crate) const RECOVER_ACTION: &str = "terminal:recover-background-sessions";

/// A client for an already running daemon, if there is one. Never starts it.
fn running_host(cx: &App) -> Option<DaemonHost> {
    DaemonHost::connect(
        &labonair_filesystem::paths::data_dir(),
        terminal_settings(cx).detached_scrollback_lines(),
        false,
    )
    .ok()
}

impl Workspace {
    /// Daemon sessions no open pane holds: what a crash or a closed window left
    /// behind.
    fn recoverable_sessions(&self, cx: &App) -> Vec<BackgroundSession> {
        let Some(alive) = running_host(cx).and_then(|host| host.list_sessions().ok()) else {
            return Vec::new();
        };
        let adopted: Vec<u64> = self
            .panes
            .values()
            .filter_map(|entry| entry.view.read(cx).handle().host_session_id())
            .collect();
        host_restore::orphans(&alive, &self.known_scrollback_ids(), &adopted)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Startup: offer unclaimed background sessions with one notification.
    pub(crate) fn announce_background_sessions(&self, cx: &mut Context<Self>) {
        let found = self.recoverable_sessions(cx);
        if found.is_empty() {
            return;
        }
        notification_center(cx).update(cx, |center, cx| {
            center.push(
                Notification::info(
                    host_restore::found_title(found.len()),
                    "They kept running after the app closed.",
                )
                .source("terminal")
                .action(RECOVER_ACTION, "Open as tabs")
                .dedupe_key("terminal:background-sessions-found"),
                cx,
            );
        });
    }

    /// `Terminal: Recover Background Sessions`: open every unclaimed session
    /// as a new tab.
    pub fn recover_background_sessions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let found = self.recoverable_sessions(cx);
        if found.is_empty() {
            notification_center(cx).update(cx, |center, cx| {
                center.push(
                    Notification::info("No background sessions", "Nothing to recover.")
                        .source("terminal")
                        .dedupe_key("terminal:no-background-sessions"),
                    cx,
                );
            });
            return;
        }
        for session in found {
            self.open_adopted_tab(&session, window, cx);
        }
    }

    fn open_adopted_tab(
        &mut self,
        session: &BackgroundSession,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let scrollback_id = session
            .pane_tag
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let adopted = match self.adopt_hosted_session(session.id, scrollback_id, cx) {
            Ok(adopted) => adopted,
            Err(err) => {
                notification_center(cx).update(cx, |center, cx| {
                    center.push(
                        Notification::error(
                            "Could not open background session",
                            "The session could not be attached.",
                        )
                        .source("terminal")
                        .details(err),
                        cx,
                    );
                });
                return;
            }
        };
        let (session_id, handle, scrollback_id) = adopted;
        let pane_id = self.alloc_pane();
        let tab_id = self.tabs.update(cx, |s, cx| {
            s.open_workspace(session_id, session.cwd.clone(), cx)
        });
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.register_pane(pane_id, PaneKind::Terminal);
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
    }
}

impl Workspace {
    /// `Terminal: Quit and Stop All Sessions`: save the session snapshot like a
    /// normal window close, stop the background host with all its sessions,
    /// then quit. App-bound sessions end with the process as usual.
    pub(crate) fn quit_and_stop_all_sessions(&self, cx: &mut App) {
        if GeneralSettings::try_get(cx)
            .map(|s| s.session_restore())
            .unwrap_or(false)
        {
            crate::session::save_snapshot(&self.session_snapshot(cx));
        }
        if let Some(host) = running_host(cx) {
            host.shutdown();
        }
        cx.quit();
    }
}
