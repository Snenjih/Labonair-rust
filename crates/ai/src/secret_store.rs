//! BYOK key storage. Keys live in the OS keyring (`keyring` crate) under the
//! `labonair-ai` service and are **never** written to disk, logs, or app state.
//!
//! The reference stores per-instance keys under account `inst-<instanceId>` and
//! (legacy) per-provider keys under `<provider>-api-key`; both schemes are kept.

use crate::config::{ProviderId, KEYRING_SERVICE};

/// Abstraction over the secret backend so tests can run without a real keyring.
pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Option<String>;
    fn set(&self, account: &str, value: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
}

fn inst_account(instance_id: &str) -> String {
    format!("inst-{instance_id}")
}

// ── Per-instance keys (the current model) ────────────────────────────────────

pub fn get_instance_key(store: &dyn SecretStore, instance_id: &str) -> Option<String> {
    store
        .get(&inst_account(instance_id))
        .filter(|v| !v.is_empty())
}

pub fn set_instance_key(
    store: &dyn SecretStore,
    instance_id: &str,
    key: &str,
) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key is empty".to_string());
    }
    store.set(&inst_account(instance_id), trimmed)
}

pub fn clear_instance_key(store: &dyn SecretStore, instance_id: &str) -> Result<(), String> {
    store.delete(&inst_account(instance_id))
}

// ── Legacy per-provider keys ────────────────────────────────────────────────

pub fn get_provider_key(store: &dyn SecretStore, provider: ProviderId) -> Option<String> {
    if !provider.needs_key() {
        return None;
    }
    store
        .get(provider.keyring_account())
        .filter(|v| !v.is_empty())
}

pub fn set_provider_key(
    store: &dyn SecretStore,
    provider: ProviderId,
    key: &str,
) -> Result<(), String> {
    if !provider.needs_key() {
        return Err(format!("{} does not use an API key", provider.as_str()));
    }
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key is empty".to_string());
    }
    store.set(provider.keyring_account(), trimmed)
}

pub fn clear_provider_key(store: &dyn SecretStore, provider: ProviderId) -> Result<(), String> {
    if !provider.needs_key() {
        return Ok(());
    }
    store.delete(provider.keyring_account())
}

// ── Real OS keyring implementation ──────────────────────────────────────────

/// Stores secrets in the OS keychain via the `keyring` crate.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    fn entry(account: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(KEYRING_SERVICE, account).map_err(|e| e.to_string())
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self, account: &str) -> Option<String> {
        match Self::entry(account).ok()?.get_password() {
            Ok(v) => Some(v),
            Err(keyring::Error::NoEntry) => None,
            Err(_) => None,
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        Self::entry(account)?
            .set_password(value)
            .map_err(|e| e.to_string())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

// ── In-memory implementation for tests ──────────────────────────────────────

/// Non-persistent store — used by unit tests and safe as a fallback.
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    inner: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl SecretStore for MemorySecretStore {
    fn get(&self, account: &str) -> Option<String> {
        self.inner.lock().unwrap().get(account).cloned()
    }
    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        self.inner
            .lock()
            .unwrap()
            .insert(account.to_string(), value.to_string());
        Ok(())
    }
    fn delete(&self, account: &str) -> Result<(), String> {
        self.inner.lock().unwrap().remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_key_lifecycle() {
        let store = MemorySecretStore::default();
        assert_eq!(get_instance_key(&store, "abc"), None);
        set_instance_key(&store, "abc", "  sk-test-123  ").unwrap();
        assert_eq!(
            get_instance_key(&store, "abc").as_deref(),
            Some("sk-test-123")
        );
        clear_instance_key(&store, "abc").unwrap();
        assert_eq!(get_instance_key(&store, "abc"), None);
    }

    #[test]
    fn empty_key_is_rejected() {
        let store = MemorySecretStore::default();
        assert!(set_instance_key(&store, "abc", "   ").is_err());
    }

    #[test]
    fn keyless_provider_cannot_set_key() {
        let store = MemorySecretStore::default();
        assert!(set_provider_key(&store, ProviderId::Ollama, "x").is_err());
        assert_eq!(get_provider_key(&store, ProviderId::Ollama), None);
    }

    #[test]
    fn provider_key_uses_account_name() {
        let store = MemorySecretStore::default();
        set_provider_key(&store, ProviderId::Anthropic, "sk-ant-1").unwrap();
        assert_eq!(store.get("anthropic-api-key").as_deref(), Some("sk-ant-1"));
    }
}
