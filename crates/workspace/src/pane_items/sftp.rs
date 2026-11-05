//! SFTP pane adapter: the dual-pane browser. A split opens a new browser on
//! the same host with its own SFTP session.

use gpui::{AnyElement, App, AppContext, Context, IntoElement, SharedString, Styled};
use labonair_ssh::SshSessionId;

use super::{PaneSpec, SplitCloneError};
use crate::pane::PaneId;
use crate::ssh_connection::ConnectionKind;
use crate::views::sftp::{SftpEvent, SftpView};
use crate::Workspace;

impl Workspace {
    pub(super) fn sftp_pane_spec(&self, pane: PaneId, cx: &App, spec: &mut PaneSpec) {
        if let Some(view) = self.sftp_views.get(&pane) {
            let view = view.read(cx);
            spec.title = SharedString::from(format!("SFTP \u{00b7} {}", view.host_label()));
        }
    }

    pub(super) fn sftp_pane_body(&mut self, pane: PaneId, cx: &mut Context<Self>) -> AnyElement {
        if let Some(session_id) = self.sftp_sessions.get(&pane).cloned() {
            if let Some(entry) = self
                .ssh_connection
                .read(cx)
                .get(&session_id)
                .filter(|e| e.state.is_blocking())
                .cloned()
            {
                return self.render_ssh_loading(entry, cx);
            }
        }
        match self.sftp_views.get(&pane) {
            Some(view) => view.clone().into_any_element(),
            None => gpui::div().size_full().into_any_element(),
        }
    }

    pub(super) fn clone_sftp_pane(
        &mut self,
        source: PaneId,
        new_id: PaneId,
        cx: &mut Context<Self>,
    ) -> Result<(), SplitCloneError> {
        let (host_id, label) = self
            .sftp_views
            .get(&source)
            .map(|v| {
                let v = v.read(cx);
                (v.host_id().to_string(), v.host_label())
            })
            .ok_or_else(|| SplitCloneError::Failed("The SFTP browser is not available.".into()))?;
        let session_id = uuid::Uuid::new_v4().to_string();
        self.ssh_connection.update(cx, |s, cx| {
            s.begin(
                session_id.clone(),
                host_id.clone(),
                label.to_string(),
                ConnectionKind::Sftp,
                None,
                cx,
            );
        });
        let view = self.new_sftp_view(session_id.clone(), host_id, label, cx);
        self.sftp_views.insert(new_id, view);
        self.sftp_sessions.insert(new_id, session_id);
        Ok(())
    }

    pub(crate) fn new_sftp_view(
        &mut self,
        session_id: String,
        host_id: String,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<SftpView> {
        let label: SharedString = label.into();
        let view = cx.new(|cx| {
            SftpView::new(
                self.ssh.clone(),
                self.ssh_remote.clone(),
                self.sftp_session.clone(),
                self.sftp_browser.clone(),
                self.tokio.clone(),
                self.theme.clone(),
                session_id,
                host_id,
                label,
                cx,
            )
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&view, |this, _view, ev: &SftpEvent, cx| {
            this.on_sftp_event(ev, cx)
        })
        .detach();
        view
    }

    /// Drop the view and close the SFTP / SSH session of an SFTP pane.
    pub(super) fn retire_sftp_pane(&mut self, pane: PaneId, cx: &mut Context<Self>) {
        let sftp_handle = self
            .sftp_views
            .get(&pane)
            .and_then(|view| view.read(cx).sftp_handle());
        self.sftp_views.remove(&pane);
        if let Some(session_id) = self.sftp_sessions.remove(&pane) {
            self.ssh_connection
                .update(cx, |s, cx| s.remove(&session_id, cx));
            let sftp = self.sftp_session.clone();
            let ssh = self.ssh.clone();
            self.tokio.spawn(async move {
                if let Some(handle) = sftp_handle {
                    let _ = sftp.close(handle).await;
                }
                let _ = ssh.disconnect(SshSessionId::new(session_id)).await;
            });
        }
    }
}
