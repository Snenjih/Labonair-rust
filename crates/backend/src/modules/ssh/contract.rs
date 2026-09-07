//! Backend adapters for the UI-free SSH capability contracts.

use super::{client, config_parser, pty, sftp as remote, tunnels};
use crate::{App, EventBus, EventChannel};
use labonair_errors::LabonairError;
use labonair_ssh::{
    ActiveTunnel, BoxFuture, ImportConflict, SharedSshEventSink, SshConfigEntry, SshConfigService,
    SshConnectRequest, SshConnectionEvent, SshConnectionService, SshConnectionTester,
    SshEventReceiver, SshEventSource, SshPtyService, SshRemoteCommandService, SshRemoteFileService,
    SshSessionEvent, SshSessionId, SshTestResult, SshTunnelService,
};
use serde::Deserialize;

#[derive(Clone)]
pub struct BackendSshService {
    app: App,
}

impl BackendSshService {
    pub fn new(app: App) -> Self {
        Self { app }
    }
}

/// Narrow adapter for PTY I/O. It deliberately owns only the SSH session
/// registry because writing and resizing an existing session do not require
/// host, trust, database, or secret state.
#[derive(Clone)]
pub struct BackendSshPtyService {
    state: super::SshState,
}

impl BackendSshPtyService {
    pub fn new(state: super::SshState) -> Self {
        Self { state }
    }
}

/// Narrow adapter for remote file operations that need connection-loss
/// reporting. It does not retain the aggregate backend App.
#[derive(Clone)]
pub struct BackendSshRemoteService {
    state: super::SshState,
    events: EventBus,
}

impl BackendSshRemoteService {
    pub fn new(state: super::SshState, events: EventBus) -> Self {
        Self { state, events }
    }
}

/// Shell-composed adapter that translates the legacy backend event stream into
/// the narrow SSH connection-event contract.
#[derive(Clone)]
pub struct BackendSshEventSource {
    events: EventBus,
}

impl BackendSshEventSource {
    pub fn new(events: EventBus) -> Self {
        Self { events }
    }
}

struct BackendSshEventReceiver {
    receiver: tokio::sync::broadcast::Receiver<crate::RawEvent>,
}

#[derive(Deserialize)]
struct ConnectLogPayload {
    session_id: String,
    message: String,
}

#[derive(Deserialize)]
struct KnownHostsWarningPayload {
    session_id: String,
    fingerprint: String,
    host: String,
    is_mismatch: bool,
}

#[derive(Deserialize)]
struct AuthRequiredPayload {
    session_id: String,
    prompt_message: String,
    is_2fa: bool,
}

#[derive(Deserialize)]
struct SessionPayload {
    session_id: String,
    #[serde(default)]
    default_path: Option<String>,
}

fn decode_connection_event(raw: &crate::RawEvent) -> Option<SshConnectionEvent> {
    match raw.name.as_str() {
        "ssh_connect_log" => serde_json::from_value::<ConnectLogPayload>(raw.payload.clone())
            .ok()
            .map(|payload| SshConnectionEvent::ConnectLog {
                session_id: payload.session_id,
                message: payload.message,
            }),
        "known_hosts_warning" => {
            serde_json::from_value::<KnownHostsWarningPayload>(raw.payload.clone())
                .ok()
                .map(|payload| SshConnectionEvent::KnownHostsWarning {
                    session_id: payload.session_id,
                    fingerprint: payload.fingerprint,
                    host: payload.host,
                    is_mismatch: payload.is_mismatch,
                })
        }
        "auth_required" => serde_json::from_value::<AuthRequiredPayload>(raw.payload.clone())
            .ok()
            .map(|payload| SshConnectionEvent::AuthRequired {
                session_id: payload.session_id,
                prompt_message: payload.prompt_message,
                is_2fa: payload.is_2fa,
            }),
        "passphrase_required" => serde_json::from_value::<SessionPayload>(raw.payload.clone())
            .ok()
            .map(|payload| SshConnectionEvent::PassphraseRequired {
                session_id: payload.session_id,
            }),
        "session_established" => serde_json::from_value::<SessionPayload>(raw.payload.clone())
            .ok()
            .map(|payload| SshConnectionEvent::SessionEstablished {
                session_id: payload.session_id,
                default_path: payload.default_path,
            }),
        "ssh_connection_lost" => serde_json::from_value::<SessionPayload>(raw.payload.clone())
            .ok()
            .map(|payload| SshConnectionEvent::ConnectionLost {
                session_id: payload.session_id,
            }),
        _ => None,
    }
}

impl SshEventReceiver for BackendSshEventReceiver {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Option<SshConnectionEvent>> {
        Box::pin(async move {
            loop {
                match self.receiver.recv().await {
                    Ok(raw) => {
                        let event = decode_connection_event(&raw);
                        if event.is_some() {
                            return event;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        log::warn!("SSH event source lagged; resyncing ({skipped} events)");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                }
            }
        })
    }
}

impl SshEventSource for BackendSshEventSource {
    fn subscribe(&self) -> Box<dyn SshEventReceiver> {
        Box::new(BackendSshEventReceiver {
            receiver: self.events.subscribe(),
        })
    }
}

impl From<config_parser::SshConfigEntry> for SshConfigEntry {
    fn from(entry: config_parser::SshConfigEntry) -> Self {
        Self {
            alias: entry.alias,
            host_address: entry.host_address,
            port: entry.port,
            username: entry.username,
            auth_method: entry.auth_method,
            private_key_path: entry.private_key_path,
            proxy_jump: entry.proxy_jump,
        }
    }
}

impl From<SshConfigEntry> for config_parser::SshConfigEntry {
    fn from(entry: SshConfigEntry) -> Self {
        Self {
            alias: entry.alias,
            host_address: entry.host_address,
            port: entry.port,
            username: entry.username,
            auth_method: entry.auth_method,
            private_key_path: entry.private_key_path,
            proxy_jump: entry.proxy_jump,
        }
    }
}

impl From<config_parser::ImportConflict> for ImportConflict {
    fn from(conflict: config_parser::ImportConflict) -> Self {
        match conflict {
            config_parser::ImportConflict::Skip => Self::Skip,
            config_parser::ImportConflict::Overwrite => Self::Overwrite,
            config_parser::ImportConflict::Rename => Self::Rename,
        }
    }
}

impl From<ImportConflict> for config_parser::ImportConflict {
    fn from(conflict: ImportConflict) -> Self {
        match conflict {
            ImportConflict::Skip => Self::Skip,
            ImportConflict::Overwrite => Self::Overwrite,
            ImportConflict::Rename => Self::Rename,
        }
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
                app.events.clone(),
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

impl SshPtyService for BackendSshPtyService {
    fn write<'a>(
        &'a self,
        session_id: SshSessionId,
        data: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        let state = self.state.clone();
        Box::pin(async move { pty::ssh_pty_write(session_id.into(), data, &state).await })
    }

    fn resize<'a>(
        &'a self,
        session_id: SshSessionId,
        cols: u32,
        rows: u32,
    ) -> BoxFuture<'a, Result<(), String>> {
        let state = self.state.clone();
        Box::pin(async move { pty::ssh_pty_resize(session_id.into(), cols, rows, &state).await })
    }
}

impl SshRemoteCommandService for BackendSshRemoteService {
    fn chown<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
        owner: String,
        group: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        let state = self.state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            remote::sftp_chown(session_id.into(), path, owner, group, &state, events)
                .await
                .map_err(|error| error.to_string())
        })
    }

    fn calculate_size<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
    ) -> BoxFuture<'a, Result<String, String>> {
        let state = self.state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            remote::sftp_calculate_size(session_id.into(), path, &state, events)
                .await
                .map_err(|error| error.to_string())
        })
    }
}

impl SshRemoteFileService for BackendSshRemoteService {
    fn prepare_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        max_bytes: Option<u64>,
    ) -> BoxFuture<'a, Result<String, String>> {
        let state = self.state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            remote::prepare_remote_edit(
                session_id.into(),
                remote_path,
                max_bytes,
                &state,
                events.clone(),
            )
            .await
            .map_err(|error| error.to_string())
        })
    }

    fn save_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        let state = self.state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            remote::save_remote_edit(
                session_id.into(),
                remote_path,
                local_temp_path,
                &state,
                events,
            )
            .await
            .map_err(|error| error.to_string())
        })
    }

    fn cleanup_remote_edit_temp<'a>(
        &'a self,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            remote::cleanup_remote_edit_temp(local_temp_path)
                .await
                .map_err(|error| error.to_string())
        })
    }
}

impl SshConnectionTester for BackendSshService {
    fn test<'a>(
        &'a self,
        host_id: String,
        passphrase: Option<String>,
        password_override: Option<String>,
        connect_timeout_secs: Option<u64>,
    ) -> BoxFuture<'a, Result<SshTestResult, LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let refs = app.clone();
            client::ssh_test_connection(
                host_id,
                passphrase,
                password_override,
                &refs.trust,
                &refs.db,
                &refs.secrets,
                app.events.clone(),
                connect_timeout_secs,
            )
            .await
            .map(|result| match result {
                client::TestConnectionResult::Success => SshTestResult::Success,
                client::TestConnectionResult::UnknownHostKey { fingerprint } => {
                    SshTestResult::UnknownHostKey { fingerprint }
                }
                client::TestConnectionResult::HostKeyChanged { fingerprint } => {
                    SshTestResult::HostKeyChanged { fingerprint }
                }
            })
        })
    }
}

impl SshConfigService for BackendSshService {
    fn parse<'a>(&'a self) -> BoxFuture<'a, Result<Vec<SshConfigEntry>, String>> {
        Box::pin(async move {
            config_parser::parse_ssh_config_cmd()
                .await
                .map(|entries| entries.into_iter().map(Into::into).collect())
        })
    }

    fn import<'a>(
        &'a self,
        entries: Vec<SshConfigEntry>,
        conflict: ImportConflict,
    ) -> BoxFuture<'a, Result<Vec<String>, String>> {
        let app = self.app.clone();
        Box::pin(async move {
            let entries = entries.into_iter().map(Into::into).collect();
            config_parser::import_ssh_config_entries(entries, conflict.into(), &app.db).await
        })
    }

    fn export<'a>(&'a self, host_ids: Vec<String>) -> BoxFuture<'a, Result<String, String>> {
        let app = self.app.clone();
        Box::pin(async move { config_parser::export_ssh_config(host_ids, &app.db).await })
    }

    fn write_export<'a>(
        &'a self,
        block: String,
        append: bool,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move { config_parser::write_ssh_config_export(block, append).await })
    }
}

impl SshTunnelService for BackendSshService {
    fn start<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>> {
        let app = self.app.clone();
        Box::pin(async move {
            let refs = app.clone();
            tunnels::ssh_start_tunnels(
                host_id,
                &refs.tunnels,
                &refs.db,
                &refs.secrets,
                &refs.trust,
                app,
                None,
            )
            .await
        })
    }

    fn stop<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>> {
        let state = tunnels::TunnelState(self.app.tunnels.0.clone());
        Box::pin(async move { tunnels::ssh_stop_tunnels(host_id, &state).await })
    }

    fn active(&self) -> Vec<ActiveTunnel> {
        tunnels::active_tunnels(&self.app.tunnels)
            .into_iter()
            .map(|tunnel| ActiveTunnel {
                host_id: tunnel.host_id,
                local_port: tunnel.local_port,
                remote_host: tunnel.remote_host,
                remote_port: tunnel.remote_port,
            })
            .collect()
    }
}
