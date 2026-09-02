use crate::modules::errors::LabonairError;
use crate::modules::sftp::net_error::is_network_error;
use crate::modules::ssh::{RushSession, SshState, TrustState};
use std::sync::Arc;

/// Proactively pings the lazily-opened SFTP subsystem with a cheap read-only
/// request (`canonicalize(".")`) every `keep_alive_interval` seconds while
/// the session stays registered. Unlike a PTY session — which has an
/// always-on reader task blocked on the channel that notices a dead
/// connection as soon as the transport's own keepalive gives up — the SFTP
/// subsystem has no background reader; without this, a socket that died
/// silently (e.g. the machine slept) is only ever discovered reactively, on
/// whatever `sftp_*` request the frontend happens to issue next (a poll tick
/// or a manual folder expand), which can lag arbitrarily behind the actual
/// drop and leaves the sidebar Explorer looking "connected" while stale.
///
/// Spawned once per newly-opened SFTP subsystem (see the call site in
/// `sftp_connect_inner`, gated by `OnceCell::get_or_try_init` so it only
/// happens once even under concurrent `sftp_connect` calls). Pinned to the
/// specific `owning_session` `Arc` it was spawned for via `Arc::ptr_eq` on
/// every tick — both to decide whether to keep polling and whether a failure
/// is allowed to remove the map entry / emit `ssh_connection_lost`. This
/// matters for two reasons: (1) a manual reconnect (`sftp_disconnect` +
/// `sftp_connect`) installs a *new* `RushSession` under the same
/// `session_id`, so a stale ping already in flight against the old session
/// must not be allowed to tear down the fresh one when it finally errors
/// out; (2) without pinning, this loop would previously just adopt whatever
/// session is currently registered instead of exiting, so every reconnect
/// during a session's lifetime leaked one more redundant concurrent poller
/// for it, forever.
fn spawn_sftp_health_check(
    session_id: String,
    state: SshState,
    app: crate::App,
    keep_alive_interval: Option<i64>,
    owning_session: Arc<RushSession>,
) {
    let interval = std::time::Duration::from_secs(keep_alive_interval.unwrap_or(25).max(10) as u64);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;

            let sftp = {
                let map = match state.0.lock() {
                    Ok(m) => m,
                    Err(_) => break,
                };
                match map.get(&session_id) {
                    Some(current) if Arc::ptr_eq(current, &owning_session) => {
                        match current.sftp.get().cloned() {
                            Some(sftp) => sftp,
                            None => break,
                        }
                    }
                    // Either disconnected, or superseded by a reconnect —
                    // this generation's watch is over either way.
                    _ => break,
                }
            };

            if let Err(e) = sftp.canonicalize(".").await {
                let msg = e.to_string();
                if is_network_error(&msg) {
                    let removed = if let Ok(mut map) = state.0.lock() {
                        match map.get(&session_id) {
                            Some(current) if Arc::ptr_eq(current, &owning_session) => {
                                map.remove(&session_id);
                                true
                            }
                            _ => false,
                        }
                    } else {
                        false
                    };
                    if removed {
                        let _ = app.emit(
                            "ssh_connection_lost",
                            serde_json::json!({ "session_id": session_id, "reason": msg }),
                        );
                    }
                    break;
                }
                // Non-network error (e.g. a transient permission hiccup on
                // ".") — keep the session registered and just try again
                // next tick.
            }
        }
    });
}

/// Establishes (or reuses) the unified per-`session_id` SSH session and
/// lazily opens its SFTP subsystem. Session storage moved from the old
/// dedicated `SftpState` into `SshState` (the same registry the terminal path
/// uses) per the russh migration's session-model decision — no code path
/// today looks up the same `session_id` from both a terminal tab and a
/// dedicated SFTP tab, so this is a pure simplification with no behavior
/// change for any existing tab.
#[allow(clippy::too_many_arguments)]
pub async fn sftp_connect(
    session_id: String,
    host_id: String,
    passphrase: Option<String>,
    password_override: Option<String>,
    state: &SshState,
    trust_state: &TrustState,
    hosts_db: &crate::modules::hosts::HostsDb,
    secrets: &crate::modules::secrets::SecretsState,
    app: crate::App,
) -> Result<(), LabonairError> {
    // Idempotent: a session already live under this session_id whose SFTP
    // subsystem is already open is left alone instead of dialing a second
    // TCP/SSH connection or reopening the subsystem. Needed for React
    // StrictMode's double-invoke of effects and for lazy sidebar-tree
    // sessions that may be requested more than once in quick succession.
    let existing = {
        let map = state
            .0
            .lock()
            .map_err(|e| LabonairError::Internal(e.to_string()))?;
        map.get(&session_id).cloned()
    };
    if let Some(ref session) = existing {
        if session.sftp.get().is_some() {
            return Ok(());
        }
    }

    // Fetch host from DB (fast, sync).
    let (
        host_address,
        port,
        username,
        auth_method,
        private_key_path,
        keep_alive_interval,
        keep_alive_tries,
        default_path_sftp,
        credential_id,
        jump_host_id,
    ) = {
        let conn = hosts_db
            .0
            .lock()
            .map_err(|e| LabonairError::Internal(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT host_address, port, username, auth_method, private_key_path, \
             keep_alive_interval, keep_alive_tries, default_path_sftp, credential_id, jump_host_id \
             FROM hosts WHERE id = ?1",
        )?;
        stmt.query_row(rusqlite::params![host_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })?
    };

    // Resolve credential overrides.
    let (auth_method, private_key_path) = if let Some(ref cid) = credential_id {
        let conn = hosts_db
            .0
            .lock()
            .map_err(|e| LabonairError::Internal(e.to_string()))?;
        let (cred_type, cred_key_path): (String, Option<String>) = conn
            .query_row(
                "SELECT cred_type, key_path FROM credentials WHERE id=?1",
                rusqlite::params![cid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| {
                LabonairError::Internal(format!(
                    "Credential '{}' not found — it may have been deleted.",
                    cid
                ))
            })?;
        (cred_type, cred_key_path)
    } else {
        (auth_method, private_key_path)
    };

    // Fetch password from keychain.
    let password: Option<String> = if auth_method == "password" {
        if password_override.is_some() {
            password_override.clone()
        } else if let Some(ref cid) = credential_id {
            crate::modules::secrets::get_password(&app, secrets, "labonair-cred", cid)
                .ok()
                .flatten()
        } else {
            crate::modules::secrets::get_password(&app, secrets, "labonair-app", &host_id)
                .ok()
                .flatten()
        }
    } else {
        None
    };

    // Passphrase from credential secret for key auth.
    let passphrase = if credential_id.is_some() && auth_method == "key" && passphrase.is_none() {
        if let Some(ref cid) = credential_id {
            crate::modules::secrets::get_password(&app, secrets, "labonair-cred", cid)
                .ok()
                .flatten()
        } else {
            passphrase
        }
    } else {
        passphrase
    };

    // Resolve jump host fields (if any) — same helper the terminal path uses.
    let jump = match jump_host_id.as_deref() {
        Some(jid) => Some(crate::modules::ssh::client::resolve_jump_host(
            hosts_db, secrets, &app, jid,
        )?),
        None => None,
    };

    let result = sftp_connect_inner(
        session_id.clone(),
        passphrase,
        host_address,
        port,
        username,
        auth_method,
        private_key_path,
        keep_alive_interval,
        keep_alive_tries,
        default_path_sftp,
        password,
        jump,
        existing,
        state.clone(),
        trust_state.clone(),
        app.clone(),
    )
    .await;

    if result.is_ok() {
        let conn = hosts_db
            .0
            .lock()
            .map_err(|e| LabonairError::Internal(e.to_string()))?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let _ = conn.execute(
            "UPDATE hosts SET last_connected_at = ?1 WHERE id = ?2",
            rusqlite::params![now, host_id],
        );
    }

    result.map_err(LabonairError::classify)
}

#[allow(clippy::too_many_arguments)]
async fn sftp_connect_inner(
    session_id: String,
    passphrase: Option<String>,
    host_address: String,
    port: i64,
    username: String,
    auth_method: String,
    private_key_path: Option<String>,
    keep_alive_interval: Option<i64>,
    keep_alive_tries: Option<i64>,
    default_path_sftp: Option<String>,
    password: Option<String>,
    jump: Option<crate::modules::ssh::client::JumpHostParams>,
    existing: Option<Arc<RushSession>>,
    state: SshState,
    trust_state: TrustState,
    app: crate::App,
) -> Result<(), String> {
    let session = match existing {
        Some(session) => session,
        None => {
            // Steps 1-6: shared TCP + SSH + auth flow — the exact same helper
            // the terminal path (`ssh_connect_async`) uses.
            let handle = crate::modules::ssh::client::establish_authenticated_session(
                &session_id,
                &host_address,
                port,
                &username,
                &auth_method,
                private_key_path.as_deref(),
                keep_alive_interval,
                keep_alive_tries,
                password.as_deref(),
                passphrase.as_deref(),
                &trust_state,
                &app,
                true, // fail fast — the sidebar Explorer has no trust-prompt UI of its own
                jump,
                None, // uses the default connect timeout — not wired to a setting on this path
            )
            .await?;

            let session = Arc::new(RushSession {
                handle,
                pty: tokio::sync::Mutex::new(None),
                sftp: tokio::sync::OnceCell::new(),
                shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                disconnect_reason: Arc::new(std::sync::Mutex::new(None)),
                agent_tap: tokio::sync::broadcast::channel(256).0,
            });
            {
                let mut map = state.0.lock().map_err(|e| e.to_string())?;
                map.insert(session_id.clone(), session.clone());
            }
            session
        }
    };

    // Step 7: lazily open the SFTP subsystem on this session's `OnceCell`.
    // Idempotent even if called again concurrently (a racing caller just sees
    // the already-populated cell). If opening the subsystem fails, the cell
    // is left uninitialized so a later `sftp_connect` retries just this step
    // against the already-authenticated handle instead of reconnecting from
    // scratch.
    let app_handle = app.clone();
    let _ = app_handle.emit(
        "ssh_connect_log",
        serde_json::json!({
            "session_id": session_id, "message": "Initialising SFTP subsystem…"
        }),
    );
    session
        .sftp
        .get_or_try_init(|| async {
            let channel = session
                .handle
                .channel_open_session()
                .await
                .map_err(|e| e.to_string())?;
            channel
                .request_subsystem(true, "sftp")
                .await
                .map_err(|e| e.to_string())?;
            let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(Arc::new(sftp))
        })
        .await?;
    let _ = app_handle.emit(
        "ssh_connect_log",
        serde_json::json!({
            "session_id": session_id, "message": "SFTP ready ✓"
        }),
    );

    // `sftp_connect_inner` is only reached when the SFTP subsystem wasn't
    // already open (the outer `sftp_connect` command early-returns Ok(())
    // otherwise) — so this always corresponds to a genuinely fresh open,
    // never a redundant spawn from an idempotent re-call.
    spawn_sftp_health_check(
        session_id.clone(),
        state.clone(),
        app.clone(),
        keep_alive_interval,
        session.clone(),
    );

    app.emit(
        "session_established",
        serde_json::json!({ "session_id": session_id, "default_path_sftp": default_path_sftp }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Removes the unified session from `SshState` and closes the connection. A
/// dedicated SFTP tab's `session_id` is never shared with a terminal or
/// lazy-explorer session today (see the russh migration's session-model
/// decision), so removing the whole entry here is exactly equivalent to the
/// old `SftpState`-only removal from the frontend's perspective.
pub fn sftp_disconnect(session_id: String, state: &SshState) -> Result<(), LabonairError> {
    let mut map = state
        .0
        .lock()
        .map_err(|e| LabonairError::Internal(e.to_string()))?;
    map.remove(&session_id);
    Ok(())
}
