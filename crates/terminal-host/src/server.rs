//! The detached session host: daemon logic as a library (ADR 0009).
//!
//! [`Server::bind`] takes the single-instance lock, listens on a private Unix
//! socket and [`Server::run`] serves clients until the daemon is idle, told to
//! shut down, or stopped by a signal ([`ServerHandle::stop`]). The
//! `labonair-host` binary is a thin wrapper; tests run the same code
//! in-process.
//!
//! ## Threads, locks, data flow
//!
//! Every session owns a PTY, a **shadow emulator** (inside the
//! [`TerminalSession`]) and a reader thread. The reader parses each chunk into
//! the shadow emulator and — still holding the emulator lock — runs the
//! [`OutputTap`](labonair_terminal_core::OutputTap), which copies the raw bytes
//! into the attached client's bounded [`Outbox`]. Because attach (resize +
//! snapshot + start of the live stream) happens under the same emulator lock,
//! snapshot and live bytes never gap or overlap. Lock order is always
//! session -> emulator -> sink -> outbox. The reader never waits on a client:
//! when a client's queue is full the live stream is dropped and a fresh
//! snapshot ([`DataKind::Resync`]) is sent once the client has drained.
//!
//! A session's byte source is either a local PTY or an SSH PTY channel
//! ([`Source`]); both feed the same shadow emulator and client queue, so
//! attach, snapshot, resync, steal and persistence are shared. The SSH side
//! (request/response RPC, events, transfers) lives in [`ssh_host`].

use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use labonair_terminal_core::{
    resize_wiggle_sizes, scrollback, OutputTap, SessionOptions, SnapshotOptions, TermDimensions,
    TerminalColors, TerminalEmulator, TerminalEvent, TerminalSession,
};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{watch, Notify};
use tokio::task::JoinSet;

use crate::protocol::{
    control_frame, data_frame, read_frame, CreateOptions, DataKind, DetachReason, Frame, Hello,
    HelloAck, Message, SessionEvent, SessionId, SessionInfo, SessionStatus, WireColors, MIN_COMPAT,
    PROTO_VERSION,
};

mod ssh_host;
pub use ssh_host::{HostServices, InstallHost};

/// `sun_path` is 104 bytes on macOS (108 on Linux), including the NUL.
const SUN_PATH_LIMIT: usize = 100;
const DEFAULT_SCROLLBACK_LINES: usize = 5000;
const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const WIGGLE_PAUSE: Duration = Duration::from_millis(30);
const JOBS_POLL: Duration = Duration::from_secs(5);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ── Paths, lock, errors ──────────────────────────────────────────────────────

/// Where the host keeps its files. All of them live in `<data_dir>/host/`
/// (mode 0700) except a socket whose path would exceed `sun_path`, which falls
/// back to `/tmp/labonair-<uid>/`.
#[derive(Debug, Clone)]
pub struct HostPaths {
    pub dir: PathBuf,
    pub lock: PathBuf,
    pub log: PathBuf,
    pub socket: PathBuf,
}

impl HostPaths {
    pub fn new(data_dir: &Path) -> Self {
        let dir = data_dir.join("host");
        let mut socket = dir.join("host.sock");
        if socket.as_os_str().len() > SUN_PATH_LIMIT {
            let uid = unsafe { libc::getuid() };
            socket = PathBuf::from(format!("/tmp/labonair-{uid}"))
                .join(format!("host-{:016x}.sock", fnv1a(data_dir)));
        }
        Self {
            lock: dir.join("host.lock"),
            log: dir.join("host.log"),
            dir,
            socket,
        }
    }
}

fn fnv1a(path: &Path) -> u64 {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str()
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

/// Create `dir` (and parents) with mode 0700 and refuse a directory that
/// belongs to someone else.
pub fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let meta = std::fs::metadata(dir)?;
    if meta.uid() != unsafe { libc::getuid() } {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is not owned by the current user", dir.display()),
        ));
    }
    if meta.mode() & 0o077 != 0 {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum ServerError {
    /// Another host process holds `host.lock`.
    AlreadyRunning,
    Io(io::Error),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "another labonair-host is already running"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Holds the `flock` for the daemon's lifetime (released on drop / exit).
struct InstanceLock(#[allow(dead_code)] std::fs::File);

impl InstanceLock {
    fn acquire(path: &Path) -> Result<Self, ServerError> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .open(path)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = io::Error::last_os_error();
            return Err(if error.kind() == io::ErrorKind::WouldBlock {
                ServerError::AlreadyRunning
            } else {
                error.into()
            });
        }
        let _ = file.set_len(0);
        let _ = write!(file, "{}", std::process::id());
        Ok(Self(file))
    }
}

// ── Configuration & handle ───────────────────────────────────────────────────

/// Why [`Server::run`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// No sessions, jobs or clients for `idle_timeout`.
    Idle,
    /// A client sent [`Message::Shutdown`]; all sessions were killed.
    Shutdown,
    /// SIGTERM/SIGHUP: scrollbacks were persisted for restore-by-replay.
    Signal,
}

pub struct ServerConfig {
    /// Application data directory; the host lives in `<data_dir>/host`,
    /// scrollbacks persist under `<data_dir>/scrollback`.
    pub data_dir: PathBuf,
    /// Self-exit after this long without sessions, jobs and clients.
    pub idle_timeout: Duration,
    /// An exited session is forgotten after this long if nobody acks it.
    pub exited_ttl: Duration,
    /// Per-client bound on queued live output (bytes).
    pub queue_limit: usize,
    /// Default scrollback lines (overridden by `Hello`/`CreateOptions`).
    pub scrollback_lines: usize,
    pub build_id: String,
    /// `true` while a background job keeps the daemon alive. It is ORed with
    /// the transfer jobs of [`ServerConfig::services`] (queued, running,
    /// paused or awaiting a conflict decision).
    pub jobs_active: Arc<dyn Fn() -> bool + Send + Sync>,
    /// The SSH / SFTP / transfer stack. `None` = those calls are refused (a
    /// terminal-only host).
    pub services: Option<HostServices>,
}

impl ServerConfig {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            idle_timeout: Duration::from_secs(60),
            exited_ttl: Duration::from_secs(24 * 3600),
            queue_limit: 2 * 1024 * 1024,
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
            build_id: env!("CARGO_PKG_VERSION").to_string(),
            jobs_active: Arc::new(|| false),
            services: None,
        }
    }

    /// Host SSH sessions, SFTP and transfers with `services`.
    pub fn with_services(mut self, services: HostServices) -> Self {
        self.services = Some(services);
        self
    }
}

/// Cloneable remote control for a running [`Server`].
#[derive(Clone)]
pub struct ServerHandle {
    state: Arc<State>,
}

impl ServerHandle {
    /// Ask [`Server::run`] to finish with `reason`. `Signal` persists
    /// scrollbacks first; `Shutdown` just kills everything.
    pub fn stop(&self, reason: ExitReason) {
        self.state.stop_tx.send_if_modified(|current| {
            let first = current.is_none();
            if first {
                *current = Some(reason);
            }
            first
        });
    }

    pub fn session_count(&self) -> usize {
        lock(&self.state.sessions).len()
    }
}

// ── Outbox: bounded per-client frame queue ───────────────────────────────────

struct QFrame {
    session: SessionId,
    /// Counts against the limit and is dropped on overflow.
    output: bool,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct OutInner {
    frames: VecDeque<QFrame>,
    output_bytes: usize,
    resync: Vec<SessionId>,
    closed: bool,
}

enum Taken {
    Frames(Vec<QFrame>),
    Resync(Vec<SessionId>),
    Closed,
    Empty,
}

/// Frames waiting for one client. Producers never block: control and snapshot
/// frames are always queued, live output is bounded by `limit`.
struct Outbox {
    inner: Mutex<OutInner>,
    notify: Notify,
    limit: usize,
}

impl Outbox {
    fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(OutInner::default()),
            notify: Notify::new(),
            limit,
        })
    }

    fn push_control(&self, message: &Message) {
        let Ok(bytes) = control_frame(message) else {
            return;
        };
        self.push(QFrame {
            session: 0,
            output: false,
            bytes,
        });
    }

    /// Queue a snapshot/resync frame unconditionally.
    fn push_data(&self, kind: DataKind, session: SessionId, data: &[u8]) {
        self.push(QFrame {
            session,
            output: false,
            bytes: data_frame(kind, session, data),
        });
    }

    fn push(&self, frame: QFrame) {
        let mut inner = lock(&self.inner);
        if inner.closed {
            return;
        }
        inner.frames.push_back(frame);
        drop(inner);
        self.notify.notify_one();
    }

    /// Queue live output. Returns `false` when the queue is full: the queued
    /// output of `session` has then been discarded and a resync is pending.
    fn push_output(&self, session: SessionId, bytes: &[u8]) -> bool {
        let mut inner = lock(&self.inner);
        if inner.closed {
            return true;
        }
        if inner.output_bytes + bytes.len() > self.limit {
            let mut dropped = 0;
            inner.frames.retain(|frame| {
                let drop_it = frame.output && frame.session == session;
                if drop_it {
                    dropped += frame.bytes.len();
                }
                !drop_it
            });
            inner.output_bytes -= dropped.min(inner.output_bytes);
            if !inner.resync.contains(&session) {
                inner.resync.push(session);
            }
            drop(inner);
            self.notify.notify_one();
            return false;
        }
        inner.output_bytes += bytes.len();
        inner.frames.push_back(QFrame {
            session,
            output: true,
            bytes: data_frame(DataKind::Output, session, bytes),
        });
        drop(inner);
        self.notify.notify_one();
        true
    }

    /// Next unit of work for the writer. Resyncs are only handed out once the
    /// queue is empty, i.e. the client has caught up.
    fn take(&self) -> Taken {
        let mut inner = lock(&self.inner);
        if !inner.frames.is_empty() {
            inner.output_bytes = 0;
            return Taken::Frames(inner.frames.drain(..).collect());
        }
        if !inner.resync.is_empty() {
            return Taken::Resync(std::mem::take(&mut inner.resync));
        }
        if inner.closed {
            Taken::Closed
        } else {
            Taken::Empty
        }
    }

    /// Stop accepting frames; the writer flushes what is queued, then ends.
    fn close(&self) {
        lock(&self.inner).closed = true;
        self.notify.notify_one();
    }
}

async fn writer_loop(write_half: OwnedWriteHalf, outbox: Arc<Outbox>, state: Arc<State>) {
    let mut writer = BufWriter::with_capacity(64 * 1024, write_half);
    loop {
        match outbox.take() {
            Taken::Frames(frames) => {
                for frame in frames {
                    if writer.write_all(&frame.bytes).await.is_err() {
                        outbox.close();
                        return;
                    }
                }
                if writer.flush().await.is_err() {
                    outbox.close();
                    return;
                }
            }
            Taken::Resync(sessions) => {
                for id in sessions {
                    if let Some(session) = state.session(id) {
                        session.resync(&outbox);
                    }
                }
            }
            Taken::Closed => {
                let _ = writer.shutdown().await;
                return;
            }
            Taken::Empty => outbox.notify.notified().await,
        }
    }
}

// ── Session ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct SinkTarget {
    conn: u64,
    outbox: Arc<Outbox>,
}

enum Sink {
    None,
    Live(SinkTarget),
    /// Queue overflowed: live output is dropped until the writer resyncs.
    Overflowed(SinkTarget),
}

impl Sink {
    fn target(&self) -> Option<&SinkTarget> {
        match self {
            Sink::None => None,
            Sink::Live(t) | Sink::Overflowed(t) => Some(t),
        }
    }
}

enum Status {
    Running,
    Exited { code: Option<i32>, at: Instant },
}

/// Where a session's bytes come from. Everything else (shadow emulator
/// access, client queue, status) is shared.
enum Source {
    Local {
        term: Mutex<TerminalSession>,
        input: std::sync::mpsc::Sender<Vec<u8>>,
    },
    Ssh(Box<SshSource>),
}

/// Commands for the task that owns the SSH PTY write side.
enum SshCmd {
    Write(Vec<u8>),
    Resize(u16, u16),
}

/// A hosted SSH shell: the shadow emulator is fed by the transport's output
/// sink; input and resizes go through [`SshCmd`] to `SshPtyService`.
struct SshSource {
    emulator: Mutex<TerminalEmulator>,
    events: Mutex<std::sync::mpsc::Receiver<TerminalEvent>>,
    cmd: tokio::sync::mpsc::UnboundedSender<SshCmd>,
    ssh_id: String,
    host_id: String,
    connection: Arc<dyn labonair_ssh::SshConnectionService>,
    /// Humanized reason once the connection is gone.
    lost: Mutex<Option<String>>,
    connected: AtomicBool,
}

struct Session {
    id: SessionId,
    pane_tag: Option<String>,
    label: Option<String>,
    persist_key: String,
    scrollback_lines: usize,
    source: Source,
    sink: Arc<Mutex<Sink>>,
    status: Mutex<Status>,
    killed: AtomicBool,
    notify: Arc<Notify>,
}

/// Mirror raw output to the attached client's queue. Runs under the emulator
/// lock, on the thread that feeds the emulator: copy and queue, never wait.
fn mirror_output(sink: &Mutex<Sink>, id: SessionId, bytes: &[u8]) {
    let mut sink = lock(sink);
    let Sink::Live(target) = &*sink else { return };
    let target = target.clone();
    if !target.outbox.push_output(id, bytes) {
        *sink = Sink::Overflowed(target);
    }
}

fn persist_key_for(id: SessionId) -> String {
    format!(
        "{:08x}-{:04x}-0000-{:04x}-000000000000",
        (id >> 32) as u32,
        (id >> 16) & 0xffff,
        id & 0xffff
    )
}

impl Session {
    fn snapshot_opts(&self) -> SnapshotOptions {
        SnapshotOptions {
            max_scrollback_lines: self.scrollback_lines,
        }
    }

    fn is_exited(&self) -> bool {
        matches!(*lock(&self.status), Status::Exited { .. })
    }

    fn attached_to(&self, conn: u64) -> bool {
        lock(&self.sink)
            .target()
            .is_some_and(|target| target.conn == conn)
    }

    fn send_control(&self, message: &Message) {
        if let Some(target) = lock(&self.sink).target() {
            target.outbox.push_control(message);
        }
    }

    /// Resize the PTY and the shadow emulator, send the snapshot and start the
    /// live stream, all under the emulator lock.
    fn attach(
        self: &Arc<Self>,
        conn: u64,
        outbox: &Arc<Outbox>,
        cols: u16,
        rows: u16,
    ) -> Result<(), String> {
        let dims = TermDimensions::new(usize::from(cols.max(1)), usize::from(rows.max(1)));
        self.with_emulator(|emulator, resize_pty| {
            emulator.resize(dims);
            resize_pty(dims);
            let snapshot = emulator.snapshot(&self.snapshot_opts());
            let mut sink = lock(&self.sink);
            if let Some(old) = sink.target() {
                if old.conn != conn {
                    old.outbox.push_control(&Message::Detached {
                        session: self.id,
                        reason: DetachReason::Stolen,
                    });
                }
            }
            outbox.push_control(&Message::Attached {
                session: self.id,
                cols,
                rows,
            });
            outbox.push_data(DataKind::Snapshot, self.id, &snapshot);
            if let Some(reason) = self.lost_reason() {
                outbox.push_control(&Message::ConnectionLost {
                    session: self.id,
                    reason,
                });
            } else if let Status::Exited { code, .. } = *lock(&self.status) {
                outbox.push_control(&Message::SessionExited {
                    session: self.id,
                    exit_code: code,
                });
            }
            *sink = Sink::Live(SinkTarget {
                conn,
                outbox: Arc::clone(outbox),
            });
        })?;
        if !self.is_exited() {
            self.wiggle(dims);
        }
        Ok(())
    }

    /// SIGWINCH safety net after a snapshot: shrink the PTY (not the
    /// emulator) by one row, then restore the real size a moment later.
    fn wiggle(self: &Arc<Self>, dims: TermDimensions) {
        let (small, _) = resize_wiggle_sizes(dims);
        if self.resize_pty(small).is_err() {
            return;
        }
        let session = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(WIGGLE_PAUSE).await;
            // Re-read the size: a Resize may have arrived in the meantime.
            let _ = session.with_emulator(|emulator, resize_pty| {
                resize_pty(emulator.dimensions());
            });
        });
    }

    /// Writer-side recovery after an overflow: a fresh snapshot replaces the
    /// dropped live output and the live stream restarts atomically.
    fn resync(&self, outbox: &Arc<Outbox>) {
        let _ = self.with_emulator(|emulator, _| {
            let mut sink = lock(&self.sink);
            let target = match &*sink {
                Sink::Overflowed(target) if Arc::ptr_eq(&target.outbox, outbox) => target.clone(),
                _ => return,
            };
            let snapshot = emulator.snapshot(&self.snapshot_opts());
            target
                .outbox
                .push_data(DataKind::Resync, self.id, &snapshot);
            *sink = Sink::Live(target);
        });
    }

    fn detach(&self, conn: u64) {
        let mut sink = lock(&self.sink);
        if sink.target().is_some_and(|target| target.conn == conn) {
            *sink = Sink::None;
        }
    }

    fn resize(&self, cols: u16, rows: u16) {
        let dims = TermDimensions::new(usize::from(cols.max(1)), usize::from(rows.max(1)));
        let _ = self.with_emulator(|emulator, resize_pty| {
            emulator.resize(dims);
            resize_pty(dims);
        });
    }

    /// Run `f` under the shadow emulator's lock. `resize_pty` resizes the
    /// byte source (local PTY or SSH channel) without taking any other lock.
    fn with_emulator<R>(
        &self,
        f: impl FnOnce(&mut TerminalEmulator, &dyn Fn(TermDimensions)) -> R,
    ) -> Result<R, String> {
        match &self.source {
            Source::Local { term, .. } => {
                let term = lock(term);
                term.with_emulator(|emulator| {
                    f(emulator, &|dims| {
                        let _ = term.resize_pty(dims);
                    })
                })
            }
            Source::Ssh(ssh) => {
                let mut emulator = ssh.emulator.lock().map_err(|_| "emulator poisoned")?;
                Ok(f(&mut emulator, &|dims| {
                    let _ = ssh.cmd.send(SshCmd::Resize(
                        dims.columns as u16,
                        dims.screen_lines as u16,
                    ));
                }))
            }
        }
    }

    /// Resize only the byte source (not the emulator).
    fn resize_pty(&self, dims: TermDimensions) -> Result<(), String> {
        match &self.source {
            Source::Local { term, .. } => lock(term).resize_pty(dims),
            Source::Ssh(ssh) => ssh
                .cmd
                .send(SshCmd::Resize(
                    dims.columns as u16,
                    dims.screen_lines as u16,
                ))
                .map_err(|e| e.to_string()),
        }
    }

    /// Queue user input for the byte source.
    fn send_input(&self, data: Vec<u8>) {
        match &self.source {
            Source::Local { input, .. } => {
                let _ = input.send(data);
            }
            Source::Ssh(ssh) => {
                let _ = ssh.cmd.send(SshCmd::Write(data));
            }
        }
    }

    /// Why the SSH connection behind this session is gone, if it is.
    fn lost_reason(&self) -> Option<String> {
        match &self.source {
            Source::Ssh(ssh) => lock(&ssh.lost).clone(),
            Source::Local { .. } => None,
        }
    }

    /// SSH output arrived: parse it into the shadow emulator and mirror it to
    /// the attached client, exactly like the local PTY reader does.
    fn feed_ssh(&self, bytes: &[u8]) {
        let Source::Ssh(ssh) = &self.source else {
            return;
        };
        let mut emulator = lock(&ssh.emulator);
        let mut events = emulator.feed(bytes);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            mirror_output(&self.sink, self.id, bytes)
        }));
        // DA/DSR answers of the shadow emulator go back to the remote shell.
        let replies = emulator.take_pty_output();
        drop(emulator);
        if !replies.is_empty() {
            let _ = ssh.cmd.send(SshCmd::Write(replies));
        }
        // Drained right here: nothing else reads this channel, and a full one
        // would block the emulator (see `send_event`).
        events.extend(lock(&ssh.events).try_iter());
        for event in events {
            if let Some(event) = map_event(event) {
                self.send_control(&Message::Event {
                    session: self.id,
                    event,
                });
            }
        }
    }

    /// The SSH connection is gone: the session becomes `Exited`-like until a
    /// client acks it. Never reconnects.
    fn mark_ssh_lost(&self, reason: String) {
        let Source::Ssh(ssh) = &self.source else {
            return;
        };
        {
            let mut lost = lock(&ssh.lost);
            if lost.is_some() {
                return;
            }
            *lost = Some(reason.clone());
        }
        ssh.connected.store(false, Ordering::SeqCst);
        *lock(&self.status) = Status::Exited {
            code: None,
            at: Instant::now(),
        };
        self.send_control(&Message::ConnectionLost {
            session: self.id,
            reason,
        });
    }

    fn info(&self) -> SessionInfo {
        let (cols, rows, cwd, title) = self
            .with_emulator(|emulator, _| {
                let dims = emulator.dimensions();
                let meta = emulator.metadata();
                (
                    dims.columns as u16,
                    dims.screen_lines as u16,
                    meta.cwd.clone(),
                    meta.title.clone(),
                )
            })
            .unwrap_or((0, 0, None, None));
        let exited = self.is_exited();
        let foreground = match &self.source {
            Source::Local { term, .. } if !exited => {
                lock(term).foreground_process().map(|process| process.name)
            }
            _ => None,
        };
        let (status, exit_code) = match *lock(&self.status) {
            Status::Running => (SessionStatus::Running, None),
            Status::Exited { code, .. } => (SessionStatus::Exited, code),
        };
        SessionInfo {
            id: self.id,
            pane_tag: self.pane_tag.clone(),
            label: self.label.clone(),
            cwd,
            title,
            status,
            exit_code,
            attached: lock(&self.sink).target().is_some(),
            cols,
            rows,
            foreground,
            persist_key: self.persist_key.clone(),
        }
    }

    /// Persist the scrollback so a reboot/logout falls back to replay.
    fn persist_scrollback(&self, data_dir: &Path) {
        let ansi = self
            .with_emulator(|emulator, _| emulator.serialize_scrollback(Some(self.scrollback_lines)))
            .unwrap_or_default();
        if let Err(error) = scrollback::save(data_dir, &self.persist_key, &ansi, None) {
            tracing::warn!(session = self.id, %error, "persisting scrollback failed");
        }
    }
}

/// Drop a session off the async threads: `TerminalSession::drop` kills the
/// child and joins the reader.
fn retire(session: Arc<Session>) {
    session.killed.store(true, Ordering::SeqCst);
    session.notify.notify_one();
    if let Source::Ssh(ssh) = &session.source {
        // Closing the connection ends the remote shell; the session and its
        // emulator are dropped with the last reference.
        let connection = Arc::clone(&ssh.connection);
        let ssh_id = labonair_ssh::SshSessionId::new(ssh.ssh_id.clone());
        tokio::spawn(async move {
            let _ = connection.disconnect(ssh_id).await;
        });
        return;
    }
    tokio::task::spawn_blocking(move || {
        let Source::Local { term, .. } = &session.source else {
            return;
        };
        let pid = {
            let term = lock(term);
            term.terminate(); // SIGHUP, like closing a terminal window
            term.shell_pid()
        };
        // Grace period for the shell to hang up, then SIGKILL its group.
        let mut gone = false;
        for _ in 0..40 {
            if lock(term).try_exit_code().is_some() {
                gone = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if let (false, Some(pid)) = (gone, pid) {
            unsafe {
                libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
        drop(session);
    });
}

fn map_event(event: TerminalEvent) -> Option<SessionEvent> {
    Some(match event {
        TerminalEvent::Title(title) => SessionEvent::Title { title: Some(title) },
        TerminalEvent::ResetTitle => SessionEvent::Title { title: None },
        TerminalEvent::Cwd(cwd) => SessionEvent::Cwd { cwd },
        TerminalEvent::PromptStart => SessionEvent::PromptStart,
        TerminalEvent::PromptEnd => SessionEvent::PromptEnd,
        TerminalEvent::CommandStart(command) => SessionEvent::CommandStart { command },
        TerminalEvent::CommandFinished(exit_code) => SessionEvent::CommandFinished { exit_code },
        TerminalEvent::Bell => SessionEvent::Bell,
        _ => return None,
    })
}

/// Per-session task: drains the emulator's event channel (the reader blocks on
/// a full channel) and turns the end of the PTY into `Exited`.
async fn session_task(state: Arc<State>, session: Arc<Session>) {
    let Source::Local { term, .. } = &session.source else {
        return;
    };
    loop {
        session.notify.notified().await;
        if session.killed.load(Ordering::SeqCst) {
            return;
        }
        let events = lock(term).drain_events();
        let mut exited = false;
        for event in events {
            if matches!(event, TerminalEvent::Exit | TerminalEvent::ChildExit(_)) {
                exited = true;
            } else if let Some(event) = map_event(event) {
                session.send_control(&Message::Event {
                    session: session.id,
                    event,
                });
            }
        }
        if !exited {
            continue;
        }
        // EOF can arrive a moment before the child is reapable.
        let mut code = None;
        for _ in 0..40 {
            code = lock(term).try_exit_code();
            if code.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        *lock(&session.status) = Status::Exited {
            code,
            at: Instant::now(),
        };
        session.send_control(&Message::SessionExited {
            session: session.id,
            exit_code: code,
        });
        state.changed.notify_one();
        return;
    }
}

/// Coalesce queued input and write it on a dedicated thread, so a child that
/// stops reading can only stall its own session.
fn spawn_input_thread(
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
) -> std::io::Result<std::sync::mpsc::Sender<Vec<u8>>> {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::Builder::new()
        .name("labonair-host-input".into())
        .stack_size(128 * 1024)
        .spawn(move || {
            while let Ok(mut bytes) = rx.recv() {
                while let Ok(more) = rx.try_recv() {
                    bytes.extend_from_slice(&more);
                }
                let mut writer = lock(&writer);
                if writer
                    .write_all(&bytes)
                    .and_then(|()| writer.flush())
                    .is_err()
                {
                    return;
                }
            }
        })?;
    Ok(tx)
}

// ── Server state ─────────────────────────────────────────────────────────────

struct State {
    cfg: ServerConfig,
    paths: HostPaths,
    sessions: Mutex<HashMap<SessionId, Arc<Session>>>,
    next_id: AtomicU64,
    next_conn: AtomicU64,
    conns: AtomicUsize,
    scrollback_lines: AtomicUsize,
    /// The SSH / SFTP / transfer stack, when the host was configured with one.
    ssh: Option<Arc<ssh_host::SshRuntime>>,
    /// SSH session id (the app's tab id) -> wire id of its hosted session.
    ssh_index: Mutex<HashMap<String, SessionId>>,
    /// Pokes the idle/expiry loop when sessions, clients or jobs change.
    changed: Notify,
    stop_tx: watch::Sender<Option<ExitReason>>,
    /// Set after the final `Detached` frames are queued: connections flush
    /// their outbox and end.
    closing_tx: watch::Sender<bool>,
}

impl State {
    fn session(&self, id: SessionId) -> Option<Arc<Session>> {
        lock(&self.sessions).get(&id).cloned()
    }

    fn create_session(
        self: &Arc<Self>,
        id: Option<SessionId>,
        options: CreateOptions,
        colors: Option<WireColors>,
    ) -> Result<SessionId, String> {
        let cols = if options.cols == 0 {
            DEFAULT_COLS
        } else {
            options.cols
        };
        let rows = if options.rows == 0 {
            DEFAULT_ROWS
        } else {
            options.rows
        };
        let id = match id {
            Some(id) if lock(&self.sessions).contains_key(&id) => {
                return Err(format!("session {id} already exists"));
            }
            Some(id) => id,
            None => self.next_id.fetch_add(1, Ordering::SeqCst),
        };
        let scrollback_lines = options
            .scrollback
            .map_or_else(
                || self.scrollback_lines.load(Ordering::SeqCst),
                |n| n as usize,
            )
            .max(1);
        let persist_key = options
            .persist_key
            .clone()
            .unwrap_or_else(|| persist_key_for(id));
        let colors = colors
            .as_ref()
            .map_or_else(TerminalColors::xterm_default, Into::into);

        let sink = Arc::new(Mutex::new(Sink::None));
        let tap_sink = Arc::clone(&sink);
        let tap: OutputTap = Box::new(move |_emulator, bytes| {
            // Runs on the PTY reader thread under the emulator lock: copy and
            // queue, never wait. A panic here must not kill the reader.
            let _ = catch_unwind(AssertUnwindSafe(|| mirror_output(&tap_sink, id, bytes)));
        });
        let notify = Arc::new(Notify::new());
        let term = TerminalSession::spawn_with_tap(
            colors,
            TermDimensions::new(usize::from(cols), usize::from(rows)),
            SessionOptions {
                shell: options.shell,
                args: options.args,
                working_directory: options.cwd,
                env: options.env,
                clear_env: true,
                blocks: options.blocks,
                startup_command: options.startup_command,
                scrollback: Some(scrollback_lines),
                ..SessionOptions::default()
            },
            Arc::clone(&notify),
            Some(tap),
        )?;
        let input = spawn_input_thread(term.writer()).map_err(|e| e.to_string())?;
        let session = Arc::new(Session {
            id,
            pane_tag: options.pane_tag,
            label: options.label,
            persist_key,
            scrollback_lines,
            source: Source::Local {
                term: Mutex::new(term),
                input,
            },
            sink,
            status: Mutex::new(Status::Running),
            killed: AtomicBool::new(false),
            notify,
        });
        lock(&self.sessions).insert(id, Arc::clone(&session));
        tokio::spawn(session_task(Arc::clone(self), session));
        self.changed.notify_one();
        Ok(id)
    }

    fn remove_session(&self, id: SessionId) -> Option<Arc<Session>> {
        let session = lock(&self.sessions).remove(&id);
        if let Some(Source::Ssh(ssh)) = session.as_ref().map(|s| &s.source) {
            let mut index = lock(&self.ssh_index);
            if index.get(&ssh.ssh_id) == Some(&id) {
                index.remove(&ssh.ssh_id);
            }
            if let Some(runtime) = &self.ssh {
                runtime.forget_session(&ssh.ssh_id);
            }
        }
        self.changed.notify_one();
        session
    }

    /// `true` while a background job (including transfers) keeps us alive.
    fn jobs_active(&self) -> bool {
        (self.cfg.jobs_active)() || self.ssh.as_ref().is_some_and(|ssh| ssh.jobs_active())
    }

    /// Stop everything: tell attached clients, kill the sessions.
    fn kill_all(&self, reason: DetachReason) {
        lock(&self.ssh_index).clear();
        let sessions: Vec<_> = lock(&self.sessions).drain().map(|(_, s)| s).collect();
        for session in sessions {
            session.send_control(&Message::Detached {
                session: session.id,
                reason,
            });
            retire(session);
        }
    }
}

// ── Connections ──────────────────────────────────────────────────────────────

struct Conn {
    id: u64,
    state: Arc<State>,
    outbox: Arc<Outbox>,
    compatible: bool,
    attached: HashSet<SessionId>,
}

impl Conn {
    fn error(&self, req_id: Option<u64>, session: Option<SessionId>, message: impl Into<String>) {
        self.outbox.push_control(&Message::Error {
            req_id,
            session,
            message: message.into(),
        });
    }

    fn hello(&mut self, hello: Hello) {
        self.compatible = hello.proto_version >= MIN_COMPAT && PROTO_VERSION >= hello.min_compat;
        if let Some(lines) = hello.scrollback_lines {
            self.state
                .scrollback_lines
                .store((lines as usize).max(1), Ordering::SeqCst);
        }
        self.outbox.push_control(&Message::HelloAck(HelloAck {
            proto_version: PROTO_VERSION,
            min_compat: MIN_COMPAT,
            build_id: self.state.cfg.build_id.clone(),
            pid: std::process::id(),
            compatible: self.compatible,
            session_count: lock(&self.state.sessions).len() as u32,
        }));
    }

    fn handle(&mut self, frame: Frame) {
        let outcome = catch_unwind(AssertUnwindSafe(|| match frame {
            Frame::Control(message) => self.handle_message(message),
            Frame::Data {
                kind: DataKind::Input,
                session,
                data,
            } => {
                if !self.compatible {
                    return;
                }
                if let Some(s) = self
                    .state
                    .session(session)
                    .filter(|s| s.attached_to(self.id))
                {
                    s.send_input(data);
                }
            }
            Frame::Data { .. } => {}
        }));
        if outcome.is_err() {
            tracing::error!("panic while handling a client frame");
            self.error(None, None, "internal error");
        }
    }

    fn handle_message(&mut self, message: Message) {
        if !self.compatible
            && !matches!(
                message,
                Message::ListSessions | Message::Kill { .. } | Message::Shutdown
            )
        {
            self.error(None, None, "incompatible protocol version");
            return;
        }
        match message {
            Message::CreateSession {
                req_id,
                id,
                options,
                colors,
            } => match self.state.create_session(id, options, colors) {
                Ok(session) => self
                    .outbox
                    .push_control(&Message::Created { req_id, session }),
                Err(message) => self.error(Some(req_id), id, message),
            },
            Message::Attach {
                session,
                cols,
                rows,
            } => match self.state.session(session) {
                None => self.error(None, Some(session), "no such session"),
                Some(s) => match s.attach(self.id, &self.outbox, cols, rows) {
                    Ok(()) => {
                        self.attached.insert(session);
                    }
                    Err(message) => self.error(None, Some(session), message),
                },
            },
            Message::Detach { session } => {
                if let Some(s) = self.state.session(session) {
                    s.detach(self.id);
                }
                self.attached.remove(&session);
            }
            Message::Resize {
                session,
                cols,
                rows,
            } => {
                if let Some(s) = self
                    .state
                    .session(session)
                    .filter(|s| s.attached_to(self.id))
                {
                    s.resize(cols, rows);
                }
            }
            Message::Kill { session } => match self.state.remove_session(session) {
                Some(s) => retire(s),
                None => self.error(None, Some(session), "no such session"),
            },
            Message::Ack { session } => match self.state.session(session) {
                Some(s) if s.is_exited() => {
                    if let Some(s) = self.state.remove_session(session) {
                        retire(s);
                    }
                }
                Some(_) => self.error(None, Some(session), "session is still running"),
                None => {}
            },
            Message::ListSessions => {
                let mut sessions: Vec<_> = lock(&self.state.sessions).values().cloned().collect();
                sessions.sort_by_key(|s| s.id);
                self.outbox.push_control(&Message::Sessions {
                    sessions: sessions.iter().map(|s| s.info()).collect(),
                });
            }
            Message::Shutdown => {
                ServerHandle {
                    state: Arc::clone(&self.state),
                }
                .stop(ExitReason::Shutdown);
            }
            Message::Rpc { req_id, call } => {
                ssh_host::spawn_rpc(&self.state, self.id, &self.outbox, req_id, call)
            }
            Message::Hello(_) => self.error(None, None, "duplicate hello"),
            // Daemon -> client messages and future variants: not ours to act on.
            _ => {}
        }
    }
}

async fn serve_connection(state: Arc<State>, stream: UnixStream) {
    let our_uid = unsafe { libc::getuid() };
    match stream.peer_cred() {
        Ok(cred) if cred.uid() == our_uid => {}
        other => {
            tracing::warn!(?other, "rejecting connection from a different user");
            return;
        }
    }
    let (mut read_half, write_half) = stream.into_split();
    let outbox = Outbox::new(state.cfg.queue_limit);
    let writer = tokio::spawn(writer_loop(
        write_half,
        Arc::clone(&outbox),
        Arc::clone(&state),
    ));
    let mut conn = Conn {
        id: state.next_conn.fetch_add(1, Ordering::SeqCst),
        state: Arc::clone(&state),
        outbox: Arc::clone(&outbox),
        compatible: false,
        attached: HashSet::new(),
    };
    let mut closing_rx = state.closing_tx.subscribe();

    let first = tokio::time::timeout(HELLO_TIMEOUT, read_frame(&mut read_half)).await;
    if let Ok(Ok(Some(Frame::Control(Message::Hello(hello))))) = first {
        conn.hello(hello);
        loop {
            tokio::select! {
                frame = read_frame(&mut read_half) => match frame {
                    Ok(Some(frame)) => conn.handle(frame),
                    Ok(None) => break,
                    Err(error) => {
                        tracing::debug!(%error, "closing connection");
                        break;
                    }
                },
                _ = wait_closing(&mut closing_rx) => break,
            }
        }
    } else {
        tracing::debug!("connection did not open with a valid hello");
    }

    for id in &conn.attached {
        if let Some(session) = state.session(*id) {
            session.detach(conn.id);
        }
    }
    if let Some(ssh) = &state.ssh {
        ssh.unsubscribe(conn.id);
    }
    outbox.close();
    let _ = writer.await;
}

// ── Server ───────────────────────────────────────────────────────────────────

// `wait_for` (unlike `changed`) also sees a value set before we subscribed.
async fn wait_stop(rx: &mut watch::Receiver<Option<ExitReason>>) -> ExitReason {
    rx.wait_for(Option::is_some)
        .await
        .map_or(ExitReason::Shutdown, |reason| {
            reason.unwrap_or(ExitReason::Shutdown)
        })
}

async fn wait_closing(rx: &mut watch::Receiver<bool>) {
    let _ = rx.wait_for(|closing| *closing).await;
}

/// A bound, locked, not-yet-running host.
pub struct Server {
    state: Arc<State>,
    listener: UnixListener,
    _lock: InstanceLock,
}

impl Server {
    /// Take the single-instance lock, clean a stale socket and listen. Must be
    /// called inside a tokio runtime.
    pub fn bind(mut cfg: ServerConfig) -> Result<Self, ServerError> {
        let paths = HostPaths::new(&cfg.data_dir);
        let ssh = cfg
            .services
            .take()
            .map(|services| Arc::new(ssh_host::SshRuntime::new(services)));
        ensure_private_dir(&paths.dir)?;
        let lock = InstanceLock::acquire(&paths.lock)?;
        // We hold the lock, so any socket file left behind is stale.
        if let Some(parent) = paths.socket.parent() {
            ensure_private_dir(parent)?;
        }
        match std::fs::remove_file(&paths.socket) {
            Ok(()) => tracing::info!("removed stale socket"),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let listener = UnixListener::bind(&paths.socket)?;
        std::fs::set_permissions(&paths.socket, std::fs::Permissions::from_mode(0o600))?;
        let (stop_tx, _) = watch::channel(None);
        let (closing_tx, _) = watch::channel(false);
        let state = Arc::new(State {
            scrollback_lines: AtomicUsize::new(cfg.scrollback_lines),
            ssh,
            ssh_index: Mutex::new(HashMap::new()),
            cfg,
            paths,
            sessions: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            next_conn: AtomicU64::new(1),
            conns: AtomicUsize::new(0),
            changed: Notify::new(),
            stop_tx,
            closing_tx,
        });
        if let Some(ssh) = &state.ssh {
            ssh.start(Arc::downgrade(&state));
        }
        Ok(Self {
            state,
            listener,
            _lock: lock,
        })
    }

    pub fn paths(&self) -> &HostPaths {
        &self.state.paths
    }

    pub fn handle(&self) -> ServerHandle {
        ServerHandle {
            state: Arc::clone(&self.state),
        }
    }

    /// Serve until idle, shut down or signalled.
    pub async fn run(self) -> ExitReason {
        let state = Arc::clone(&self.state);
        let mut connections = JoinSet::new();
        let mut stop_rx = state.stop_tx.subscribe();
        let idle = idle_loop(Arc::clone(&state));
        tokio::pin!(idle);
        let reason = loop {
            tokio::select! {
                accepted = self.listener.accept() => match accepted {
                    Ok((stream, _)) => {
                        state.conns.fetch_add(1, Ordering::SeqCst);
                        let conn_state = Arc::clone(&state);
                        connections.spawn(async move {
                            serve_connection(Arc::clone(&conn_state), stream).await;
                            conn_state.conns.fetch_sub(1, Ordering::SeqCst);
                            conn_state.changed.notify_one();
                        });
                    }
                    Err(error) => {
                        tracing::warn!(%error, "accept failed");
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                },
                _ = &mut idle => break ExitReason::Idle,
                reason = wait_stop(&mut stop_rx) => break reason,
            }
        };

        if reason == ExitReason::Signal {
            let sessions: Vec<_> = lock(&state.sessions).values().cloned().collect();
            for session in sessions {
                session.persist_scrollback(&state.cfg.data_dir);
            }
        }
        state.kill_all(DetachReason::Shutdown);
        // Connections see the stop flag, flush their queues and end.
        state.closing_tx.send_replace(true);
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            while connections.join_next().await.is_some() {}
        })
        .await;
        let _ = std::fs::remove_file(&state.paths.socket);
        reason
    }
}

/// Resolves when the daemon has been idle for `idle_timeout`; also forgets
/// exited sessions nobody acked within `exited_ttl`. Event driven: it sleeps
/// until the nearest deadline or the next state change, never polls (except
/// while the `jobs_active` hook reports work).
async fn idle_loop(state: Arc<State>) {
    let mut idle_since: Option<Instant> = None;
    loop {
        let now = Instant::now();
        let mut deadline: Option<Instant> = None;
        let mut soonest = |candidate: Instant| {
            deadline = Some(deadline.map_or(candidate, |d| d.min(candidate)));
        };

        let expired: Vec<_> = lock(&state.sessions)
            .values()
            .filter_map(|s| match *lock(&s.status) {
                Status::Exited { at, .. } => Some((s.id, at + state.cfg.exited_ttl)),
                Status::Running => None,
            })
            .collect();
        for (id, expiry) in expired {
            if expiry <= now {
                if let Some(session) = state.remove_session(id) {
                    retire(session);
                }
            } else {
                soonest(expiry);
            }
        }

        let jobs = state.jobs_active();
        let busy =
            !lock(&state.sessions).is_empty() || state.conns.load(Ordering::SeqCst) > 0 || jobs;
        if busy {
            idle_since = None;
            if jobs {
                soonest(now + JOBS_POLL.min(state.cfg.idle_timeout));
            }
        } else {
            let since = *idle_since.get_or_insert(now);
            let exit_at = since + state.cfg.idle_timeout;
            if now >= exit_at {
                return;
            }
            soonest(exit_at);
        }

        match deadline {
            Some(deadline) => {
                tokio::select! {
                    _ = tokio::time::sleep_until(deadline.into()) => {}
                    _ = state.changed.notified() => {}
                }
            }
            None => state.changed.notified().await,
        }
    }
}
