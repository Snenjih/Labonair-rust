//! Client side of the detached session host (ADR 0009): ONE socket connection
//! per app process, shared by every hosted session.
//!
//! The client is deliberately synchronous and GPUI-free: a reader thread and a
//! writer thread own the socket (idle = both blocked, zero wakeups), callers
//! never wait on the network except for the request/response calls
//! ([`HostClient::create_session`], [`HostClient::list_sessions`]) which are
//! bounded by [`REQUEST_TIMEOUT`]. Input, resize and detach are fire-and-forget
//! frames pushed onto the writer's queue.
//!
//! Output is routed per session to a [`SessionSink`] registered with
//! [`HostClient::register`]; the app wires the sink to a `RemoteFeed`. When the
//! connection drops every sink is told ([`SessionSink::disconnected`]); the
//! sinks stay registered so [`HostClient::reconnect`] can resume them.

use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::protocol::{
    control_frame, data_frame, read_frame_blocking, CreateOptions, DataKind, DetachReason, Frame,
    Hello, HelloAck, Message, SessionEvent, SessionId, SessionInfo, WireColors,
};
use crate::server::HostPaths;

/// Upper bound for one request/response round trip.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Receives everything the daemon sends for one session. Called on the
/// client's reader thread: implementations must be quick and must not block.
pub trait SessionSink: Send + Sync {
    /// The daemon attached this client; the snapshot follows.
    fn attached(&self, cols: u16, rows: u16);
    /// Emulator state as escape sequences: after [`Self::attached`], or
    /// (`resync`) after the client fell behind and live output was dropped.
    fn snapshot(&self, data: &[u8], resync: bool);
    /// Live PTY output.
    fn output(&self, data: &[u8]);
    /// The child exited; the session stays until acknowledged.
    fn exited(&self, exit_code: Option<i32>);
    /// The daemon stopped sending output to this client.
    fn detached(&self, reason: DetachReason);
    /// The daemon rejected a request for this session (e.g. it is gone).
    fn error(&self, message: &str);
    /// The connection to the daemon was lost.
    fn disconnected(&self);
    /// Semantic event mirrored from the daemon's shadow emulator. The app's
    /// own emulator derives the same from the byte stream, so the default is
    /// to ignore it.
    fn event(&self, _event: SessionEvent) {}
}

/// State of the connection, for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostStatus {
    Connected,
    /// A daemon is running but speaks an incompatible protocol; the UI offers
    /// a restart (`sessions` sessions end).
    Incompatible {
        sessions: u32,
        build_id: String,
    },
    Disconnected,
}

#[derive(Debug)]
pub enum ClientError {
    NotConnected,
    Timeout,
    Incompatible {
        sessions: u32,
        build_id: String,
    },
    /// The daemon answered with an error.
    Host(String),
    Io(io::Error),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => write!(f, "not connected to the background session host"),
            Self::Timeout => write!(f, "the background session host did not answer in time"),
            Self::Incompatible { sessions, build_id } => write!(
                f,
                "the running background session host ({build_id}, {sessions} sessions) is \
                 incompatible with this version; restart it to continue"
            ),
            Self::Host(message) => write!(f, "{message}"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<io::Error> for ClientError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

struct Conn {
    tx: Sender<Vec<u8>>,
    stream: UnixStream,
}

struct State {
    conn: Option<Conn>,
    status: HostStatus,
    generation: u64,
    sinks: HashMap<SessionId, Arc<dyn SessionSink>>,
    creates: HashMap<u64, SyncSender<Result<SessionId, String>>>,
    lists: VecDeque<SyncSender<Vec<SessionInfo>>>,
    cached: Option<(Instant, Vec<SessionInfo>)>,
}

struct Inner {
    paths: HostPaths,
    data_dir: PathBuf,
    hello: Hello,
    next_req: AtomicU64,
    state: Mutex<State>,
}

/// Handle to the daemon connection. Cheap to clone; all clones share one
/// socket.
#[derive(Clone)]
pub struct HostClient {
    inner: Arc<Inner>,
}

impl HostClient {
    /// A client for the daemon of `data_dir`. Does not connect; call
    /// [`Self::reconnect`].
    pub fn new(data_dir: PathBuf, hello: Hello) -> Self {
        Self {
            inner: Arc::new(Inner {
                paths: HostPaths::new(&data_dir),
                data_dir,
                hello,
                next_req: AtomicU64::new(1),
                state: Mutex::new(State {
                    conn: None,
                    status: HostStatus::Disconnected,
                    generation: 0,
                    sinks: HashMap::new(),
                    creates: HashMap::new(),
                    lists: VecDeque::new(),
                    cached: None,
                }),
            }),
        }
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.inner.data_dir
    }

    pub fn status(&self) -> HostStatus {
        lock(&self.inner.state).status.clone()
    }

    /// Whether a connection exists (a compatible one or an incompatible one).
    pub fn is_connected(&self) -> bool {
        lock(&self.inner.state).conn.is_some()
    }

    /// (Re)open the connection to an already running daemon and say hello.
    /// Registered sinks are kept. A no-op while connected.
    pub fn reconnect(&self) -> Result<(), ClientError> {
        if self.is_connected() {
            return Ok(());
        }
        let mut stream = UnixStream::connect(&self.inner.paths.socket)?;
        stream.set_read_timeout(Some(HELLO_TIMEOUT))?;
        stream.write_all(
            &control_frame(&Message::Hello(self.inner.hello.clone()))
                .map_err(|e| ClientError::Host(e.to_string()))?,
        )?;
        let ack: HelloAck = match read_frame_blocking(&mut stream) {
            Ok(Some(Frame::Control(Message::HelloAck(ack)))) => ack,
            Ok(_) => return Err(ClientError::Host("unexpected handshake reply".into())),
            Err(error) => return Err(ClientError::Host(error.to_string())),
        };
        stream.set_read_timeout(None)?;

        let reader = stream.try_clone()?;
        let writer = stream.try_clone()?;
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let generation = {
            let mut state = lock(&self.inner.state);
            state.generation += 1;
            state.status = if ack.compatible {
                HostStatus::Connected
            } else {
                HostStatus::Incompatible {
                    sessions: ack.session_count,
                    build_id: ack.build_id.clone(),
                }
            };
            state.conn = Some(Conn { tx, stream });
            state.generation
        };
        let inner = Arc::clone(&self.inner);
        std::thread::Builder::new()
            .name("labonair-host-read".into())
            .spawn(move || reader_loop(inner, reader, generation))?;
        std::thread::Builder::new()
            .name("labonair-host-write".into())
            .spawn(move || writer_loop(writer, rx))?;
        Ok(())
    }

    fn send_raw(&self, frame: Vec<u8>) -> Result<(), ClientError> {
        let state = lock(&self.inner.state);
        let conn = state.conn.as_ref().ok_or(ClientError::NotConnected)?;
        conn.tx.send(frame).map_err(|_| ClientError::NotConnected)
    }

    fn send(&self, message: &Message) -> Result<(), ClientError> {
        let frame = control_frame(message).map_err(|e| ClientError::Host(e.to_string()))?;
        self.send_raw(frame)
    }

    fn require_compatible(&self) -> Result<(), ClientError> {
        match lock(&self.inner.state).status.clone() {
            HostStatus::Incompatible { sessions, build_id } => {
                Err(ClientError::Incompatible { sessions, build_id })
            }
            HostStatus::Disconnected => Err(ClientError::NotConnected),
            HostStatus::Connected => Ok(()),
        }
    }

    /// Spawn a session. The creating client is not attached yet: register a
    /// sink, then call [`Self::attach`].
    pub fn create_session(
        &self,
        options: CreateOptions,
        colors: Option<WireColors>,
    ) -> Result<SessionId, ClientError> {
        self.require_compatible()?;
        let req_id = self.inner.next_req.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::sync_channel(1);
        lock(&self.inner.state).creates.insert(req_id, tx);
        let sent = self.send(&Message::CreateSession {
            req_id,
            id: None,
            options,
            colors,
        });
        if let Err(error) = sent {
            lock(&self.inner.state).creates.remove(&req_id);
            return Err(error);
        }
        match rx.recv_timeout(REQUEST_TIMEOUT) {
            Ok(Ok(id)) => Ok(id),
            Ok(Err(message)) => Err(ClientError::Host(message)),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                lock(&self.inner.state).creates.remove(&req_id);
                Err(ClientError::Timeout)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(ClientError::NotConnected),
        }
    }

    /// All sessions the daemon knows. Works against an incompatible daemon too
    /// (the app needs the count to offer a restart).
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>, ClientError> {
        let (tx, rx) = mpsc::sync_channel(1);
        lock(&self.inner.state).lists.push_back(tx);
        self.send(&Message::ListSessions)?;
        match rx.recv_timeout(REQUEST_TIMEOUT) {
            Ok(sessions) => Ok(sessions),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(ClientError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(ClientError::NotConnected),
        }
    }

    /// [`Self::list_sessions`] answered from a cache no older than `max_age`;
    /// keeps per-pane polling (foreground process) to one round trip per
    /// interval for the whole app. Falls back to the stale cache on error.
    pub fn list_sessions_cached(&self, max_age: Duration) -> Vec<SessionInfo> {
        if let Some((at, sessions)) = &lock(&self.inner.state).cached {
            if at.elapsed() <= max_age {
                return sessions.clone();
            }
        }
        match self.list_sessions() {
            Ok(sessions) => sessions,
            Err(_) => lock(&self.inner.state)
                .cached
                .as_ref()
                .map(|(_, sessions)| sessions.clone())
                .unwrap_or_default(),
        }
    }

    /// Route daemon output for `session` to `sink`. Register before attaching.
    pub fn register(&self, session: SessionId, sink: Arc<dyn SessionSink>) {
        lock(&self.inner.state).sinks.insert(session, sink);
    }

    pub fn unregister(&self, session: SessionId) {
        lock(&self.inner.state).sinks.remove(&session);
    }

    /// Attach (stealing from another client if needed). The daemon answers
    /// with `attached`, `snapshot`, then live output on the registered sink.
    pub fn attach(&self, session: SessionId, cols: u16, rows: u16) -> Result<(), ClientError> {
        self.require_compatible()?;
        self.send(&Message::Attach {
            session,
            cols,
            rows,
        })
    }

    /// Stop receiving output; the session keeps running.
    pub fn detach(&self, session: SessionId) {
        let _ = self.send(&Message::Detach { session });
    }

    /// User input for the session's PTY.
    pub fn input(&self, session: SessionId, bytes: &[u8]) {
        let _ = self.send_raw(data_frame(DataKind::Input, session, bytes));
    }

    pub fn resize(&self, session: SessionId, cols: u16, rows: u16) {
        let _ = self.send(&Message::Resize {
            session,
            cols,
            rows,
        });
    }

    /// Terminate the session and forget it (closing a tab).
    pub fn kill(&self, session: SessionId) {
        let _ = self.send(&Message::Kill { session });
    }

    /// Acknowledge an exited session so the daemon forgets it.
    pub fn ack(&self, session: SessionId) {
        let _ = self.send(&Message::Ack { session });
    }

    /// Kill every session and stop the daemon.
    pub fn shutdown(&self) {
        let _ = self.send(&Message::Shutdown);
    }

    /// Close the socket (the daemon detaches our sessions, which keep running).
    pub fn disconnect(&self) {
        let conn = lock(&self.inner.state).conn.take();
        if let Some(conn) = conn {
            let _ = conn.stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

fn writer_loop(mut stream: UnixStream, rx: Receiver<Vec<u8>>) {
    while let Ok(mut buf) = rx.recv() {
        // Coalesce whatever queued up meanwhile into one write.
        while let Ok(more) = rx.try_recv() {
            buf.extend_from_slice(&more);
        }
        if stream
            .write_all(&buf)
            .and_then(|()| stream.flush())
            .is_err()
        {
            break;
        }
    }
    // Wakes the reader so the connection is torn down on either side's failure.
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

fn reader_loop(inner: Arc<Inner>, mut stream: UnixStream, generation: u64) {
    loop {
        match read_frame_blocking(&mut stream) {
            Ok(Some(frame)) => dispatch(&inner, frame),
            Ok(None) => break,
            Err(error) => {
                tracing::debug!(%error, "host connection closed");
                break;
            }
        }
    }
    connection_lost(&inner, generation);
}

fn sink_for(inner: &Inner, session: SessionId) -> Option<Arc<dyn SessionSink>> {
    lock(&inner.state).sinks.get(&session).cloned()
}

fn dispatch(inner: &Inner, frame: Frame) {
    match frame {
        Frame::Data {
            kind,
            session,
            data,
        } => {
            let Some(sink) = sink_for(inner, session) else {
                return;
            };
            match kind {
                DataKind::Output => sink.output(&data),
                DataKind::Snapshot => sink.snapshot(&data, false),
                DataKind::Resync => sink.snapshot(&data, true),
                DataKind::Input => {}
            }
        }
        Frame::Control(message) => match message {
            Message::Created { req_id, session } => {
                if let Some(tx) = lock(&inner.state).creates.remove(&req_id) {
                    let _ = tx.send(Ok(session));
                }
            }
            Message::Error {
                req_id: Some(req_id),
                message,
                ..
            } => {
                if let Some(tx) = lock(&inner.state).creates.remove(&req_id) {
                    let _ = tx.send(Err(message));
                }
            }
            Message::Error {
                session: Some(session),
                message,
                ..
            } => {
                if let Some(sink) = sink_for(inner, session) {
                    sink.error(&message);
                }
            }
            Message::Error { message, .. } => {
                tracing::warn!(%message, "background host reported an error");
            }
            Message::Sessions { sessions } => {
                let waiter = {
                    let mut state = lock(&inner.state);
                    state.cached = Some((Instant::now(), sessions.clone()));
                    state.lists.pop_front()
                };
                if let Some(tx) = waiter {
                    let _ = tx.send(sessions);
                }
            }
            Message::Attached {
                session,
                cols,
                rows,
            } => {
                if let Some(sink) = sink_for(inner, session) {
                    sink.attached(cols, rows);
                }
            }
            Message::Event { session, event } => {
                if let Some(sink) = sink_for(inner, session) {
                    sink.event(event);
                }
            }
            Message::SessionExited { session, exit_code } => {
                if let Some(sink) = sink_for(inner, session) {
                    sink.exited(exit_code);
                }
            }
            Message::Detached { session, reason } => {
                if let Some(sink) = sink_for(inner, session) {
                    sink.detached(reason);
                }
            }
            _ => {}
        },
    }
}

fn connection_lost(inner: &Inner, generation: u64) {
    let sinks: Vec<_> = {
        let mut state = lock(&inner.state);
        if state.generation != generation {
            return; // a newer connection already replaced this one
        }
        state.conn = None;
        state.status = HostStatus::Disconnected;
        // Dropping the senders fails every waiting request.
        state.creates.clear();
        state.lists.clear();
        state.sinks.values().cloned().collect()
    };
    for sink in sinks {
        sink.disconnected();
    }
}
