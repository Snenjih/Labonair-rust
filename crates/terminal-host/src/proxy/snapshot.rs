//! Builds the [`SshHostSnapshot`] a daemon-hosted connect needs, from the
//! app's own hosts database and secret store (what `ssh_connect` reads). The
//! daemon never opens either. Secrets travel in the snapshot only; `Debug` of
//! every carrier redacts them.

use labonair_persistence::Database;
use labonair_secrets::{get_password, SecretsState};
use rusqlite::{params, Connection, OptionalExtension};

use crate::protocol::{CredentialRecord, HostRecord, SecretEntry, SshHostSnapshot};

const HOST_SECRET: &str = "labonair-app";
const CREDENTIAL_SECRET: &str = "labonair-cred";

fn host_row(conn: &Connection, id: &str) -> Result<Option<HostRecord>, String> {
    conn.query_row(
        "SELECT id, name, host_address, port, username, auth_method, private_key_path, \
         keep_alive_interval, keep_alive_tries, default_path_ssh, credential_id, jump_host_id, \
         tunnels FROM hosts WHERE id = ?1",
        params![id],
        |row| {
            Ok(HostRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                host_address: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                auth_method: row.get(5)?,
                private_key_path: row.get(6)?,
                keep_alive_interval: row.get(7)?,
                keep_alive_tries: row.get(8)?,
                default_path_ssh: row.get(9)?,
                credential_id: row.get(10)?,
                jump_host_id: row.get(11)?,
                tunnels: row.get(12)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn credential_row(conn: &Connection, id: &str) -> Result<Option<CredentialRecord>, String> {
    conn.query_row(
        "SELECT id, name, cred_type, key_path, has_secret FROM credentials WHERE id = ?1",
        params![id],
        |row| {
            Ok(CredentialRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                cred_type: row.get(2)?,
                key_path: row.get(3)?,
                has_secret: row.get::<_, i64>(4)? != 0,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// The host, its jump host (single hop, like the transport), the credentials
/// they reference and the secrets a connect would read from the store.
pub fn build(
    db: &Database,
    secrets: &SecretsState,
    host_id: &str,
) -> Result<SshHostSnapshot, String> {
    let mut snapshot = SshHostSnapshot::default();
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let host =
            host_row(&conn, host_id)?.ok_or_else(|| format!("host '{host_id}' not found"))?;
        // Single hop: a jump host's own jump host is never followed.
        let jump = match host.jump_host_id.as_deref() {
            Some(jump_id) => Some(
                host_row(&conn, jump_id)?
                    .ok_or_else(|| format!("Jump host '{jump_id}' not found"))?,
            ),
            None => None,
        };
        for record in std::iter::once(&host).chain(jump.as_ref()) {
            if let Some(credential_id) = &record.credential_id {
                let credential = credential_row(&conn, credential_id)?.ok_or_else(|| {
                    format!(
                        "Credential '{credential_id}' not found — it may have been deleted. \
                         Please update the host's auth settings."
                    )
                })?;
                if !snapshot.credentials.iter().any(|c| c.id == credential.id) {
                    snapshot.credentials.push(credential);
                }
            }
        }
        snapshot.hosts.push(host);
        snapshot.hosts.extend(jump);
    }
    let mut add_secret = |service: &str, account: &str| {
        if let Ok(Some(value)) = get_password(secrets, service, account) {
            snapshot.secrets.push(SecretEntry {
                service: service.to_string(),
                account: account.to_string(),
                value,
            });
        }
    };
    let (hosts, credentials) = (snapshot.hosts.clone(), snapshot.credentials.clone());
    for host in &hosts {
        if host.auth_method == "password" {
            add_secret(HOST_SECRET, &host.id);
        }
    }
    for credential in credentials.iter().filter(|c| c.has_secret) {
        add_secret(CREDENTIAL_SECRET, &credential.id);
    }
    Ok(snapshot)
}

/// Record a successful connect in the app's hosts table (the recents order),
/// which the in-process transport does itself.
pub fn record_connected(db: &Database, host_id: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    if let Ok(conn) = db.0.lock() {
        let _ = conn.execute(
            "UPDATE hosts SET last_connected_at = ?1 WHERE id = ?2",
            params![now, host_id],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn db() -> Database {
        Database(Arc::new(Mutex::new(
            labonair_persistence::initialize_in_memory_database().unwrap(),
        )))
    }

    fn insert_host(db: &Database, id: &str, auth: &str, cred: Option<&str>, jump: Option<&str>) {
        db.0.lock()
            .unwrap()
            .execute(
                "INSERT INTO hosts (id, name, host_address, port, username, auth_method, \
                 credential_id, jump_host_id, created_at) VALUES (?1, ?1, 'h', 22, 'u', ?2, ?3, ?4, 0)",
                params![id, auth, cred, jump],
            )
            .unwrap();
    }

    #[test]
    fn collects_host_jump_host_credential_and_only_the_secrets_a_connect_reads() {
        let db = db();
        let secrets = SecretsState::in_memory();
        db.0.lock()
            .unwrap()
            .execute(
                "INSERT INTO credentials (id, name, cred_type, has_secret, created_at) \
                 VALUES ('c1', 'key', 'key', 1, 0)",
                [],
            )
            .unwrap();
        insert_host(&db, "jump", "password", None, None);
        insert_host(&db, "target", "key", Some("c1"), Some("jump"));
        labonair_secrets::store_password(&secrets, HOST_SECRET, "jump", "jump-pw").unwrap();
        labonair_secrets::store_password(&secrets, CREDENTIAL_SECRET, "c1", "passphrase").unwrap();
        // Not read by a key-auth connect: must not travel.
        labonair_secrets::store_password(&secrets, HOST_SECRET, "target", "unused").unwrap();

        let snapshot = build(&db, &secrets, "target").unwrap();
        let ids: Vec<_> = snapshot.hosts.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["target", "jump"]);
        assert_eq!(snapshot.credentials.len(), 1);
        let secret_ids: Vec<_> = snapshot
            .secrets
            .iter()
            .map(|s| format!("{}/{}", s.service, s.account))
            .collect();
        assert_eq!(secret_ids, ["labonair-app/jump", "labonair-cred/c1"]);
        // Debug never shows a secret value.
        let debug = format!("{snapshot:?}");
        assert!(!debug.contains("jump-pw") && !debug.contains("passphrase"));
    }

    #[test]
    fn unknown_host_is_an_error() {
        let err = build(&db(), &SecretsState::in_memory(), "nope").unwrap_err();
        assert!(err.contains("not found"));
    }
}
