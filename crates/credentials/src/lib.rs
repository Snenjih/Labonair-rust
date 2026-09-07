//! Credential management for SSH and SFTP capabilities.
//!
//! This crate owns credential metadata, secret references, and generated key
//! material. It has no application or UI dependency; composition-specific
//! adapters belong in named integration siblings composed by the application
//! root.

use base64::Engine as _;
use labonair_persistence::Database;
use labonair_secrets::{delete_password, store_password, SecretsState};
use std::path::Path;

const CRED_SERVICE: &str = "labonair-cred";
const SELECT_CREDS: &str =
    "SELECT id, name, cred_type, key_path, key_type, public_key, has_secret, created_at FROM credentials";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Credential {
    pub id: String,
    pub name: String,
    pub cred_type: String,
    pub key_path: Option<String>,
    pub key_type: Option<String>,
    pub public_key: Option<String>,
    pub has_secret: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HostRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerateKeypairResult {
    pub key_path: String,
    pub public_key: String,
}

fn row_to_credential(row: &rusqlite::Row<'_>) -> rusqlite::Result<Credential> {
    Ok(Credential {
        id: row.get(0)?,
        name: row.get(1)?,
        cred_type: row.get(2)?,
        key_path: row.get(3)?,
        key_type: row.get(4)?,
        public_key: row.get(5)?,
        has_secret: row
            .get::<_, i64>(6)
            .map(|value| value != 0)
            .unwrap_or(false),
        created_at: row.get(7)?,
    })
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Return all credential metadata. Secret values never enter this model.
pub async fn credentials_get_all(database: &Database) -> Result<Vec<Credential>, String> {
    let connection = database.0.lock().map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare(&format!("{SELECT_CREDS} ORDER BY name ASC"))
        .map_err(|error| error.to_string())?;
    let credentials = statement
        .query_map([], row_to_credential)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(credentials)
}

#[allow(clippy::too_many_arguments)]
pub async fn credentials_create(
    database: &Database,
    secrets: &SecretsState,
    name: String,
    cred_type: String,
    key_path: Option<String>,
    key_type: Option<String>,
    public_key: Option<String>,
    secret: Option<String>,
) -> Result<Credential, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = now_millis();
    let has_secret = secret
        .as_deref()
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    {
        let connection = database.0.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO credentials (id, name, cred_type, key_path, key_type, public_key, has_secret, created_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite::params![
                    id,
                    name,
                    cred_type,
                    key_path,
                    key_type,
                    public_key,
                    has_secret as i64,
                    created_at
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(secret) = secret {
        if !secret.is_empty() {
            store_password(secrets, CRED_SERVICE, &id, &secret)?;
        }
    }
    let connection = database.0.lock().map_err(|error| error.to_string())?;
    connection
        .query_row(
            &format!("{SELECT_CREDS} WHERE id=?1"),
            rusqlite::params![id],
            row_to_credential,
        )
        .map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
pub async fn credentials_update(
    database: &Database,
    secrets: &SecretsState,
    id: String,
    name: Option<String>,
    cred_type: Option<String>,
    key_path: Option<String>,
    key_type: Option<String>,
    public_key: Option<String>,
    secret: Option<String>,
) -> Result<Credential, String> {
    {
        let connection = database.0.lock().map_err(|error| error.to_string())?;
        if let Some(value) = &name {
            connection
                .execute(
                    "UPDATE credentials SET name=?1 WHERE id=?2",
                    rusqlite::params![value, id],
                )
                .map_err(|error| error.to_string())?;
        }
        if let Some(value) = &cred_type {
            connection
                .execute(
                    "UPDATE credentials SET cred_type=?1 WHERE id=?2",
                    rusqlite::params![value, id],
                )
                .map_err(|error| error.to_string())?;
        }
        if key_path.is_some() {
            connection
                .execute(
                    "UPDATE credentials SET key_path=?1 WHERE id=?2",
                    rusqlite::params![key_path, id],
                )
                .map_err(|error| error.to_string())?;
        }
        if key_type.is_some() {
            connection
                .execute(
                    "UPDATE credentials SET key_type=?1 WHERE id=?2",
                    rusqlite::params![key_type, id],
                )
                .map_err(|error| error.to_string())?;
        }
        if public_key.is_some() {
            connection
                .execute(
                    "UPDATE credentials SET public_key=?1 WHERE id=?2",
                    rusqlite::params![public_key, id],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    if let Some(secret) = secret {
        if secret.is_empty() {
            let _ = delete_password(secrets, CRED_SERVICE, &id);
            let connection = database.0.lock().map_err(|error| error.to_string())?;
            let _ = connection.execute(
                "UPDATE credentials SET has_secret=0 WHERE id=?1",
                rusqlite::params![id],
            );
        } else {
            store_password(secrets, CRED_SERVICE, &id, &secret)?;
            let connection = database.0.lock().map_err(|error| error.to_string())?;
            let _ = connection.execute(
                "UPDATE credentials SET has_secret=1 WHERE id=?1",
                rusqlite::params![id],
            );
        }
    }
    let connection = database.0.lock().map_err(|error| error.to_string())?;
    connection
        .query_row(
            &format!("{SELECT_CREDS} WHERE id=?1"),
            rusqlite::params![id],
            row_to_credential,
        )
        .map_err(|error| error.to_string())
}

pub async fn credentials_delete(
    database: &Database,
    secrets: &SecretsState,
    data_dir: &Path,
    id: String,
) -> Result<(), String> {
    let key_path: Option<String> = {
        let connection = database.0.lock().map_err(|error| error.to_string())?;
        connection
            .query_row(
                "SELECT key_path FROM credentials WHERE id=?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten()
    };
    {
        let connection = database.0.lock().map_err(|error| error.to_string())?;
        connection
            .execute("DELETE FROM credentials WHERE id=?1", rusqlite::params![id])
            .map_err(|error| error.to_string())?;
    }
    let _ = delete_password(secrets, CRED_SERVICE, &id);

    if let Some(path) = key_path {
        let keys_dir = data_dir.join("keys");
        let key_file = Path::new(&path);
        if key_file.starts_with(&keys_dir) {
            let _ = std::fs::remove_file(key_file);
        }
    }
    Ok(())
}

pub async fn credentials_get_hosts_using(
    database: &Database,
    id: String,
) -> Result<Vec<HostRef>, String> {
    let connection = database.0.lock().map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare("SELECT id, name FROM hosts WHERE credential_id=?1 ORDER BY name")
        .map_err(|error| error.to_string())?;
    let references = statement
        .query_map(rusqlite::params![id], |row| {
            Ok(HostRef {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(references)
}

fn ssh_wire_string(value: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(4 + value.len());
    result.extend_from_slice(&(value.len() as u32).to_be_bytes());
    result.extend_from_slice(value);
    result
}

fn ssh_wire_mpint(bytes: &[u8]) -> Vec<u8> {
    let position = bytes
        .iter()
        .position(|&byte| byte != 0)
        .unwrap_or(bytes.len().saturating_sub(1));
    let trimmed = &bytes[position..];
    if trimmed.is_empty() || (trimmed[0] & 0x80 != 0) {
        let mut padded = vec![0u8];
        padded.extend_from_slice(trimmed);
        ssh_wire_string(&padded)
    } else {
        ssh_wire_string(trimmed)
    }
}

fn pkey_to_openssh_pubkey(
    pkey: &openssl::pkey::PKey<openssl::pkey::Private>,
    key_type: &str,
) -> Result<String, String> {
    let mut wire = Vec::new();
    match key_type {
        "ed25519" => {
            let raw = pkey.raw_public_key().map_err(|error| error.to_string())?;
            wire.extend(ssh_wire_string(b"ssh-ed25519"));
            wire.extend(ssh_wire_string(&raw));
        }
        "rsa-4096" => {
            let rsa = pkey.rsa().map_err(|error| error.to_string())?;
            wire.extend(ssh_wire_string(b"ssh-rsa"));
            wire.extend(ssh_wire_mpint(&rsa.e().to_vec()));
            wire.extend(ssh_wire_mpint(&rsa.n().to_vec()));
        }
        _ => return Err(format!("Unknown key_type: {key_type}")),
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(&wire);
    let prefix = if key_type == "ed25519" {
        "ssh-ed25519"
    } else {
        "ssh-rsa"
    };
    Ok(format!("{prefix} {encoded} labonair-generated"))
}

/// Generate a private key in the app data directory and persist its public
/// metadata. The caller supplies the directory so this crate remains
/// independent of process-global application state.
pub async fn credential_generate_keypair(
    data_dir: &Path,
    database: &Database,
    secrets: &SecretsState,
    cred_id: String,
    key_type: String,
    passphrase: Option<String>,
) -> Result<GenerateKeypairResult, String> {
    let private_key: openssl::pkey::PKey<openssl::pkey::Private> = match key_type.as_str() {
        "ed25519" => openssl::pkey::PKey::generate_ed25519().map_err(|error| error.to_string())?,
        "rsa-4096" => {
            let rsa = openssl::rsa::Rsa::generate(4096).map_err(|error| error.to_string())?;
            openssl::pkey::PKey::from_rsa(rsa).map_err(|error| error.to_string())?
        }
        _ => return Err(format!("Unknown key_type: {key_type}")),
    };
    let pem = match passphrase.as_deref().filter(|value| !value.is_empty()) {
        Some(passphrase) => private_key
            .private_key_to_pem_pkcs8_passphrase(
                openssl::symm::Cipher::aes_256_cbc(),
                passphrase.as_bytes(),
            )
            .map_err(|error| error.to_string())?,
        None => private_key
            .private_key_to_pem_pkcs8()
            .map_err(|error| error.to_string())?,
    };
    let keys_dir = data_dir.join("keys");
    std::fs::create_dir_all(&keys_dir).map_err(|error| error.to_string())?;
    let key_path = keys_dir.join(&cred_id);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&key_path)
            .map_err(|error| error.to_string())?;
        std::io::Write::write_all(&mut file, &pem).map_err(|error| error.to_string())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&key_path, &pem).map_err(|error| error.to_string())?;

    let key_path_string = key_path.to_string_lossy().to_string();
    let public_key = pkey_to_openssh_pubkey(&private_key, &key_type)?;
    {
        let connection = database.0.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE credentials SET key_path=?1, key_type=?2, public_key=?3 WHERE id=?4",
                rusqlite::params![key_path_string, key_type, public_key, cred_id],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(passphrase) = passphrase.as_deref().filter(|value| !value.is_empty()) {
        store_password(secrets, CRED_SERVICE, &cred_id, passphrase)?;
        let connection = database.0.lock().map_err(|error| error.to_string())?;
        let _ = connection.execute(
            "UPDATE credentials SET has_secret=1 WHERE id=?1",
            rusqlite::params![cred_id],
        );
    }
    Ok(GenerateKeypairResult {
        key_path: key_path_string,
        public_key,
    })
}

#[cfg(test)]
mod tests {
    fn pem_for(key_type: &str, passphrase: Option<&str>) -> String {
        let key = match key_type {
            "ed25519" => openssl::pkey::PKey::generate_ed25519().unwrap(),
            "rsa-4096" => {
                let rsa = openssl::rsa::Rsa::generate(4096).unwrap();
                openssl::pkey::PKey::from_rsa(rsa).unwrap()
            }
            _ => panic!("unknown key type"),
        };
        let pem = match passphrase {
            Some(passphrase) => key
                .private_key_to_pem_pkcs8_passphrase(
                    openssl::symm::Cipher::aes_256_cbc(),
                    passphrase.as_bytes(),
                )
                .unwrap(),
            None => key.private_key_to_pem_pkcs8().unwrap(),
        };
        String::from_utf8(pem).unwrap()
    }

    #[test]
    fn generated_ed25519_key_is_compatible_with_russh() {
        let pem = pem_for("ed25519", None);
        russh::keys::decode_secret_key(&pem, None).expect("generated key decodes in russh");
    }

    #[test]
    fn generated_encrypted_key_requires_the_right_passphrase() {
        let pem = pem_for("ed25519", Some("correct horse battery staple"));
        assert!(russh::keys::decode_secret_key(&pem, Some("wrong password")).is_err());
        russh::keys::decode_secret_key(&pem, Some("correct horse battery staple"))
            .expect("correct passphrase decodes key");
    }
}
