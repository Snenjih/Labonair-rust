//! Settings > Terminal > "Background host" status block (ADR 0009): whether
//! the detached session host is up, the sessions living in it with a Kill
//! button each, "Stop all sessions", and, for a host built for another
//! protocol, "Restart host".
//!
//! The host is polled through [`BackgroundHostService`] (futures on the tokio
//! runtime, never on the UI thread) while the page is rendered: every render
//! of the block marks it seen, and the poll loop ends the first tick nobody
//! saw it.

use std::time::Duration;

use crate::services::{
    BackgroundHostService, BackgroundHostSession, BackgroundHostSnapshot, BackgroundHostStatus,
};
use crate::view::*;
use labonair_ui_kit::button_disabled;

/// The section that carries the live rows (the "Background host" group).
pub(crate) const SECTION: &str = "Background host";

const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// Per-window state of the block.
#[derive(Default)]
pub(crate) struct HostBlock {
    snapshot: Option<BackgroundHostSnapshot>,
    polling: bool,
    /// Rendered since the last poll tick.
    seen: bool,
    /// A kill / stop / restart is in flight.
    busy: bool,
}

#[derive(Clone, Copy)]
enum HostAction {
    Kill(u64),
    StopAll,
    Restart,
}

/// One live row of the block.
#[derive(Clone)]
pub(crate) enum HostItem {
    Status(BackgroundHostSnapshot),
    Session(HostSessionRow),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostSessionRow {
    pub id: u64,
    pub title: String,
    pub detail: String,
}

fn plural(count: usize) -> String {
    match count {
        1 => "1 session".to_string(),
        n => format!("{n} sessions"),
    }
}

/// The status line: "Not running", "Running, 3 sessions" or
/// "Incompatible (build abc), 2 sessions".
pub(crate) fn status_text(snapshot: &BackgroundHostSnapshot) -> String {
    match &snapshot.status {
        BackgroundHostStatus::NotRunning => "Not running".to_string(),
        BackgroundHostStatus::Running => format!("Running, {}", plural(snapshot.sessions.len())),
        BackgroundHostStatus::Incompatible { build_id, sessions } => {
            format!(
                "Incompatible (build {build_id}), {}",
                plural(*sessions as usize)
            )
        }
    }
}

fn last_component(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

fn session_row(session: &BackgroundHostSession) -> HostSessionRow {
    let title = session
        .label
        .clone()
        .filter(|label| !label.is_empty())
        .or_else(|| {
            session
                .cwd
                .as_deref()
                .map(|cwd| last_component(cwd).to_string())
        })
        .unwrap_or_else(|| format!("Session {}", session.id));
    let state = if session.exited {
        "exited"
    } else if session.attached {
        "attached"
    } else {
        "detached"
    };
    let detail = [
        session.cwd.clone(),
        session.foreground.clone(),
        Some(state.to_string()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    HostSessionRow {
        id: session.id,
        title,
        detail,
    }
}

/// The rows of the block for `snapshot`: the status row, then one per session.
pub(crate) fn items(snapshot: &BackgroundHostSnapshot) -> Vec<HostItem> {
    let mut items = vec![HostItem::Status(snapshot.clone())];
    items.extend(
        snapshot
            .sessions
            .iter()
            .map(|session| HostItem::Session(session_row(session))),
    );
    items
}

async fn fetch(
    service: std::sync::Arc<dyn BackgroundHostService>,
    tokio: TokioHandle,
) -> Option<BackgroundHostSnapshot> {
    match tokio.spawn(service.snapshot()).await {
        Ok(Ok(snapshot)) => Some(snapshot),
        Ok(Err(error)) => {
            tracing::warn!("background host status unavailable: {error}");
            None
        }
        Err(error) => {
            tracing::warn!("background host status task failed: {error}");
            None
        }
    }
}

impl SettingsView {
    /// The live rows to append to the "Background host" group, and (re)start
    /// the poll while they are on screen. Empty without a service (non-unix).
    pub(crate) fn background_host_items(&mut self, cx: &mut Context<Self>) -> Vec<HostItem> {
        let Some(service) = self.services.background_host() else {
            return Vec::new();
        };
        self.host.seen = true;
        if !self.host.polling {
            self.host.polling = true;
            let tokio = self.tokio.clone();
            cx.spawn(async move |this, cx| loop {
                let snapshot = fetch(service.clone(), tokio.clone()).await;
                let alive = this.update(cx, |this, cx| {
                    if snapshot.is_some() {
                        this.host.snapshot = snapshot;
                    }
                    // Repaint also marks the block seen again.
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
                cx.background_executor().timer(POLL_INTERVAL).await;
                let visible = this
                    .update(cx, |this, _| {
                        let seen = std::mem::take(&mut this.host.seen);
                        this.host.polling = seen;
                        seen
                    })
                    .unwrap_or(false);
                if !visible {
                    break;
                }
            })
            .detach();
        }
        self.host.snapshot.as_ref().map(items).unwrap_or_default()
    }

    fn run_host_action(&mut self, action: HostAction, cx: &mut Context<Self>) {
        let Some(service) = self.services.background_host() else {
            return;
        };
        if self.host.busy {
            return;
        }
        self.host.busy = true;
        cx.notify();
        let tokio = self.tokio.clone();
        cx.spawn(async move |this, cx| {
            let call = match action {
                HostAction::Kill(id) => service.kill(id),
                HostAction::StopAll => service.stop_all(),
                HostAction::Restart => service.restart(),
            };
            let result = tokio.spawn(call).await;
            let snapshot = fetch(service, tokio).await;
            let _ = this.update(cx, |this, cx| {
                this.host.busy = false;
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => this.notify_error(cx, "Background host", error),
                    Err(error) => this.notify_error(cx, "Background host", error.to_string()),
                }
                if snapshot.is_some() {
                    this.host.snapshot = snapshot;
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn host_button(
        &self,
        id: impl Into<SharedString>,
        label: String,
        variant: ButtonVariant,
        action: HostAction,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = id.into();
        if self.host.busy {
            return button_disabled(id, *c, variant, ButtonSize::Xs)
                .child(label)
                .into_any_element();
        }
        button(id, *c, variant, ButtonSize::Xs)
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.run_host_action(action, cx);
            }))
            .into_any_element()
    }

    pub(crate) fn render_host_item(
        &mut self,
        item: &HostItem,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match item {
            HostItem::Status(snapshot) => {
                let mut buttons = h_stack().gap(c.space(8.0));
                if let BackgroundHostStatus::Incompatible { sessions, .. } = &snapshot.status {
                    buttons = buttons.child(self.host_button(
                        "settings-host-restart",
                        format!("Restart host ({} end)", plural(*sessions as usize)),
                        ButtonVariant::TintedError,
                        HostAction::Restart,
                        c,
                        cx,
                    ));
                }
                if snapshot.status == BackgroundHostStatus::Running {
                    buttons = buttons.child(self.host_button(
                        "settings-host-stop-all",
                        "Stop all sessions".to_string(),
                        ButtonVariant::OutlinedGhost,
                        HostAction::StopAll,
                        c,
                        cx,
                    ));
                }
                settings_row("settings-host-status", *c, "Host status")
                    .description(status_text(snapshot))
                    .control(buttons)
                    .into_any_element()
            }
            HostItem::Session(row) => settings_row(
                SharedString::from(format!("settings-host-session-{}", row.id)),
                *c,
                row.title.clone(),
            )
            .description(row.detail.clone())
            .control(self.host_button(
                SharedString::from(format!("settings-host-kill-{}", row.id)),
                "Kill".to_string(),
                ButtonVariant::TintedError,
                HostAction::Kill(row.id),
                c,
                cx,
            ))
            .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: u64) -> BackgroundHostSession {
        BackgroundHostSession {
            id,
            label: None,
            cwd: None,
            foreground: None,
            attached: false,
            exited: false,
        }
    }

    #[test]
    fn status_line_names_state_and_session_count() {
        let snap = |status, n| BackgroundHostSnapshot {
            status,
            sessions: (0..n).map(session).collect(),
        };
        assert_eq!(
            status_text(&snap(BackgroundHostStatus::NotRunning, 0)),
            "Not running"
        );
        assert_eq!(
            status_text(&snap(BackgroundHostStatus::Running, 1)),
            "Running, 1 session"
        );
        assert_eq!(
            status_text(&snap(BackgroundHostStatus::Running, 3)),
            "Running, 3 sessions"
        );
        assert_eq!(
            status_text(&snap(
                BackgroundHostStatus::Incompatible {
                    build_id: "abc".into(),
                    sessions: 2
                },
                2
            )),
            "Incompatible (build abc), 2 sessions"
        );
    }

    #[test]
    fn session_rows_show_label_or_directory_process_and_state() {
        let row = session_row(&BackgroundHostSession {
            label: Some("build".into()),
            cwd: Some("/work/app".into()),
            foreground: Some("cargo".into()),
            attached: true,
            ..session(7)
        });
        assert_eq!(row.title, "build");
        assert_eq!(row.detail, "/work/app · cargo · attached");

        let row = session_row(&BackgroundHostSession {
            cwd: Some("/work/app/".into()),
            exited: true,
            ..session(8)
        });
        assert_eq!(row.title, "app");
        assert_eq!(row.detail, "/work/app/ · exited");

        let row = session_row(&session(9));
        assert_eq!(row.title, "Session 9");
        assert_eq!(row.detail, "detached");
    }

    #[test]
    fn items_are_the_status_row_followed_by_the_sessions() {
        let snap = BackgroundHostSnapshot {
            status: BackgroundHostStatus::Running,
            sessions: vec![session(1), session(2)],
        };
        let items = items(&snap);
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], HostItem::Status(_)));
        assert!(matches!(&items[2], HostItem::Session(row) if row.id == 2));
    }
}
