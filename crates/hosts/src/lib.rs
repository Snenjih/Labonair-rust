//! Domain models for saved hosts and host groups.
//!
//! Persistence, transport, and UI are intentionally outside this crate. This
//! keeps the host contract reusable by host management, SSH, SFTP, snippets,
//! and command-palette providers without importing the application backend.

pub mod store;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub host_address: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub private_key_path: Option<String>,
    pub group_id: Option<String>,
    pub tags: Option<String>,
    pub created_at: i64,
    pub last_connected_at: Option<i64>,
    pub default_path_ssh: Option<String>,
    pub default_path_sftp: Option<String>,
    pub pin_to_top: bool,
    pub sudo_password_set: bool,
    pub keep_alive_interval: Option<i64>,
    pub keep_alive_tries: Option<i64>,
    pub sort_order: i64,
    pub tunnels: Option<String>,
    pub startup_snippet_id: Option<String>,
    pub startup_snippet_mode: Option<String>,
    pub credential_id: Option<String>,
    pub jump_host_id: Option<String>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    /// Prevents this host's sessions from being used by an AI integration.
    pub block_agent_access: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderItem {
    pub id: String,
    pub sort_order: i64,
}
