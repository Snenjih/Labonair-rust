//! `hosts` area — a **top-level** custom category (`docs/architecture.md`
//! §8.1/§8.3), peer of `themes`, not nested under `connections`.
//!
//! Non-secret host metadata only. Credentials (passwords / private keys)
//! never round-trip through `SettingsContent` / `settings.json` — a host
//! entry carries only `credential_ref`, an opaque reference into the OS
//! keychain (`labonair-backend::modules::secrets`). This mirrors, but does
//! not replace, `labonair-backend::modules::hosts::db::Host` (the SQLite
//! row) — that stays the authoritative runtime store; T19-010 is where the
//! Settings › Hosts page decides how the two are reconciled.

use serde::{Deserialize, Serialize};

use crate::MergeFrom;

/// How a host authenticates. Mirrors the non-secret half of
/// `labonair-backend::modules::hosts::db::Host::auth_method` (a free string
/// there; typed here since this is a fresh model).
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

impl MergeFrom for HostAuthMethod {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// One SSH local/remote port forward attached to a host.
#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct HostTunnel {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

/// A single saved host — non-secret fields only.
#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct HostEntry {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub user: String,
    pub auth_method: HostAuthMethod,
    /// `id` of another `HostEntry` used as an SSH jump host, if any.
    pub jump_host_ref: Option<String>,
    pub tunnels: Vec<HostTunnel>,
    pub last_connected_at: Option<i64>,
    pub group: Option<String>,
    pub tags: Vec<String>,
    /// Opaque reference into the OS keychain. Never a password/key itself —
    /// see the module doc comment.
    pub credential_ref: Option<String>,
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct KeepaliveSettings {
    pub interval_secs: Option<u32>,
    pub max_missed: Option<u32>,
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct SshConfigImportSettings {
    /// Import hosts from `~/.ssh/config` on startup.
    pub auto_import: Option<bool>,
    /// Path to the SSH config file to read (empty = `~/.ssh/config`).
    pub config_path: Option<String>,
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct HostsContent {
    pub entries: Option<Vec<HostEntry>>,
    /// Shell command used for new SSH sessions when a host doesn't specify
    /// one (empty = remote login shell default).
    pub default_shell: Option<String>,
    pub keepalive: Option<KeepaliveSettings>,
    pub ssh_config_import: Option<SshConfigImportSettings>,
    /// Host-Manager card/list UI (formerly `Preferences::hm_layout`).
    /// `"grid"` | `"list"`.
    pub layout: Option<String>,
    /// `"last_connected"` | `"name"` | `"manual"`.
    pub sort: Option<String>,
    pub card_scale: Option<u32>,
}

impl HostsContent {
    pub fn defaults() -> Self {
        Self {
            entries: Some(Vec::new()),
            default_shell: Some(String::new()),
            keepalive: Some(KeepaliveSettings {
                interval_secs: Some(30),
                max_missed: Some(3),
            }),
            ssh_config_import: Some(SshConfigImportSettings {
                auto_import: Some(false),
                config_path: Some(String::new()),
            }),
            layout: Some("grid".to_string()),
            sort: Some("last_connected".to_string()),
            card_scale: Some(100),
        }
    }
}
