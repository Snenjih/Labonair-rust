//! Local terminal session registry (T03-005).
//!
//! Owns every live local PTY-backed [`TerminalSession`] and hands out cheap,
//! `Clone`able [`SessionHandle`]s keyed by a monotonic [`SessionId`]. Sessions
//! run fully independently: each has its own PTY, its own reader thread and its
//! own emulator, so any number can be alive at once and none is ever paused —
//! switching which tab is *visible* is purely a UI concern and never touches
//! this layer.
//!
//! # Interface for the Phase 3 tab system
//!
//! The tab system does **not** own terminal logic. It holds an
//! `Arc<TerminalRegistry>` plus, per local-terminal tab, a [`SessionId`]. It
//! calls:
//!
//! * [`TerminalRegistry::create`] when a new local terminal tab is opened —
//!   passing the resolved shell, the inherited working directory (from the
//!   previous tab's [`crate::SessionMetadata::cwd`], see T03-004) and an
//!   optional startup command.
//! * [`TerminalRegistry::handle`] to get a [`SessionHandle`] for writing input,
//!   resizing, rendering and draining events for the currently visible tab.
//! * [`SessionHandle::status`] / [`SessionHandle::has_foreground_job`] to render
//!   the "shell exited — click to restart" screen and the close-confirmation
//!   prompt (Labonair's KeepTerminal behaviour).
//! * [`SessionHandle::restart`] when the user clicks that screen — a fresh shell
//!   is spawned in place with the same options, keeping the same [`SessionId`]
//!   and therefore the same tab.
//! * [`TerminalRegistry::close`] when the tab is closed — sends `SIGHUP`, then
//!   drops the session (hard `SIGKILL` + thread join in `Drop`), so it always
//!   returns promptly even with a hung foreground job.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::engine::{TermDimensions, TerminalEvent};
use crate::host::{HostedLink, HostedSession, InProcessHost, SessionHost};
use crate::session::{
    ForegroundProcess, RemoteFeed, RemoteResizer, RemoteSession, RemoteWriter, SessionAccess,
    SessionOptions, TerminalSession,
};
use crate::TerminalColors;

/// The transport backing one registered session: a local PTY, an external
/// (SSH) transport, or a session hosted by the detached session host. All
/// expose the same operations to [`SessionHandle`]. Public only so a
/// [`crate::SessionHost`] can return it.
pub enum SessionBackend {
    Local(TerminalSession),
    Remote(RemoteSession),
    /// A session living in the detached session host daemon (ADR 0009). It
    /// reuses the [`RemoteSession`] / [`RemoteFeed`] seam: the host client
    /// pumps daemon output into the feed and forwards writes/resizes.
    Hosted(HostedSession),
}

impl SessionBackend {
    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        match self {
            SessionBackend::Local(s) => s.write(bytes),
            SessionBackend::Remote(s) => s.write(bytes),
            SessionBackend::Hosted(s) => s.remote.write(bytes),
        }
    }

    fn resize(&mut self, dimensions: TermDimensions) -> Result<(), String> {
        match self {
            SessionBackend::Local(s) => s.resize(dimensions),
            SessionBackend::Remote(s) => s.resize(dimensions),
            SessionBackend::Hosted(s) => s.remote.resize(dimensions),
        }
    }

    fn set_colors(&self, colors: TerminalColors) -> Result<(), String> {
        match self {
            SessionBackend::Local(s) => s.set_colors(colors),
            SessionBackend::Remote(s) => s.set_colors(colors),
            SessionBackend::Hosted(s) => s.remote.set_colors(colors),
        }
    }

    fn drain_events(&self) -> Vec<TerminalEvent> {
        match self {
            SessionBackend::Local(s) => s.drain_events(),
            SessionBackend::Remote(s) => s.drain_events(),
            SessionBackend::Hosted(s) => s.remote.drain_events(),
        }
    }

    fn has_foreground_job(&self) -> bool {
        match self {
            SessionBackend::Local(s) => s.has_foreground_job(),
            SessionBackend::Remote(_) => false,
            SessionBackend::Hosted(s) => s.control.foreground().is_some(),
        }
    }

    fn foreground_process(&self) -> Option<ForegroundProcess> {
        match self {
            SessionBackend::Local(s) => s.foreground_process(),
            SessionBackend::Remote(_) => None,
            SessionBackend::Hosted(s) => s.control.foreground(),
        }
    }

    /// Closing the session's tab: hang up a local shell, stop a hosted one.
    fn terminate(&self) {
        match self {
            SessionBackend::Local(s) => s.terminate(),
            SessionBackend::Remote(_) => {}
            SessionBackend::Hosted(s) => s.control.kill(),
        }
    }

    /// The app is going away: hosted sessions keep running in the daemon,
    /// everything else is terminated as on close.
    fn release(&self) {
        match self {
            SessionBackend::Hosted(s) => s.control.detach(),
            other => other.terminate(),
        }
    }

    fn access(&self) -> &dyn SessionAccess {
        match self {
            SessionBackend::Local(s) => s,
            SessionBackend::Remote(s) => s,
            SessionBackend::Hosted(s) => &s.remote,
        }
    }

    fn serialize_scrollback(&self, max_lines: Option<usize>) -> Option<String> {
        match self {
            SessionBackend::Local(s) => s
                .with_emulator(|e| e.serialize_scrollback(max_lines))
                .ok()
                .filter(|t| !t.trim().is_empty()),
            // The hosted pane's local emulator holds the full history.
            SessionBackend::Hosted(s) => s.remote.serialize_scrollback(max_lines),
            // Remote (SSH) panes are not persisted with scrollback.
            SessionBackend::Remote(_) => None,
        }
    }

    fn subscribe_agent_output(&self) -> Result<tokio::sync::broadcast::Receiver<Vec<u8>>, String> {
        match self {
            SessionBackend::Local(session) => Ok(session.subscribe_agent_output()),
            SessionBackend::Hosted(s) => Ok(s.remote.subscribe_agent_output()),
            SessionBackend::Remote(_) => {
                Err("agent output subscription is only available for local sessions".to_string())
            }
        }
    }
}

/// Opaque, process-unique identifier for a local terminal session. Starts at 1
/// (0 reads as "unset" in some call sites) and never reused.
pub type SessionId = u64;

/// Lifecycle state of a session's shell process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// The shell is alive.
    Running,
    /// The shell exited with this status code (0 when only a bare EOF was seen).
    Exited(i32),
}

struct Slot {
    /// The live session. Behind a `Mutex` only so [`restart`] can swap the
    /// whole value; all hot-path access (`write`, `render`) is `&self` on the
    /// inner session and never contends.
    session: Mutex<SessionBackend>,
    /// Everything needed to respawn this session in place on restart.
    colors: Mutex<TerminalColors>,
    options: SessionOptions,
    status: Mutex<SessionStatus>,
    event_notify: Arc<tokio::sync::Notify>,
}

/// A cheap, `Clone`able reference to one registered session.
#[derive(Clone)]
pub struct SessionHandle {
    id: SessionId,
    slot: Arc<Slot>,
}

impl SessionHandle {
    /// This session's id.
    pub fn id(&self) -> SessionId {
        self.id
    }

    /// Write user input bytes to the shell.
    pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.slot.session.lock().unwrap().write(bytes)
    }

    /// Subscribe to raw output from this session without disturbing the
    /// terminal renderer. Remote sessions are intentionally not exposed here;
    /// SSH has its own transport-owned observation path.
    pub fn subscribe_agent_output(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<Vec<u8>>, String> {
        self.slot.session.lock().unwrap().subscribe_agent_output()
    }

    /// Resize the emulator grid and the underlying PTY.
    pub fn resize(&self, dimensions: TermDimensions) -> Result<(), String> {
        self.slot.session.lock().unwrap().resize(dimensions)
    }

    /// Swap the theme palette (light/dark switch). Also remembered for restart.
    pub fn set_colors(&self, colors: TerminalColors) -> Result<(), String> {
        *self.slot.colors.lock().unwrap() = colors;
        self.slot.session.lock().unwrap().set_colors(colors)
    }

    /// Run `f` with the locked session (advanced access: selection, scroll,
    /// metadata, `ai_context`, `render`, …).
    pub fn with<R>(&self, f: impl FnOnce(&dyn SessionAccess) -> R) -> R {
        let guard = self.slot.session.lock().unwrap();
        f(guard.access())
    }

    /// Drain pending terminal events. Exit events flip [`status`](Self::status)
    /// to [`SessionStatus::Exited`] as a side effect, so a UI poll loop that
    /// already drains events gets shell-exit tracking for free.
    pub fn drain_events(&self) -> Vec<TerminalEvent> {
        let events = self.slot.session.lock().unwrap().drain_events();
        for ev in &events {
            match ev {
                TerminalEvent::ChildExit(code) => {
                    *self.slot.status.lock().unwrap() = SessionStatus::Exited(*code);
                }
                TerminalEvent::Exit => {
                    let mut s = self.slot.status.lock().unwrap();
                    if *s == SessionStatus::Running {
                        *s = SessionStatus::Exited(0);
                    }
                }
                _ => {}
            }
        }
        events
    }

    /// Wait for a session event without polling. The notifier belongs to the
    /// registry slot rather than a concrete backend so it remains valid when a
    /// local terminal is restarted in place.
    pub async fn wait_for_events(&self) {
        self.slot.event_notify.notified().await;
    }

    /// Serialize this session's scrollback + visible screen as plain text for
    /// session persistence (T14-002). `None` for a remote session, an
    /// alt-screen buffer, or an empty grid.
    pub fn serialize_scrollback(&self, max_lines: Option<usize>) -> Option<String> {
        self.slot
            .session
            .lock()
            .unwrap()
            .serialize_scrollback(max_lines)
    }

    /// Current lifecycle state of the shell.
    pub fn status(&self) -> SessionStatus {
        *self.slot.status.lock().unwrap()
    }

    /// Whether a foreground job other than the shell owns the tty (drives the
    /// close-confirmation prompt).
    pub fn has_foreground_job(&self) -> bool {
        self.slot.session.lock().unwrap().has_foreground_job()
    }

    /// The foreground job (other than the shell) and its process name, for
    /// local and hosted sessions — remote (SSH) sessions report `None`.
    pub fn foreground_process(&self) -> Option<ForegroundProcess> {
        self.slot.session.lock().unwrap().foreground_process()
    }

    /// The daemon's id of this session, when it is hosted.
    pub fn host_session_id(&self) -> Option<u64> {
        match &*self.slot.session.lock().unwrap() {
            SessionBackend::Hosted(s) => Some(s.control.host_session_id()),
            _ => None,
        }
    }

    /// Whether the session lives in the detached session host.
    pub fn is_hosted(&self) -> bool {
        matches!(
            &*self.slot.session.lock().unwrap(),
            SessionBackend::Hosted(_)
        )
    }

    /// Link state of a hosted session (`None` for every other kind).
    pub fn hosted_link(&self) -> Option<HostedLink> {
        match &*self.slot.session.lock().unwrap() {
            SessionBackend::Hosted(s) => Some(s.control.link()),
            _ => None,
        }
    }

    /// Reattach a hosted session after [`HostedLink::Stolen`] or
    /// [`HostedLink::HostLost`].
    pub fn reattach(&self) -> Result<(), String> {
        let control = match &*self.slot.session.lock().unwrap() {
            SessionBackend::Hosted(s) => Arc::clone(&s.control),
            _ => return Err("not a hosted session".to_string()),
        };
        // Outside the slot lock: this may wait on the daemon.
        control.reattach()
    }

    /// Respawn the shell in place after it exited, reusing the original options
    /// (shell, cwd, startup command) and the latest palette. Keeps the same
    /// [`SessionId`], so the owning tab is untouched. Errors if the session is
    /// still running.
    pub fn restart(&self, dimensions: TermDimensions) -> Result<(), String> {
        let mut session = self.slot.session.lock().unwrap();
        if !matches!(&*session, SessionBackend::Local(_)) {
            return Err("cannot restart a remote session".to_string());
        }
        if !matches!(*self.slot.status.lock().unwrap(), SessionStatus::Exited(_)) {
            return Err("session is still running".to_string());
        }
        let colors = *self.slot.colors.lock().unwrap();
        let fresh = TerminalSession::spawn_with_notifier(
            colors,
            dimensions,
            self.slot.options.clone(),
            Arc::clone(&self.slot.event_notify),
        )?;
        session.terminate();
        *session = SessionBackend::Local(fresh);
        *self.slot.status.lock().unwrap() = SessionStatus::Running;
        self.slot.event_notify.notify_one();
        Ok(())
    }
}

/// Thread-safe map of all live local terminal sessions.
pub struct TerminalRegistry {
    sessions: RwLock<HashMap<SessionId, Arc<Slot>>>,
    next_id: AtomicU64,
}

impl Default for TerminalRegistry {
    fn default() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }
}

impl TerminalRegistry {
    /// A fresh, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new local session and register it. Returns its id.
    pub fn create(
        &self,
        colors: TerminalColors,
        dimensions: TermDimensions,
        options: SessionOptions,
    ) -> Result<SessionId, String> {
        self.create_with_host(&InProcessHost, colors, dimensions, options)
    }

    /// Create a session through `host` (in-process PTY or the detached session
    /// host) and register it. The host is chosen per session, so one registry
    /// can hold both kinds.
    pub fn create_with_host(
        &self,
        host: &dyn SessionHost,
        colors: TerminalColors,
        dimensions: TermDimensions,
        options: SessionOptions,
    ) -> Result<SessionId, String> {
        let event_notify = Arc::new(tokio::sync::Notify::new());
        let session = host.create_session(
            colors,
            dimensions,
            options.clone(),
            Arc::clone(&event_notify),
        )?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let slot = Arc::new(Slot {
            session: Mutex::new(session),
            colors: Mutex::new(colors),
            options,
            status: Mutex::new(SessionStatus::Running),
            event_notify,
        });
        self.sessions.write().unwrap().insert(id, slot);
        Ok(id)
    }

    /// Register a session backed by an external transport (SSH). Returns its id
    /// plus a [`RemoteFeed`] the caller wires to the transport's output reader
    /// (T07-001). `writer` / `resizer` carry user input and resize requests back
    /// out over the transport.
    pub fn create_remote(
        &self,
        colors: TerminalColors,
        dimensions: TermDimensions,
        writer: RemoteWriter,
        resizer: RemoteResizer,
    ) -> (SessionId, RemoteFeed) {
        let event_notify = Arc::new(tokio::sync::Notify::new());
        let (session, feed) = RemoteSession::new_with_notifier(
            colors,
            dimensions,
            writer,
            resizer,
            Arc::clone(&event_notify),
        );
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let slot = Arc::new(Slot {
            session: Mutex::new(SessionBackend::Remote(session)),
            colors: Mutex::new(colors),
            options: SessionOptions::default(),
            status: Mutex::new(SessionStatus::Running),
            event_notify,
        });
        self.sessions.write().unwrap().insert(id, slot);
        (id, feed)
    }

    /// A handle to a registered session, if `id` is known.
    pub fn handle(&self, id: SessionId) -> Option<SessionHandle> {
        self.sessions
            .read()
            .unwrap()
            .get(&id)
            .map(|slot| SessionHandle {
                id,
                slot: Arc::clone(slot),
            })
    }

    /// Ids of all live sessions, ascending.
    pub fn ids(&self) -> Vec<SessionId> {
        let mut ids: Vec<SessionId> = self.sessions.read().unwrap().keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// Number of registered sessions.
    pub fn len(&self) -> usize {
        self.sessions.read().unwrap().len()
    }

    /// Whether the registry has no sessions.
    pub fn is_empty(&self) -> bool {
        self.sessions.read().unwrap().is_empty()
    }

    /// Terminate and unregister a session. `SIGHUP` first, then the session is
    /// dropped (hard `SIGKILL` + reader-thread join in `Drop`). Returns
    /// promptly even if a foreground job is wedged. No-op for unknown ids.
    pub fn close(&self, id: SessionId) {
        let slot = self.sessions.write().unwrap().remove(&id);
        if let Some(slot) = slot {
            slot.session.lock().unwrap().terminate();
            // `slot` (and the inner `TerminalSession`) drops here.
        }
    }

    /// Terminate and unregister every session. Hosted sessions are only
    /// detached: they keep running in the daemon.
    pub fn close_all(&self) {
        let slots: Vec<_> = self
            .sessions
            .write()
            .unwrap()
            .drain()
            .map(|(_, slot)| slot)
            .collect();
        for slot in slots {
            slot.session.lock().unwrap().release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn colors() -> TerminalColors {
        TerminalColors::xterm_default()
    }

    fn dims() -> TermDimensions {
        TermDimensions::new(80, 24)
    }

    fn sh(startup: Option<&str>) -> SessionOptions {
        SessionOptions {
            shell: Some("/bin/sh".to_string()),
            startup_command: startup.map(str::to_string),
            env: vec![("PS1".to_string(), "$ ".to_string())],
            ..SessionOptions::default()
        }
    }

    fn wait_for(h: &SessionHandle, timeout: Duration, pred: impl Fn(&str) -> bool) -> String {
        let start = Instant::now();
        loop {
            let _ = h.drain_events();
            let text = h.with(|s| s.render().unwrap().to_text());
            if pred(&text) || start.elapsed() > timeout {
                return text;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn manages_multiple_independent_sessions() {
        let reg = TerminalRegistry::new();
        let a = reg.create(colors(), dims(), sh(None)).unwrap();
        let b = reg.create(colors(), dims(), sh(None)).unwrap();
        assert_ne!(a, b);
        assert_eq!(reg.len(), 2);

        let ha = reg.handle(a).unwrap();
        let hb = reg.handle(b).unwrap();
        ha.write(b"printf 'AAA_MARKER\\n'\n").unwrap();
        hb.write(b"printf 'BBB_MARKER\\n'\n").unwrap();

        let ta = wait_for(&ha, Duration::from_secs(5), |t| t.contains("AAA_MARKER"));
        let tb = wait_for(&hb, Duration::from_secs(5), |t| t.contains("BBB_MARKER"));
        assert!(
            ta.contains("AAA_MARKER") && !ta.contains("BBB_MARKER"),
            "A:\n{ta}"
        );
        assert!(
            tb.contains("BBB_MARKER") && !tb.contains("AAA_MARKER"),
            "B:\n{tb}"
        );

        reg.close_all();
        assert!(reg.is_empty());
    }

    #[test]
    fn runs_a_startup_command() {
        let reg = TerminalRegistry::new();
        let id = reg
            .create(colors(), dims(), sh(Some("printf 'STARTUP_OK\\n'")))
            .unwrap();
        let h = reg.handle(id).unwrap();
        let text = wait_for(&h, Duration::from_secs(5), |t| t.contains("STARTUP_OK"));
        assert!(text.contains("STARTUP_OK"), "screen:\n{text}");
        reg.close(id);
    }

    #[test]
    fn closes_cleanly_with_a_foreground_process() {
        let reg = TerminalRegistry::new();
        let id = reg.create(colors(), dims(), sh(None)).unwrap();
        let h = reg.handle(id).unwrap();
        h.write(b"sleep 300\n").unwrap();

        // Wait until the `sleep` owns the tty.
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) && !h.has_foreground_job() {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(h.has_foreground_job(), "sleep never took the foreground");

        let close_started = Instant::now();
        reg.close(id);
        assert!(
            close_started.elapsed() < Duration::from_secs(3),
            "close() hung on a foreground job"
        );
        assert!(reg.handle(id).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn reports_the_foreground_process_by_name() {
        let reg = TerminalRegistry::new();
        let id = reg.create(colors(), dims(), sh(None)).unwrap();
        let h = reg.handle(id).unwrap();
        assert_eq!(h.foreground_process(), None, "idle shell is not a job");
        h.write(b"sleep 300\n").unwrap();
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) && h.foreground_process().is_none() {
            std::thread::sleep(Duration::from_millis(20));
        }
        let fg = h
            .foreground_process()
            .expect("sleep never took the foreground");
        assert_eq!(fg.name, "sleep");
        reg.close(id);
    }

    #[test]
    fn keeps_running_while_not_the_visible_tab() {
        let reg = TerminalRegistry::new();
        let bg = reg.create(colors(), dims(), sh(None)).unwrap();
        let fg = reg.create(colors(), dims(), sh(None)).unwrap();
        let hbg = reg.handle(bg).unwrap();

        // Kick off work on the "background" session, then only ever touch the
        // "foreground" one for a while (simulating a tab switch).
        hbg.write(b"sleep 1; printf 'BG_DONE\\n'\n").unwrap();
        let hfg = reg.handle(fg).unwrap();
        for _ in 0..30 {
            let _ = hfg.drain_events();
            std::thread::sleep(Duration::from_millis(50));
        }

        let text = wait_for(&hbg, Duration::from_secs(5), |t| t.contains("BG_DONE"));
        assert!(
            text.contains("BG_DONE"),
            "background session did not progress:\n{text}"
        );
        reg.close_all();
    }

    #[test]
    fn restarts_in_place_after_the_shell_exits() {
        let reg = TerminalRegistry::new();
        let id = reg.create(colors(), dims(), sh(None)).unwrap();
        let h = reg.handle(id).unwrap();

        h.write(b"exit 7\n").unwrap();
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) && h.status() == SessionStatus::Running {
            let _ = h.drain_events();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            matches!(h.status(), SessionStatus::Exited(_)),
            "shell did not exit"
        );

        h.restart(dims()).unwrap();
        assert!(
            h.restart(dims()).is_err(),
            "restart on a running session must fail"
        );
        assert_eq!(h.status(), SessionStatus::Running);
        h.write(b"printf 'REBORN\\n'\n").unwrap();
        let text = wait_for(&h, Duration::from_secs(5), |t| t.contains("REBORN"));
        assert!(text.contains("REBORN"), "restarted shell dead:\n{text}");
        reg.close(id);
    }

    /// A hosted backend with no daemon behind it, counting what the registry
    /// asks of its control surface.
    #[derive(Default)]
    struct CountingControl {
        kills: std::sync::atomic::AtomicUsize,
        detaches: std::sync::atomic::AtomicUsize,
    }

    impl crate::host::HostedControl for CountingControl {
        fn host_session_id(&self) -> u64 {
            42
        }
        fn link(&self) -> HostedLink {
            HostedLink::Live
        }
        fn reattach(&self) -> Result<(), String> {
            Ok(())
        }
        fn foreground(&self) -> Option<ForegroundProcess> {
            None
        }
        fn kill(&self) {
            self.kills.fetch_add(1, Ordering::SeqCst);
        }
        fn detach(&self) {
            self.detaches.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct FakeHostedHost(Arc<CountingControl>);

    impl SessionHost for FakeHostedHost {
        fn create_session(
            &self,
            colors: TerminalColors,
            dimensions: TermDimensions,
            _options: SessionOptions,
            event_notify: Arc<tokio::sync::Notify>,
        ) -> Result<SessionBackend, String> {
            let (remote, _feed) = RemoteSession::new_with_notifier(
                colors,
                dimensions,
                Arc::new(|_| {}),
                Arc::new(|_, _| {}),
                event_notify,
            );
            Ok(SessionBackend::Hosted(HostedSession::new(
                remote,
                self.0.clone(),
            )))
        }
    }

    #[test]
    fn closing_a_tab_kills_a_hosted_session_but_quitting_only_detaches() {
        use std::sync::atomic::Ordering::SeqCst;

        let control = Arc::new(CountingControl::default());
        let host = FakeHostedHost(control.clone());
        let reg = TerminalRegistry::new();

        // Close tab: kill.
        let id = reg
            .create_with_host(&host, colors(), dims(), sh(None))
            .unwrap();
        let handle = reg.handle(id).unwrap();
        assert!(handle.is_hosted());
        assert_eq!(handle.host_session_id(), Some(42));
        assert_eq!(handle.hosted_link(), Some(HostedLink::Live));
        reg.close(id);
        assert_eq!(
            (control.kills.load(SeqCst), control.detaches.load(SeqCst)),
            (1, 0)
        );

        // Quit with a hosted and an app-bound session side by side: the hosted
        // one detaches, the app-bound one is terminated as before.
        let hosted = reg
            .create_with_host(&host, colors(), dims(), sh(None))
            .unwrap();
        let local = reg.create(colors(), dims(), sh(None)).unwrap();
        assert!(!reg.handle(local).unwrap().is_hosted());
        reg.close_all();
        assert!(reg.is_empty());
        assert!(reg.handle(hosted).is_none() && reg.handle(local).is_none());
        assert_eq!(
            (control.kills.load(SeqCst), control.detaches.load(SeqCst)),
            (1, 1)
        );
    }

    #[test]
    fn remote_session_feeds_output_and_forwards_input() {
        use std::sync::Mutex as StdMutex;

        let reg = TerminalRegistry::new();
        let sent: Arc<StdMutex<Vec<u8>>> = Arc::new(StdMutex::new(Vec::new()));
        let resized: Arc<StdMutex<Vec<(u16, u16)>>> = Arc::new(StdMutex::new(Vec::new()));
        let writer: RemoteWriter = {
            let sent = Arc::clone(&sent);
            Arc::new(move |b| sent.lock().unwrap().extend(b))
        };
        let resizer: RemoteResizer = {
            let resized = Arc::clone(&resized);
            Arc::new(move |c, r| resized.lock().unwrap().push((c, r)))
        };

        let (id, feed) = reg.create_remote(colors(), dims(), writer, resizer);
        let h = reg.handle(id).unwrap();

        // Remote output → emulator grid.
        feed.feed(b"REMOTE_HELLO\r\n");
        let _ = h.drain_events();
        assert!(h
            .with(|s| s.render().unwrap().to_text())
            .contains("REMOTE_HELLO"));

        // User input → transport writer.
        h.write(b"ls\n").unwrap();
        assert_eq!(&*sent.lock().unwrap(), b"ls\n");

        // Resize → transport resizer + emulator grid.
        h.resize(TermDimensions::new(120, 40)).unwrap();
        assert_eq!(resized.lock().unwrap().last().copied(), Some((120, 40)));
        assert_eq!(h.with(|s| s.render().unwrap().columns), 120);

        // Transport loss surfaces as a shell exit.
        feed.mark_disconnected();
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(2) && h.status() == SessionStatus::Running {
            let _ = h.drain_events();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(matches!(h.status(), SessionStatus::Exited(_)));

        // Remote sessions cannot be restarted in place.
        assert!(h.restart(dims()).is_err());
        reg.close(id);
    }
}
