//! [`SshHostedHost`]: the [`SessionHost`] of an SSH terminal pane whose
//! connection lives in the detached session host (ADR 0009).
//!
//! The SSH connect itself goes through the app's `SshConnectionService`
//! (routed to the daemon, see `labonair_terminal_host::proxy`); this host only
//! creates the pane's side: a local emulator fed by the daemon's attach
//! snapshot and live bytes, input and resizes towards the daemon, and the
//! lifecycle: closing the tab ends the session (acknowledging a lost one),
//! quitting the app only detaches. Steal and host-lost notices use the same
//! [`HostedLink`] states as a local hosted pane.

#[cfg(unix)]
pub use imp::SshHostedHost;

/// An SSH session alive in the detached host (restore).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedSshSession {
    /// The daemon's id (`hostSessionId` of the pane snapshot).
    pub wire_id: u64,
    /// The app-side SSH session id (the tab's id).
    pub ssh_id: String,
    pub host_id: String,
    /// `false` after the connection was lost.
    pub connected: bool,
    /// The pane's scrollback uuid the session was created with.
    pub pane_tag: Option<String>,
}

#[cfg(unix)]
mod imp {
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard};

    use labonair_terminal_host::client::SessionSink;
    use labonair_terminal_host::protocol::{DetachReason, WireColors};
    use labonair_terminal_host::proxy::DaemonServices;
    use tokio::sync::Notify;

    use crate::host::{HostedControl, HostedLink, HostedSession, SessionHost};
    use crate::registry::SessionBackend;
    use crate::session::{ForegroundProcess, RemoteFeed, RemoteSession};
    use crate::{SessionOptions, TermDimensions, TerminalColors};

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn grid(dimensions: TermDimensions) -> (u16, u16) {
        (
            u16::try_from(dimensions.columns).unwrap_or(u16::MAX),
            u16::try_from(dimensions.screen_lines).unwrap_or(u16::MAX),
        )
    }

    /// Creates the hosted pane of the SSH session `ssh_id`. `pane_tag` (the
    /// pane's scrollback uuid) lets a restart find the session again.
    pub struct SshHostedHost {
        services: DaemonServices,
        ssh_id: String,
        pane_tag: String,
        /// Daemon session to show instead of waiting for a new connect.
        adopt: Option<u64>,
        feed: Mutex<Option<RemoteFeed>>,
    }

    impl SshHostedHost {
        /// On the shared daemon connection of `data_dir`. With `start` a
        /// missing daemon is started (new connection); without it only a
        /// running one is used (restore).
        pub fn connect(
            data_dir: &Path,
            scrollback_lines: u32,
            start: bool,
            ssh_id: impl Into<String>,
            pane_tag: impl Into<String>,
        ) -> Result<Self, String> {
            Ok(Self::with_services(
                DaemonServices::shared(data_dir, scrollback_lines, start)?,
                ssh_id,
                pane_tag,
            ))
        }

        pub fn with_services(
            services: DaemonServices,
            ssh_id: impl Into<String>,
            pane_tag: impl Into<String>,
        ) -> Self {
            Self {
                services,
                ssh_id: ssh_id.into(),
                pane_tag: pane_tag.into(),
                adopt: None,
                feed: Mutex::new(None),
            }
        }

        /// Attach the live daemon session `wire` (session restore).
        pub fn adopting(mut self, wire: u64) -> Self {
            self.adopt = Some(wire);
            self
        }

        /// The feed of the created pane, for the connect flow's retry path.
        pub fn feed(&self) -> Option<RemoteFeed> {
            lock(&self.feed).clone()
        }

        /// Daemon SSH sessions that carry a pane tag (restore). Blocking.
        pub fn list_sessions(&self) -> Result<Vec<super::HostedSshSession>, String> {
            Ok(self
                .services
                .list_ssh_sessions_blocking()?
                .into_iter()
                .map(|info| super::HostedSshSession {
                    wire_id: info.session,
                    ssh_id: info.ssh_session_id,
                    host_id: info.host_id,
                    connected: info.connected,
                    pane_tag: info.pane_tag,
                })
                .collect())
        }
    }

    struct Shared {
        services: DaemonServices,
        ssh_id: String,
        feed: RemoteFeed,
        link: Mutex<HostedLink>,
        dims: Mutex<(u16, u16)>,
        finished: AtomicBool,
    }

    impl Shared {
        fn set_link(&self, link: HostedLink) {
            *lock(&self.link) = link;
            self.feed.notify_changed();
        }
    }

    impl SessionHost for SshHostedHost {
        fn create_session(
            &self,
            colors: TerminalColors,
            dimensions: TermDimensions,
            _options: SessionOptions,
            event_notify: Arc<Notify>,
        ) -> Result<SessionBackend, String> {
            let (cols, rows) = grid(dimensions);
            let late: Arc<Mutex<Option<Arc<Shared>>>> = Arc::new(Mutex::new(None));
            let writer = {
                let (services, ssh_id) = (self.services.clone(), self.ssh_id.clone());
                Arc::new(move |bytes: Vec<u8>| services.input(&ssh_id, &bytes))
            };
            let resizer = {
                let late = Arc::clone(&late);
                Arc::new(move |cols: u16, rows: u16| {
                    if let Some(shared) = lock(&late).as_ref() {
                        *lock(&shared.dims) = (cols, rows);
                        shared.services.resize(&shared.ssh_id, cols, rows);
                    }
                })
            };
            let (remote, feed) =
                RemoteSession::new_with_notifier(colors, dimensions, writer, resizer, event_notify);
            *lock(&self.feed) = Some(feed.clone());
            let shared = Arc::new(Shared {
                services: self.services.clone(),
                ssh_id: self.ssh_id.clone(),
                feed,
                link: Mutex::new(HostedLink::Live),
                dims: Mutex::new((cols, rows)),
                finished: AtomicBool::new(false),
            });
            *lock(&late) = Some(Arc::clone(&shared));
            let sink = Arc::new(Sink(Arc::clone(&shared)));
            match self.adopt {
                Some(wire) => {
                    self.services
                        .adopt(&self.ssh_id, wire, &self.pane_tag, (cols, rows), sink)?
                }
                None => self.services.bind_pane(
                    &self.ssh_id,
                    &self.pane_tag,
                    Some(WireColors::from(&colors)),
                    (cols, rows),
                    sink,
                ),
            }
            Ok(SessionBackend::Hosted(HostedSession::new(
                remote,
                Arc::new(Control(shared)),
            )))
        }
    }

    /// Daemon -> pane.
    struct Sink(Arc<Shared>);

    impl SessionSink for Sink {
        fn attached(&self, cols: u16, rows: u16) {
            self.0
                .feed
                .resize_emulator(TermDimensions::new(cols.into(), rows.into()));
            self.0.set_link(HostedLink::Live);
        }

        fn snapshot(&self, data: &[u8], resync: bool) {
            self.0.feed.feed_hosted(data, true);
            if !resync {
                // The pane was resized while the attach was in flight.
                let want = *lock(&self.0.dims);
                self.0
                    .feed
                    .resize_emulator(TermDimensions::new(want.0.into(), want.1.into()));
            }
        }

        fn output(&self, data: &[u8]) {
            self.0.feed.feed_hosted(data, false);
        }

        fn exited(&self, exit_code: Option<i32>) {
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
            // The daemon no longer knows the session (it was restarted).
            self.0.feed.mark_disconnected();
        }

        fn disconnected(&self) {
            self.0.set_link(HostedLink::HostLost);
        }

        fn connection_lost(&self, reason: &str) {
            // The pane's existing "connection lost" state, with the cause.
            self.0.feed.feed_hosted(
                format!("\r\n\x1b[31mConnection lost: {reason}\x1b[0m\r\n").as_bytes(),
                false,
            );
            self.0.feed.mark_disconnected();
        }
    }

    /// Registry -> daemon.
    struct Control(Arc<Shared>);

    impl HostedControl for Control {
        fn host_session_id(&self) -> u64 {
            self.0.services.wire_id(&self.0.ssh_id).unwrap_or(0)
        }

        fn link(&self) -> HostedLink {
            *lock(&self.0.link)
        }

        fn reattach(&self) -> Result<(), String> {
            self.0.services.reattach(&self.0.ssh_id)?;
            self.0.set_link(HostedLink::Live);
            Ok(())
        }

        fn foreground(&self) -> Option<ForegroundProcess> {
            None
        }

        fn kill(&self) {
            if !self.0.finished.swap(true, Ordering::SeqCst) {
                self.0.services.close_pane(&self.0.ssh_id);
            }
        }

        fn detach(&self) {
            if !self.0.finished.swap(true, Ordering::SeqCst) {
                self.0.services.detach_pane(&self.0.ssh_id);
            }
        }
    }

    impl Drop for Control {
        /// Dropping a handle without closing the tab (app quit) only detaches.
        fn drop(&mut self) {
            self.detach();
        }
    }
}
