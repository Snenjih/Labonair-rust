//! Terminal pane adapter: a local shell or an SSH shell.

use gpui::{AnyElement, App, Context, IntoElement, SharedString, Styled, Window};

use super::{PaneSpec, SplitCloneError};
use crate::pane::PaneId;
use crate::{PaneEntry, Workspace};

impl Workspace {
    pub(super) fn terminal_pane_spec(&self, pane: PaneId, cx: &App, spec: &mut PaneSpec) {
        let Some(entry) = self.panes.get(&pane) else {
            return;
        };
        let (cwd, process) = entry.view.read(cx).shell_meta();
        if let Some(ssh) = self.ssh_tabs.get(&entry.session_id) {
            let host = self
                .host_view
                .host_name(&ssh.host_id, cx)
                .unwrap_or_else(|| ssh.host_id.clone());
            spec.title = SharedString::from(process.filter(|t| !t.is_empty()).unwrap_or(host));
        } else {
            spec.title = SharedString::from(
                process
                    .filter(|t| !t.is_empty())
                    .or_else(|| {
                        cwd.as_deref().map(|c| {
                            c.rsplit('/')
                                .find(|s| !s.is_empty())
                                .unwrap_or("/")
                                .to_string()
                        })
                    })
                    .unwrap_or_else(|| "Terminal".to_string()),
            );
        }
        spec.trailing = cwd.map(SharedString::from);
    }

    /// The terminal view, or the SSH loading screen while its connection is
    /// still being established (per pane, so a split next to a connecting
    /// pane keeps working).
    pub(super) fn terminal_pane_body(
        &mut self,
        pane: PaneId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(entry) = self.panes.get(&pane) else {
            return gpui::div().size_full().into_any_element();
        };
        let view = entry.view.clone();
        let blocking = self
            .ssh_tabs
            .get(&entry.session_id)
            .map(|t| t.ssh_id.clone())
            .and_then(|ssh_id| {
                self.ssh_connection
                    .read(cx)
                    .get(&ssh_id)
                    .filter(|e| e.state.is_blocking())
                    .cloned()
            });
        match blocking {
            Some(connection) => self.render_ssh_loading(connection, cx),
            None => view.into_any_element(),
        }
    }

    /// New shell in the same cwd. An SSH pane opens a new connection to the
    /// same host: the SSH service has no channel multiplexing, so a host that
    /// needs a typed password asks for it again.
    pub(super) fn clone_terminal_pane(
        &mut self,
        source: PaneId,
        new_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), SplitCloneError> {
        let ssh_host = self
            .panes
            .get(&source)
            .and_then(|entry| self.ssh_tabs.get(&entry.session_id))
            .map(|t| t.host_id.clone());
        if let Some(host_id) = ssh_host {
            let tab_id = self.tab_of_pane(source).unwrap_or(source);
            let (ssh_id, session_id, feed, handle) =
                self.new_ssh_session(&host_id, cx).ok_or_else(|| {
                    SplitCloneError::Failed(
                        "The SSH session for the new pane could not be created.".to_string(),
                    )
                })?;
            self.attach_ssh_terminal(
                host_id, ssh_id, session_id, feed, handle, new_id, tab_id, window, cx,
            );
            return Ok(());
        }
        let cwd = self
            .panes
            .get(&source)
            .and_then(|entry| entry.view.read(cx).cwd())
            .or_else(|| self.terminal_spawn_cwd(cx));
        let (session_id, handle, scrollback_id) =
            self.spawn_session(cwd, None, cx).ok_or_else(|| {
                SplitCloneError::Failed("The shell could not be started.".to_string())
            })?;
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            new_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.save_session_soon(cx);
        Ok(())
    }

    /// Close the session and drop the view of a terminal pane.
    pub(super) fn retire_terminal_pane(&mut self, pane: PaneId, cx: &mut Context<Self>) {
        let session = self.panes.get(&pane).map(|e| e.session_id);
        self.retire_pane(pane);
        if let Some(session) = session {
            self.disconnect_ssh_session(session, cx);
        }
    }
}
