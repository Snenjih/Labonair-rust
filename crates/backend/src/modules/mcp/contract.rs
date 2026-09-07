//! Backend adapter for the UI-free MCP session-access contract.

use labonair_mcp_core::{
    BoxFuture, McpEvent, McpEventReceiver, McpEventSource, McpSessionAccessService,
    McpTabOperationService, SessionGrantRequest, TabOpResult,
};
use serde::Deserialize;

use super::{mcp_set_session_grant, mcp_tab_op_response};

/// Bridges aggregate backend MCP state to the narrow service consumed by the
/// workspace's agent-access mirror. Construction stays in the shell; the
/// workspace never receives the backend application just to grant a tab.
#[derive(Clone)]
pub struct BackendMcpSessionAccess {
    state: super::McpState,
    hosts_db: labonair_persistence::Database,
}

impl BackendMcpSessionAccess {
    pub fn new(state: super::McpState, hosts_db: labonair_persistence::Database) -> Self {
        Self { state, hosts_db }
    }
}

impl McpSessionAccessService for BackendMcpSessionAccess {
    fn set_session_grant(&self, request: SessionGrantRequest) -> BoxFuture<'_, Result<(), String>> {
        let state = self.state.clone();
        let hosts_db = self.hosts_db.clone();
        Box::pin(async move {
            mcp_set_session_grant(
                request.tab_id,
                request.session_id,
                request.granted,
                request.label,
                request.kind,
                request.local_pty_id,
                request.host_id,
                &hosts_db,
                &state,
            )
            .await
        })
    }
}

impl McpTabOperationService for BackendMcpSessionAccess {
    fn respond_tab_operation(
        &self,
        request_id: String,
        result: TabOpResult,
    ) -> BoxFuture<'_, Result<(), String>> {
        let state = self.state.clone();
        Box::pin(async move { mcp_tab_op_response(request_id, result, &state).await })
    }
}

/// Shell-composed adapter that translates legacy backend events into the
/// narrow MCP event contract consumed by Workspace.
#[derive(Clone)]
pub struct BackendMcpEventSource {
    events: crate::EventBus,
}

impl BackendMcpEventSource {
    pub fn new(events: crate::EventBus) -> Self {
        Self { events }
    }
}

struct BackendMcpEventReceiver {
    receiver: tokio::sync::broadcast::Receiver<crate::RawEvent>,
}

#[derive(Deserialize)]
struct OpenTabPayload {
    request_id: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    host_id: Option<String>,
}

#[derive(Deserialize)]
struct CloseTabPayload {
    request_id: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Deserialize)]
struct TabIdPayload {
    tab_id: String,
}

#[derive(Deserialize)]
struct ServerErrorPayload {
    message: String,
}

#[derive(Deserialize)]
struct ActivityPayload {
    label: String,
    action: String,
    detail: String,
}

fn decode_mcp_event(raw: &crate::RawEvent) -> Option<McpEvent> {
    match raw.name.as_str() {
        "mcp_open_tab_request" => serde_json::from_value::<OpenTabPayload>(raw.payload.clone())
            .ok()
            .map(|payload| McpEvent::OpenTabRequest {
                request_id: payload.request_id,
                path: payload.path,
                host_id: payload.host_id,
            }),
        "mcp_close_tab_request" => serde_json::from_value::<CloseTabPayload>(raw.payload.clone())
            .ok()
            .map(|payload| McpEvent::CloseTabRequest {
                request_id: payload.request_id,
                session_id: payload.session_id,
            }),
        "mcp_grant_expired" => serde_json::from_value::<TabIdPayload>(raw.payload.clone())
            .ok()
            .map(|payload| McpEvent::GrantExpired {
                tab_id: payload.tab_id,
            }),
        "mcp_server_error" => serde_json::from_value::<ServerErrorPayload>(raw.payload.clone())
            .ok()
            .map(|payload| McpEvent::ServerError {
                message: payload.message,
            }),
        "mcp_activity" => serde_json::from_value::<ActivityPayload>(raw.payload.clone())
            .ok()
            .map(|payload| McpEvent::Activity {
                label: payload.label,
                action: payload.action,
                detail: payload.detail,
            }),
        _ => None,
    }
}

impl McpEventReceiver for BackendMcpEventReceiver {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Option<McpEvent>> {
        Box::pin(async move {
            loop {
                match self.receiver.recv().await {
                    Ok(raw) => {
                        let event = decode_mcp_event(&raw);
                        if event.is_some() {
                            return event;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        log::warn!("MCP event source lagged; resyncing ({skipped} events)");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                }
            }
        })
    }
}

impl McpEventSource for BackendMcpEventSource {
    fn subscribe(&self) -> Box<dyn McpEventReceiver> {
        Box::new(BackendMcpEventReceiver {
            receiver: self.events.subscribe(),
        })
    }
}
