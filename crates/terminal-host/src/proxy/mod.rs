//! App-side proxies of the daemon's SSH / SFTP / transfer services (ADR 0009).
//!
//! [`DaemonServices`] implements the UI-free service traits (`SshConnectionService`,
//! `SshPtyService`, `SshRemoteCommandService`, `SshRemoteFileService`,
//! `SshTunnelService`, `SftpSessionService`, `SftpBrowserService`,
//! `TransferService`) over the shared [`HostClient`], so app code does not
//! change. Daemon service events are re-emitted on the app's [`EventBus`] and
//! reach the existing `SshEventSourceAdapter` / `TransferEventSourceAdapter`.
//!
//! * Calls never block the client's reader: every request is answered
//!   out of order (an `SshConnect` waits minutes for a host-key decision).
//! * [`router::RoutedServices`] picks app or daemon per SSH connection when
//!   it is made and keeps a session -> host map so later calls, and transfer
//!   jobs (which belong to the host of their SSH session), follow it.
//! * Credentials are resolved from the app's own hosts database and secret
//!   store ([`snapshot::build`]) and sent with the connect request.
//!
//! GPUI-free. Unix only, like the client.

mod hosted;
pub mod router;
pub mod snapshot;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use base64::Engine as _;
use labonair_errors::LabonairError;
use labonair_events::EventBus;
use labonair_sftp::{
    BoxFuture as SftpFuture, RemoteEntry, SftpBrowserService, SftpSessionHandle, SftpSessionService,
};
use labonair_ssh::{
    ActiveTunnel, BoxFuture, SharedSshEventSink, SshConnectRequest, SshConnectionService,
    SshExecOutput, SshHostedAccess, SshPtyService, SshRemoteCommandService, SshRemoteFileService,
    SshSessionId, SshTunnelService,
};
use labonair_transfers::{TransferRequest, TransferResolution, TransferService};

use self::hosted::HostedSsh;
use crate::client::{HostClient, SessionSink};
use crate::protocol::{
    RpcCall, RpcValue, SessionId, SshConnectParams, SshHostSnapshot, SshSessionInfo,
    TransferConfig, WireColors, WireError,
};
use crate::spawn::shared_client;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Bound for the few calls that are not async (restore on the UI thread).
const BLOCKING_TIMEOUT: Duration = Duration::from_secs(5);

/// Rows of the app's hosts database and secret store for one host.
pub type HostSnapshots = Arc<dyn Fn(&str) -> Result<SshHostSnapshot, String> + Send + Sync>;

/// What the proxies need from the application, supplied by the composition root.
#[derive(Clone)]
pub struct ProxyContext {
    /// The app's event bus: daemon service events are re-emitted here.
    pub bus: EventBus,
    pub hosts: HostSnapshots,
    /// The transfer settings sent to the daemon after every connect.
    pub transfer_config: Arc<dyn Fn() -> TransferConfig + Send + Sync>,
    /// A host connected through the daemon (the app records its recents).
    pub connected: Arc<dyn Fn(&str) + Send + Sync>,
}

/// The daemon's SSH / SFTP / transfer services as seen from the app. Cheap to
/// clone; all clones share the connection and the session table.
#[derive(Clone)]
pub struct DaemonServices {
    inner: Arc<Inner>,
}

struct Inner {
    client: HostClient,
    ctx: ProxyContext,
    /// By SSH session id (the app's tab id).
    sessions: Mutex<HashMap<String, Arc<HostedSsh>>>,
    /// Last known forwards of the daemon's tunnel service (`active` is sync).
    tunnels: Mutex<Vec<ActiveTunnel>>,
}

static CONTEXT: Mutex<Option<ProxyContext>> = Mutex::new(None);
static SHARED: Mutex<Option<DaemonServices>> = Mutex::new(None);

/// Install the application context once, at composition. Needed before
/// [`DaemonServices::shared`].
pub fn install_context(ctx: ProxyContext) {
    *lock(&CONTEXT) = Some(ctx);
}

impl DaemonServices {
    /// Proxies over `client`. Subscribes to the daemon's service events: the
    /// daemon replays its current state first (live SSH sessions, transfer
    /// jobs), then streams live events onto `ctx.bus`.
    pub fn new(client: HostClient, ctx: ProxyContext) -> Self {
        let bus = ctx.bus.clone();
        client.subscribe_events(Arc::new(move |name, payload| {
            let _ = bus.emit(&name, payload);
        }));
        Self {
            inner: Arc::new(Inner {
                client,
                ctx,
                sessions: Mutex::new(HashMap::new()),
                tunnels: Mutex::new(Vec::new()),
            }),
        }
    }

    /// The process-wide instance, if one was created (never connects).
    pub fn peek() -> Option<Self> {
        lock(&SHARED).clone()
    }

    /// The process-wide instance on the shared daemon connection. With `start`
    /// a missing daemon is started; without it only a running one is used
    /// (restore). Blocks while the daemon starts: call off the UI thread or
    /// accept the bounded wait.
    pub fn shared(data_dir: &Path, scrollback_lines: u32, start: bool) -> Result<Self, String> {
        let client = shared_client(data_dir, scrollback_lines, start)?;
        let mut slot = lock(&SHARED);
        if let Some(existing) = slot.as_ref() {
            if existing.inner.client.data_dir() == client.data_dir() {
                return Ok(existing.clone());
            }
        }
        let ctx = lock(&CONTEXT)
            .clone()
            .ok_or_else(|| "the session host proxies are not configured".to_string())?;
        let services = Self::new(client, ctx);
        *slot = Some(services.clone());
        Ok(services)
    }

    pub fn client(&self) -> &HostClient {
        &self.inner.client
    }

    fn session(&self, ssh_id: &str) -> Option<Arc<HostedSsh>> {
        lock(&self.inner.sessions).get(ssh_id).cloned()
    }

    fn session_entry(&self, ssh_id: &str) -> Arc<HostedSsh> {
        Arc::clone(
            lock(&self.inner.sessions)
                .entry(ssh_id.to_string())
                .or_insert_with(|| {
                    HostedSsh::new(
                        self.inner.client.clone(),
                        self.inner.ctx.bus.clone(),
                        ssh_id.to_string(),
                    )
                }),
        )
    }

    /// Whether the daemon owns the SSH session `ssh_id` (as far as this app
    /// knows: it connected it or adopted it).
    pub fn owns(&self, ssh_id: &str) -> bool {
        lock(&self.inner.sessions).contains_key(ssh_id)
    }

    async fn call(&self, call: RpcCall) -> Result<RpcValue, WireError> {
        if !self.inner.client.is_connected() {
            // The daemon may still be there (the connection alone dropped).
            let _ = self.inner.client.reconnect();
        }
        self.inner.client.rpc(call).await
    }

    async fn unit(&self, call: RpcCall) -> Result<(), WireError> {
        self.call(call).await.map(|_| ())
    }

    // ── Hosted terminal panes ────────────────────────────────────────────────

    /// A terminal pane will show the SSH session `ssh_id`: from now on its
    /// output, steal / loss notices go to `pane` (not to the connect call's
    /// sink). Call before connecting.
    pub fn bind_pane(
        &self,
        ssh_id: &str,
        pane_tag: &str,
        colors: Option<WireColors>,
        size: (u16, u16),
        pane: Arc<dyn SessionSink>,
    ) {
        self.session_entry(ssh_id)
            .bind_pane(pane, Some(pane_tag.to_string()), colors, size);
    }

    /// Show a session that is already alive in the daemon (restore after an
    /// app restart): register the pane and attach it.
    pub fn adopt(
        &self,
        ssh_id: &str,
        wire: SessionId,
        pane_tag: &str,
        size: (u16, u16),
        pane: Arc<dyn SessionSink>,
    ) -> Result<(), String> {
        let hosted = self.session_entry(ssh_id);
        hosted.bind_pane(pane, Some(pane_tag.to_string()), None, size);
        hosted.reopen();
        hosted.set_wire(wire);
        self.inner
            .client
            .attach(wire, size.0, size.1)
            .map_err(|e| e.to_string())
    }

    /// The wire id of the daemon session behind `ssh_id`, once connected.
    pub fn wire_id(&self, ssh_id: &str) -> Option<SessionId> {
        self.session(ssh_id).and_then(|s| s.wire())
    }

    /// Whether the session's SSH connection was reported lost.
    pub fn is_lost(&self, ssh_id: &str) -> bool {
        self.session(ssh_id).is_some_and(|s| s.is_lost())
    }

    /// User input of the pane (the hot path: no RPC).
    pub fn input(&self, ssh_id: &str, bytes: &[u8]) {
        if let Some(wire) = self.wire_id(ssh_id) {
            self.inner.client.input(wire, bytes);
        }
    }

    pub fn resize(&self, ssh_id: &str, cols: u16, rows: u16) {
        if let Some(session) = self.session(ssh_id) {
            session.set_size(cols, rows);
            if let Some(wire) = session.wire() {
                self.inner.client.resize(wire, cols, rows);
            }
        }
    }

    /// Attach again after the pane was stolen or the host connection dropped.
    pub fn reattach(&self, ssh_id: &str) -> Result<(), String> {
        let session = self
            .session(ssh_id)
            .ok_or_else(|| "the session is gone".to_string())?;
        let wire = session
            .wire()
            .ok_or_else(|| "the session is not connected yet".to_string())?;
        self.inner.client.reconnect().map_err(|e| e.to_string())?;
        let (cols, rows) = session.size();
        self.inner
            .client
            .attach(wire, cols, rows)
            .map_err(|e| e.to_string())
    }

    /// The tab was closed: end the session (acknowledge it when the connection
    /// was already lost) and forget it.
    pub fn close_pane(&self, ssh_id: &str) {
        let Some(session) = lock(&self.inner.sessions).remove(ssh_id) else {
            return;
        };
        let lost = session.is_lost();
        if let Some(wire) = session.clear_wire() {
            if lost {
                self.inner.client.ack(wire);
            } else {
                self.inner.client.kill(wire);
            }
        }
    }

    /// The app is quitting: leave the session running in the daemon.
    pub fn detach_pane(&self, ssh_id: &str) {
        if let Some(session) = self.session(ssh_id) {
            if let Some(wire) = session.clear_wire() {
                self.inner.client.detach(wire);
            }
        }
    }

    /// SSH sessions alive in the daemon (restore, recovery). Blocking.
    pub fn list_ssh_sessions_blocking(&self) -> Result<Vec<SshSessionInfo>, String> {
        match self
            .inner
            .client
            .rpc_blocking(RpcCall::ListSshSessions, BLOCKING_TIMEOUT)
        {
            Ok(RpcValue::SshSessions(sessions)) => Ok(sessions),
            Ok(_) => Err("unexpected reply".into()),
            Err(error) => Err(error.message),
        }
    }

    async fn refresh_tunnels(&self) {
        if let Ok(RpcValue::Tunnels(active)) = self.call(RpcCall::SshTunnelsActive).await {
            *lock(&self.inner.tunnels) = active;
        }
    }
}

fn unexpected() -> WireError {
    WireError::plain("unexpected reply from the background session host")
}

fn typed(error: WireError) -> LabonairError {
    error.into_labonair()
}

fn plain(error: WireError) -> String {
    error.message
}

fn text(value: RpcValue) -> Result<String, WireError> {
    match value {
        RpcValue::Text(text) => Ok(text),
        _ => Err(unexpected()),
    }
}

impl SshConnectionService for DaemonServices {
    fn connect<'a>(
        &'a self,
        request: SshConnectRequest,
        events: SharedSshEventSink,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            let ssh_id = request.session_id.as_str().to_string();
            let host = (self.inner.ctx.hosts)(&request.host_id).map_err(LabonairError::Internal)?;
            let session = self.session_entry(&ssh_id);
            session.reopen();
            session.set_events(events);
            // Once per connect, from the app's current transfer settings.
            let _ = self
                .unit(RpcCall::TransferConfigure(
                    (self.inner.ctx.transfer_config)(),
                ))
                .await;
            // A pane's own size wins over the request's placeholder.
            let (pane_cols, pane_rows) = session.size();
            let params = SshConnectParams {
                session_id: ssh_id.clone(),
                host_id: request.host_id.clone(),
                password: request.password,
                passphrase: request.passphrase,
                initial_cols: Some(if session.has_pane() {
                    u32::from(pane_cols)
                } else {
                    request.initial_cols.unwrap_or(u32::from(pane_cols))
                }),
                initial_rows: Some(if session.has_pane() {
                    u32::from(pane_rows)
                } else {
                    request.initial_rows.unwrap_or(u32::from(pane_rows))
                }),
                blocks: request.blocks,
                connect_timeout_secs: request.connect_timeout_secs,
                keepalive_interval_secs: request.keepalive_interval_secs,
                keepalive_max_failures: request.keepalive_max_failures,
                host,
                pane_tag: session.pane_tag(),
                colors: session.colors(),
                ..SshConnectParams::default()
            };
            let wire = match self.call(RpcCall::SshConnect(params)).await {
                Ok(RpcValue::SshConnected { session }) => session,
                Ok(_) => return Err(typed(unexpected())),
                Err(error) => {
                    // An unbound session (SFTP, agent tab) has nothing to keep.
                    if !session.has_pane() {
                        lock(&self.inner.sessions).remove(&ssh_id);
                    }
                    return Err(typed(error));
                }
            };
            if session.is_closed() {
                // The tab went away while the connect was running.
                self.inner.client.kill(wire);
                lock(&self.inner.sessions).remove(&ssh_id);
                return Err(LabonairError::Internal("the session was closed".into()));
            }
            session.set_wire(wire);
            let (cols, rows) = session.size();
            let _ = self.inner.client.attach(wire, cols, rows);
            (self.inner.ctx.connected)(&request.host_id);
            Ok(())
        })
    }

    fn disconnect<'a>(&'a self, session_id: SshSessionId) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let ssh_id = session_id.as_str();
            if let Some(session) = lock(&self.inner.sessions).remove(ssh_id) {
                session.clear_wire();
            }
            self.unit(RpcCall::SshDisconnect {
                session_id: ssh_id.to_string(),
            })
            .await
            .map_err(plain)
        })
    }

    fn trust_host<'a>(
        &'a self,
        session_id: SshSessionId,
        accepted: bool,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::SshTrustHost {
                session_id: session_id.as_str().to_string(),
                accepted,
            })
            .await
            .map_err(plain)
        })
    }
}

impl SshPtyService for DaemonServices {
    fn write<'a>(
        &'a self,
        session_id: SshSessionId,
        data: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            if let Some(wire) = self.wire_id(session_id.as_str()) {
                self.inner.client.input(wire, data.as_bytes());
                return Ok(());
            }
            self.unit(RpcCall::SshPtyWrite {
                session_id: session_id.as_str().to_string(),
                data,
            })
            .await
            .map_err(plain)
        })
    }

    fn resize<'a>(
        &'a self,
        session_id: SshSessionId,
        cols: u32,
        rows: u32,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::SshPtyResize {
                session_id: session_id.as_str().to_string(),
                cols,
                rows,
            })
            .await
            .map_err(plain)
        })
    }
}

impl SshRemoteCommandService for DaemonServices {
    fn chown<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
        owner: String,
        group: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::SshChown {
                session_id: session_id.as_str().to_string(),
                path,
                owner,
                group,
            })
            .await
            .map_err(plain)
        })
    }

    fn calculate_size<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            self.call(RpcCall::SshCalculateSize {
                session_id: session_id.as_str().to_string(),
                path,
            })
            .await
            .and_then(text)
            .map_err(plain)
        })
    }
}

impl SshRemoteFileService for DaemonServices {
    fn prepare_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        max_bytes: Option<u64>,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            self.call(RpcCall::SshPrepareRemoteEdit {
                session_id: session_id.as_str().to_string(),
                remote_path,
                max_bytes,
            })
            .await
            .and_then(text)
            .map_err(plain)
        })
    }

    fn save_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::SshSaveRemoteEdit {
                session_id: session_id.as_str().to_string(),
                remote_path,
                local_temp_path,
            })
            .await
            .map_err(plain)
        })
    }

    fn cleanup_remote_edit_temp<'a>(
        &'a self,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::SshCleanupRemoteEditTemp { local_temp_path })
                .await
                .map_err(plain)
        })
    }
}

impl SshTunnelService for DaemonServices {
    fn start<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let host = (self.inner.ctx.hosts)(&host_id)?;
            let result = self
                .unit(RpcCall::SshTunnelStart { host_id, host })
                .await
                .map_err(plain);
            self.refresh_tunnels().await;
            result
        })
    }

    fn stop<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let result = self
                .unit(RpcCall::SshTunnelStop { host_id })
                .await
                .map_err(plain);
            self.refresh_tunnels().await;
            result
        })
    }

    fn active(&self) -> Vec<ActiveTunnel> {
        lock(&self.inner.tunnels).clone()
    }
}

impl SftpSessionService for DaemonServices {
    fn open<'a>(
        &'a self,
        ssh_session: SshSessionId,
    ) -> SftpFuture<'a, Result<SftpSessionHandle, LabonairError>> {
        Box::pin(async move {
            self.call(RpcCall::SftpOpen {
                ssh_session: ssh_session.as_str().to_string(),
            })
            .await
            .map(|_| SftpSessionHandle::from_ssh_session(ssh_session))
            .map_err(typed)
        })
    }

    fn close<'a>(&'a self, session: SftpSessionHandle) -> SftpFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::SftpClose {
                session: session.as_str().to_string(),
            })
            .await
            .map_err(plain)
        })
    }
}

impl SftpBrowserService for DaemonServices {
    fn read_dir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> SftpFuture<'a, Result<Vec<RemoteEntry>, LabonairError>> {
        Box::pin(async move {
            match self
                .call(RpcCall::SftpReadDir {
                    session: session.as_str().to_string(),
                    path,
                })
                .await
            {
                Ok(RpcValue::Entries(entries)) => Ok(entries),
                Ok(_) => Err(typed(unexpected())),
                Err(error) => Err(typed(error)),
            }
        })
    }

    fn rename<'a>(
        &'a self,
        session: SftpSessionHandle,
        old_path: String,
        new_path: String,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            self.unit(RpcCall::SftpRename {
                session: session.as_str().to_string(),
                old_path,
                new_path,
            })
            .await
            .map_err(typed)
        })
    }

    fn delete<'a>(
        &'a self,
        session: SftpSessionHandle,
        paths: Vec<String>,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            self.unit(RpcCall::SftpDelete {
                session: session.as_str().to_string(),
                paths,
            })
            .await
            .map_err(typed)
        })
    }

    fn mkdir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        recursive: bool,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            self.unit(RpcCall::SftpMkdir {
                session: session.as_str().to_string(),
                path,
                recursive,
            })
            .await
            .map_err(typed)
        })
    }

    fn create_file<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            self.unit(RpcCall::SftpCreateFile {
                session: session.as_str().to_string(),
                path,
            })
            .await
            .map_err(typed)
        })
    }

    fn chmod<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        mode: u32,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            self.unit(RpcCall::SftpChmod {
                session: session.as_str().to_string(),
                path,
                mode,
            })
            .await
            .map_err(typed)
        })
    }
}

impl TransferService for DaemonServices {
    fn enqueue<'a>(
        &'a self,
        request: TransferRequest,
    ) -> labonair_transfers::BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            match self
                .call(RpcCall::TransferEnqueue {
                    session_id: request.session_id,
                    src_path: request.src_path,
                    dest_path: request.dest_path,
                    direction: request.direction,
                })
                .await
            {
                Ok(RpcValue::JobId(id)) => Ok(id),
                Ok(_) => Err(unexpected().message),
                Err(error) => Err(error.message),
            }
        })
    }

    fn cancel<'a>(
        &'a self,
        job_id: String,
    ) -> labonair_transfers::BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::TransferCancel { job_id })
                .await
                .map_err(plain)
        })
    }

    fn resolve<'a>(
        &'a self,
        job_id: String,
        resolution: TransferResolution,
    ) -> labonair_transfers::BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.unit(RpcCall::TransferResolve {
                job_id,
                resolution: resolution.into(),
            })
            .await
            .map_err(plain)
        })
    }
}

impl SshHostedAccess for DaemonServices {
    fn is_hosted(&self, session_id: &str) -> bool {
        self.owns(session_id)
    }

    fn exec<'a>(
        &'a self,
        session_id: SshSessionId,
        command: String,
        stdin: Option<Vec<u8>>,
    ) -> BoxFuture<'a, Result<SshExecOutput, String>> {
        Box::pin(async move {
            let b64 = base64::engine::general_purpose::STANDARD;
            let reply = self
                .call(RpcCall::SshExec {
                    session_id: session_id.as_str().to_string(),
                    command,
                    stdin: stdin.map(|bytes| b64.encode(bytes)),
                })
                .await
                .map_err(plain)?;
            let RpcValue::Exec {
                stdout,
                stderr,
                exit_code,
            } = reply
            else {
                return Err(unexpected().message);
            };
            let decode = |text: String| b64.decode(text).map_err(|e| e.to_string());
            Ok(SshExecOutput {
                stdout: decode(stdout)?,
                stderr: decode(stderr)?,
                exit_code,
            })
        })
    }

    fn write_pty<'a>(
        &'a self,
        session_id: SshSessionId,
        data: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        SshPtyService::write(self, session_id, data)
    }

    fn agent_tap(&self, session_id: &str) -> Option<tokio::sync::broadcast::Receiver<String>> {
        self.session(session_id).map(|s| s.tap.subscribe())
    }
}
