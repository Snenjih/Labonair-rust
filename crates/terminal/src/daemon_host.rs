//! [`DaemonHost`]: the [`SessionHost`] that puts terminal sessions into the
//! detached session host daemon (ADR 0009).
//!
//! The pane keeps its own emulator as the render source (selection, search and
//! scroll stay local). Daemon output -- the attach snapshot, resyncs and live
//! bytes -- is fed through the [`RemoteFeed`](crate::RemoteFeed) of a
//! [`RemoteSession`](crate::RemoteSession), so the view layer does not learn
//! about the daemon. User input and resizes go out through the shared
//! [`HostClient`](labonair_terminal_host::client::HostClient).
//!
//! Lifecycle: the registry's `close` kills the session (acknowledges it when it
//! already exited); `close_all` and dropping the handle at app quit only
//! detach.

use std::path::Path;

use crate::host::BackgroundSession;

pub use imp::{DaemonHost, HostState};

#[cfg(unix)]
mod imp {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::time::{Duration, Instant};

    use labonair_terminal_host::client::{HostClient, HostStatus, SessionSink};
    use labonair_terminal_host::protocol::{
        CreateOptions, DetachReason, SessionId, SessionInfo, SessionStatus, WireColors,
    };
    use labonair_terminal_host::spawn::shared_client;
    use tokio::sync::Notify;

    use super::{BackgroundSession, Path};
    use crate::host::{HostedControl, HostedLink, HostedSession, SessionHost};
    use crate::registry::SessionBackend;
    use crate::session::{ForegroundProcess, RemoteFeed, RemoteSession};
    use crate::{SessionOptions, TermDimensions, TerminalColors};

    /// How long the foreground job of a hosted pane may be stale before a
    /// background refresh is started.
    const FOREGROUND_TTL: Duration = Duration::from_secs(3);

    /// Cache of a hosted pane's foreground job. Reads never wait on the
    /// daemon: they return the cached name and say when to refresh it in the
    /// background.
    #[derive(Default)]
    pub(super) struct ForegroundCache {
        name: Option<String>,
        at: Option<Instant>,
        refreshing: bool,
    }

    impl ForegroundCache {
        /// The cached name, and whether the caller must start a refresh now
        /// (stale and none in flight).
        pub(super) fn read(&mut self, now: Instant, ttl: Duration) -> (Option<String>, bool) {
            let stale = self.at.is_none_or(|at| now.duration_since(at) >= ttl);
            let start = stale && !self.refreshing;
            self.refreshing |= start;
            (self.name.clone(), start)
        }

        pub(super) fn store(&mut self, name: Option<String>, now: Instant) {
            self.name = name;
            self.at = Some(now);
            self.refreshing = false;
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Connection state of the daemon, for the settings status UI.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum HostState {
        /// No daemon is running (or this app never connected to one).
        NotRunning,
        Running {
            sessions: usize,
        },
        /// Running, but speaking a protocol this build cannot use; stopping it
        /// ends `sessions` sessions.
        Incompatible {
            sessions: u32,
            build_id: String,
        },
    }

    /// Creates (or re-attaches) hosted sessions on the shared daemon connection.
    #[derive(Clone)]
    pub struct DaemonHost {
        client: HostClient,
        pane_tag: Option<String>,
        adopt: Option<SessionId>,
    }

    impl DaemonHost {
        /// Connect to the daemon of `data_dir`. With `start` a missing daemon
        /// is started (setting `detached`); without it only a running one is
        /// used (restore, recovery).
        pub fn connect(
            data_dir: &Path,
            scrollback_lines: u32,
            start: bool,
        ) -> Result<Self, String> {
            let client = shared_client(data_dir, scrollback_lines, start)?;
            Ok(Self {
                client,
                pane_tag: None,
                adopt: None,
            })
        }

        /// A host on an existing client (tests, embedding).
        pub fn with_client(client: HostClient) -> Self {
            Self {
                client,
                pane_tag: None,
                adopt: None,
            }
        }

        /// Tag new sessions with the pane's scrollback uuid. The daemon matches
        /// sessions to panes by this tag on reconcile and persists their
        /// scrollback under it.
        pub fn for_pane(mut self, pane_tag: impl Into<String>) -> Self {
            self.pane_tag = Some(pane_tag.into());
            self
        }

        /// Attach the existing daemon session `id` instead of creating one.
        pub fn adopting(mut self, id: u64) -> Self {
            self.adopt = Some(id);
            self
        }

        /// Sessions currently known to the daemon (cached for a second).
        pub fn list_sessions(&self) -> Result<Vec<BackgroundSession>, String> {
            self.client
                .list_sessions()
                .map(|sessions| sessions.iter().map(background_session).collect())
                .map_err(|e| e.to_string())
        }

        pub fn state(&self) -> HostState {
            match self.client.status() {
                HostStatus::Connected => HostState::Running {
                    sessions: self.client.list_sessions().map_or(0, |s| s.len()),
                },
                HostStatus::Incompatible { sessions, build_id } => {
                    HostState::Incompatible { sessions, build_id }
                }
                HostStatus::Disconnected => HostState::NotRunning,
            }
        }

        /// Kill one daemon session (settings session list).
        pub fn kill_session(&self, id: u64) {
            self.client.kill(id);
        }

        /// Kill every session and stop the daemon. Returns once the daemon
        /// closed the connection (or after a short bound), so the caller may
        /// quit right away.
        pub fn shutdown(&self) {
            self.client.shutdown();
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            while self.client.is_connected() && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    fn background_session(info: &SessionInfo) -> BackgroundSession {
        BackgroundSession {
            id: info.id,
            pane_tag: info.pane_tag.clone(),
            label: info.label.clone(),
            cwd: info.cwd.clone(),
            title: info.title.clone(),
            exited: info.status == SessionStatus::Exited,
            attached: info.attached,
            foreground: info.foreground.clone(),
        }
    }

    /// The daemon never inherits an environment: send the app's complete one
    /// (PATH, HOME, SHELL, SSH_AUTH_SOCK, ...) with the session's extras on top.
    fn full_env(extra: Vec<(String, String)>) -> Vec<(String, String)> {
        let mut env: BTreeMap<String, String> = std::env::vars().collect();
        env.extend(extra);
        env.into_iter().collect()
    }

    fn grid(dimensions: TermDimensions) -> (u16, u16) {
        (
            u16::try_from(dimensions.columns).unwrap_or(u16::MAX),
            u16::try_from(dimensions.screen_lines).unwrap_or(u16::MAX),
        )
    }

    impl SessionHost for DaemonHost {
        fn create_session(
            &self,
            colors: TerminalColors,
            dimensions: TermDimensions,
            options: SessionOptions,
            event_notify: Arc<Notify>,
        ) -> Result<SessionBackend, String> {
            let (cols, rows) = grid(dimensions);
            let emulator_config = options.emulator_config();
            let id = match self.adopt {
                Some(id) => id,
                None => {
                    let shell = options
                        .shell
                        .or_else(|| std::env::var("SHELL").ok())
                        .unwrap_or_else(|| "/bin/zsh".to_string());
                    let create = CreateOptions {
                        shell: Some(shell),
                        args: options.args,
                        cwd: options.working_directory,
                        env: full_env(options.env),
                        startup_command: options.startup_command,
                        cols,
                        rows,
                        pane_tag: self.pane_tag.clone(),
                        persist_key: self.pane_tag.clone(),
                        blocks: options.blocks,
                        replay_scrollback: options.replay_scrollback,
                        ..CreateOptions::default()
                    };
                    self.client
                        .create_session(create, Some(WireColors::from(&colors)))
                        .map_err(|e| e.to_string())?
                }
            };

            // The writer / resizer closures need the shared state, which needs
            // the feed that `RemoteSession::new` returns: tie the knot with
            // a late-bound cell.
            let late: Arc<Mutex<Option<Arc<Shared>>>> = Arc::new(Mutex::new(None));
            let writer = {
                let client = self.client.clone();
                Arc::new(move |bytes: Vec<u8>| client.input(id, &bytes))
            };
            let resizer = {
                let late = Arc::clone(&late);
                Arc::new(move |cols: u16, rows: u16| {
                    if let Some(shared) = lock(&late).as_ref() {
                        *lock(&shared.dims) = (cols, rows);
                        shared.client.resize(id, cols, rows);
                    }
                })
            };
            let (remote, feed) = RemoteSession::new_with_config(
                colors,
                dimensions,
                writer,
                resizer,
                event_notify,
                emulator_config,
            );
            let shared = Arc::new(Shared {
                client: self.client.clone(),
                id,
                feed,
                link: Mutex::new(HostedLink::Live),
                dims: Mutex::new((cols, rows)),
                attach_dims: Mutex::new((cols, rows)),
                foreground: Mutex::new(ForegroundCache::default()),
                exited: AtomicBool::new(false),
                finished: AtomicBool::new(false),
            });
            *lock(&late) = Some(Arc::clone(&shared));
            // Register before attaching: nothing can arrive unrouted.
            self.client
                .register(id, Arc::new(Sink(Arc::clone(&shared))));
            if let Err(error) = self.client.attach(id, cols, rows) {
                self.client.unregister(id);
                shared.finished.store(true, Ordering::SeqCst);
                if self.adopt.is_none() {
                    self.client.kill(id);
                }
                return Err(error.to_string());
            }
            Ok(SessionBackend::Hosted(HostedSession::new(
                remote,
                Arc::new(Control(shared)),
            )))
        }
    }

    /// State shared between the pane (registry slot) and the client's reader.
    struct Shared {
        client: HostClient,
        id: SessionId,
        feed: RemoteFeed,
        link: Mutex<HostedLink>,
        /// Size the pane wants right now.
        dims: Mutex<(u16, u16)>,
        /// Size the daemon attached us at (what the snapshot was rendered for).
        attach_dims: Mutex<(u16, u16)>,
        foreground: Mutex<ForegroundCache>,
        exited: AtomicBool,
        /// Killed or detached: nothing more to tell the daemon.
        finished: AtomicBool,
    }

    impl Shared {
        fn set_link(&self, link: HostedLink) {
            *lock(&self.link) = link;
            self.feed.notify_changed();
        }
    }

    /// Daemon -> pane.
    struct Sink(Arc<Shared>);

    impl SessionSink for Sink {
        fn attached(&self, cols: u16, rows: u16) {
            *lock(&self.0.attach_dims) = (cols, rows);
            self.0
                .feed
                .resize_emulator(TermDimensions::new(cols.into(), rows.into()));
            self.0.set_link(HostedLink::Live);
        }

        fn snapshot(&self, data: &[u8], resync: bool) {
            // History, not activity: no prompt/command events, no bell.
            self.0.feed.feed_hosted(data, true);
            if !resync {
                // The pane was resized while the attach was in flight; the
                // daemon already got that `Resize`, so only the local grid
                // is behind.
                let want = *lock(&self.0.dims);
                if want != *lock(&self.0.attach_dims) {
                    self.0
                        .feed
                        .resize_emulator(TermDimensions::new(want.0.into(), want.1.into()));
                }
            }
        }

        fn output(&self, data: &[u8]) {
            self.0.feed.feed_hosted(data, false);
        }

        fn exited(&self, exit_code: Option<i32>) {
            self.0.exited.store(true, Ordering::SeqCst);
            self.0.feed.mark_exited(exit_code.unwrap_or(0));
        }

        fn detached(&self, reason: DetachReason) {
            match reason {
                DetachReason::Stolen => self.0.set_link(HostedLink::Stolen),
                DetachReason::Shutdown => self.0.set_link(HostedLink::HostLost),
                _ => {}
            }
        }

        fn error(&self, _message: &str) {
            // The daemon no longer knows the session (e.g. it was restarted).
            self.0.exited.store(true, Ordering::SeqCst);
            self.0.feed.mark_disconnected();
        }

        fn disconnected(&self) {
            self.0.set_link(HostedLink::HostLost);
        }
    }

    /// Registry -> daemon.
    struct Control(Arc<Shared>);

    impl HostedControl for Control {
        fn host_session_id(&self) -> u64 {
            self.0.id
        }

        fn link(&self) -> HostedLink {
            *lock(&self.0.link)
        }

        fn reattach(&self) -> Result<(), String> {
            let shared = &self.0;
            // Only an already running daemon: if it is gone so are the sessions.
            shared.client.reconnect().map_err(|e| e.to_string())?;
            let (cols, rows) = *lock(&shared.dims);
            shared
                .client
                .attach(shared.id, cols, rows)
                .map_err(|e| e.to_string())?;
            shared.set_link(HostedLink::Live);
            Ok(())
        }

        fn foreground(&self) -> Option<ForegroundProcess> {
            let shared = &self.0;
            if shared.exited.load(Ordering::SeqCst) || self.link() != HostedLink::Live {
                return None;
            }
            let (name, refresh) = lock(&shared.foreground).read(Instant::now(), FOREGROUND_TTL);
            if refresh {
                // A hung daemon must not stall the UI: ask on a thread and
                // keep answering from the cache meanwhile.
                let worker = Arc::clone(shared);
                let spawned = std::thread::Builder::new()
                    .name("labonair-host-foreground".into())
                    .spawn(move || {
                        let name = worker
                            .client
                            .list_sessions_cached(Duration::from_secs(1))
                            .into_iter()
                            .find(|info| info.id == worker.id)
                            .and_then(|info| info.foreground);
                        lock(&worker.foreground).store(name, Instant::now());
                    });
                if spawned.is_err() {
                    lock(&shared.foreground).store(name.clone(), Instant::now());
                }
            }
            name.map(|name| ForegroundProcess { name, pid: 0 })
        }

        fn kill(&self) {
            let shared = &self.0;
            if shared.finished.swap(true, Ordering::SeqCst) {
                return;
            }
            shared.client.unregister(shared.id);
            if shared.exited.load(Ordering::SeqCst) {
                shared.client.ack(shared.id);
            } else {
                shared.client.kill(shared.id);
            }
        }

        fn detach(&self) {
            let shared = &self.0;
            if shared.finished.swap(true, Ordering::SeqCst) {
                return;
            }
            shared.client.unregister(shared.id);
            shared.client.detach(shared.id);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn foreground_cache_refreshes_in_the_background_and_never_waits() {
            let t0 = Instant::now();
            let ttl = Duration::from_secs(3);
            let mut cache = ForegroundCache::default();
            // Cold: nothing cached, a refresh is requested once.
            assert_eq!(cache.read(t0, ttl), (None, true));
            assert_eq!(cache.read(t0, ttl), (None, false));
            cache.store(Some("claude".into()), t0);
            // Fresh: served from the cache, no refresh.
            assert_eq!(
                cache.read(t0 + Duration::from_secs(1), ttl),
                (Some("claude".into()), false)
            );
            // Stale: still served, one refresh requested.
            let late = t0 + Duration::from_secs(4);
            assert_eq!(cache.read(late, ttl), (Some("claude".into()), true));
            assert_eq!(cache.read(late, ttl), (Some("claude".into()), false));
            cache.store(None, late);
            assert_eq!(cache.read(late, ttl), (None, false));
        }
    }

    impl Drop for Control {
        /// Dropping a handle without closing the tab (app quit) only detaches.
        fn drop(&mut self) {
            self.detach();
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use std::sync::Arc;

    use tokio::sync::Notify;

    use super::{BackgroundSession, Path};
    use crate::host::SessionHost;
    use crate::registry::SessionBackend;
    use crate::{SessionOptions, TermDimensions, TerminalColors};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum HostState {
        NotRunning,
    }

    /// The detached session host is Unix-only; every operation reports that.
    #[derive(Clone)]
    pub struct DaemonHost;

    const UNSUPPORTED: &str = "detached sessions are not supported on this platform";

    impl DaemonHost {
        pub fn connect(_: &Path, _: u32, _: bool) -> Result<Self, String> {
            Err(UNSUPPORTED.to_string())
        }
        pub fn for_pane(self, _: impl Into<String>) -> Self {
            self
        }
        pub fn adopting(self, _: u64) -> Self {
            self
        }
        pub fn list_sessions(&self) -> Result<Vec<BackgroundSession>, String> {
            Err(UNSUPPORTED.to_string())
        }
        pub fn state(&self) -> HostState {
            HostState::NotRunning
        }
        pub fn kill_session(&self, _: u64) {}
        pub fn shutdown(&self) {}
    }

    impl SessionHost for DaemonHost {
        fn create_session(
            &self,
            _: TerminalColors,
            _: TermDimensions,
            _: SessionOptions,
            _: Arc<Notify>,
        ) -> Result<SessionBackend, String> {
            Err(UNSUPPORTED.to_string())
        }
    }
}
