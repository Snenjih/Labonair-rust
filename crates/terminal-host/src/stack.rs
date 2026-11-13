//! The concrete SSH / SFTP / transfer stack for the `labonair-host` binary:
//! the same adapters the app composes in `labonair-shell`, wired with
//! daemon-side equivalents of what the app provides.
//!
//! * a private in-memory hosts/credentials database (rows arrive with each
//!   connect request, see [`install_host`]) instead of the app's SQLite file;
//! * a RAM-only secret store ([`SecretsState::in_memory`]): the daemon never
//!   reads the app's secrets and never writes any;
//! * its own `EventBus`, `SshState`, `TrustState`, tunnel state and transfer
//!   worker. Transfer history stays in memory, as it does in the app.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use labonair_events::EventBus;
use labonair_persistence::Database;
use labonair_secrets::SecretsState;
use labonair_ssh_transport::contract::{
    SshConnectionServiceAdapter, SshPtyServiceAdapter, SshRemoteServiceAdapter,
    SshTunnelServiceAdapter,
};
use labonair_ssh_transport::tunnels::TunnelState;
use labonair_ssh_transport::{SshState, TrustState};
use labonair_transfers::{ConflictMap, TransferSettings, TransferWorkerState, WorkerMessage};

use crate::protocol::SshHostSnapshot;
use crate::server::HostServices;

const DEFAULT_PORT: i64 = 22;
const DEFAULT_KEEP_ALIVE_INTERVAL: i64 = 25;
const DEFAULT_KEEP_ALIVE_TRIES: i64 = 3;

/// Build the concrete services and start the transfer worker. Must be called
/// inside a tokio runtime.
pub fn concrete_services() -> Result<HostServices, String> {
    let events = EventBus::new();
    let db = Database(Arc::new(Mutex::new(
        labonair_persistence::initialize_in_memory_database()?,
    )));
    let secrets = Arc::new(SecretsState::in_memory());
    let ssh = SshState::default();
    let trust = TrustState::default();
    let tunnels = TunnelState::default();

    let (sender, receiver) = tokio::sync::mpsc::channel::<WorkerMessage>(100);
    let conflicts: ConflictMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let transfer_settings = Arc::new(TransferSettings::default());
    tokio::spawn(labonair_transfers_ssh::run_worker(
        receiver,
        ssh.clone(),
        events.clone(),
        conflicts.clone(),
        transfer_settings.clone(),
    ));
    let worker = TransferWorkerState {
        sender,
        conflicts,
        settings: transfer_settings.clone(),
    };

    let remote = Arc::new(SshRemoteServiceAdapter::new(ssh.clone(), events.clone()));
    let sftp = Arc::new(labonair_sftp_ssh::contract::SftpTransportService::new(
        ssh.clone(),
        events.clone(),
    ));
    let install_host = {
        let db = db.clone();
        let secrets = Arc::clone(&secrets);
        Arc::new(move |snapshot: &SshHostSnapshot| install_host(&db, &secrets, snapshot))
    };
    Ok(HostServices {
        connection: Arc::new(SshConnectionServiceAdapter::new(
            ssh.clone(),
            trust.clone(),
            db.clone(),
            Arc::clone(&secrets),
            events.clone(),
        )),
        pty: Arc::new(SshPtyServiceAdapter::new(ssh)),
        remote_command: remote.clone(),
        remote_file: remote,
        tunnels: Arc::new(SshTunnelServiceAdapter::new(
            tunnels,
            db,
            secrets,
            trust,
            events.clone(),
        )),
        sftp_session: sftp.clone(),
        sftp_browser: sftp,
        transfers: Arc::new(labonair_transfers_ssh::adapter::TransferServiceAdapter::new(worker)),
        transfer_settings,
        install_host,
        events,
    })
}

/// Upsert the rows and secrets of a request into the daemon's RAM-only stores.
/// Upserts (not `INSERT OR REPLACE`) so a row another host points at is never
/// deleted and re-created underneath it.
pub fn install_host(
    db: &Database,
    secrets: &SecretsState,
    snapshot: &SshHostSnapshot,
) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        for credential in &snapshot.credentials {
            if credential.id.is_empty() {
                return Err("credential without an id".into());
            }
            conn.execute(
                "INSERT INTO credentials (id, name, cred_type, key_path, has_secret, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 0) \
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, cred_type = excluded.cred_type, \
                 key_path = excluded.key_path, has_secret = excluded.has_secret",
                rusqlite::params![
                    credential.id,
                    credential.name,
                    credential.cred_type,
                    credential.key_path,
                    credential.has_secret,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        for host in &snapshot.hosts {
            if host.id.is_empty() {
                return Err("host without an id".into());
            }
            let port = if host.port > 0 {
                host.port
            } else {
                DEFAULT_PORT
            };
            let auth_method = if host.auth_method.is_empty() {
                "password"
            } else {
                host.auth_method.as_str()
            };
            conn.execute(
                "INSERT INTO hosts (id, name, host_address, port, username, auth_method, \
                 private_key_path, keep_alive_interval, keep_alive_tries, default_path_ssh, \
                 credential_id, jump_host_id, tunnels, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0) \
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, \
                 host_address = excluded.host_address, port = excluded.port, \
                 username = excluded.username, auth_method = excluded.auth_method, \
                 private_key_path = excluded.private_key_path, \
                 keep_alive_interval = excluded.keep_alive_interval, \
                 keep_alive_tries = excluded.keep_alive_tries, \
                 default_path_ssh = excluded.default_path_ssh, \
                 credential_id = excluded.credential_id, jump_host_id = excluded.jump_host_id, \
                 tunnels = excluded.tunnels",
                rusqlite::params![
                    host.id,
                    host.name,
                    host.host_address,
                    port,
                    host.username,
                    auth_method,
                    host.private_key_path,
                    // Keepalive stays on unless the record says otherwise.
                    host.keep_alive_interval
                        .unwrap_or(DEFAULT_KEEP_ALIVE_INTERVAL),
                    host.keep_alive_tries.unwrap_or(DEFAULT_KEEP_ALIVE_TRIES),
                    host.default_path_ssh,
                    host.credential_id,
                    host.jump_host_id,
                    host.tunnels,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    for secret in &snapshot.secrets {
        labonair_secrets::store_password(secrets, &secret.service, &secret.account, &secret.value)?;
    }
    Ok(())
}
