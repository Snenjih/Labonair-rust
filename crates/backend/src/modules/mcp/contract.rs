//! Backend adapter for the UI-free MCP session-access contract.

use labonair_mcp_core::{
    BoxFuture, McpEvent, McpEventReceiver, McpEventSource, McpSessionAccessService,
    McpTabOperationService, SessionGrantRequest, TabOpResult,
};

use super::{mcp_set_session_grant, mcp_tab_op_response};

/// Bridges aggregate backend MCP state to the narrow service consumed by the
/// workspace's agent-access mirror. Construction stays in the shell; the
/// workspace never receives the backend application just to grant a tab.
#[derive(Clone)]
pub struct BackendMcpSessionAccess {
    app: crate::App,
}

impl BackendMcpSessionAccess {
    pub fn new(app: crate::App) -> Self {
        Self { app }
    }
}

impl McpSessionAccessService for BackendMcpSessionAccess {
    fn set_session_grant(&self, request: SessionGrantRequest) -> BoxFuture<'_, Result<(), String>> {
        let app = self.app.clone();
        Box::pin(async move {
            mcp_set_session_grant(
                request.tab_id,
                request.session_id,
                request.granted,
                request.label,
                request.kind,
                request.local_pty_id,
                request.host_id,
                app.clone(),
                &app.mcp,
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
        let app = self.app.clone();
        Box::pin(async move { mcp_tab_op_response(request_id, result, &app.mcp).await })
    }
}

/// Shell-composed adapter that translates legacy backend events into the
/// narrow MCP event contract consumed by Workspace.
#[derive(Clone)]
pub struct BackendMcpEventSource {
    app: crate::App,
}

impl BackendMcpEventSource {
    pub fn new(app: crate::App) -> Self {
        Self { app }
    }
}

struct BackendMcpEventReceiver {
    receiver: tokio::sync::broadcast::Receiver<crate::RawEvent>,
}

impl McpEventReceiver for BackendMcpEventReceiver {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Option<McpEvent>> {
        Box::pin(async move {
            loop {
                match self.receiver.recv().await {
                    Ok(raw) => {
                        let event = crate::AppEvent::from_raw(&raw).and_then(|event| match event {
                            crate::AppEvent::McpOpenTabRequest {
                                request_id,
                                path,
                                host_id,
                            } => Some(McpEvent::OpenTabRequest {
                                request_id,
                                path,
                                host_id,
                            }),
                            crate::AppEvent::McpCloseTabRequest {
                                request_id,
                                session_id,
                            } => Some(McpEvent::CloseTabRequest {
                                request_id,
                                session_id,
                            }),
                            crate::AppEvent::McpGrantExpired { tab_id } => {
                                Some(McpEvent::GrantExpired { tab_id })
                            }
                            crate::AppEvent::McpServerError { message } => {
                                Some(McpEvent::ServerError { message })
                            }
                            crate::AppEvent::McpActivity {
                                label,
                                action,
                                detail,
                            } => Some(McpEvent::Activity {
                                label,
                                action,
                                detail,
                            }),
                            _ => None,
                        });
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
            receiver: self.app.events.subscribe(),
        })
    }
}
