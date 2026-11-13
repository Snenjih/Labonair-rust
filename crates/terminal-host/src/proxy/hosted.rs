//! The app-side end of one daemon-hosted SSH session: receives everything the
//! daemon sends for it (PTY bytes, steal / loss notices) on the client's
//! reader thread and hands it to whoever shows the session: a hosted terminal
//! pane, the output sink the connect call came with, and the MCP agent tap.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use labonair_events::EventBus;
use labonair_ssh::{SharedSshEventSink, SshSessionEvent};
use tokio::sync::broadcast;

use crate::client::{HostClient, SessionSink};
use crate::protocol::{DetachReason, SessionId, WireColors};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Splits a byte stream into UTF-8 text: the longest valid prefix goes out,
/// an incomplete trailing sequence waits for its rest, and genuinely invalid
/// bytes become U+FFFD (the in-process transport does the same).
#[derive(Default)]
struct Utf8Carry(Vec<u8>);

impl Utf8Carry {
    fn push(&mut self, bytes: &[u8]) -> String {
        self.0.extend_from_slice(bytes);
        let cut = match std::str::from_utf8(&self.0) {
            Ok(_) => self.0.len(),
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            // Invalid bytes in the middle: lossy-convert everything.
            Err(_) => self.0.len(),
        };
        let text = String::from_utf8_lossy(&self.0[..cut]).into_owned();
        self.0.drain(..cut);
        text
    }
}

#[derive(Default)]
struct Targets {
    /// The pane showing the session. Wins over `events`.
    pane: Option<Arc<dyn SessionSink>>,
    /// The output sink the connect call came with (SFTP browsers ignore it).
    events: Option<SharedSshEventSink>,
    size: (u16, u16),
    pane_tag: Option<String>,
    colors: Option<WireColors>,
    carry: Utf8Carry,
}

pub(crate) struct HostedSsh {
    pub(crate) ssh_id: String,
    client: HostClient,
    bus: EventBus,
    /// Wire id of the daemon session; 0 until the connect answered.
    wire: AtomicU64,
    /// The tab was closed; a connect still in flight must not leave a session.
    closed: AtomicBool,
    /// The session's connection was reported lost (pane close => `Ack`).
    lost: AtomicBool,
    targets: Mutex<Targets>,
    pub(crate) tap: broadcast::Sender<String>,
}

impl HostedSsh {
    pub(crate) fn new(client: HostClient, bus: EventBus, ssh_id: String) -> Arc<Self> {
        let (tap, _) = broadcast::channel(256);
        Arc::new(Self {
            ssh_id,
            client,
            bus,
            wire: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            lost: AtomicBool::new(false),
            targets: Mutex::new(Targets {
                size: (80, 24),
                ..Targets::default()
            }),
            tap,
        })
    }

    pub(crate) fn wire(&self) -> Option<SessionId> {
        match self.wire.load(Ordering::SeqCst) {
            0 => None,
            id => Some(id),
        }
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub(crate) fn is_lost(&self) -> bool {
        self.lost.load(Ordering::SeqCst)
    }

    pub(crate) fn size(&self) -> (u16, u16) {
        lock(&self.targets).size
    }

    pub(crate) fn set_size(&self, cols: u16, rows: u16) {
        lock(&self.targets).size = (cols, rows);
    }

    pub(crate) fn pane_tag(&self) -> Option<String> {
        lock(&self.targets).pane_tag.clone()
    }

    pub(crate) fn colors(&self) -> Option<WireColors> {
        lock(&self.targets).colors.clone()
    }

    pub(crate) fn has_pane(&self) -> bool {
        lock(&self.targets).pane.is_some()
    }

    pub(crate) fn bind_pane(
        &self,
        pane: Arc<dyn SessionSink>,
        pane_tag: Option<String>,
        colors: Option<WireColors>,
        size: (u16, u16),
    ) {
        let mut targets = lock(&self.targets);
        targets.pane = Some(pane);
        targets.pane_tag = pane_tag;
        targets.colors = colors;
        targets.size = size;
    }

    pub(crate) fn set_events(&self, events: SharedSshEventSink) {
        lock(&self.targets).events = Some(events);
    }

    /// Point the session at the daemon's `wire` session: the sink is
    /// registered before anything can be attached, so nothing arrives unrouted.
    pub(crate) fn set_wire(self: &Arc<Self>, wire: SessionId) {
        let old = self.wire.swap(wire, Ordering::SeqCst);
        if old != 0 && old != wire {
            self.client.unregister(old);
        }
        self.lost.store(false, Ordering::SeqCst);
        self.client
            .register(wire, Arc::clone(self) as Arc<dyn SessionSink>);
    }

    /// Forget the wire session (disconnect / close): returns what it was.
    pub(crate) fn clear_wire(&self) -> Option<SessionId> {
        self.closed.store(true, Ordering::SeqCst);
        let wire = self.wire.swap(0, Ordering::SeqCst);
        (wire != 0).then(|| {
            self.client.unregister(wire);
            wire
        })
    }

    /// A retry reuses the object: it is open again.
    pub(crate) fn reopen(&self) {
        self.closed.store(false, Ordering::SeqCst);
    }

    fn pane(&self) -> Option<Arc<dyn SessionSink>> {
        lock(&self.targets).pane.clone()
    }

    fn text(&self, bytes: &[u8]) -> String {
        lock(&self.targets).carry.push(bytes)
    }

    fn to_events(&self, data: String) {
        let events = lock(&self.targets).events.clone();
        if let Some(events) = events {
            let _ = events.send(SshSessionEvent::Output { data });
        }
    }
}

impl SessionSink for HostedSsh {
    fn attached(&self, cols: u16, rows: u16) {
        if let Some(pane) = self.pane() {
            pane.attached(cols, rows);
        }
    }

    fn snapshot(&self, data: &[u8], resync: bool) {
        if let Some(pane) = self.pane() {
            pane.snapshot(data, resync);
        } else {
            // History for a sink that only knows text output; not for the tap.
            self.to_events(String::from_utf8_lossy(data).into_owned());
        }
    }

    fn output(&self, data: &[u8]) {
        let pane = self.pane();
        let tapped = self.tap.receiver_count() > 0;
        if tapped || pane.is_none() {
            let text = self.text(data);
            if !text.is_empty() {
                if tapped {
                    let _ = self.tap.send(text.clone());
                }
                if pane.is_none() {
                    self.to_events(text);
                }
            }
        }
        if let Some(pane) = pane {
            pane.output(data);
        }
    }

    fn exited(&self, exit_code: Option<i32>) {
        if let Some(pane) = self.pane() {
            pane.exited(exit_code);
        }
    }

    fn detached(&self, reason: DetachReason) {
        if let Some(pane) = self.pane() {
            pane.detached(reason);
        }
    }

    fn error(&self, message: &str) {
        if let Some(pane) = self.pane() {
            pane.error(message);
        }
    }

    fn disconnected(&self) {
        if let Some(pane) = self.pane() {
            pane.disconnected();
        }
        // The same signal the in-process transport gives when the connection
        // drops, so every surface reacts the way it already does.
        let _ = self.bus.emit(
            "ssh_connection_lost",
            serde_json::json!({
                "session_id": self.ssh_id,
                "reason": "Lost the connection to the background session host",
            }),
        );
    }

    fn connection_lost(&self, reason: &str) {
        self.lost.store(true, Ordering::SeqCst);
        if let Some(pane) = self.pane() {
            pane.connection_lost(reason);
        }
    }
}
