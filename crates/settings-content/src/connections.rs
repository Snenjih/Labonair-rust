//! `connections` area — SSH / Explorer / host-availability polling knobs.
//! The host **entries** themselves live in [`crate::hosts`], not here (see
//! T19-001's Anweisungen #2).

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct ConnectionsContent {
    pub host_ping_interval: Option<u32>,
    pub ssh_connect_timeout_secs: Option<u32>,
    pub ssh_auto_reconnect: Option<bool>,
    pub ssh_auto_reconnect_delay: Option<u32>,
    pub ssh_auto_reconnect_max_attempts: Option<u32>,
    pub explorer_remote_poll_interval: Option<u32>,
    pub explorer_auto_reconnect: Option<bool>,
    pub explorer_idle_session_timeout_min: Option<u32>,
    pub explorer_max_idle_sessions: Option<u32>,
    pub explorer_max_cached_remote_scopes: Option<u32>,
}

impl ConnectionsContent {
    pub fn defaults() -> Self {
        Self {
            host_ping_interval: Some(60),
            ssh_connect_timeout_secs: Some(10),
            ssh_auto_reconnect: Some(false),
            ssh_auto_reconnect_delay: Some(5),
            ssh_auto_reconnect_max_attempts: Some(3),
            explorer_remote_poll_interval: Some(20),
            explorer_auto_reconnect: Some(false),
            explorer_idle_session_timeout_min: Some(5),
            explorer_max_idle_sessions: Some(3),
            explorer_max_cached_remote_scopes: Some(5),
        }
    }
}
