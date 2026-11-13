//! Session creation seam for the detached session host (ADR 0009).
//!
//! [`TerminalRegistry`](crate::TerminalRegistry) asks a [`SessionHost`] to
//! create the backend of each new session. [`InProcessHost`] is today's
//! behaviour (a local PTY owned by the app). The daemon client implements the
//! same trait and returns [`SessionBackend::Hosted`], built on the
//! [`RemoteSession`](crate::RemoteSession) / [`RemoteFeed`](crate::RemoteFeed)
//! seam. This trait lives in `labonair-terminal` (not `labonair-terminal-core`)
//! because it returns the registry's `SessionBackend`.

use std::sync::Arc;

use tokio::sync::Notify;

use crate::registry::SessionBackend;
use crate::session::ForegroundProcess;
use crate::{RemoteSession, SessionOptions, TermDimensions, TerminalColors, TerminalSession};

/// Creates the backend for a new terminal session.
pub trait SessionHost: Send + Sync {
    /// Create a session. `event_notify` is the registry slot's notifier and
    /// must be signalled whenever the session has new events.
    fn create_session(
        &self,
        colors: TerminalColors,
        dimensions: TermDimensions,
        options: SessionOptions,
        event_notify: Arc<Notify>,
    ) -> Result<SessionBackend, String>;
}

/// Sessions live and die with the app process: a local PTY.
pub struct InProcessHost;

impl SessionHost for InProcessHost {
    fn create_session(
        &self,
        colors: TerminalColors,
        dimensions: TermDimensions,
        options: SessionOptions,
        event_notify: Arc<Notify>,
    ) -> Result<SessionBackend, String> {
        TerminalSession::spawn_with_notifier(colors, dimensions, options, event_notify)
            .map(SessionBackend::Local)
    }
}

/// Link between a hosted session's pane and the daemon, for the pane's
/// inline notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedLink {
    /// Attached and streaming.
    Live,
    /// Another window or app instance attached the session; offer Reattach.
    Stolen,
    /// The connection to the daemon was lost; offer Reconnect.
    HostLost,
}

/// A session living in the detached session host, as listed by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundSession {
    pub id: u64,
    /// Tag the app set at creation (the pane's scrollback uuid).
    pub pane_tag: Option<String>,
    pub label: Option<String>,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub exited: bool,
    /// Some client holds the session right now.
    pub attached: bool,
    /// Name of the foreground job when it is not the shell.
    pub foreground: Option<String>,
}

/// What the registry needs from the daemon connection of one hosted session.
pub trait HostedControl: Send + Sync {
    /// The daemon's id for the session (persisted for restore).
    fn host_session_id(&self) -> u64;
    fn link(&self) -> HostedLink;
    /// Reconnect to the daemon if needed and attach again.
    fn reattach(&self) -> Result<(), String>;
    /// The foreground job other than the shell (cached, no per-call IPC).
    fn foreground(&self) -> Option<ForegroundProcess>;
    /// The tab was closed: stop the session (acknowledge it when it exited).
    fn kill(&self);
    /// The app is quitting: leave the session running in the daemon.
    fn detach(&self);
}

/// The backend of a hosted session: the local render emulator plus the
/// control surface towards the daemon.
pub struct HostedSession {
    pub(crate) remote: RemoteSession,
    pub(crate) control: Arc<dyn HostedControl>,
}

impl HostedSession {
    pub fn new(remote: RemoteSession, control: Arc<dyn HostedControl>) -> Self {
        Self { remote, control }
    }
}
