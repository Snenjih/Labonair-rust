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
use crate::{SessionOptions, TermDimensions, TerminalColors, TerminalSession};

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
