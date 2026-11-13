//! Client <-> daemon integration: the real `labonair-host` server on a temp
//! socket (in-process, own runtime thread), driven through `DaemonHost` and the
//! terminal registry exactly as the workspace does. Unix only.
#![cfg(unix)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use labonair_terminal::{
    DaemonHost, HostedLink, SessionHandle, SessionOptions, SessionStatus, TermDimensions,
    TerminalColors, TerminalRegistry,
};
use labonair_terminal_host::client::{HostClient, HostStatus};
use labonair_terminal_host::protocol::{Hello, SessionStatus as WireStatus};
use labonair_terminal_host::server::{ExitReason, Server, ServerConfig, ServerHandle};

static COUNTER: AtomicUsize = AtomicUsize::new(0);
const WAIT: Duration = Duration::from_secs(20);

struct TestHost {
    dir: PathBuf,
    handle: ServerHandle,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl TestHost {
    fn start() -> Self {
        let dir = PathBuf::from(format!(
            "/tmp/lth-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let (tx, rx) = mpsc::channel();
        let server_dir = dir.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let server = Server::bind(ServerConfig::new(server_dir)).expect("bind");
                tx.send(server.handle()).unwrap();
                server.run().await;
            });
        });
        Self {
            dir,
            handle: rx.recv().unwrap(),
            thread: Some(thread),
        }
    }

    fn client(&self) -> HostClient {
        let client = HostClient::new(self.dir.clone(), Hello::current("test", Some(1000)));
        client.reconnect().expect("connect");
        client
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        self.handle.stop(ExitReason::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn sh() -> SessionOptions {
    SessionOptions {
        shell: Some("/bin/sh".to_string()),
        env: vec![("PS1".to_string(), "$ ".to_string())],
        ..SessionOptions::default()
    }
}

fn create(registry: &TerminalRegistry, host: &DaemonHost) -> SessionHandle {
    let id = registry
        .create_with_host(
            host,
            TerminalColors::xterm_default(),
            TermDimensions::new(80, 24),
            sh(),
        )
        .expect("create hosted session");
    registry.handle(id).unwrap()
}

fn until(what: &str, mut ready: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < WAIT {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {what}");
}

fn screen(handle: &SessionHandle) -> String {
    let _ = handle.drain_events();
    handle.with(|s| s.render().unwrap().to_text())
}

fn wait_text(handle: &SessionHandle, needle: &str) {
    until(&format!("{needle:?} on screen"), || {
        screen(handle).contains(needle)
    });
}

#[test]
fn create_echo_detach_reattach_exit_and_close() {
    let host = TestHost::start();
    let client = host.client();
    let daemon = DaemonHost::with_client(client.clone());
    let registry = TerminalRegistry::new();

    let handle = create(&registry, &daemon.clone().for_pane("pane-a"));
    assert!(handle.is_hosted());
    assert_eq!(handle.hosted_link(), Some(HostedLink::Live));
    let host_id = handle.host_session_id().expect("hosted id");
    // `%s` keeps the marker out of the echoed command line.
    handle.write(b"printf 'AB%sCD\\n' X\n").unwrap();
    wait_text(&handle, "ABXCD");

    // App quit: only detaches; the session keeps running in the daemon.
    drop(handle);
    registry.close_all();
    assert!(registry.is_empty());
    until("session detached but alive", || {
        client.list_sessions().unwrap().iter().any(|s| {
            s.id == host_id
                && s.status == WireStatus::Running
                && !s.attached
                && s.pane_tag.as_deref() == Some("pane-a")
        })
    });

    // Restore: attach the same session, the snapshot repaints the old output.
    let registry = TerminalRegistry::new();
    let handle = create(
        &registry,
        &daemon.clone().for_pane("pane-a").adopting(host_id),
    );
    assert_eq!(handle.host_session_id(), Some(host_id));
    wait_text(&handle, "ABXCD");
    handle.write(b"printf 'EF%sGH\\n' Y\n").unwrap();
    wait_text(&handle, "EFYGH");

    // The foreground job is reported from the daemon's session info.
    handle.write(b"sleep 300\n").unwrap();
    until("foreground job", || {
        handle
            .foreground_process()
            .is_some_and(|f| f.name == "sleep")
    });
    assert!(handle.has_foreground_job());
    // Interrupt, then exit: the exit state reaches the pane.
    handle.write(b"\x03").unwrap();
    handle.write(b"exit 7\n").unwrap();
    until("exit state", || {
        let _ = handle.drain_events();
        handle.status() == SessionStatus::Exited(7)
    });

    // Closing the tab of an exited session acknowledges it.
    let id = handle.id();
    drop(handle);
    registry.close(id);
    until("session forgotten", || {
        !client
            .list_sessions()
            .unwrap()
            .iter()
            .any(|s| s.id == host_id)
    });
}

#[test]
fn closing_a_running_tab_kills_the_session() {
    let host = TestHost::start();
    let client = host.client();
    let daemon = DaemonHost::with_client(client.clone());
    let registry = TerminalRegistry::new();
    let handle = create(&registry, &daemon);
    let host_id = handle.host_session_id().unwrap();
    assert_eq!(client.list_sessions().unwrap().len(), 1);
    registry.close(handle.id());
    until("session killed", || {
        !client
            .list_sessions()
            .unwrap()
            .iter()
            .any(|s| s.id == host_id)
    });
}

#[test]
fn a_second_client_steals_the_session_and_reattach_steals_it_back() {
    let host = TestHost::start();
    let daemon_a = DaemonHost::with_client(host.client());
    let daemon_b = DaemonHost::with_client(host.client());
    let registry_a = TerminalRegistry::new();
    let registry_b = TerminalRegistry::new();

    let a = create(&registry_a, &daemon_a);
    let host_id = a.host_session_id().unwrap();
    // The two clients race on separate sockets: let A's attach land first.
    let observer = host.client();
    until("A to be attached", || {
        observer
            .list_sessions()
            .unwrap()
            .iter()
            .any(|s| s.id == host_id && s.attached)
    });
    let b = create(&registry_b, &daemon_b.adopting(host_id));
    until("A to be told it was taken over", || {
        a.hosted_link() == Some(HostedLink::Stolen)
    });
    assert_eq!(b.hosted_link(), Some(HostedLink::Live));
    b.write(b"printf 'ST%sEAL\\n' Z\n").unwrap();
    wait_text(&b, "STZEAL");

    a.reattach().unwrap();
    assert_eq!(a.hosted_link(), Some(HostedLink::Live));
    until("B to be taken over in turn", || {
        b.hosted_link() == Some(HostedLink::Stolen)
    });
    a.write(b"printf 'BA%sCK\\n' W\n").unwrap();
    wait_text(&a, "BAWCK");
}

#[test]
fn a_lost_connection_shows_host_lost_and_reconnect_resumes() {
    let host = TestHost::start();
    let client = host.client();
    let daemon = DaemonHost::with_client(client.clone());
    let registry = TerminalRegistry::new();
    let handle = create(&registry, &daemon);
    handle.write(b"printf 'BE%sFORE\\n' Q\n").unwrap();
    wait_text(&handle, "BEQFORE");

    client.disconnect();
    until("host lost", || {
        handle.hosted_link() == Some(HostedLink::HostLost)
    });
    // Writing while disconnected must not panic.
    handle.write(b"ignored\n").unwrap();

    handle.reattach().unwrap();
    assert_eq!(handle.hosted_link(), Some(HostedLink::Live));
    handle.write(b"printf 'AF%sTER\\n' R\n").unwrap();
    wait_text(&handle, "AFRTER");
    // The snapshot after reattach keeps what happened before.
    assert!(screen(&handle).contains("BEQFORE"));
}

#[test]
fn an_incompatible_host_is_reported_and_refuses_sessions() {
    let host = TestHost::start();
    let hello = Hello {
        min_compat: 99,
        ..Hello::current("future", None)
    };
    let client = HostClient::new(host.dir.clone(), hello);
    client.reconnect().unwrap();
    assert!(matches!(client.status(), HostStatus::Incompatible { .. }));
    // The session list still works, so the UI can offer a restart.
    assert!(client.list_sessions().unwrap().is_empty());
    let registry = TerminalRegistry::new();
    let err = registry
        .create_with_host(
            &DaemonHost::with_client(client),
            TerminalColors::xterm_default(),
            TermDimensions::new(80, 24),
            sh(),
        )
        .unwrap_err();
    assert!(err.contains("incompatible"), "{err}");
}
