//! Unified local secret storage for all platforms.
//!
//! Secrets are stored in the app's local data directory as either:
//!   - `secrets.json`  — plain JSON (default, protected by OS file permissions)
//!   - `secrets.enc`   — AES-256-GCM encrypted JSON (opt-in via app settings)
//!
//! An app-managed encryption key (`enc_key.bin`, mode 0600 on Unix) is generated
//! once on first run and reused on every subsequent start. No master password is
//! required from the user.
//!
//! Frontend talks to `secrets_get`, `secrets_set`, `secrets_delete`,
//! `secrets_get_all`, `secrets_get_encryption_enabled`, and
//! `secrets_set_encryption_enabled`.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

const PLAIN_FILE: &str = "secrets.json";
const ENC_FILE: &str = "secrets.enc";
const KEY_FILE: &str = "enc_key.bin";
const ENC_FLAG_FILE: &str = "enc_enabled.flag";

pub struct SecretsState {
    data_dir: PathBuf,
    cache: Mutex<Option<HashMap<String, String>>>,
    enc_key: Mutex<Option<[u8; 32]>>,
}

impl SecretsState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            cache: Mutex::new(None),
            enc_key: Mutex::new(None),
        }
    }
}

impl Default for SecretsState {
    fn default() -> Self {
        Self::new(labonair_filesystem::paths::data_dir())
    }
}

// ── Paths ────────────────────────────────────────────────────────────────────

fn plain_path(state: &SecretsState) -> Result<PathBuf, String> {
    Ok(state.data_dir.join(PLAIN_FILE))
}

fn enc_path(state: &SecretsState) -> Result<PathBuf, String> {
    Ok(state.data_dir.join(ENC_FILE))
}

fn key_path(state: &SecretsState) -> Result<PathBuf, String> {
    Ok(state.data_dir.join(KEY_FILE))
}

fn flag_path(state: &SecretsState) -> Result<PathBuf, String> {
    Ok(state.data_dir.join(ENC_FLAG_FILE))
}

// ── Encryption-key lifecycle ─────────────────────────────────────────────────

fn load_or_create_key(state: &SecretsState) -> Result<[u8; 32], String> {
    let path = key_path(state)?;
    if path.exists() {
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    write_protected(&path, &key)?;
    Ok(key)
}

fn get_enc_key(state: &SecretsState) -> Result<[u8; 32], String> {
    let mut guard = state.enc_key.lock().map_err(|e| e.to_string())?;
    if let Some(k) = *guard {
        return Ok(k);
    }
    let k = load_or_create_key(state)?;
    *guard = Some(k);
    Ok(k)
}

// ── File helpers ─────────────────────────────────────────────────────────────

/// Write bytes to a file atomically, mode 0600 on Unix.
fn write_protected(path: &PathBuf, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        f.write_all(data).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        f.write_all(data).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Encryption / decryption ───────────────────────────────────────────────────

fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| e.to_string())?;
    // Layout: nonce (12 bytes) || ciphertext
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 12 {
        return Err("encrypted data too short".to_string());
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())
}

// ── Store read / write ────────────────────────────────────────────────────────

fn is_encryption_enabled(state: &SecretsState) -> bool {
    flag_path(state).map(|p| p.exists()).unwrap_or(false)
}

fn read_map(state: &SecretsState) -> Result<HashMap<String, String>, String> {
    if is_encryption_enabled(state) {
        let path = enc_path(state)?;
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let raw = fs::read(&path).map_err(|e| e.to_string())?;
        let key = get_enc_key(state)?;
        let json_bytes = decrypt(&key, &raw)?;
        serde_json::from_slice(&json_bytes).map_err(|e| e.to_string())
    } else {
        let path = plain_path(state)?;
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}

fn write_map(state: &SecretsState, map: &HashMap<String, String>) -> Result<(), String> {
    let json = serde_json::to_vec(map).map_err(|e| e.to_string())?;
    if is_encryption_enabled(state) {
        let key = get_enc_key(state)?;
        let encrypted = encrypt(&key, &json)?;
        write_protected(&enc_path(state)?, &encrypted)
    } else {
        write_protected(&plain_path(state)?, &json)
    }
}

// ── Cache helpers ─────────────────────────────────────────────────────────────

fn with_cache<F, R>(state: &SecretsState, f: F) -> Result<R, String>
where
    F: FnOnce(&mut HashMap<String, String>) -> R,
{
    let mut guard = state.cache.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(read_map(state)?);
    }
    Ok(f(guard.as_mut().expect("initialized above")))
}

fn invalidate_cache(state: &SecretsState) {
    if let Ok(mut g) = state.cache.lock() {
        *g = None;
    }
}

fn composite_key(service: &str, account: &str) -> String {
    format!("{}::{}", service, account)
}

// ── Tauri commands ────────────────────────────────────────────────────────────

pub async fn secrets_get(
    state: &SecretsState,
    service: String,
    account: String,
) -> Result<Option<String>, String> {
    let k = composite_key(&service, &account);
    with_cache(state, |m| m.get(&k).cloned())
}

pub async fn secrets_set(
    state: &SecretsState,
    service: String,
    account: String,
    password: String,
) -> Result<(), String> {
    let k = composite_key(&service, &account);
    with_cache(state, |m| {
        m.insert(k, password);
    })?;
    let snapshot = {
        let guard = state.cache.lock().map_err(|e| e.to_string())?;
        guard.as_ref().cloned().unwrap_or_default()
    };
    write_map(state, &snapshot)
}

pub async fn secrets_delete(
    state: &SecretsState,
    service: String,
    account: String,
) -> Result<(), String> {
    let k = composite_key(&service, &account);
    with_cache(state, |m| {
        m.remove(&k);
    })?;
    let snapshot = {
        let guard = state.cache.lock().map_err(|e| e.to_string())?;
        guard.as_ref().cloned().unwrap_or_default()
    };
    write_map(state, &snapshot)
}

pub async fn secrets_get_all(
    state: &SecretsState,
    service: String,
    accounts: Vec<String>,
) -> Result<Vec<Option<String>>, String> {
    with_cache(state, |m| {
        accounts
            .iter()
            .map(|a| m.get(&composite_key(&service, a)).cloned())
            .collect()
    })
}

/// Returns whether AES-256-GCM encryption is currently enabled.
pub async fn secrets_get_encryption_enabled(state: &SecretsState) -> Result<bool, String> {
    Ok(is_encryption_enabled(state))
}

/// Enable or disable AES-256-GCM encryption.
/// Migrates the existing store to the new format atomically.
pub async fn secrets_set_encryption_enabled(
    state: &SecretsState,
    enabled: bool,
) -> Result<(), String> {
    let currently = is_encryption_enabled(state);
    if currently == enabled {
        return Ok(());
    }

    // Read current secrets (from whichever format is active now).
    let map = read_map(state)?;

    if enabled {
        // Write encrypted file, set flag, remove plain file.
        let key = get_enc_key(state)?;
        let json = serde_json::to_vec(&map).map_err(|e| e.to_string())?;
        let encrypted = encrypt(&key, &json)?;
        write_protected(&enc_path(state)?, &encrypted)?;
        write_protected(&flag_path(state)?, b"1")?;
        let _ = fs::remove_file(plain_path(state)?);
    } else {
        // Write plain file, remove flag and encrypted file.
        let json = serde_json::to_vec(&map).map_err(|e| e.to_string())?;
        write_protected(&plain_path(state)?, &json)?;
        let _ = fs::remove_file(flag_path(state)?);
        let _ = fs::remove_file(enc_path(state)?);
    }

    invalidate_cache(state);
    Ok(())
}

// ── Keychain migration (nexum-* → labonair-*) ────────────────────────────────

/// Migrate all stored secrets from old nexum-* service names to labonair-*.
/// Called once on first launch after the rename. Safe to call multiple times
/// (subsequent calls are no-ops because old keys will already be gone).
#[allow(dead_code)] // wired into app startup by a later task
pub fn migrate_service_names(state: &SecretsState) {
    let renames = [
        ("nexum-app", "labonair-app"),
        ("nexum-cred", "labonair-cred"),
        ("nexum-sudo", "labonair-sudo"),
    ];
    // Read the current map; skip silently on error (first run, empty store, etc.)
    let Ok(mut map) = read_map(state) else {
        return;
    };
    let mut changed = false;
    for (old_svc, new_svc) in &renames {
        let old_prefix = format!("{}::", old_svc);
        let new_prefix = format!("{}::", new_svc);
        // Collect keys that need renaming (avoid mutating while iterating).
        let to_rename: Vec<String> = map
            .keys()
            .filter(|k| k.starts_with(&old_prefix))
            .cloned()
            .collect();
        for old_key in to_rename {
            let new_key = format!("{}{}", new_prefix, &old_key[old_prefix.len()..]);
            if let Some(value) = map.remove(&old_key) {
                map.insert(new_key, value);
                changed = true;
            }
        }
    }
    if changed {
        let _ = write_map(state, &map);
        // Invalidate the in-memory cache so subsequent reads see the new keys.
        invalidate_cache(state);
    }
}

// ── Internal helpers for Rust callers (db.rs, client.rs) ─────────────────────

pub fn store_password(
    state: &SecretsState,
    service: &str,
    account: &str,
    password: &str,
) -> Result<(), String> {
    let k = composite_key(service, account);
    with_cache(state, |m| {
        m.insert(k, password.to_string());
    })?;
    let snapshot = {
        let guard = state.cache.lock().map_err(|e| e.to_string())?;
        guard.as_ref().cloned().unwrap_or_default()
    };
    write_map(state, &snapshot)
}

pub fn delete_password(state: &SecretsState, service: &str, account: &str) -> Result<(), String> {
    let k = composite_key(service, account);
    with_cache(state, |m| {
        m.remove(&k);
    })?;
    let snapshot = {
        let guard = state.cache.lock().map_err(|e| e.to_string())?;
        guard.as_ref().cloned().unwrap_or_default()
    };
    write_map(state, &snapshot)
}

pub fn get_password(
    state: &SecretsState,
    service: &str,
    account: &str,
) -> Result<Option<String>, String> {
    let k = composite_key(service, account);
    with_cache(state, |m| m.get(&k).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "labonair-secrets-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time must be after the Unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("test directory must be creatable");
        dir
    }

    #[tokio::test]
    async fn plain_store_round_trips_in_state_directory() {
        let dir = test_dir("plain");
        let state = SecretsState::new(dir.clone());

        secrets_set(
            &state,
            "service".to_string(),
            "account".to_string(),
            "password".to_string(),
        )
        .await
        .expect("secret write must succeed");

        assert_eq!(
            secrets_get(&state, "service".to_string(), "account".to_string())
                .await
                .expect("secret read must succeed"),
            Some("password".to_string())
        );
        assert!(dir.join(PLAIN_FILE).is_file());
        assert!(!dir.join(ENC_FILE).exists());

        std::fs::remove_dir_all(dir).expect("test directory must be removable");
    }

    #[tokio::test]
    async fn encryption_toggle_migrates_without_losing_values() {
        let dir = test_dir("encryption");
        let state = SecretsState::new(dir.clone());

        store_password(&state, "service", "account", "password")
            .expect("secret write must succeed");
        secrets_set_encryption_enabled(&state, true)
            .await
            .expect("encryption must enable");

        assert!(secrets_get_encryption_enabled(&state)
            .await
            .expect("encryption status must be readable"));
        assert!(!dir.join(PLAIN_FILE).exists());
        assert!(dir.join(ENC_FILE).is_file());
        assert_eq!(
            get_password(&state, "service", "account").expect("secret read must succeed"),
            Some("password".to_string())
        );

        secrets_set_encryption_enabled(&state, false)
            .await
            .expect("encryption must disable");
        assert!(!dir.join(ENC_FILE).exists());
        assert!(dir.join(PLAIN_FILE).is_file());

        std::fs::remove_dir_all(dir).expect("test directory must be removable");
    }
}
