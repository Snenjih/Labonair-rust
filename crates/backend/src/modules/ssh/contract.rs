//! Backend adapters for the UI-free SSH capability contracts.

use super::{client, pty};
use crate::{App, EventChannel};
use labonair_errors::LabonairError;
use labonair_ssh::{
    BoxFuture, SharedSshEventSink, SshConnectRequest, SshConnectionService, SshPtyService,
    SshSessionEvent, SshSessionId,
};

#[derive(Clone)]
pub struct BackendSshService {
    app: App,
}

impl BackendSshService {
    pub fn new(app: App) -> Self {
        Self { app }
    }
}

impl SshConnectionService for BackendSshService {
    fn connect<'a>(
        &'a self,
        request: SshConnectRequest,
        events: SharedSshEventSink,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let session_id = request.session_id.into();
            let on_event = EventChannel::new(move |event: pty::SshPtyEvent| match event {
                pty::SshPtyEvent::Data { data } => events.send(SshSessionEvent::Output { data }),
            });
            client::ssh_connect(
                session_id,
                request.host_id,
                request.passphrase,
                request.password,
                request.initial_cols,
                request.initial_rows,
                request.blocks,
                on_event,
                &app.ssh,
                &app.trust,
                &app.db,
                &app.secrets,
                app.clone(),
                request.connect_timeout_secs,
            )
            .await
        })
    }

    fn disconnect<'a>(&'a self, session_id: SshSessionId) -> BoxFuture<'a, Result<(), String>> {
        let app = self.app.clone();
        Box::pin(async move { client::ssh_disconnect(session_id.into(), &app.ssh).await })
    }

    fn trust_host<'a>(
        &'a self,
        session_id: SshSessionId,
        accepted: bool,
    ) -> BoxFuture<'a, Result<(), String>> {
        let app = self.app.clone();
        Box::pin(
            async move { client::ssh_trust_host(session_id.into(), accepted, &app.trust).await },
        )
    }
}

impl SshPtyService for BackendSshService {
    fn write<'a>(
        &'a self,
        session_id: SshSessionId,
        data: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        let state = self.app.ssh.clone();
        Box::pin(async move { pty::ssh_pty_write(session_id.into(), data, &state).await })
    }

    fn resize<'a>(
        &'a self,
        session_id: SshSessionId,
        cols: u32,
        rows: u32,
    ) -> BoxFuture<'a, Result<(), String>> {
        let state = self.app.ssh.clone();
        Box::pin(async move { pty::ssh_pty_resize(session_id.into(), cols, rows, &state).await })
    }
}
