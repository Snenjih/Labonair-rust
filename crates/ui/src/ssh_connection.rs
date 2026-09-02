//! Per-session SSH connection status store (T16-015).
//!
//! Port of `reference-src/src/modules/hosts/store/connectionStatusStore.ts`: a
//! single observable map, keyed by backend SSH session id, that the
//! [`SshLoadingScreen`](crate::workspace) full-pane view, the status bar and
//! the command palette all read. It tracks, per session:
//!
//! * the [`ConnectionState`] state-machine value (the reference `Status` union
//!   plus `Connected`/`Idle`),
//! * the last error message (structured error screen),
//! * the jump-host name (jump-host badge),
//! * the [`ConnectionKind`] (terminal vs. SFTP),
//! * the 4-stage progress (`TCP → Handshake → Auth → Shell`/`SFTP`), derived
//!   from the `ssh_connect_log` line stream by [`detect_stage`], and
//! * the live connection log lines.
//!
//! Nothing here does I/O — the workspace feeds it transitions off the backend
//! event bus.

use std::collections::HashMap;

use gpui::{Context, EventEmitter};

/// Terminal PTY session vs. SFTP browser session — drives the last progress
/// stage's label and the palette/status-bar icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Terminal,
    Sftp,
}

/// One of the four connection stages shown in the progress indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnStage {
    Tcp,
    Handshake,
    Auth,
    Shell,
}

impl ConnStage {
    /// Fixed left-to-right order — index into this is what [`ConnectionEntry`]
    /// stores as `stage`.
    pub const ORDER: [ConnStage; 4] = [
        ConnStage::Tcp,
        ConnStage::Handshake,
        ConnStage::Auth,
        ConnStage::Shell,
    ];

    pub fn index(self) -> usize {
        match self {
            ConnStage::Tcp => 0,
            ConnStage::Handshake => 1,
            ConnStage::Auth => 2,
            ConnStage::Shell => 3,
        }
    }

    /// Label; the last stage reads "Shell" for a terminal session and "SFTP"
    /// for an SFTP session (`SSH_STAGES` / `SFTP_STAGES` in the reference).
    pub fn label(self, kind: ConnectionKind) -> &'static str {
        match self {
            ConnStage::Tcp => "TCP Connect",
            ConnStage::Handshake => "Handshake",
            ConnStage::Auth => "Auth",
            ConnStage::Shell => match kind {
                ConnectionKind::Terminal => "Shell",
                ConnectionKind::Sftp => "SFTP",
            },
        }
    }
}

/// Progress state of a single stage, relative to how far the connection got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Active,
    Done,
}

/// The connection state machine — `Status` from `SshLoadingScreen.tsx:30` plus
/// the resting `Idle`/`Connected` values `connectionStatusStore` carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Idle,
    /// Quick-connect: we have a `user@host` target but still need a password.
    QuickConnectPassword,
    Connecting,
    WaitingTrust,
    WaitingAuth,
    WaitingPassphrase,
    Connected,
    Error,
}

impl ConnectionState {
    /// Whether the loading screen should be shown instead of the terminal /
    /// SFTP view for this session.
    pub fn is_blocking(self) -> bool {
        !matches!(self, ConnectionState::Idle | ConnectionState::Connected)
    }
}

/// One tracked session.
#[derive(Debug, Clone)]
pub struct ConnectionEntry {
    pub session_id: String,
    pub host_id: String,
    /// Display name snapshot (host may be renamed / deleted mid-connection).
    pub host_label: String,
    pub kind: ConnectionKind,
    pub state: ConnectionState,
    pub error: Option<String>,
    /// Prompt text for `WaitingAuth` (`prompt_message` from `auth_required`).
    pub prompt_message: Option<String>,
    pub is_2fa: bool,
    /// Unknown-vs-mismatch host key, for `WaitingTrust` styling.
    pub trust_fingerprint: Option<String>,
    pub trust_mismatch: bool,
    pub jump_host_name: Option<String>,
    /// Index into [`ConnStage::ORDER`] of the furthest stage reached.
    pub stage: usize,
    /// Whether `stage` itself is finished (vs. in progress).
    pub stage_done: bool,
    /// Live `ssh_connect_log` lines, oldest first.
    pub log: Vec<String>,
}

impl ConnectionEntry {
    fn new(
        session_id: String,
        host_id: String,
        host_label: String,
        kind: ConnectionKind,
        jump_host_name: Option<String>,
    ) -> Self {
        Self {
            session_id,
            host_id,
            host_label,
            kind,
            state: ConnectionState::Connecting,
            error: None,
            prompt_message: None,
            is_2fa: false,
            trust_fingerprint: None,
            trust_mismatch: false,
            jump_host_name,
            stage: 0,
            stage_done: false,
            log: Vec::new(),
        }
    }

    /// [`StageStatus`] for `stage` given how far this connection got.
    pub fn stage_status(&self, stage: ConnStage) -> StageStatus {
        let idx = stage.index();
        if self.state == ConnectionState::Connected {
            return StageStatus::Done;
        }
        match idx.cmp(&self.stage) {
            std::cmp::Ordering::Less => StageStatus::Done,
            std::cmp::Ordering::Greater => StageStatus::Pending,
            std::cmp::Ordering::Equal => {
                if self.stage_done {
                    StageStatus::Done
                } else {
                    StageStatus::Active
                }
            }
        }
    }

    fn ingest_log(&mut self, line: &str) {
        if let Some((idx, done)) = detect_stage(line) {
            // Never move backwards (interleaved credential / jump-host lines).
            if idx > self.stage || (idx == self.stage && done && !self.stage_done) {
                self.stage = idx.max(self.stage);
                self.stage_done = if idx == self.stage { done } else { false };
            }
        }
        self.log.push(line.to_string());
        // Bound memory on a pathological log flood.
        if self.log.len() > 500 {
            self.log.drain(0..self.log.len() - 500);
        }
    }
}

/// Map one `ssh_connect_log` message to `(stage_index, stage_complete)`, or
/// `None` if the line isn't a stage marker. Markers come from `log_step!` in
/// `crates/backend/src/modules/ssh/client.rs`.
pub fn detect_stage(line: &str) -> Option<(usize, bool)> {
    let l = line;
    if l.contains("TCP connection established") {
        Some((0, true))
    } else if l.contains("Starting SSH handshake") {
        Some((1, false))
    } else if l.contains("SSH handshake complete") {
        Some((1, true))
    } else if l.contains("Authenticating") {
        Some((2, false))
    } else if l.contains("Authenticated") && l.contains('\u{2713}') {
        Some((2, true))
    } else if l.contains("Opening shell channel") {
        Some((3, false))
    } else if l.contains("Session established") {
        Some((3, true))
    } else if l.contains("Reading host configuration")
        || l.contains("Resolving credential")
        || l.contains("Resolving jump host")
        || l.contains("Retrieving credentials")
    {
        Some((0, false))
    } else {
        None
    }
}

/// Emitted on any change so views can `cx.observe` / re-render.
pub struct ConnectionStatusChanged;

/// The store itself — one per [`Workspace`](crate::workspace).
#[derive(Default)]
pub struct ConnectionStatusStore {
    entries: HashMap<String, ConnectionEntry>,
}

impl EventEmitter<ConnectionStatusChanged> for ConnectionStatusStore {}

impl ConnectionStatusStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, session_id: &str) -> Option<&ConnectionEntry> {
        self.entries.get(session_id)
    }

    /// Every tracked session (status bar / palette reconnect targets).
    pub fn entries(&self) -> Vec<ConnectionEntry> {
        self.entries.values().cloned().collect()
    }

    /// Sessions currently showing the loading screen.
    pub fn blocking(&self) -> Vec<ConnectionEntry> {
        self.entries
            .values()
            .filter(|e| e.state.is_blocking())
            .cloned()
            .collect()
    }

    /// Start (or restart) tracking a session in `Connecting`.
    pub fn begin(
        &mut self,
        session_id: impl Into<String>,
        host_id: impl Into<String>,
        host_label: impl Into<String>,
        kind: ConnectionKind,
        jump_host_name: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let session_id = session_id.into();
        let entry = ConnectionEntry::new(
            session_id.clone(),
            host_id.into(),
            host_label.into(),
            kind,
            jump_host_name,
        );
        self.entries.insert(session_id, entry);
        self.changed(cx);
    }

    fn with<R>(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut ConnectionEntry) -> R,
    ) -> Option<R> {
        let r = self.entries.get_mut(session_id).map(f);
        if r.is_some() {
            self.changed(cx);
        }
        r
    }

    pub fn set_state(&mut self, session_id: &str, state: ConnectionState, cx: &mut Context<Self>) {
        self.with(session_id, cx, |e| {
            e.state = state;
            if state != ConnectionState::Error {
                e.error = None;
            }
            if state == ConnectionState::Connected {
                e.stage = ConnStage::ORDER.len() - 1;
                e.stage_done = true;
            }
        });
    }

    pub fn set_error(
        &mut self,
        session_id: &str,
        message: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.with(session_id, cx, |e| {
            e.state = ConnectionState::Error;
            e.error = Some(message);
        });
    }

    pub fn set_auth_prompt(
        &mut self,
        session_id: &str,
        message: String,
        is_2fa: bool,
        cx: &mut Context<Self>,
    ) {
        self.with(session_id, cx, |e| {
            e.state = ConnectionState::WaitingAuth;
            e.prompt_message = Some(message);
            e.is_2fa = is_2fa;
        });
    }

    pub fn set_trust(
        &mut self,
        session_id: &str,
        fingerprint: String,
        mismatch: bool,
        cx: &mut Context<Self>,
    ) {
        self.with(session_id, cx, |e| {
            e.state = ConnectionState::WaitingTrust;
            e.trust_fingerprint = Some(fingerprint);
            e.trust_mismatch = mismatch;
        });
    }

    pub fn set_passphrase(&mut self, session_id: &str, cx: &mut Context<Self>) {
        self.with(session_id, cx, |e| {
            e.state = ConnectionState::WaitingPassphrase;
        });
    }

    /// Back to `Connecting` after the user answered a prompt / hit retry.
    pub fn resume(&mut self, session_id: &str, cx: &mut Context<Self>) {
        self.with(session_id, cx, |e| {
            if e.state != ConnectionState::Connected {
                e.state = ConnectionState::Connecting;
                e.error = None;
            }
        });
    }

    pub fn push_log(&mut self, session_id: &str, line: impl AsRef<str>, cx: &mut Context<Self>) {
        let line = line.as_ref();
        self.with(session_id, cx, |e| e.ingest_log(line));
    }

    pub fn remove(&mut self, session_id: &str, cx: &mut Context<Self>) {
        if self.entries.remove(session_id).is_some() {
            self.changed(cx);
        }
    }

    fn changed(&self, cx: &mut Context<Self>) {
        cx.emit(ConnectionStatusChanged);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn detect_stage_maps_the_backend_log_markers() {
        assert_eq!(detect_stage("TCP connection established."), Some((0, true)));
        assert_eq!(
            detect_stage("Starting SSH handshake\u{2026}"),
            Some((1, false))
        );
        assert_eq!(detect_stage("SSH handshake complete."), Some((1, true)));
        assert_eq!(detect_stage("Authenticating\u{2026}"), Some((2, false)));
        assert_eq!(detect_stage("Authenticated \u{2713}"), Some((2, true)));
        assert_eq!(
            detect_stage("Opening shell channel\u{2026}"),
            Some((3, false))
        );
        assert_eq!(
            detect_stage("Session established \u{2713}"),
            Some((3, true))
        );
        assert_eq!(detect_stage("some unrelated chatter"), None);
    }

    #[test]
    fn ingest_log_advances_stage_monotonically() {
        let mut e = ConnectionEntry::new(
            "s".into(),
            "h".into(),
            "Host".into(),
            ConnectionKind::Terminal,
            None,
        );
        assert_eq!(e.stage_status(ConnStage::Tcp), StageStatus::Active);

        e.ingest_log("Reading host configuration\u{2026}");
        assert_eq!(e.stage_status(ConnStage::Tcp), StageStatus::Active);
        e.ingest_log("TCP connection established.");
        assert_eq!(e.stage_status(ConnStage::Tcp), StageStatus::Done);
        assert_eq!(e.stage_status(ConnStage::Handshake), StageStatus::Pending);

        e.ingest_log("Starting SSH handshake\u{2026}");
        assert_eq!(e.stage_status(ConnStage::Handshake), StageStatus::Active);
        e.ingest_log("SSH handshake complete.");
        e.ingest_log("Authenticating\u{2026}");
        assert_eq!(e.stage_status(ConnStage::Handshake), StageStatus::Done);
        assert_eq!(e.stage_status(ConnStage::Auth), StageStatus::Active);

        // A late credential line must not drag the stage back to 0.
        e.ingest_log("Resolving credential\u{2026}");
        assert_eq!(e.stage_status(ConnStage::Auth), StageStatus::Active);

        e.ingest_log("Authenticated \u{2713}");
        e.ingest_log("Opening shell channel\u{2026}");
        e.ingest_log("Session established \u{2713}");
        assert_eq!(e.stage_status(ConnStage::Shell), StageStatus::Done);
        assert_eq!(e.log.len(), 9);
    }

    #[gpui::test]
    fn store_drives_the_full_state_machine(cx: &mut TestAppContext) {
        let store = cx.update(|cx| cx.new(|_| ConnectionStatusStore::new()));
        cx.update(|cx| {
            store.update(cx, |s, cx| {
                s.begin(
                    "sess",
                    "host",
                    "Web",
                    ConnectionKind::Terminal,
                    Some("bastion".into()),
                    cx,
                );
                assert_eq!(s.get("sess").unwrap().state, ConnectionState::Connecting);
                assert_eq!(
                    s.get("sess").unwrap().jump_host_name.as_deref(),
                    Some("bastion")
                );

                s.push_log("sess", "TCP connection established.", cx);
                s.set_trust("sess", "aa:bb".into(), true, cx);
                assert_eq!(s.get("sess").unwrap().state, ConnectionState::WaitingTrust);
                assert!(s.get("sess").unwrap().trust_mismatch);

                s.resume("sess", cx);
                assert_eq!(s.get("sess").unwrap().state, ConnectionState::Connecting);

                s.set_auth_prompt("sess", "Password:".into(), true, cx);
                assert_eq!(s.get("sess").unwrap().state, ConnectionState::WaitingAuth);
                assert!(s.get("sess").unwrap().is_2fa);

                s.set_passphrase("sess", cx);
                assert_eq!(
                    s.get("sess").unwrap().state,
                    ConnectionState::WaitingPassphrase
                );

                s.set_error("sess", "boom", cx);
                assert_eq!(s.get("sess").unwrap().state, ConnectionState::Error);
                assert_eq!(s.get("sess").unwrap().error.as_deref(), Some("boom"));
                assert_eq!(s.blocking().len(), 1);

                s.set_state("sess", ConnectionState::Connected, cx);
                assert_eq!(s.get("sess").unwrap().state, ConnectionState::Connected);
                assert!(s.get("sess").unwrap().error.is_none());
                assert_eq!(
                    s.get("sess").unwrap().stage_status(ConnStage::Shell),
                    StageStatus::Done
                );
                assert_eq!(s.blocking().len(), 0);

                s.remove("sess", cx);
                assert!(s.get("sess").is_none());
            });
        });
    }
}
