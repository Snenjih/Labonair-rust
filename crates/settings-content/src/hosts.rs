//! Migration-only host wire types.
//!
//! Hosts are owned and persisted by `labonair-hosts`. These types remain
//! available only because the legacy settings migrator still needs to decode
//! old host-shaped data while it moves users to the canonical host store.
//! They are deliberately not fields of [`crate::SettingsContent`] and must
//! not be used for runtime host management or settings persistence.

use serde::{Deserialize, Serialize};

/// The non-secret authentication method encoded by legacy host data.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum HostAuthMethod {
    #[default]
    Password,
    PublicKey,
    Agent,
}

/// One SSH local/remote port forward encoded by legacy host data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostTunnel {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

/// Non-secret host fields decoded by the legacy settings migrator.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HostEntry {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub user: String,
    pub auth_method: HostAuthMethod,
    pub jump_host_ref: Option<String>,
    pub tunnels: Vec<HostTunnel>,
    pub last_connected_at: Option<i64>,
    pub group: Option<String>,
    pub tags: Vec<String>,
    /// Opaque reference into the OS keychain; never a credential itself.
    pub credential_ref: Option<String>,
}
