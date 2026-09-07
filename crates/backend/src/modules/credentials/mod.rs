//! Compatibility adapters for the standalone `labonair-credentials` module.
//!
//! Credential metadata, secret storage, and keypair generation live in the
//! capability crate. These wrappers preserve the old App-based API while
//! callers migrate to the typed capability boundary.

use crate::modules::secrets::SecretsState;
use labonair_persistence::Database;

pub use labonair_credentials::{Credential, GenerateKeypairResult, HostRef};

pub async fn credentials_get_all(db: &Database) -> Result<Vec<Credential>, String> {
    labonair_credentials::credentials_get_all(db).await
}

#[allow(clippy::too_many_arguments)]
pub async fn credentials_create(
    _app: crate::App,
    db: &Database,
    secrets: &SecretsState,
    name: String,
    cred_type: String,
    key_path: Option<String>,
    key_type: Option<String>,
    public_key: Option<String>,
    secret: Option<String>,
) -> Result<Credential, String> {
    labonair_credentials::credentials_create(
        db, secrets, name, cred_type, key_path, key_type, public_key, secret,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn credentials_update(
    _app: crate::App,
    db: &Database,
    secrets: &SecretsState,
    id: String,
    name: Option<String>,
    cred_type: Option<String>,
    key_path: Option<String>,
    key_type: Option<String>,
    public_key: Option<String>,
    secret: Option<String>,
) -> Result<Credential, String> {
    labonair_credentials::credentials_update(
        db, secrets, id, name, cred_type, key_path, key_type, public_key, secret,
    )
    .await
}

pub async fn credentials_delete(
    _app: crate::App,
    db: &Database,
    secrets: &SecretsState,
    id: String,
) -> Result<(), String> {
    let data_dir = labonair_filesystem::paths::data_dir();
    labonair_credentials::credentials_delete(db, secrets, &data_dir, id).await
}

pub async fn credentials_get_hosts_using(
    db: &Database,
    id: String,
) -> Result<Vec<HostRef>, String> {
    labonair_credentials::credentials_get_hosts_using(db, id).await
}

pub async fn credential_generate_keypair(
    _app: crate::App,
    db: &Database,
    secrets: &SecretsState,
    cred_id: String,
    key_type: String,
    passphrase: Option<String>,
) -> Result<GenerateKeypairResult, String> {
    let data_dir = labonair_filesystem::paths::data_dir();
    labonair_credentials::credential_generate_keypair(
        &data_dir, db, secrets, cred_id, key_type, passphrase,
    )
    .await
}
