//! In-process tests of the SSH / SFTP / transfer side of the detached session
//! host: the real server on a temp socket with fake services injected through
//! the `HostServices` seam (plus one run against the concrete stack).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use labonair_errors::LabonairError;
use labonair_events::EventBus;
use labonair_sftp::{
    BoxFuture, RemoteEntry, SftpBrowserService, SftpSessionHandle, SftpSessionService,
};
use labonair_ssh::{
    ActiveTunnel, SharedSshEventSink, SshConnectRequest, SshConnectionService, SshPtyService,
    SshRemoteCommandService, SshRemoteFileService, SshSessionEvent, SshSessionId, SshTunnelService,
};
use labonair_terminal_core::{TermDimensions, TerminalColors, TerminalEmulator, TerminalEvent};
use labonair_terminal_host::protocol::*;
use labonair_terminal_host::server::*;
use labonair_transfers::{
    TransferDirection, TransferJob, TransferRequest, TransferResolution, TransferService,
    TransferSettings, TransferStatus,
};
use tokio::io::AsyncWriteExt;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::sync::{oneshot, Notify};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};

static COUNTER: AtomicUsize = AtomicUsize::new(0);
const WAIT: Duration = Duration::from_secs(20);

// ── Fake services ────────────────────────────────────────────────────────────

#[derive(Default)]
struct Fake {
    events: EventBus,
    sinks: Mutex<HashMap<String, SharedSshEventSink>>,
    writes: Mutex<Vec<String>>,
    resizes: Mutex<Vec<(u32, u32)>>,
    trust_calls: Mutex<Vec<(String, bool)>>,
    trust_waiters: Mutex<HashMap<String, oneshot::Sender<bool>>>,
    disconnects: Mutex<Vec<String>>,
    installed: Mutex<Vec<SshHostSnapshot>>,
    resolutions: Mutex<Vec<(String, TransferResolution)>>,
    resolve_waiters: Mutex<HashMap<String, oneshot::Sender<()>>>,
    finish: Arc<Notify>,
    jobs: AtomicUsize,
    settings: Arc<TransferSettings>,
}

impl Fake {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn services(self: &Arc<Self>) -> HostServices {
        let install = {
            let fake = Arc::clone(self);
            Arc::new(move |snapshot: &SshHostSnapshot| {
                fake.installed.lock().unwrap().push(snapshot.clone());
                Ok(())
            })
        };
        HostServices {
            events: self.events.clone(),
            connection: self.clone(),
            pty: self.clone(),
            remote_command: self.clone(),
            remote_file: self.clone(),
            tunnels: self.clone(),
            sftp_session: self.clone(),
            sftp_browser: self.clone(),
            transfers: self.clone(),
            transfer_settings: Arc::clone(&self.settings),
            install_host: install,
        }
    }

    /// Remote output for an SSH session (as the transport reader would send).
    fn push_output(&self, ssh_id: &str, text: &str) {
        let sink = self.sinks.lock().unwrap().get(ssh_id).cloned().unwrap();
        sink.send(SshSessionEvent::Output { data: text.into() })
            .unwrap();
    }

    fn job(id: &str, status: TransferStatus) -> TransferJob {
        TransferJob {
            id: id.into(),
            session_id: "tab-1".into(),
            src_path: "/tmp/src".into(),
            dest_path: "/remote/dest".into(),
            direction: TransferDirection::Upload,
            status,
            bytes_total: 10,
            bytes_transferred: 0,
            speed_bps: 0.0,
            skipped_count: 0,
        }
    }
}

impl SshConnectionService for Fake {
    fn connect<'a>(
        &'a self,
        request: SshConnectRequest,
        events: SharedSshEventSink,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async move {
            let sid = request.session_id.as_str().to_string();
            if request.host_id == "prompt-host" {
                let (tx, rx) = oneshot::channel();
                self.trust_waiters.lock().unwrap().insert(sid.clone(), tx);
                self.events
                    .emit(
                        "known_hosts_warning",
                        serde_json::json!({
                            "session_id": sid, "fingerprint": "aa:bb",
                            "host": "prompt-host", "is_mismatch": false
                        }),
                    )
                    .unwrap();
                if !matches!(rx.await, Ok(true)) {
                    return Err(LabonairError::HostKeyMismatch("User rejected host".into()));
                }
            }
            self.sinks
                .lock()
                .unwrap()
                .insert(sid.clone(), events.clone());
            let _ = self.events.emit(
                "session_established",
                serde_json::json!({ "session_id": sid, "default_path_ssh": "/home/me" }),
            );
            events
                .send(SshSessionEvent::Output {
                    data: "ready\r\n".into(),
                })
                .unwrap();
            Ok(())
        })
    }

    fn disconnect<'a>(&'a self, session_id: SshSessionId) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.disconnects
                .lock()
                .unwrap()
                .push(session_id.as_str().to_string());
            Ok(())
        })
    }

    fn trust_host<'a>(
        &'a self,
        session_id: SshSessionId,
        accepted: bool,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let sid = session_id.as_str().to_string();
            self.trust_calls
                .lock()
                .unwrap()
                .push((sid.clone(), accepted));
            if let Some(tx) = self.trust_waiters.lock().unwrap().remove(&sid) {
                let _ = tx.send(accepted);
            }
            Ok(())
        })
    }
}

impl SshPtyService for Fake {
    fn write<'a>(
        &'a self,
        session_id: SshSessionId,
        data: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.writes.lock().unwrap().push(data.clone());
            // The remote shell echoes what it is typed.
            let sink = self.sinks.lock().unwrap().get(session_id.as_str()).cloned();
            if let Some(sink) = sink {
                let _ = sink.send(SshSessionEvent::Output { data });
            }
            Ok(())
        })
    }

    fn resize<'a>(
        &'a self,
        _session_id: SshSessionId,
        cols: u32,
        rows: u32,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.resizes.lock().unwrap().push((cols, rows));
            Ok(())
        })
    }
}

impl SshRemoteCommandService for Fake {
    fn chown<'a>(
        &'a self,
        _: SshSessionId,
        _: String,
        _: String,
        _: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn calculate_size<'a>(
        &'a self,
        _: SshSessionId,
        path: String,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move { Ok(format!("size of {path}")) })
    }
}

impl SshRemoteFileService for Fake {
    fn prepare_remote_edit<'a>(
        &'a self,
        _: SshSessionId,
        _: String,
        _: Option<u64>,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async { Ok("/tmp/edit".into()) })
    }

    fn save_remote_edit<'a>(
        &'a self,
        _: SshSessionId,
        _: String,
        _: String,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn cleanup_remote_edit_temp<'a>(&'a self, _: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}

impl SshTunnelService for Fake {
    fn start<'a>(&'a self, _: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn stop<'a>(&'a self, _: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn active(&self) -> Vec<ActiveTunnel> {
        vec![ActiveTunnel {
            host_id: "h".into(),
            local_port: 1,
            remote_host: "r".into(),
            remote_port: 2,
        }]
    }
}

impl SftpSessionService for Fake {
    fn open<'a>(
        &'a self,
        ssh_session: SshSessionId,
    ) -> BoxFuture<'a, Result<SftpSessionHandle, LabonairError>> {
        Box::pin(async move { Ok(SftpSessionHandle::from_ssh_session(ssh_session)) })
    }

    fn close<'a>(&'a self, _: SftpSessionHandle) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}

impl SftpBrowserService for Fake {
    fn read_dir<'a>(
        &'a self,
        _: SftpSessionHandle,
        path: String,
    ) -> BoxFuture<'a, Result<Vec<RemoteEntry>, LabonairError>> {
        Box::pin(async move {
            if path == "/missing" {
                return Err(LabonairError::NotFound("no such directory".into()));
            }
            Ok(vec![RemoteEntry {
                name: "f".into(),
                path: format!("{path}/f"),
                size: 3,
                modified_at: 4,
                is_dir: false,
                is_symlink: false,
                symlink_target: None,
                permissions: "rw-r--r--".into(),
            }])
        })
    }

    fn rename<'a>(
        &'a self,
        _: SftpSessionHandle,
        _: String,
        _: String,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async { Ok(()) })
    }

    fn delete<'a>(
        &'a self,
        _: SftpSessionHandle,
        _: Vec<String>,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async { Ok(()) })
    }

    fn mkdir<'a>(
        &'a self,
        _: SftpSessionHandle,
        _: String,
        _: bool,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async { Ok(()) })
    }

    fn create_file<'a>(
        &'a self,
        _: SftpSessionHandle,
        _: String,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async { Ok(()) })
    }

    fn chmod<'a>(
        &'a self,
        _: SftpSessionHandle,
        _: String,
        _: u32,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        Box::pin(async { Ok(()) })
    }
}

impl TransferService for Fake {
    /// A job that runs until the test calls `finish`. A source path containing
    /// "conflict" asks about a conflict first and waits for `resolve`.
    fn enqueue<'a>(&'a self, request: TransferRequest) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let id = format!("job-{}", self.jobs.fetch_add(1, Ordering::SeqCst) + 1);
            let events = self.events.clone();
            let job_id = id.clone();
            let (tx, rx) = oneshot::channel();
            self.resolve_waiters.lock().unwrap().insert(id.clone(), tx);
            let finish = Arc::clone(&self.finish);
            let conflict = request.src_path.contains("conflict");
            tokio::spawn(async move {
                let mut job = Fake::job(&job_id, TransferStatus::Running);
                job.src_path = request.src_path;
                job.dest_path = request.dest_path;
                let _ = events.emit("transfer_progress", &job);
                if conflict {
                    let _ = events.emit(
                        "file_conflict",
                        serde_json::json!({
                            "job_id": job_id, "src_path": job.src_path, "dest_path": job.dest_path
                        }),
                    );
                    let _ = rx.await;
                    let _ = events.emit("transfer_progress", &job);
                }
                finish.notified().await;
                job.status = TransferStatus::Completed;
                let _ = events.emit("transfer_progress", &job);
            });
            Ok(id)
        })
    }

    fn cancel<'a>(&'a self, _: String) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn resolve<'a>(
        &'a self,
        job_id: String,
        resolution: TransferResolution,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            self.resolutions
                .lock()
                .unwrap()
                .push((job_id.clone(), resolution));
            if let Some(tx) = self.resolve_waiters.lock().unwrap().remove(&job_id) {
                let _ = tx.send(());
            }
            Ok(())
        })
    }
}

// ── Harness ──────────────────────────────────────────────────────────────────

struct DirGuard(PathBuf);

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Harness {
    dir: PathBuf,
    socket: PathBuf,
    handle: ServerHandle,
    task: JoinHandle<ExitReason>,
    _cleanup: DirGuard,
}

impl Harness {
    async fn start(services: Option<HostServices>) -> Self {
        Self::start_with(services, |_| {}).await
    }

    async fn start_with(
        services: Option<HostServices>,
        tweak: impl FnOnce(&mut ServerConfig),
    ) -> Self {
        let dir = PathBuf::from(format!(
            "/tmp/lhs-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let mut cfg = ServerConfig::new(&dir);
        if let Some(services) = services {
            cfg = cfg.with_services(services);
        }
        tweak(&mut cfg);
        let server = Server::bind(cfg).expect("bind");
        let socket = server.paths().socket.clone();
        let handle = server.handle();
        let task = tokio::spawn(server.run());
        Self {
            _cleanup: DirGuard(dir.clone()),
            dir,
            socket,
            handle,
            task,
        }
    }

    async fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).await.expect("connect");
        let (read, write) = stream.into_split();
        let mut client = Client {
            read,
            write,
            screen: Screen::new(80, 24),
            msgs: Vec::new(),
            events: Vec::new(),
            next_req: 1,
        };
        client
            .send(Message::Hello(Hello::current("test", None)))
            .await;
        client.wait_msg(|m| matches!(m, Message::HelloAck(_))).await;
        client
    }

    /// A client that already subscribed to service events.
    async fn ui_client(&self) -> Client {
        let mut client = self.client().await;
        client.rpc(RpcCall::SubscribeEvents).await.unwrap();
        client
    }

    async fn shutdown(self) -> ExitReason {
        self.handle.stop(ExitReason::Shutdown);
        timeout(WAIT, self.task).await.unwrap().unwrap()
    }
}

struct Screen {
    emulator: TerminalEmulator,
    events: Receiver<TerminalEvent>,
}

impl Screen {
    fn new(cols: u16, rows: u16) -> Self {
        let (tx, events) = sync_channel(4096);
        let emulator = TerminalEmulator::new(
            TerminalColors::xterm_default(),
            TermDimensions::new(usize::from(cols), usize::from(rows)),
            tx,
        );
        Self { emulator, events }
    }

    fn feed(&mut self, bytes: &[u8]) {
        let _ = self.emulator.feed(bytes);
        let _ = self.emulator.take_pty_output();
        while self.events.try_recv().is_ok() {}
    }

    fn text(&self) -> String {
        self.emulator.render().to_text()
    }
}

struct Client {
    read: OwnedReadHalf,
    write: OwnedWriteHalf,
    screen: Screen,
    msgs: Vec<Message>,
    /// Service events (`BusEvent`s) in arrival order: `(name, payload)`.
    events: Vec<(String, serde_json::Value)>,
    next_req: u64,
}

impl Client {
    async fn send(&mut self, message: Message) {
        let bytes = control_frame(&message).unwrap();
        self.write.write_all(&bytes).await.unwrap();
    }

    async fn input(&mut self, session: SessionId, text: &str) {
        let bytes = data_frame(DataKind::Input, session, text.as_bytes());
        self.write.write_all(&bytes).await.unwrap();
    }

    async fn step(&mut self) {
        let frame = timeout(WAIT, read_frame(&mut self.read))
            .await
            .expect("timed out waiting for a frame")
            .expect("protocol error")
            .expect("host closed the connection");
        match frame {
            Frame::Data {
                kind: DataKind::Output | DataKind::Snapshot,
                data,
                ..
            } => self.screen.feed(&data),
            Frame::Data { .. } => {}
            Frame::Control(Message::BusEvent { name, payload }) => {
                self.events.push((name, payload));
            }
            Frame::Control(message) => {
                if let Message::Attached { cols, rows, .. } = message {
                    self.screen = Screen::new(cols, rows);
                }
                self.msgs.push(message);
            }
        }
    }

    async fn wait_msg(&mut self, pred: impl Fn(&Message) -> bool) -> Message {
        loop {
            if let Some(i) = self.msgs.iter().position(&pred) {
                return self.msgs.remove(i);
            }
            self.step().await;
        }
    }

    async fn wait_event(&mut self, name: &str) -> serde_json::Value {
        loop {
            if let Some(i) = self.events.iter().position(|(n, _)| n == name) {
                return self.events.remove(i).1;
            }
            self.step().await;
        }
    }

    async fn wait_text(&mut self, needle: &str) {
        let result = timeout(WAIT, async {
            while !self.screen.text().contains(needle) {
                self.step().await;
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "{needle:?} never appeared; screen:\n{}",
            self.screen.text()
        );
    }

    async fn send_rpc(&mut self, call: RpcCall) -> u64 {
        let req_id = self.next_req;
        self.next_req += 1;
        self.send(Message::Rpc { req_id, call }).await;
        req_id
    }

    async fn reply(&mut self, req_id: u64) -> Result<RpcValue, WireError> {
        let message = self
            .wait_msg(|m| match m {
                Message::RpcOk { req_id: r, .. } | Message::RpcErr { req_id: r, .. } => {
                    *r == req_id
                }
                _ => false,
            })
            .await;
        match message {
            Message::RpcOk { value, .. } => Ok(value),
            Message::RpcErr { error, .. } => Err(error),
            _ => unreachable!(),
        }
    }

    async fn rpc(&mut self, call: RpcCall) -> Result<RpcValue, WireError> {
        let req_id = self.send_rpc(call).await;
        self.reply(req_id).await
    }

    async fn connect_ssh(&mut self, tab: &str, host: &str) -> Result<SessionId, WireError> {
        match self.rpc(connect_call(tab, host)).await? {
            RpcValue::SshConnected { session } => Ok(session),
            other => panic!("unexpected {other:?}"),
        }
    }

    async fn sessions(&mut self) -> Vec<SessionInfo> {
        self.send(Message::ListSessions).await;
        match self
            .wait_msg(|m| matches!(m, Message::Sessions { .. }))
            .await
        {
            Message::Sessions { sessions } => sessions,
            _ => unreachable!(),
        }
    }
}

fn connect_call(tab: &str, host: &str) -> RpcCall {
    RpcCall::SshConnect(SshConnectParams {
        session_id: tab.into(),
        host_id: host.into(),
        password: Some("hunter2-password".into()),
        host: SshHostSnapshot {
            hosts: vec![HostRecord {
                id: host.into(),
                host_address: "127.0.0.1".into(),
                port: 1,
                username: "me".into(),
                auth_method: "password".into(),
                ..Default::default()
            }],
            secrets: vec![SecretEntry {
                service: "labonair-app".into(),
                account: host.into(),
                value: "hunter2-secret".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    })
}

async fn eventually(what: &str, mut cond: impl FnMut() -> bool) {
    let result = timeout(WAIT, async {
        while !cond() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert!(result.is_ok(), "timed out waiting for {what}");
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ssh_shell_uses_the_attach_snapshot_and_live_machinery() {
    let fake = Fake::new();
    let host = Harness::start(Some(fake.services())).await;
    let mut ui = host.ui_client().await;

    let session = ui.connect_ssh("tab-1", "host-1").await.unwrap();
    // The request's host rows and secrets reached the stack, in RAM.
    {
        let installed = fake.installed.lock().unwrap();
        assert_eq!(installed[0].hosts[0].id, "host-1");
        assert_eq!(installed[0].secrets[0].value, "hunter2-secret");
    }
    let established = ui.wait_event("session_established").await;
    assert_eq!(established["session_id"], "tab-1");

    // Output that arrived before the attach is in the snapshot.
    ui.send(Message::Attach {
        session,
        cols: 100,
        rows: 30,
    })
    .await;
    ui.wait_msg(|m| matches!(m, Message::Attached { .. })).await;
    ui.wait_text("ready").await;
    // ... and the remote PTY was resized to the attach size (wiggle included).
    eventually("PTY resize to the attach size", || {
        fake.resizes.lock().unwrap().last() == Some(&(100, 30))
    })
    .await;

    // Input goes to the remote shell, its echo comes back live.
    ui.input(session, "abc").await;
    ui.wait_text("abc").await;
    assert_eq!(fake.writes.lock().unwrap().concat(), "abc");

    // Output produced while detached shows up in the next snapshot.
    ui.send(Message::Detach { session }).await;
    fake.push_output("tab-1", "while away\r\n");
    ui.send(Message::Attach {
        session,
        cols: 100,
        rows: 30,
    })
    .await;
    ui.wait_msg(|m| matches!(m, Message::Attached { .. })).await;
    ui.wait_text("while away").await;

    // The session is listed like any other and by `ListSshSessions`.
    let listed = ui.sessions().await;
    assert!(listed
        .iter()
        .any(|s| s.id == session && s.status == SessionStatus::Running));
    let Ok(RpcValue::SshSessions(ssh)) = ui.rpc(RpcCall::ListSshSessions).await else {
        panic!("ssh sessions");
    };
    assert_eq!(ssh[0].ssh_session_id, "tab-1");
    assert!(ssh[0].connected);

    // Kill disconnects the transport.
    ui.send(Message::Kill { session }).await;
    eventually("transport disconnect", || {
        fake.disconnects
            .lock()
            .unwrap()
            .contains(&"tab-1".to_string())
    })
    .await;
    host.shutdown().await;
}

#[tokio::test]
async fn connection_loss_is_reported_and_the_session_waits_for_ack() {
    let fake = Fake::new();
    let host = Harness::start(Some(fake.services())).await;
    let mut ui = host.ui_client().await;
    let session = ui.connect_ssh("tab-1", "host-1").await.unwrap();
    ui.send(Message::Attach {
        session,
        cols: 80,
        rows: 24,
    })
    .await;
    ui.wait_msg(|m| matches!(m, Message::Attached { .. })).await;

    fake.events
        .emit(
            "ssh_connection_lost",
            serde_json::json!({ "session_id": "tab-1", "reason": "Connection timed out" }),
        )
        .unwrap();
    let lost = ui
        .wait_msg(|m| matches!(m, Message::ConnectionLost { .. }))
        .await;
    assert_eq!(
        lost,
        Message::ConnectionLost {
            session,
            reason: "Connection timed out".into()
        }
    );
    let listed = ui.sessions().await;
    let info = listed.iter().find(|s| s.id == session).unwrap();
    assert_eq!(info.status, SessionStatus::Exited);

    // No silent reconnect: nothing new was connected, the old entry stays.
    let Ok(RpcValue::SshSessions(ssh)) = ui.rpc(RpcCall::ListSshSessions).await else {
        panic!("ssh sessions");
    };
    assert!(!ssh[0].connected);
    assert_eq!(ssh[0].lost_reason.as_deref(), Some("Connection timed out"));

    // A fresh client attaching later still learns why.
    let mut later = host.client().await;
    later
        .send(Message::Attach {
            session,
            cols: 80,
            rows: 24,
        })
        .await;
    later
        .wait_msg(|m| matches!(m, Message::ConnectionLost { .. }))
        .await;

    ui.send(Message::Ack { session }).await;
    eventually("ack removes the session", || {
        host.handle.session_count() == 0
    })
    .await;
    host.shutdown().await;
}

#[tokio::test]
async fn host_key_prompt_goes_to_the_ui_and_fails_closed_without_one() {
    let fake = Fake::new();
    let host = Harness::start(Some(fake.services())).await;

    // A subscribed client sees the prompt and answers it.
    let mut ui = host.ui_client().await;
    let req = ui.send_rpc(connect_call("tab-1", "prompt-host")).await;
    let prompt = ui.wait_event("known_hosts_warning").await;
    assert_eq!(prompt["session_id"], "tab-1");
    ui.rpc(RpcCall::SshTrustHost {
        session_id: "tab-1".into(),
        accepted: true,
    })
    .await
    .unwrap();
    assert!(matches!(
        ui.reply(req).await,
        Ok(RpcValue::SshConnected { .. })
    ));

    // No client subscribed: rejected immediately, nothing connects.
    ui.rpc(RpcCall::UnsubscribeEvents).await.unwrap();
    let mut headless = host.client().await;
    let err = headless
        .rpc(connect_call("tab-2", "prompt-host"))
        .await
        .unwrap_err();
    assert_eq!(err.code.as_deref(), Some("HostKeyMismatch"));
    assert!(fake
        .trust_calls
        .lock()
        .unwrap()
        .contains(&("tab-2".to_string(), false)));

    // A prompt stays open while any UI is subscribed, and is rejected when
    // the last one goes away.
    ui.rpc(RpcCall::SubscribeEvents).await.unwrap();
    let mut doomed = host.ui_client().await;
    doomed.send_rpc(connect_call("tab-3", "prompt-host")).await;
    doomed.wait_event("known_hosts_warning").await;
    drop(doomed);
    sleep(Duration::from_millis(300)).await;
    assert!(!fake
        .trust_calls
        .lock()
        .unwrap()
        .contains(&("tab-3".to_string(), false)));
    ui.rpc(RpcCall::UnsubscribeEvents).await.unwrap();
    eventually("prompt rejected after the last client left", || {
        fake.trust_calls
            .lock()
            .unwrap()
            .contains(&("tab-3".to_string(), false))
    })
    .await;
    eventually("failed connects leave no session", || {
        host.handle.session_count() == 1 // only tab-1
    })
    .await;
    host.shutdown().await;
}

#[tokio::test]
async fn transfers_run_unattended_park_on_conflicts_and_hold_the_daemon_open() {
    let fake = Fake::new();
    let host = Harness::start_with(Some(fake.services()), |cfg| {
        cfg.idle_timeout = Duration::from_millis(300);
    })
    .await;

    let mut ui = host.ui_client().await;
    ui.rpc(RpcCall::TransferConfigure(TransferConfig {
        max_concurrent: 2,
        chunk_size: 65536,
        default_conflict_resolution: "overwrite".into(),
        on_folder_file_error: "ask".into(),
    }))
    .await
    .unwrap();
    // Subscribed: the user's explicit setting is honoured ...
    assert_eq!(
        *fake.settings.default_conflict_resolution.lock().unwrap(),
        "overwrite"
    );

    // Relative and empty local paths are refused.
    for (src, dest) in [("relative/file", "/remote/x"), ("", "/remote/x")] {
        let err = ui
            .rpc(RpcCall::TransferEnqueue {
                session_id: "tab-1".into(),
                src_path: src.into(),
                dest_path: dest.into(),
                direction: TransferDirection::Upload,
            })
            .await
            .unwrap_err();
        assert_eq!(err.code.as_deref(), Some("InvalidInput"));
    }
    let Ok(RpcValue::JobId(job)) = ui
        .rpc(RpcCall::TransferEnqueue {
            session_id: "tab-1".into(),
            src_path: "/tmp/conflict-file".into(),
            dest_path: "/remote/x".into(),
            direction: TransferDirection::Upload,
        })
        .await
    else {
        panic!("job id");
    };
    ui.wait_event("file_conflict").await;

    // The UI goes away. The job is parked, nothing resolves it, and the
    // daemon (idle timeout 300 ms) stays up because the job is not finished.
    drop(ui);
    eventually("unattended policy", || {
        *fake.settings.default_conflict_resolution.lock().unwrap() == "ask"
    })
    .await;
    sleep(Duration::from_millis(1200)).await;
    assert!(!host.task.is_finished(), "idle exit must wait for the job");
    assert!(fake.resolutions.lock().unwrap().is_empty());

    // A fresh UI sees the parked job and its pending conflict.
    let mut ui = host.ui_client().await;
    let progress = ui.wait_event("transfer_progress").await;
    assert_eq!(progress["id"], job.as_str());
    let conflict = ui.wait_event("file_conflict").await;
    assert_eq!(conflict["job_id"], job.as_str());
    let Ok(RpcValue::Transfers(snapshots)) = ui.rpc(RpcCall::ListTransfers).await else {
        panic!("snapshots");
    };
    assert!(snapshots[0].conflict.is_some());
    assert_eq!(snapshots[0].job.status, TransferStatus::Paused);

    // Only an explicit decision resumes it; then it finishes and the daemon
    // may idle out.
    ui.rpc(RpcCall::TransferResolve {
        job_id: job.clone(),
        resolution: WireResolution::Skip,
    })
    .await
    .unwrap();
    assert_eq!(
        fake.resolutions.lock().unwrap().as_slice(),
        &[(job, TransferResolution::Skip)]
    );
    drop(ui);
    fake.finish.notify_one();
    let reason = timeout(WAIT, host.task).await.expect("idle exit").unwrap();
    assert_eq!(reason, ExitReason::Idle);
}

#[tokio::test]
async fn a_download_to_a_relative_local_path_is_refused() {
    let fake = Fake::new();
    let host = Harness::start(Some(fake.services())).await;
    let mut ui = host.ui_client().await;
    let err = ui
        .rpc(RpcCall::TransferEnqueue {
            session_id: "tab-1".into(),
            src_path: "/remote/x".into(),
            dest_path: "downloads/x".into(),
            direction: TransferDirection::Download,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code.as_deref(), Some("InvalidInput"));
    host.shutdown().await;
}

#[tokio::test]
async fn sftp_and_remote_calls_map_one_to_one_with_typed_errors() {
    let fake = Fake::new();
    let host = Harness::start(Some(fake.services())).await;
    let mut ui = host.ui_client().await;

    let Ok(RpcValue::SftpHandle(handle)) = ui
        .rpc(RpcCall::SftpOpen {
            ssh_session: "tab-1".into(),
        })
        .await
    else {
        panic!("handle");
    };
    assert_eq!(handle, "tab-1");
    let Ok(RpcValue::Entries(entries)) = ui
        .rpc(RpcCall::SftpReadDir {
            session: handle.clone(),
            path: "/srv".into(),
        })
        .await
    else {
        panic!("entries");
    };
    assert_eq!(entries[0].path, "/srv/f");
    let err = ui
        .rpc(RpcCall::SftpReadDir {
            session: handle,
            path: "/missing".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(
        err.into_labonair(),
        LabonairError::NotFound("no such directory".into())
    );
    let Ok(RpcValue::Text(size)) = ui
        .rpc(RpcCall::SshCalculateSize {
            session_id: "tab-1".into(),
            path: "/x".into(),
        })
        .await
    else {
        panic!("size");
    };
    assert_eq!(size, "size of /x");
    assert!(matches!(
        ui.rpc(RpcCall::SshTunnelsActive).await,
        Ok(RpcValue::Tunnels(t)) if t.len() == 1
    ));
    // The cleanup call deletes a local file: relative paths are refused.
    let err = ui
        .rpc(RpcCall::SshCleanupRemoteEditTemp {
            local_temp_path: "tmp/edit".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code.as_deref(), Some("InvalidInput"));
    host.shutdown().await;
}

#[tokio::test]
async fn a_terminal_only_host_refuses_service_calls() {
    let host = Harness::start(None).await;
    let mut client = host.client().await;
    let err = client.rpc(RpcCall::ListTransfers).await.unwrap_err();
    assert!(err.message.contains("no SSH"));
    host.shutdown().await;
}

/// The concrete stack: an unreachable host fails with a typed error, leaves no
/// session behind, and the credentials of the request never touch the disk.
#[tokio::test]
async fn concrete_stack_keeps_credentials_in_ram_only() {
    let services = labonair_terminal_host::stack::concrete_services().expect("stack");
    let host = Harness::start(Some(services)).await;
    let mut ui = host.ui_client().await;
    let err = ui.connect_ssh("tab-1", "host-1").await.unwrap_err();
    assert!(!err.message.is_empty());
    assert_eq!(host.handle.session_count(), 0);

    fn files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                files(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut all = Vec::new();
    files(&host.dir, &mut all);
    for path in all {
        if path.extension().is_some_and(|e| e == "sock") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_default();
        assert!(
            !String::from_utf8_lossy(&bytes).contains("hunter2"),
            "secret found in {}",
            path.display()
        );
    }
    host.shutdown().await;
}
