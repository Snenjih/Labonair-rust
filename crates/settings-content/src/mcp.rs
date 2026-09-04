//! `mcp` area — the AI Agent Bridge, today's `"mcp"` settings-file key
//! (`labonair-backend::modules::settings::mcp::McpPrefs`).

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct McpContent {
    /// Whether the bridge listener should be started at launch.
    pub bridge_enabled: Option<bool>,
    /// Local port the Streamable-HTTP listener binds to (1024–65535).
    pub bridge_port: Option<u32>,
    /// Upper bound on a single agent-run command before it returns
    /// `still_running`.
    pub max_command_timeout_secs: Option<u32>,
    /// Revoke a granted tab after this many minutes without agent activity.
    /// `0` disables auto-revoke.
    pub auto_revoke_minutes: Option<u32>,
    pub notify_on_activity: Option<bool>,
}

impl McpContent {
    pub fn defaults() -> Self {
        Self {
            bridge_enabled: Some(false),
            bridge_port: Some(47823),
            max_command_timeout_secs: Some(300),
            auto_revoke_minutes: Some(0),
            notify_on_activity: Some(false),
        }
    }
}
