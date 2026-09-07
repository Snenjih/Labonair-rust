//! Compatibility adapter for the standalone `labonair-secrets` service.
//!
//! Secret storage is no longer implemented by the backend. These wrappers
//! preserve the current backend API while host, SSH, and MCP callers migrate
//! to the capability-owned service directly.

pub use labonair_secrets::SecretsState;

pub async fn secrets_get(
    state: &SecretsState,
    service: String,
    account: String,
) -> Result<Option<String>, String> {
    labonair_secrets::secrets_get(state, service, account).await
}

pub async fn secrets_set(
    state: &SecretsState,
    service: String,
    account: String,
    password: String,
) -> Result<(), String> {
    labonair_secrets::secrets_set(state, service, account, password).await
}

pub async fn secrets_delete(
    state: &SecretsState,
    service: String,
    account: String,
) -> Result<(), String> {
    labonair_secrets::secrets_delete(state, service, account).await
}

pub async fn secrets_get_all(
    state: &SecretsState,
    service: String,
    accounts: Vec<String>,
) -> Result<Vec<Option<String>>, String> {
    labonair_secrets::secrets_get_all(state, service, accounts).await
}

pub async fn secrets_get_encryption_enabled(state: &SecretsState) -> Result<bool, String> {
    labonair_secrets::secrets_get_encryption_enabled(state).await
}

pub async fn secrets_set_encryption_enabled(
    state: &SecretsState,
    enabled: bool,
) -> Result<(), String> {
    labonair_secrets::secrets_set_encryption_enabled(state, enabled).await
}

#[allow(dead_code)]
pub(crate) fn migrate_service_names(state: &SecretsState) {
    labonair_secrets::migrate_service_names(state);
}

pub(crate) fn store_password(
    state: &SecretsState,
    service: &str,
    account: &str,
    password: &str,
) -> Result<(), String> {
    labonair_secrets::store_password(state, service, account, password)
}

pub(crate) fn get_password(
    state: &SecretsState,
    service: &str,
    account: &str,
) -> Result<Option<String>, String> {
    labonair_secrets::get_password(state, service, account)
}
