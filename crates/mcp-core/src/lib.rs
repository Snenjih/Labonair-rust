//! UI-free contracts shared by the MCP bridge and its terminal/workspace hosts.
//!
//! The bridge implementation may live behind a platform adapter, but these
//! values describe the stable boundary between an agent-access UI and the MCP
//! service. They must not depend on the aggregate backend application state.

use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Receives raw output from one local terminal session without exposing the
/// terminal implementation to the MCP bridge.
pub trait LocalTerminalOutputReceiver: Send {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Result<Vec<u8>, String>>;
}

/// Narrow local-terminal capability used by MCP tools. The shell composes an
/// adapter from the application's terminal registry; the MCP crate does not
/// own or discover terminal sessions.
pub trait LocalTerminalAccess: Send + Sync {
    fn write(&self, session_id: &str, data: String) -> Result<(), String>;
    fn subscribe(&self, session_id: &str) -> Result<Box<dyn LocalTerminalOutputReceiver>, String>;
}

pub mod preferences;

/// Which terminal target a grant addresses.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    #[default]
    Ssh,
    Local,
}

/// A completed response to an MCP open-tab or close-tab request.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct TabOpResult {
    pub ok: bool,
    pub session_id: Option<String>,
    pub tab_id: Option<String>,
    pub error: Option<String>,
}

/// Request used by a workspace to grant or revoke one tab's agent access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionGrantRequest {
    pub tab_id: String,
    pub session_id: String,
    pub granted: bool,
    pub label: String,
    pub kind: SessionKind,
    pub host_id: Option<String>,
}

/// Narrow MCP capability consumed by the workspace's local grant mirror.
pub trait McpSessionAccessService: Send + Sync {
    fn set_session_grant(&self, request: SessionGrantRequest) -> BoxFuture<'_, Result<(), String>>;
}

/// Narrow MCP capability used to complete an agent-requested tab operation.
pub trait McpTabOperationService: Send + Sync {
    fn respond_tab_operation(
        &self,
        request_id: String,
        result: TabOpResult,
    ) -> BoxFuture<'_, Result<(), String>>;
}

/// Events emitted by the MCP bridge and consumed by the application UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpEvent {
    OpenTabRequest {
        request_id: String,
        path: Option<String>,
        host_id: Option<String>,
    },
    CloseTabRequest {
        request_id: String,
        session_id: Option<String>,
    },
    GrantExpired {
        tab_id: String,
    },
    ServerError {
        message: String,
    },
    Activity {
        label: String,
        action: String,
        detail: String,
    },
}

/// Asynchronous receiver for typed MCP bridge events.
pub trait McpEventReceiver: Send {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Option<McpEvent>>;
}

/// Source of MCP bridge events. Implementations are composed at the shell
/// boundary; consumers never subscribe to aggregate application state.
pub trait McpEventSource: Send + Sync {
    fn subscribe(&self) -> Box<dyn McpEventReceiver>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_kind_uses_stable_wire_names() {
        assert_eq!(serde_json::to_string(&SessionKind::Ssh).unwrap(), "\"ssh\"");
        assert_eq!(
            serde_json::to_string(&SessionKind::Local).unwrap(),
            "\"local\""
        );
    }

    #[test]
    fn tab_operation_result_defaults_to_pending_failure_shape() {
        let result = TabOpResult::default();
        assert!(!result.ok);
        assert!(result.error.is_none());
    }
}
