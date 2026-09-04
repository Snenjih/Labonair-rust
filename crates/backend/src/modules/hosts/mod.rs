pub mod db;

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
    /// Prevents this host's SSH tabs from ever being granted or used via the
    /// AI Agent Bridge (`modules::mcp`), even if the bridge is enabled
    /// elsewhere — checked both at grant time and live at every tool
    /// execution (see `mcp::host_blocks_agent_access`).
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

pub struct HostsDb(pub std::sync::Mutex<rusqlite::Connection>);

/// The opaque `credential_ref` a `hosts.entries` (`SettingsContent`) row
/// carries when the host has a secret stored under the `"labonair-app"`
/// service in `backend::modules::secrets` (the scheme `db.rs`'s
/// `hosts_create`/`hosts_update` already use, and
/// `settings::migrate_v2::credential_ref_for` reads back) — `None` if no
/// such secret exists. Public wrapper (T19-010) so `labonair-hosts-ui`'s
/// `apply_host_change` can project the *reference* into `settings.json`
/// without ever touching the secret itself.
pub fn credential_ref(app: &crate::App, id: &str) -> Option<String> {
    crate::modules::secrets::get_password(app, &app.secrets, "labonair-app", id)
        .ok()
        .flatten()
        .map(|_| format!("secrets:labonair-app:{id}"))
}
