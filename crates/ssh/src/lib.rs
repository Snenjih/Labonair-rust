//! UI-free SSH capability contracts.
//!
//! Transport implementations live in `labonair-backend`. Feature crates use
//! these focused contracts instead of importing russh or backend state. The
//! traits are split by responsibility so a consumer that only needs PTY I/O
//! does not depend on host configuration or tunnel management.

use labonair_errors::LabonairError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SshSessionId(String);

impl SshSessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for SshSessionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<SshSessionId> for String {
    fn from(value: SshSessionId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshSessionEvent {
    Output { data: String },
}

pub trait SshEventSink: Send + Sync {
    fn send(&self, event: SshSessionEvent) -> Result<(), String>;
}

pub type SharedSshEventSink = Arc<dyn SshEventSink>;

#[derive(Debug, Clone)]
pub struct SshConnectRequest {
    pub session_id: SshSessionId,
    pub host_id: String,
    pub passphrase: Option<String>,
    pub password: Option<String>,
    pub initial_cols: Option<u32>,
    pub initial_rows: Option<u32>,
    pub blocks: bool,
    pub connect_timeout_secs: Option<u64>,
}

pub trait SshConnectionService: Send + Sync {
    fn connect<'a>(
        &'a self,
        request: SshConnectRequest,
        events: SharedSshEventSink,
    ) -> BoxFuture<'a, Result<(), LabonairError>>;

    fn disconnect<'a>(&'a self, session_id: SshSessionId) -> BoxFuture<'a, Result<(), String>>;

    fn trust_host<'a>(
        &'a self,
        session_id: SshSessionId,
        accepted: bool,
    ) -> BoxFuture<'a, Result<(), String>>;
}

pub trait SshPtyService: Send + Sync {
    fn write<'a>(
        &'a self,
        session_id: SshSessionId,
        data: String,
    ) -> BoxFuture<'a, Result<(), String>>;

    fn resize<'a>(
        &'a self,
        session_id: SshSessionId,
        cols: u32,
        rows: u32,
    ) -> BoxFuture<'a, Result<(), String>>;
}

/// Typed boundary for remote commands that are not SFTP protocol operations.
/// `chown`, size calculation, search, and checksums stay separate from the
/// SFTP browser contract even though they are surfaced by remote-file UI.
pub trait SshRemoteCommandService: Send + Sync {
    fn chown<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
        owner: String,
        group: String,
    ) -> BoxFuture<'a, Result<(), String>>;

    fn calculate_size<'a>(
        &'a self,
        session_id: SshSessionId,
        path: String,
    ) -> BoxFuture<'a, Result<String, String>>;
}

/// Transitional remote-file lifecycle contract used by the editor bridge.
/// The implementation owns the temporary-file mechanics; the workspace only
/// receives paths and coordinates editor tabs. This keeps backend transport
/// modules out of the workspace until streaming file I/O is introduced.
pub trait SshRemoteFileService: Send + Sync {
    fn prepare_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        max_bytes: Option<u64>,
    ) -> BoxFuture<'a, Result<String, String>>;

    fn save_remote_edit<'a>(
        &'a self,
        session_id: SshSessionId,
        remote_path: String,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>>;

    fn cleanup_remote_edit_temp<'a>(
        &'a self,
        local_temp_path: String,
    ) -> BoxFuture<'a, Result<(), String>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshTestResult {
    Success,
    UnknownHostKey { fingerprint: String },
    HostKeyChanged { fingerprint: String },
}

pub trait SshConnectionTester: Send + Sync {
    fn test<'a>(
        &'a self,
        host_id: String,
        passphrase: Option<String>,
        password_override: Option<String>,
        connect_timeout_secs: Option<u64>,
    ) -> BoxFuture<'a, Result<SshTestResult, LabonairError>>;
}

pub trait SshTunnelService: Send + Sync {
    fn start<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>>;
    fn stop<'a>(&'a self, host_id: String) -> BoxFuture<'a, Result<(), String>>;
    fn active(&self) -> Vec<ActiveTunnel>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshConfigEntry {
    pub alias: String,
    pub host_address: String,
    pub port: i64,
    pub username: Option<String>,
    pub auth_method: String,
    pub private_key_path: Option<String>,
    pub proxy_jump: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportConflict {
    #[default]
    Skip,
    Overwrite,
    Rename,
}

pub trait SshConfigService: Send + Sync {
    fn parse<'a>(&'a self) -> BoxFuture<'a, Result<Vec<SshConfigEntry>, String>>;
    fn import<'a>(
        &'a self,
        entries: Vec<SshConfigEntry>,
        conflict: ImportConflict,
    ) -> BoxFuture<'a, Result<Vec<String>, String>>;
    fn export<'a>(&'a self, host_ids: Vec<String>) -> BoxFuture<'a, Result<String, String>>;
    fn write_export<'a>(
        &'a self,
        block: String,
        append: bool,
    ) -> BoxFuture<'a, Result<String, String>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub id: String,
    pub tunnel_type: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTunnel {
    pub host_id: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_opaque_and_round_trip() {
        let id = SshSessionId::new("tab-1");
        assert_eq!(id.as_str(), "tab-1");
        assert_eq!(String::from(id), "tab-1");
    }

    #[test]
    fn config_conflict_defaults_to_skip() {
        assert_eq!(ImportConflict::default(), ImportConflict::Skip);
    }
}
