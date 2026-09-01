use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

/// A single configured port-forward. Only `type: "local"` is supported — matching
/// the reference implementation, whose UI (`reference-src/src/modules/hosts/types.ts`)
/// only ever writes local forwards. `tunnel_type` is kept so the on-disk JSON
/// round-trips unchanged.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TunnelConfig {
    pub id: String,
    #[serde(rename = "type", default = "local_type")]
    pub tunnel_type: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

fn local_type() -> String {
    "local".to_string()
}

/// One running forward, as surfaced to the UI's active-tunnel panel.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActiveTunnel {
    pub host_id: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

/// Per-host tunnel entry: shutdown sender + reference count of active SSH sessions.
/// The tunnel runs as long as ref_count > 0; ssh_stop_tunnels decrements and only
/// sends the shutdown signal when the count reaches zero.
pub struct TunnelEntry {
    shutdown: tokio::sync::oneshot::Sender<()>,
    ref_count: usize,
    /// The forwards this entry runs — reported by [`active_tunnels`].
    configs: Vec<TunnelConfig>,
}

/// Snapshot of every forward currently bound, across all hosts. Sorted by
/// `(host_id, local_port)` for stable rendering.
pub fn active_tunnels(state: &TunnelState) -> Vec<ActiveTunnel> {
    let Ok(map) = state.0.lock() else {
        return Vec::new();
    };
    let mut out: Vec<ActiveTunnel> = map
        .iter()
        .flat_map(|(host_id, entry)| {
            entry.configs.iter().map(move |c| ActiveTunnel {
                host_id: host_id.clone(),
                local_port: c.local_port,
                remote_host: c.remote_host.clone(),
                remote_port: c.remote_port,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.host_id
            .cmp(&b.host_id)
            .then(a.local_port.cmp(&b.local_port))
    });
    out
}

pub struct TunnelState(pub Arc<Mutex<HashMap<String, TunnelEntry>>>);

impl Default for TunnelState {
    fn default() -> Self {
        TunnelState(Arc::new(Mutex::new(HashMap::new())))
    }
}

pub type TunnelMap = Arc<Mutex<HashMap<String, TunnelEntry>>>;

/// Relays bytes between one accepted local TCP connection and a `direct-tcpip`
/// channel opened on the tunnel's shared SSH `Handle`. Replaces the old
/// per-connection OS thread that manually polled a non-blocking TCP stream
/// and SSH channel with 1ms sleeps: `Handle::channel_open_direct_tcpip`
/// (the same call `client.rs::connect_via_jump` uses for jump-host bridging)
/// gives us a `Channel`, whose `.into_stream()` adapter (also reused from
/// `client.rs`) implements `AsyncRead + AsyncWrite` — so the whole bridge is
/// one `tokio::io::copy_bidirectional` call, no manual read/write loop.
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    handle: Arc<russh::client::Handle<super::client::ClientHandler>>,
    remote_host: String,
    remote_port: u16,
    local_port: u16,
) {
    let channel = match handle
        .channel_open_direct_tcpip(
            remote_host,
            remote_port as u32,
            "127.0.0.1",
            local_port as u32,
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("tunnel: channel_open_direct_tcpip failed: {e}");
            return;
        }
    };

    let mut channel_stream = channel.into_stream();
    if let Err(e) = tokio::io::copy_bidirectional(&mut stream, &mut channel_stream).await {
        log::debug!("tunnel: connection closed: {e}");
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn ssh_start_tunnels(
    host_id: String,
    tunnel_state: &TunnelState,
    hosts_db: &crate::modules::hosts::HostsDb,
    secrets: &crate::modules::secrets::SecretsState,
    trust_state: &super::TrustState,
    app: crate::App,
    connect_timeout_secs: Option<u64>,
) -> Result<(), String> {
    // If tunnel already running for this host, just increment the ref count.
    {
        let mut map = tunnel_state.0.lock().map_err(|e| e.to_string())?;
        if let Some(entry) = map.get_mut(&host_id) {
            entry.ref_count += 1;
            return Ok(());
        }
    }

    let (host_address, port, username, auth_method, private_key_path, tunnels_json, jump_host_id) = {
        let conn = hosts_db.0.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT host_address, port, username, auth_method, private_key_path, tunnels, jump_host_id \
             FROM hosts WHERE id = ?1",
            rusqlite::params![host_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?
    };

    let tunnels: Vec<TunnelConfig> = match tunnels_json.as_deref() {
        Some(j) if !j.is_empty() && j != "[]" => serde_json::from_str(j).unwrap_or_default(),
        _ => return Ok(()),
    };

    if tunnels.is_empty() {
        return Ok(());
    }

    let password: Option<String> = if auth_method == "password" {
        crate::modules::secrets::get_password(&app, secrets, "labonair-app", &host_id)
            .ok()
            .flatten()
    } else {
        None
    };

    // Resolve jump host fields (if any) — same helper the terminal/SFTP paths use.
    let jump = match jump_host_id.as_deref() {
        Some(jid) => Some(
            super::client::resolve_jump_host(hosts_db, secrets, &app, jid)
                .map_err(|e| e.to_string())?,
        ),
        None => None,
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    {
        let mut map = tunnel_state.0.lock().map_err(|e| e.to_string())?;
        map.insert(
            host_id.clone(),
            TunnelEntry {
                shutdown: shutdown_tx,
                ref_count: 1,
                configs: tunnels.clone(),
            },
        );
    }

    let host_id_clone = host_id.clone();
    let state_arc = tunnel_state.0.clone();
    let trust_inner = trust_state.clone();
    let app_clone = app.clone();

    tokio::spawn(run_tunnel_loop(
        host_address,
        port,
        username,
        auth_method,
        private_key_path,
        password,
        jump,
        tunnels,
        shutdown_rx,
        host_id_clone,
        state_arc,
        trust_inner,
        app_clone,
        connect_timeout_secs,
    ));

    Ok(())
}

/// Connects and authenticates once (via the same shared
/// `establish_authenticated_session` helper the terminal/SFTP paths use —
/// giving tunnels agent auth, passphrase-protected-key support and real
/// known-hosts verification for the first time), then bridges local TCP
/// connections through `direct-tcpip` channels on that one session until
/// `shutdown_rx` fires. `fail_fast_untrusted_host=true` is passed since
/// tunnels have no trust-dialog UI of their own — an unrecognized/mismatched
/// host key fails the tunnel start cleanly instead of hanging.
#[allow(clippy::too_many_arguments)]
async fn run_tunnel_loop(
    host_address: String,
    port: i64,
    username: String,
    auth_method: String,
    private_key_path: Option<String>,
    password: Option<String>,
    jump: Option<super::client::JumpHostParams>,
    tunnels: Vec<TunnelConfig>,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    host_id: String,
    tunnel_state: TunnelMap,
    trust_state: super::TrustState,
    app: crate::App,
    connect_timeout_secs: Option<u64>,
) {
    let session_id = format!("tunnel_{host_id}");

    let handle = match super::client::establish_authenticated_session(
        &session_id,
        &host_address,
        port,
        &username,
        &auth_method,
        private_key_path.as_deref(),
        None,
        None,
        password.as_deref(),
        None,
        &trust_state,
        &app,
        true,
        jump,
        connect_timeout_secs,
    )
    .await
    {
        Ok(h) => h,
        Err(e) => {
            log::error!("tunnel: failed to establish session for host {host_id}: {e}");
            if let Ok(mut m) = tunnel_state.lock() {
                m.remove(&host_id);
            }
            return;
        }
    };

    // Bind listeners. Port-in-use errors are warned and skipped.
    let mut listeners: Vec<(TcpListener, TunnelConfig)> = Vec::new();
    for tunnel in &tunnels {
        match TcpListener::bind(format!("127.0.0.1:{}", tunnel.local_port)).await {
            Ok(listener) => {
                log::info!(
                    "tunnel: bound 127.0.0.1:{} → {}:{}",
                    tunnel.local_port,
                    tunnel.remote_host,
                    tunnel.remote_port
                );
                listeners.push((listener, tunnel.clone()));
            }
            Err(e) => {
                log::warn!(
                    "tunnel: port {} already in use ({}), skipping",
                    tunnel.local_port,
                    e
                );
            }
        }
    }

    if listeners.is_empty() {
        if let Ok(mut m) = tunnel_state.lock() {
            m.remove(&host_id);
        }
        return;
    }

    // One accept task per listener, raced against a shared cancellation token
    // instead of polling a stop flag every 20ms — reacts to shutdown as soon
    // as `cancel.cancel()` is called, with no sleep in between.
    let cancel = CancellationToken::new();
    let mut accept_tasks = Vec::new();

    for (listener, config) in listeners {
        let handle = handle.clone();
        let cancel = cancel.clone();
        let remote_host = config.remote_host;
        let remote_port = config.remote_port;
        let local_port = config.local_port;

        accept_tasks.push(tokio::spawn(async move {
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _)) => {
                                tokio::spawn(handle_connection(
                                    stream,
                                    handle.clone(),
                                    remote_host.clone(),
                                    remote_port,
                                    local_port,
                                ));
                            }
                            Err(e) => {
                                log::warn!("tunnel: accept failed: {e}");
                                break;
                            }
                        }
                    }
                    _ = cancel.cancelled() => break,
                }
            }
        }));
    }

    // Block until the shutdown signal arrives (oneshot from ssh_stop_tunnels),
    // then cancel all accept loops immediately — no sleep-poll teardown delay.
    let _ = shutdown_rx.await;
    cancel.cancel();
    for task in accept_tasks {
        let _ = task.await;
    }

    if let Ok(mut m) = tunnel_state.lock() {
        m.remove(&host_id);
    }
    log::info!("tunnel: stopped for host {host_id}");
}

pub async fn ssh_stop_tunnels(host_id: String, tunnel_state: &TunnelState) -> Result<(), String> {
    let mut map = tunnel_state.0.lock().map_err(|e| e.to_string())?;
    if let Some(entry) = map.get_mut(&host_id) {
        entry.ref_count = entry.ref_count.saturating_sub(1);
        if entry.ref_count == 0 {
            // Last SSH session for this host closed — shut down the tunnel.
            if let Some(entry) = map.remove(&host_id) {
                let _ = entry.shutdown.send(());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(local: u16) -> TunnelConfig {
        TunnelConfig {
            id: format!("t{local}"),
            tunnel_type: "local".into(),
            local_port: local,
            remote_host: "10.0.0.1".into(),
            remote_port: 80,
        }
    }

    fn insert(state: &TunnelState, host: &str, ref_count: usize, configs: Vec<TunnelConfig>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        // Keep the receiver alive so `shutdown.send` doesn't fail; leak it for the test.
        Box::leak(Box::new(rx));
        state.0.lock().unwrap().insert(
            host.to_string(),
            TunnelEntry {
                shutdown: tx,
                ref_count,
                configs,
            },
        );
    }

    #[test]
    fn parses_the_ui_tunnel_json_shape() {
        let raw = r#"[{"id":"a","type":"local","local_port":8080,"remote_host":"db","remote_port":5432}]"#;
        let v: Vec<TunnelConfig> = serde_json::from_str(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].local_port, 8080);
        assert_eq!(v[0].remote_host, "db");
        assert_eq!(v[0].remote_port, 5432);
        // Round-trips back out unchanged.
        let out = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<Vec<TunnelConfig>>(&out).unwrap(), v);
    }

    #[test]
    fn active_tunnels_lists_every_forward_sorted() {
        let state = TunnelState::default();
        insert(&state, "hb", 1, vec![cfg(2000)]);
        insert(&state, "ha", 1, vec![cfg(1100), cfg(1000)]);
        let list = active_tunnels(&state);
        assert_eq!(list.len(), 3);
        assert_eq!((list[0].host_id.as_str(), list[0].local_port), ("ha", 1000));
        assert_eq!((list[1].host_id.as_str(), list[1].local_port), ("ha", 1100));
        assert_eq!((list[2].host_id.as_str(), list[2].local_port), ("hb", 2000));
    }

    #[tokio::test]
    async fn stop_tunnels_only_shuts_down_at_zero_refs() {
        let state = TunnelState::default();
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        state.0.lock().unwrap().insert(
            "h1".into(),
            TunnelEntry {
                shutdown: tx,
                ref_count: 2,
                configs: vec![cfg(1000)],
            },
        );

        ssh_stop_tunnels("h1".into(), &state).await.unwrap();
        assert!(state.0.lock().unwrap().contains_key("h1"));
        assert!(rx.try_recv().is_err());

        ssh_stop_tunnels("h1".into(), &state).await.unwrap();
        assert!(!state.0.lock().unwrap().contains_key("h1"));
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn stop_tunnels_is_a_noop_for_an_unknown_host() {
        let state = TunnelState::default();
        ssh_stop_tunnels("nope".into(), &state).await.unwrap();
    }
}
