//! Domain models for saved hosts and host groups.
//!
//! Persistence, transport, and UI are intentionally outside this crate. This
//! keeps the host contract reusable by host management, SSH, SFTP, snippets,
//! and command-palette providers without importing application composition.

pub mod command_provider;
pub mod store;

/// The transport requested when a saved host is opened from a Hosts surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOpenMode {
    Ssh,
    Sftp,
}

/// Typed request emitted by Hosts-owned UI and consumed by the workspace
/// composition boundary. Hosts never opens a transport implementation itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostOpenRequest {
    pub host_id: String,
    pub mode: HostOpenMode,
}

impl HostOpenRequest {
    pub fn ssh(host_id: impl Into<String>) -> Self {
        Self {
            host_id: host_id.into(),
            mode: HostOpenMode::Ssh,
        }
    }

    pub fn sftp(host_id: impl Into<String>) -> Self {
        Self {
            host_id: host_id.into(),
            mode: HostOpenMode::Sftp,
        }
    }
}

/// Immutable row data shared by the Hosts manager and the command palette.
/// The palette does not reconstruct labels from the persistence model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPickerRow {
    pub id: String,
    pub name: String,
    pub subtitle: String,
}

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

impl Host {
    /// Build the canonical label shown by host pickers.
    pub fn picker_row(&self) -> HostPickerRow {
        let identity = match (self.username.trim(), self.host_address.trim()) {
            (user, address) if !user.is_empty() && !address.is_empty() => {
                format!("{user}@{address}:{}", self.port)
            }
            (_, address) if !address.is_empty() => format!("{address}:{}", self.port),
            _ => format!("port {}", self.port),
        };
        HostPickerRow {
            id: self.id.clone(),
            name: if self.name.trim().is_empty() {
                self.host_address.clone()
            } else {
                self.name.clone()
            },
            subtitle: identity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(name: &str, address: &str, username: &str, port: i64) -> Host {
        Host {
            id: "host-1".to_string(),
            name: name.to_string(),
            host_address: address.to_string(),
            port,
            username: username.to_string(),
            auth_method: "password".to_string(),
            private_key_path: None,
            group_id: None,
            tags: None,
            created_at: 0,
            last_connected_at: None,
            default_path_ssh: None,
            default_path_sftp: None,
            pin_to_top: false,
            sudo_password_set: false,
            keep_alive_interval: None,
            keep_alive_tries: None,
            sort_order: 0,
            tunnels: None,
            startup_snippet_id: None,
            startup_snippet_mode: None,
            credential_id: None,
            jump_host_id: None,
            notes: None,
            icon: None,
            block_agent_access: false,
        }
    }

    #[test]
    fn picker_row_uses_stable_display_identity_and_fallbacks() {
        assert_eq!(
            host("Production", "prod.example.com", "deploy", 2222).picker_row(),
            HostPickerRow {
                id: "host-1".to_string(),
                name: "Production".to_string(),
                subtitle: "deploy@prod.example.com:2222".to_string(),
            }
        );
        assert_eq!(
            host("", "192.0.2.10", "", 22).picker_row().name,
            "192.0.2.10"
        );
    }

    #[test]
    fn open_request_preserves_transport_intent() {
        assert_eq!(
            HostOpenRequest::ssh("prod"),
            HostOpenRequest {
                host_id: "prod".to_string(),
                mode: HostOpenMode::Ssh,
            }
        );
        assert_eq!(
            HostOpenRequest::sftp("prod"),
            HostOpenRequest {
                host_id: "prod".to_string(),
                mode: HostOpenMode::Sftp,
            }
        );
    }
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
