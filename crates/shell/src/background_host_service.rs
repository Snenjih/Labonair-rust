//! Composition adapter for the Settings "Background host" status block: maps
//! the terminal's detached session host client (`DaemonHost`, ADR 0009) onto
//! the UI-free `BackgroundHostService` contract. Unix only.

use std::time::{Duration, Instant};

use labonair_settings_ui::{
    BackgroundHostService, BackgroundHostSession, BackgroundHostSnapshot, BackgroundHostStatus,
    ServiceFuture,
};
use labonair_terminal::{DaemonHost, HostState};

pub(crate) struct DaemonHostService {
    /// `terminal.detachedScrollbackLines` as of startup (the connection's
    /// handshake value; the app-wide client is created once).
    scrollback_lines: u32,
}

impl DaemonHostService {
    pub(crate) fn new(scrollback_lines: u32) -> Self {
        Self { scrollback_lines }
    }
}

/// Run a blocking host call off the async worker.
fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> ServiceFuture<T> {
    Box::pin(async move {
        tokio::task::spawn_blocking(work)
            .await
            .map_err(|error| error.to_string())?
    })
}

/// A client for an already running host; never starts one.
fn running(lines: u32) -> Result<DaemonHost, String> {
    DaemonHost::connect(&labonair_filesystem::paths::data_dir(), lines, false)
}

impl BackgroundHostService for DaemonHostService {
    fn snapshot(&self) -> ServiceFuture<BackgroundHostSnapshot> {
        let lines = self.scrollback_lines;
        blocking(move || {
            // No reachable host is a state, not an error.
            let Ok(host) = running(lines) else {
                return Ok(BackgroundHostSnapshot {
                    status: BackgroundHostStatus::NotRunning,
                    sessions: Vec::new(),
                });
            };
            let status = match host.state() {
                HostState::NotRunning => BackgroundHostStatus::NotRunning,
                HostState::Running { .. } => BackgroundHostStatus::Running,
                HostState::Incompatible { sessions, build_id } => {
                    BackgroundHostStatus::Incompatible { build_id, sessions }
                }
            };
            let sessions = if status == BackgroundHostStatus::NotRunning {
                Vec::new()
            } else {
                host.list_sessions()?
                    .into_iter()
                    .map(|session| BackgroundHostSession {
                        id: session.id,
                        label: session.label,
                        cwd: session.cwd,
                        foreground: session.foreground,
                        attached: session.attached,
                        exited: session.exited,
                    })
                    .collect()
            };
            Ok(BackgroundHostSnapshot { status, sessions })
        })
    }

    fn kill(&self, id: u64) -> ServiceFuture<()> {
        let lines = self.scrollback_lines;
        blocking(move || {
            running(lines)?.kill_session(id);
            Ok(())
        })
    }

    fn stop_all(&self) -> ServiceFuture<()> {
        let lines = self.scrollback_lines;
        blocking(move || {
            running(lines)?.shutdown();
            Ok(())
        })
    }

    fn restart(&self) -> ServiceFuture<()> {
        let lines = self.scrollback_lines;
        blocking(move || {
            running(lines)?.shutdown();
            // The old host may still hold its socket for a moment: retry.
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match DaemonHost::connect(&labonair_filesystem::paths::data_dir(), lines, true) {
                    Ok(_) => return Ok(()),
                    Err(error) if Instant::now() >= deadline => return Err(error),
                    Err(_) => std::thread::sleep(Duration::from_millis(250)),
                }
            }
        })
    }
}
