//! Picks app or daemon per SSH connection (ADR 0009: the host is fixed when a
//! session is created) and routes every later call to the host that owns it.
//!
//! In `app` mode nothing here touches the daemon: a call for a session this
//! router never routed does one hash lookup and goes to the in-process
//! service, which is exactly what the app composed before.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use labonair_errors::LabonairError;
use labonair_sftp::{
    BoxFuture as SftpFuture, RemoteEntry, SftpBrowserService, SftpSessionHandle, SftpSessionService,
};
use labonair_ssh::{
    ActiveTunnel, BoxFuture, SharedSshEventSink, SshConnectRequest, SshConnectionService,
    SshExecOutput, SshHostedAccess, SshPtyService, SshRemoteCommandService, SshRemoteFileService,
    SshSessionId, SshTunnelService,
};
use labonair_transfers::{TransferRequest, TransferResolution, TransferService};

use super::DaemonServices;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The in-process services: what the app composes without a daemon.
#[derive(Clone)]
pub struct LocalServices {
    pub connection: Arc<dyn SshConnectionService>,
    pub pty: Arc<dyn SshPtyService>,
    pub remote_command: Arc<dyn SshRemoteCommandService>,
    pub remote_file: Arc<dyn SshRemoteFileService>,
    pub tunnels: Arc<dyn SshTunnelService>,
    pub sftp_session: Arc<dyn SftpSessionService>,
    pub sftp_browser: Arc<dyn SftpBrowserService>,
    pub transfers: Arc<dyn TransferService>,
}

/// How the router reaches the daemon (replaced in tests).
#[derive(Clone)]
pub struct DaemonAccess {
    /// The instance created earlier, never connecting.
    pub peek: Arc<dyn Fn() -> Option<DaemonServices> + Send + Sync>,
    /// Connect to the daemon; with `true` start it when missing. May block.
    pub connect: Arc<dyn Fn(bool) -> Result<DaemonServices, String> + Send + Sync>,
}

/// Daemon access on the process-wide shared connection of `data_dir`.
pub fn shared_access(
    data_dir: PathBuf,
    scrollback_lines: Arc<dyn Fn() -> u32 + Send + Sync>,
) -> DaemonAccess {
    DaemonAccess {
        peek: Arc::new(DaemonServices::peek),
        connect: Arc::new(move |start| {
            DaemonServices::shared(&data_dir, scrollback_lines(), start)
        }),
    }
}

/// Whether the next new SSH connection should live in the detached host
/// (`terminal.sessionHost = detached`). Read on every connect.
pub type HostPolicy = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Clone)]
enum Route {
    Local,
    Daemon(DaemonServices),
}

#[derive(Default)]
struct Routes {
    /// SSH session id -> where it lives. Fixed for the session.
    sessions: HashMap<String, Route>,
    /// The host of a host id's last connection: where its tunnels run.
    hosts: HashMap<String, Route>,
    /// Transfer job id -> the host of its SSH session.
    jobs: HashMap<String, Route>,
}

struct Inner {
    local: LocalServices,
    daemon: DaemonAccess,
    detached: HostPolicy,
    /// Reports why the daemon could not be used (the connection then falls
    /// back to the app).
    on_fallback: Arc<dyn Fn(String) + Send + Sync>,
    routes: Mutex<Routes>,
}

/// All SSH, SFTP and transfer service traits, routed per session.
#[derive(Clone)]
pub struct RoutedServices {
    inner: Arc<Inner>,
}

impl RoutedServices {
    pub fn new(
        local: LocalServices,
        daemon: DaemonAccess,
        detached: HostPolicy,
        on_fallback: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                local,
                daemon,
                detached,
                on_fallback,
                routes: Mutex::new(Routes::default()),
            }),
        }
    }

    /// At startup: join a daemon that is already running (never start one) so
    /// its live SSH sessions and transfer jobs replay onto the event bus and
    /// show up in the transfers panel. Blocks briefly; call off the UI thread.
    pub fn attach_running_daemon(&self) {
        if let Err(error) = (self.inner.daemon.connect)(false) {
            tracing::debug!(%error, "no background session host to rejoin");
        }
    }

    fn peek(&self) -> Option<DaemonServices> {
        (self.inner.daemon.peek)()
    }

    /// The host that owns `ssh_id`: a recorded route, a session a pane or a
    /// restore registered with the daemon proxies, else the app.
    fn route(&self, ssh_id: &str) -> Route {
        if let Some(route) = lock(&self.inner.routes).sessions.get(ssh_id) {
            return route.clone();
        }
        match self.peek() {
            Some(daemon) if daemon.owns(ssh_id) => Route::Daemon(daemon),
            _ => Route::Local,
        }
    }

    fn route_host(&self, host_id: &str) -> Route {
        lock(&self.inner.routes)
            .hosts
            .get(host_id)
            .cloned()
            .unwrap_or(Route::Local)
    }

    /// A job this app did not enqueue (replayed after a restart) can only be
    /// the daemon's: app-bound jobs die with the app.
    fn route_job(&self, job_id: &str) -> Route {
        if let Some(route) = lock(&self.inner.routes).jobs.get(job_id) {
            return route.clone();
        }
        self.peek().map_or(Route::Local, Route::Daemon)
    }

    /// Decide where a new connection lives. A retry of a known session stays
    /// where it is.
    async fn choose(&self, ssh_id: &str) -> Route {
        let routed = lock(&self.inner.routes).sessions.get(ssh_id).cloned();
        if let Some(route) = routed {
            return route;
        }
        if let Some(daemon) = self.peek().filter(|daemon| daemon.owns(ssh_id)) {
            // A pane registered the session with the daemon already.
            return Route::Daemon(daemon);
        }
        if !(self.inner.detached)() {
            return Route::Local;
        }
        let connect = Arc::clone(&self.inner.daemon.connect);
        match tokio::task::spawn_blocking(move || connect(true)).await {
            Ok(Ok(daemon)) => Route::Daemon(daemon),
            Ok(Err(error)) => self.fall_back(error),
            Err(error) => self.fall_back(error.to_string()),
        }
    }

    fn fall_back(&self, error: String) -> Route {
        tracing::warn!(%error, "background session host unavailable; connecting in the app");
        (self.inner.on_fallback)(error);
        Route::Local
    }

    fn forget(&self, ssh_id: &str) {
        lock(&self.inner.routes).sessions.remove(ssh_id);
    }
}

impl SshConnectionService for RoutedServices {
    fn connect<'a>(
        &'a self,
        request: SshConnectRequest,
        events: SharedSshEventSink,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            let ssh_id = request.session_id.as_str().to_string();
            let route = self.choose(&ssh_id).await;
            {
                let mut routes = lock(&self.inner.routes);
                routes.sessions.insert(ssh_id, route.clone());
                routes.hosts.insert(request.host_id.clone(), route.clone());
            }
            match route {
                Route::Local => self.inner.local.connection.connect(request, events).await,
                Route::Daemon(daemon) => daemon.connect(request, events).await,
            }
        })
    }

    fn disconnect<'a>(&'a self, session_id: SshSessionId) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let route = self.route(session_id.as_str());
            self.forget(session_id.as_str());
            match route {
                Route::Local => self.inner.local.connection.disconnect(session_id).await,
                Route::Daemon(daemon) => daemon.disconnect(session_id).await,
            }
        })
    }

    fn trust_host<'a>(
        &'a self,
        session_id: SshSessionId,
        accepted: bool,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .connection
                        .trust_host(session_id, accepted)
                        .await
                }
                Route::Daemon(daemon) => daemon.trust_host(session_id, accepted).await,
            }
        })
    }
}

impl SshPtyService for RoutedServices {
    fn write<'a>(
        &'a self,
        session_id: SshSessionId,
        data: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Local => self.inner.local.pty.write(session_id, data).await,
                Route::Daemon(daemon) => SshPtyService::write(&daemon, session_id, data).await,
            }
        })
    }

    fn resize<'a>(
        &'a self,
        session_id: SshSessionId,
        cols: u32,
        rows: u32,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Local => self.inner.local.pty.resize(session_id, cols, rows).await,
                Route::Daemon(daemon) => {
                    SshPtyService::resize(&daemon, session_id, cols, rows).await
                }
            }
        })
    }
}

impl SshRemoteCommandService for RoutedServices {
    fn chown<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
        owner: String,
        group: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .remote_command
                        .chown(session_id, path, owner, group)
                        .await
                }
                Route::Daemon(daemon) => daemon.chown(session_id, path, owner, group).await,
            }
        })
    }

    fn calculate_size<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .remote_command
                        .calculate_size(session_id, path)
                        .await
                }
                Route::Daemon(daemon) => daemon.calculate_size(session_id, path).await,
            }
        })
    }
}

impl SshRemoteFileService for RoutedServices {
    fn prepare_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        max_bytes: Option<u64>,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .remote_file
                        .prepare_remote_edit(session_id, remote_path, max_bytes)
                        .await
                }
                Route::Daemon(daemon) => {
                    daemon
                        .prepare_remote_edit(session_id, remote_path, max_bytes)
                        .await
                }
            }
        })
    }

    fn save_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .remote_file
                        .save_remote_edit(session_id, remote_path, local_temp_path)
                        .await
                }
                Route::Daemon(daemon) => {
                    daemon
                        .save_remote_edit(session_id, remote_path, local_temp_path)
                        .await
                }
            }
        })
    }

    /// Only a local temp file is involved: the app's own implementation.
    fn cleanup_remote_edit_temp<'a>(
        &'a self,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        self.inner
            .local
            .remote_file
            .cleanup_remote_edit_temp(local_temp_path)
    }
}

impl SshTunnelService for RoutedServices {
    fn start<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route_host(&host_id) {
                Route::Local => self.inner.local.tunnels.start(host_id).await,
                Route::Daemon(daemon) => SshTunnelService::start(&daemon, host_id).await,
            }
        })
    }

    fn stop<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route_host(&host_id) {
                Route::Local => self.inner.local.tunnels.stop(host_id).await,
                Route::Daemon(daemon) => SshTunnelService::stop(&daemon, host_id).await,
            }
        })
    }

    fn active(&self) -> Vec<ActiveTunnel> {
        let mut active = self.inner.local.tunnels.active();
        if let Some(daemon) = self.peek() {
            active.extend(SshTunnelService::active(&daemon));
        }
        active
    }
}

impl SftpSessionService for RoutedServices {
    fn open<'a>(
        &'a self,
        ssh_session: SshSessionId,
    ) -> SftpFuture<'a, Result<SftpSessionHandle, LabonairError>> {
        Box::pin(async move {
            match self.route(ssh_session.as_str()) {
                Route::Local => self.inner.local.sftp_session.open(ssh_session).await,
                Route::Daemon(daemon) => SftpSessionService::open(&daemon, ssh_session).await,
            }
        })
    }

    fn close<'a>(&'a self, session: SftpSessionHandle) -> SftpFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route(session.as_str()) {
                Route::Local => self.inner.local.sftp_session.close(session).await,
                Route::Daemon(daemon) => SftpSessionService::close(&daemon, session).await,
            }
        })
    }
}

impl SftpBrowserService for RoutedServices {
    fn read_dir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> SftpFuture<'a, Result<Vec<RemoteEntry>, LabonairError>> {
        Box::pin(async move {
            match self.route(session.as_str()) {
                Route::Local => self.inner.local.sftp_browser.read_dir(session, path).await,
                Route::Daemon(daemon) => daemon.read_dir(session, path).await,
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
            match self.route(session.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .sftp_browser
                        .rename(session, old_path, new_path)
                        .await
                }
                Route::Daemon(daemon) => daemon.rename(session, old_path, new_path).await,
            }
        })
    }

    fn delete<'a>(
        &'a self,
        session: SftpSessionHandle,
        paths: Vec<String>,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            match self.route(session.as_str()) {
                Route::Local => self.inner.local.sftp_browser.delete(session, paths).await,
                Route::Daemon(daemon) => daemon.delete(session, paths).await,
            }
        })
    }

    fn mkdir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        recursive: bool,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            match self.route(session.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .sftp_browser
                        .mkdir(session, path, recursive)
                        .await
                }
                Route::Daemon(daemon) => daemon.mkdir(session, path, recursive).await,
            }
        })
    }

    fn create_file<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            match self.route(session.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .sftp_browser
                        .create_file(session, path)
                        .await
                }
                Route::Daemon(daemon) => daemon.create_file(session, path).await,
            }
        })
    }

    fn chmod<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        mode: u32,
    ) -> SftpFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            match self.route(session.as_str()) {
                Route::Local => {
                    self.inner
                        .local
                        .sftp_browser
                        .chmod(session, path, mode)
                        .await
                }
                Route::Daemon(daemon) => daemon.chmod(session, path, mode).await,
            }
        })
    }
}

impl TransferService for RoutedServices {
    fn enqueue<'a>(
        &'a self,
        request: TransferRequest,
    ) -> labonair_transfers::BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let route = self.route(&request.session_id);
            let id = match &route {
                Route::Local => self.inner.local.transfers.enqueue(request).await?,
                Route::Daemon(daemon) => TransferService::enqueue(daemon, request).await?,
            };
            lock(&self.inner.routes).jobs.insert(id.clone(), route);
            Ok(id)
        })
    }

    fn cancel<'a>(
        &'a self,
        job_id: String,
    ) -> labonair_transfers::BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route_job(&job_id) {
                Route::Local => self.inner.local.transfers.cancel(job_id).await,
                Route::Daemon(daemon) => TransferService::cancel(&daemon, job_id).await,
            }
        })
    }

    fn resolve<'a>(
        &'a self,
        job_id: String,
        resolution: TransferResolution,
    ) -> labonair_transfers::BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            match self.route_job(&job_id) {
                Route::Local => self.inner.local.transfers.resolve(job_id, resolution).await,
                Route::Daemon(daemon) => {
                    TransferService::resolve(&daemon, job_id, resolution).await
                }
            }
        })
    }
}

impl SshHostedAccess for RoutedServices {
    fn is_hosted(&self, session_id: &str) -> bool {
        matches!(self.route(session_id), Route::Daemon(_))
    }

    fn exec<'a>(
        &'a self,
        session_id: SshSessionId,
        command: String,
        stdin: Option<Vec<u8>>,
    ) -> BoxFuture<'a, Result<SshExecOutput, String>> {
        Box::pin(async move {
            match self.route(session_id.as_str()) {
                Route::Daemon(daemon) => {
                    SshHostedAccess::exec(&daemon, session_id, command, stdin).await
                }
                Route::Local => Err("the session is not hosted by the background host".into()),
            }
        })
    }

    fn agent_tap(&self, session_id: &str) -> Option<tokio::sync::broadcast::Receiver<String>> {
        match self.route(session_id) {
            Route::Daemon(daemon) => daemon.agent_tap(session_id),
            Route::Local => None,
        }
    }
}
